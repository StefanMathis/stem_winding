use std::num::{NonZeroU16, NonZeroUsize};

use compare_variables::compare_variables;

use dyn_clone::clone_box;
use num::rational::Ratio;

use stem_coil_layout::{CoilLayout, Zone};
use stem_wire::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    coils::{Coil, CoilFull, Coils},
    error::{Error, WindingTableCreationError},
    winding::{Connection, Winding, hole_number, periodicity},
    winding_table::{WindingTable, WindingTableMethod},
};

/**
Tries to create a new instange of an `DistributedWinding`. Fails if the combination of input parameters does not lead to a legal winding.

Arguments:
 - `concentric_coils`: If `true`, the coils of a coil group are constructed from concentric coils, otherwise each coil of a group has the same span.

Example concentric coils:
```ignore
┌─────────────────┐
│ ┌────────────┐  │
1 1 -3 -3 2 2 -1 -1
│ └────────────┘  │
└─────────────────┘
```

Example coils with same span:
```ignore
┌──────────────┐
│ ┌────────────│──┐
1 1 -3 -3 2 2 -1 -1
│ └────────────│──┘
└──────────────┘
```
 */
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct DistributedWinding {
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    phases: NonZeroU16,
    layers: NonZeroU16,
    coil_span_reduction: i32,
    zone_span_variation: u16,
    turns_per_coil: NonZeroUsize,
    parallel_paths: NonZeroU16,
    connection: Connection,
    end_winding_leakage_coefficient: f64,
    wire: Box<dyn Wire>,
    #[cfg_attr(feature = "serde", serde(skip))]
    coils: Coils,
    winding_table_method: WindingTableMethod,
    concentric_coils: bool,
}

impl DistributedWinding {
    pub fn new<W>(builder: W) -> Result<Self, Error>
    where
        W: TryInto<DistributedWinding>,
        W::Error: Into<Error>,
    {
        builder.try_into().map_err(Into::into)
    }
    /**
    Return the distribution and the pitch factor as `[distribution, pitch]`.
    The product of those two values equals the winding factor calculated from `self.winding_factor()`
     */
    pub fn distribution_and_pitch_factor(&self, phase: NonZeroU16, ordinal: f64) -> [f64; 2] {
        // If layers==1, the distribution factor equals the winding factor,
        // meaning that the pitch factor is 1 by default. If layers == 2,
        // distribution and pitch factor are calculated by temporarily changing the
        // pitch to zero.
        let coil_span_reduction = self.coil_span_reduction();
        if self.layers().get() == 1 || coil_span_reduction == 0 {
            return [self.winding_factor(phase, ordinal), 1.0];
        } else {
            // Create a copy of this winding, but without the short pitch. We use
            // the DistributionTable here, since it can deal with all slot / pole
            // combinations as long as there are one or two layers (which is
            // always the case for a DistributedWinding).
            let temp_winding: DistributedWinding = DistributedMinimalBuilder {
                slots: self.slots(),
                pole_pairs: self.pole_pairs(),
                phases: self.phases(),
                layers: self.layers(),
                coil_span_reduction: 0,
                zone_span_variation: self.zone_span_variation,
                winding_table_method: self.winding_table_method,
            }
            .try_into()
            .expect("all input parameters result in a valid winding, since self is valid as well");

            // Calculate winding factor with and without chording
            let k_d = temp_winding.winding_factor(phase, ordinal); // The factor without chording equals the distribution factor
            let k_w_chorded = self.winding_factor(phase, ordinal);

            // The pitch factor is calculated as the quotient of factor_w_chording and
            // factor_wo_chording
            let k_p = k_w_chorded / k_d;
            return [k_d, k_p];
        }
    }

    /// Return the short pitch of the winding, which describes the "shift"
    /// between the zones of the upper and lower layer. A positive value
    /// indicates a short-pitching, while a negative value results in
    /// lengthening the pitch between upper and lower layer.
    pub fn coil_span_reduction(&self) -> i32 {
        self.coil_span_reduction
    }

    pub fn zone_span_variation(&self) -> u16 {
        self.zone_span_variation
    }

    pub fn wire(&self) -> &dyn Wire {
        &*self.wire
    }

    pub fn winding_table_method(&self) -> &WindingTableMethod {
        &self.winding_table_method
    }

    /**
    Returns `true`, if the winding is build with concentric coils.
     */
    pub fn concentric_coils(&self) -> bool {
        return self.concentric_coils;
    }

    /**
    If `concentric_coils` is `true`, the coil layout of `self` is changed to a concentric layout.
    If `concentric_coils` is `false`, the coil layout of `self` is changed to an even span layout.
     */
    pub fn set_concentric_coils(&mut self, concentric_coils: bool) {
        // Do nothing if the layout already equals the desired layout.
        if concentric_coils == self.concentric_coils() {
            return ();
        }

        self.concentric_coils = concentric_coils;

        // Get the zone plan
        let winding_table = self.winding_table(false);

        // Clear all coils
        self.coils.0.clear();

        // Rebuild the coils with the zone plan
        self.create_coils(&winding_table).expect(
            "since self already had a legal coil configuration, rebuilding it must succeed.",
        );
    }

    fn create_coils(&mut self, winding_table: &WindingTable) -> Result<(), Error> {
        // This algorithm creates the coils of self by first grouping the zones
        // into pairs of coil groups and then creating coils going from one pair
        // to the other.
        let return_layer = if self.layers().get() == 1 { 0 } else { 1 };

        // The three phases are handled independently from each other, since they don't
        // interact.
        for phase in 1..(self.phases().get() as i32 + 1) {
            let start_slot = winding_table.start_at_largest_possible_span(phase);
            for slot in start_slot..(self.slots().get() + start_slot) {
                let slot = slot.rem_euclid(self.slots().get());

                /*
                It is sufficient to search on layer 0, since all coils in a
                double-layer winding go from layer 0 to layer 1 (return_layer).
                 */
                let zone = Zone::new(slot, 0);

                // Skip all zones which don't belong to the current phase
                if winding_table.get_cyclic(zone).abs() != phase {
                    continue;
                }

                // Identify the coil group pair
                if let Some(seed_group) = AvailableSlotsForCoilGroup::new(
                    winding_table,
                    &self.coils,
                    zone,
                    self.slots().get(),
                ) {
                    if let Some((partner_group, found_partner_while_ascending)) = seed_group
                        .find_partner(winding_table, &self.coils, self.slots().get(), return_layer)
                    {
                        let (pos, neg, clockwise) = if seed_group.phase > 0 {
                            (seed_group, partner_group, found_partner_while_ascending)
                        } else {
                            (partner_group, seed_group, !found_partner_while_ascending)
                        };
                        if self.concentric_coils {
                            // Connect pos.start to neg.stop
                            for (pos_slot, neg_slot) in (pos.start..(pos.start + pos.len + 1))
                                .into_iter()
                                .zip((neg.start..(neg.start + neg.len + 1)).into_iter().rev())
                            {
                                let positive_zone =
                                    Zone::new(pos_slot.rem_euclid(self.slots().get()), pos.layer);
                                let negative_zone =
                                    Zone::new(neg_slot.rem_euclid(self.slots().get()), neg.layer);
                                let coil = CoilFull::new(
                                    positive_zone,
                                    negative_zone,
                                    true,
                                    clockwise,
                                    self.turns_per_coil,
                                    NonZeroU16::new(pos.phase as u16).expect("cannot be zero"),
                                    clone_box(&*self.wire),
                                )?;
                                self.coils.0.insert_many(
                                    vec![positive_zone, negative_zone],
                                    Coil::Full(coil),
                                )?;
                            }
                        } else {
                            // Connect pos.start to neg.start and pos.stop to neg.stop
                            for (pos_slot, neg_slot) in (pos.start..(pos.start + pos.len + 1))
                                .into_iter()
                                .zip((neg.start..(neg.start + neg.len + 1)).into_iter())
                            {
                                let positive_zone =
                                    Zone::new(pos_slot.rem_euclid(self.slots().get()), pos.layer);
                                let negative_zone =
                                    Zone::new(neg_slot.rem_euclid(self.slots().get()), neg.layer);
                                let coil = CoilFull::new(
                                    positive_zone,
                                    negative_zone,
                                    true,
                                    clockwise,
                                    self.turns_per_coil,
                                    NonZeroU16::new(pos.phase as u16).expect("cannot be zero"),
                                    clone_box(&*self.wire),
                                )?;
                                self.coils.0.insert_many(
                                    vec![positive_zone, negative_zone],
                                    Coil::Full(coil),
                                )?;
                            }
                        }
                    }
                }
            }
        }

        // Check if all zones are filled.
        for (zone, _) in winding_table.iter_slots() {
            // Check if the zone is already occupied
            if !self.coils.0.contains_key(&zone) {
                return Err(crate::error::WindingTableCreationError::EmptyZone(Some(zone)).into());
            }
        }
        return Ok(());
    }
}

/**
A coil group starts in slot `start` and extends for `len` slots.
This extension is wrapping: If e.g. the total number of slots is 12, start is 11 and len is 2,
the coil group is located in the slots 11 and 0.
    */
#[derive(Clone, Debug, PartialEq)]
struct AvailableSlotsForCoilGroup {
    start: u16,
    stop: u16,
    layer: u16,
    len: u16,
    phase: i32,
}

impl AvailableSlotsForCoilGroup {
    /**
    Get the coil group zones of the coil at `zone`.
     */
    fn new(
        winding_table: &WindingTable,
        coils: &Coils,
        zone: Zone,
        slots: u16,
    ) -> Option<AvailableSlotsForCoilGroup> {
        // If the "seed" is already occupied, abort.
        if coils.0.contains_key(&zone) {
            return None;
        }

        let phase = winding_table.get_cyclic(zone);

        // Find the start
        let mut start = zone.slot;
        for slot in (zone.slot..(zone.slot + slots)).rev() {
            let new_zone = Zone::new(slot.rem_euclid(slots), zone.layer);
            if coils.0.contains_key(&new_zone) || winding_table.get_cyclic(new_zone) != phase {
                break;
            } else {
                start = new_zone.slot;
            }
        }

        // Find the length
        let mut stop = zone.slot;
        for slot in (zone.slot + 1)..(zone.slot + slots) {
            let new_zone = Zone::new(slot.rem_euclid(slots), zone.layer);
            if coils.0.contains_key(&new_zone) || winding_table.get_cyclic(new_zone) != phase {
                break;
            } else {
                stop = new_zone.slot;
            }
        }
        let len = (stop + slots - start).rem_euclid(slots);

        return Some(AvailableSlotsForCoilGroup {
            start,
            stop,
            len,
            phase: *phase,
            layer: zone.layer,
        });
    }

    /**
    Try to find a partner for `self` on the specified `layer`. If no partner can be identified, return None.
    If a partner could be identified, also return the info whether the partner has been found while ascending
     */
    fn find_partner(
        &self,
        winding_table: &WindingTable,
        coils: &Coils,
        slots: u16,
        search_layer: u16,
    ) -> Option<(Self, bool)> {
        for delta in 0..slots {
            let slot_asc = (self.stop + delta).rem_euclid(slots);
            let slot_des = (self.start + slots - delta).rem_euclid(slots);
            for (is_des, search_slot) in [slot_asc, slot_des].into_iter().enumerate() {
                let search_zone = Zone::new(search_slot, search_layer);
                if *winding_table.get_cyclic(search_zone) == -self.phase {
                    if let Some(potential_partner) =
                        Self::new(winding_table, coils, search_zone, slots)
                    {
                        if potential_partner.len >= self.len {
                            return Some((potential_partner, is_des == 0));
                        } else {
                            return None;
                        }
                    }
                }
            }
        }
        return None;
    }
}

impl Default for DistributedWinding {
    fn default() -> Self {
        DistributedBuilder {
            slots: NonZeroU16::new(6).expect("not zero"),
            pole_pairs: NonZeroU16::MIN,
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::MIN,
            coil_span_reduction: 0,
            zone_span_variation: 0,
            turns_per_coil: NonZeroUsize::MIN,
            parallel_paths: NonZeroU16::MIN,
            connection: Connection::Star,
            end_winding_leakage_coefficient: 0.0,
            wire: Box::new(RoundWire::default()),
            winding_table_method: WindingTableMethod::Tingley,
            concentric_coils: false,
        }
        .try_into()
        .expect("valid inputs")
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Winding for DistributedWinding {
    fn phases(&self) -> NonZeroU16 {
        return self.phases;
    }

    fn slots(&self) -> NonZeroU16 {
        return self.slots;
    }

    fn pole_pairs(&self) -> NonZeroU16 {
        return self.pole_pairs;
    }

    fn layers(&self) -> NonZeroU16 {
        return self.layers;
    }

    fn periodicity(&self) -> NonZeroU16 {
        return periodicity(
            self.slots(),
            self.pole_pairs(),
            self.phases(),
            self.layers(),
        );
    }

    fn turns_at(&self, _zone: Zone) -> usize {
        return self.turns_per_coil.get();
    }

    fn coil_layout(&self) -> CoilLayout {
        if self.layers().get() == 1 {
            return CoilLayout::Single;
        } else {
            return CoilLayout::DoubleVertical;
        }
    }

    fn turns_per_phase(&self, _phase: NonZeroU16) -> Ratio<usize> {
        return Ratio::new_raw(
            usize::from(self.layers().get() * self.slots().get()) * self.turns_per_coil.get()
                / usize::from(2 * self.phases().get() * self.parallel_paths().get()),
            1,
        );
    }

    fn parallel_paths(&self) -> NonZeroU16 {
        return self.parallel_paths;
    }

    fn connection(&self) -> Connection {
        return self.connection;
    }

    fn end_winding_leakage_coefficient(&self) -> f64 {
        return self.end_winding_leakage_coefficient;
    }

    fn coil_at(&self, zone: Zone) -> Option<&Coil> {
        return self.coils.0.get(&zone);
    }

    fn as_dyn(&self) -> &dyn Winding {
        self
    }

    #[cfg(feature = "stem_core")]
    fn is_symmetric(&self, _core: CoreRef<'_>, _overrides: &Overrides) -> bool {
        return true;
    }

    /// Phase resistance calculation of a distributed winding according to
    /// [Mat19], eq. (3.48).
    #[cfg(feature = "stem_core")]
    fn resistance_components(
        &self,
        core: CoreRef<'_>,
        overrides: &Overrides,
    ) -> Option<crate::ResistanceComponents> {
        let material = self.wire().material_conductor().clone();
        if let Some(resistance_constant) = overrides.resistance_constant {
            return Some(crate::ResistanceComponents {
                resistance_constant,
                material,
            });
        }

        let resistance = self.resistance(1, core, &[], overrides);
        let electrical_resistivity = material.electrical_resistivity().get(&[]);
        return Some(crate::ResistanceComponents {
            resistance_constant: resistance / electrical_resistivity,
            material,
        });
    }

    #[cfg(feature = "stem_core")]
    fn resistance(
        &self,
        phase: u16,
        core: CoreRef<'_>,
        conditions: &[InfluencingQuantity],
        overrides: &Overrides,
    ) -> ElectricalResistance {
        let electrical_resistivity = self
            .wire()
            .material_conductor()
            .electrical_resistivity()
            .get(conditions);

        if let Some(resistance_constant) = overrides.resistance_constant {
            return resistance_constant * electrical_resistivity;
        }

        let mean_coil_turn_length = 2.0
            * (core.axial_coil_length()
                + core.axial_coil_overhang()
                + self
                    .end_winding_half_turn_length(core, Zone::new(0, 0), overrides)
                    .expect("must contain a coil"));

        return self.wire().resistance(
            core.zone_area() / self.turns_in_slot(0) as f64,
            mean_coil_turn_length,
            conditions,
        ) * self.turns_per_phase(phase).to_integer() as f64
            / self.parallel_paths() as f64;
    }

    #[cfg(feature = "stem_core")]
    fn end_winding_leakage_inductance(
        &self,
        phase: u16,
        core: CoreRef<'_>,
        overrides: &Overrides,
    ) -> Inductance {
        if let Some(end_winding_leakage_inductance) = overrides.end_winding_leakage_inductance {
            return end_winding_leakage_inductance;
        }

        return 2.0
            * self.end_winding_leakage_coefficient()
            * *material::VACUUM_PERMEABILITY
            * self.turns_per_phase(phase).to_integer().pow(2) as f64
            * (self
                .end_winding_half_turn_length(core, Zone::new(0, 0), overrides)
                .expect("must contain a coil")
                + core.axial_coil_overhang())
            / self.pole_pairs() as f64;
    }

    #[cfg(feature = "stem_core")]
    fn end_winding_half_turn_length(
        &self,
        core: CoreRef<'_>,
        _zone: Zone,
        overrides: &Overrides,
    ) -> Option<Length> {
        use crate::CoilExt;
        use std::f64::consts::PI;

        if let Some(end_winding_half_turn_length) = overrides.end_winding_half_turn_length {
            return Some(end_winding_half_turn_length);
        }

        // This is an approximation of the end winding calculation based on heuristic
        // values
        let mut span = None;
        for coil in self.coils() {
            span = Some(coil.span(self.slots()));
            break;
        }
        match span {
            Some(_) => {
                let pitch_ratio = (self.pole_pitch() as f64 - self.coil_span_reduction() as f64)
                    / self.pole_pitch() as f64; // W/tau_p
                return Some(
                    PI / (4.0 * self.pole_pairs() as f64)
                        * core.mean_slot_distance()
                        * self.slots() as f64
                        * pitch_ratio,
                );
            }
            None => return None,
        }
    }
}

// =============================================================================
// Builders

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DistributedBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub layers: NonZeroU16,
    pub coil_span_reduction: i32,
    pub zone_span_variation: u16,
    pub winding_table_method: WindingTableMethod,
    pub turns_per_coil: NonZeroUsize,
    pub parallel_paths: NonZeroU16,
    pub connection: Connection,
    pub end_winding_leakage_coefficient: f64,
    pub wire: Box<dyn Wire>,
    pub concentric_coils: bool,
}

impl TryFrom<DistributedBuilder> for DistributedWinding {
    type Error = Error;

    fn try_from(builder: DistributedBuilder) -> Result<Self, Self::Error> {
        let layers = builder.layers.get();
        let slots = builder.slots.get();
        let pole_pairs = builder.pole_pairs.get();

        compare_variables!(layers < 3)?;
        compare_variables!(0.0 <= builder.end_winding_leakage_coefficient)?;

        // Calculate the basic winding parameters
        let t = periodicity(
            builder.slots,
            builder.pole_pairs,
            builder.phases,
            builder.layers,
        );
        let slots_basic = slots / t;
        let pole_pairs_basic = pole_pairs / t;

        let span =
            (slots as f64 / (2.0 * pole_pairs as f64)).floor() as i32 - builder.coil_span_reduction;

        // Create the zone plan by method
        let mut winding_table = WindingTable::with_method(
            &builder.winding_table_method,
            NonZeroU16::new(slots_basic).expect("not zero"),
            builder.layers,
            NonZeroU16::new(pole_pairs_basic).expect("not zero"),
            builder.phases,
            span,
        )?;

        // Modify the zone span accordingly
        winding_table.shift_zones(builder.zone_span_variation as i32);

        // First, the winding is constructed with a placeholder coil hashmap. Then, it
        // is used to create the actual hashmap
        let mut winding = DistributedWinding {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            layers: builder.layers,
            coil_span_reduction: builder.coil_span_reduction,
            zone_span_variation: builder.zone_span_variation,
            turns_per_coil: builder.turns_per_coil,
            parallel_paths: builder.parallel_paths,
            connection: builder.connection,
            end_winding_leakage_coefficient: builder.end_winding_leakage_coefficient,
            wire: builder.wire,
            coils: Coils::with_capacity((slots * layers / 2).into()),
            winding_table_method: builder.winding_table_method,
            concentric_coils: builder.concentric_coils,
        };
        winding.create_coils(&winding_table)?;

        // Check if the number of parallel paths is valid
        if winding
            .possible_parallel_paths()
            .all(|possible_path| possible_path != winding.parallel_paths)
        {
            return Err(Error::InvalidNumberParallelPaths);
        }

        if winding.equal_winding_factors() {
            return Ok(winding);
        } else {
            return Err(WindingTableCreationError::NotSymmetric.into());
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DistributedDoubleZoneSpanBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub coil_span_reduction: i32,
    pub turns_per_coil: NonZeroUsize,
    pub parallel_paths: NonZeroU16,
    pub connection: Connection,
    pub end_winding_leakage_coefficient: f64,
    pub wire: Box<dyn Wire>,
    pub winding_table_method: WindingTableMethod,
    pub concentric_coils: bool,
}

impl TryFrom<DistributedDoubleZoneSpanBuilder> for DistributedWinding {
    type Error = Error;

    fn try_from(builder: DistributedDoubleZoneSpanBuilder) -> Result<Self, Self::Error> {
        // Check if an integer slot winding can be constructed from the input
        let hn = hole_number(builder.slots, builder.pole_pairs, builder.phases);
        let g = hn.trunc().numer().clone();
        let n = hn.denom().clone();
        let zone_span_variation = if n == 1 {
            g
        } else {
            return Err(Error::NeedsIntegerSlot);
        };

        DistributedBuilder {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            layers: NonZeroU16::new(2).expect("not zero"),
            coil_span_reduction: builder.coil_span_reduction,
            zone_span_variation,
            winding_table_method: builder.winding_table_method,
            turns_per_coil: builder.turns_per_coil,
            parallel_paths: builder.parallel_paths,
            connection: builder.connection,
            end_winding_leakage_coefficient: builder.end_winding_leakage_coefficient,
            wire: builder.wire,
            concentric_coils: builder.concentric_coils,
        }
        .try_into()
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DistributedMinimalBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub layers: NonZeroU16,
    /// Coil span reduction in slots ("Wickelschrittverkürzung aufgrund der
    /// Sehnung")
    pub coil_span_reduction: i32,
    pub zone_span_variation: u16,
    pub winding_table_method: WindingTableMethod,
}

impl TryFrom<DistributedMinimalBuilder> for DistributedWinding {
    type Error = Error;

    fn try_from(builder: DistributedMinimalBuilder) -> Result<Self, Self::Error> {
        return DistributedBuilder {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            layers: builder.layers,
            coil_span_reduction: builder.coil_span_reduction,
            zone_span_variation: builder.zone_span_variation,
            winding_table_method: builder.winding_table_method,
            turns_per_coil: NonZeroUsize::MIN,
            parallel_paths: NonZeroU16::MIN,
            connection: Connection::Star,
            end_winding_leakage_coefficient: 0.0,
            wire: Box::new(RoundWire::default()),
            concentric_coils: false,
        }
        .try_into();
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for DistributedWinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(deserialize_untagged_verbose_error::DeserializeUntaggedVerboseError)]
        enum DistributedEnum {
            DistributedBuilder(DistributedBuilder),
            DistributedDoubleZoneSpanBuilder(DistributedDoubleZoneSpanBuilder),
            DistributedMinimalBuilder(DistributedMinimalBuilder),
        }
        let w = DistributedEnum::deserialize(deserializer)?;
        match w {
            DistributedEnum::DistributedBuilder(w) => {
                w.try_into().map_err(serde::de::Error::custom)
            }
            DistributedEnum::DistributedDoubleZoneSpanBuilder(w) => {
                w.try_into().map_err(serde::de::Error::custom)
            }
            DistributedEnum::DistributedMinimalBuilder(w) => {
                w.try_into().map_err(serde::de::Error::custom)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_coil_group() {
        let winding_table = WindingTable::with_method(
            &WindingTableMethod::Tingley,
            18.try_into().unwrap(),
            1.try_into().unwrap(),
            1.try_into().unwrap(),
            3.try_into().unwrap(),
            9,
        )
        .unwrap();
        let coils = Coils::new();

        let cg =
            AvailableSlotsForCoilGroup::new(&winding_table, &coils, Zone::new(0, 0), 18).unwrap();
        assert_eq!(cg.start, 0);
        assert_eq!(cg.stop, 2);
        assert_eq!(cg.len, 2);
        assert_eq!(cg.phase, 1);

        let cg =
            AvailableSlotsForCoilGroup::new(&winding_table, &coils, Zone::new(1, 0), 18).unwrap();
        assert_eq!(cg.start, 0);
        assert_eq!(cg.stop, 2);
        assert_eq!(cg.len, 2);
        assert_eq!(cg.phase, 1);
    }

    #[test]
    fn test_find_partners() {
        let winding_table = WindingTable::with_method(
            &WindingTableMethod::Tingley,
            18.try_into().unwrap(),
            1.try_into().unwrap(),
            1.try_into().unwrap(),
            3.try_into().unwrap(),
            9,
        )
        .unwrap();
        let coils = Coils::new();

        let cg =
            AvailableSlotsForCoilGroup::new(&winding_table, &coils, Zone::new(0, 0), 18).unwrap();
        let (partner, found_while_ascending) =
            cg.find_partner(&winding_table, &coils, 18, 0).unwrap();
        assert!(found_while_ascending);
        assert_eq!(partner.start, 9);
        assert_eq!(partner.stop, 11);
        assert_eq!(partner.len, 2);
        assert_eq!(partner.phase, -1);
    }
}
