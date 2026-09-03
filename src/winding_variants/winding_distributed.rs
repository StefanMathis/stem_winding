use dyn_clone::clone_box;
use num::rational::Ratio;
use slot::CoilLayout;
use wire::{IsWire, RoundWire};


use material::InfluencingQuantity;


use magnetic_core::{CoreRef, IsCoreRef};


use stem_primitives::Overrides;


use uom::si::f64::*;

use compare_variables::compare_variables;

use crate::{
    Coil, CoilFull, Winding, Zone,
    common::{Connection, hole_number, periodicity},
    zones::{self, WindingTableMethod},
};
use crate::{Coils, WindingTable};

#[cfg(feature = "serde")]
use deserialize_untagged_verbose_error::DeserializeAlias;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, DeserializeAlias))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "serde_impl::WindingDistributedVariants")
)]
#[cfg_attr(
    feature = "serde",
    deserialize_alias(name = "NewFull", error = "stem_primitives::Error")
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct WindingDistributed {
    slots: u16,
    pole_pairs: u16,
    phases: u16,
    layers: u16,
    coil_span_reduction: i32, /* Coil span reduction in slots ("Wickelschrittverkürzung aufgrund
                               * der Sehnung") */
    zone_span_variation: u16,
    turns_per_coil: usize,
    parallel_paths: u16,
    connection: Connection, /* Connection type, e.g. star (Y), delta (D) or combinations like YY
                             * (double star) or DY (star-delta) */
    end_winding_leakage_coefficient: f64, // End winding flux leakage coefficient
    wire: Box<dyn IsWire>,                // Wire
    coils: Coils,
    concentric_coils: bool,
}

impl WindingDistributed {
    /**
    Tries to create a new instange of an `WindingDistributed`. Fails if the combination of input parameters does not lead to a legal winding.

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
    pub fn new(
        slots: u16,
        pole_pairs: u16,
        phases: u16,
        layers: u16,
        coil_span_reduction: i32,
        zone_span_variation: u16,
        turns_per_coil: usize,
        parallel_paths: u16,
        connection: Connection,
        end_winding_leakage_coefficient: f64,
        wire: Box<dyn IsWire>,
        winding_table_method: WindingTableMethod,
        concentric_coils: bool,
    ) -> stem_primitives::Result<Self> {
        // Calculate the basic winding parameters
        let t = periodicity(slots, pole_pairs, phases, layers);
        let slots_basic = slots / t;
        let pole_pairs_basic = pole_pairs / t;

        let span = (slots as f64 / (2.0 * pole_pairs as f64)).floor() as i32 - coil_span_reduction;

        // Create the zone plan by method
        let mut winding_table = zones::create_winding_table_by_method(
            winding_table_method,
            slots_basic,
            pole_pairs_basic,
            phases,
            layers,
            span,
            false,
        )?;

        // Modify the zone span accordingly
        winding_table.shift_zones(zone_span_variation as i32);

        // First, the winding is constructed with a placeholder coil hashmap. Then, it
        // is used to create the actual hashmap
        let mut winding = WindingDistributed {
            slots,
            pole_pairs,
            phases,
            layers,
            coil_span_reduction,
            zone_span_variation,
            turns_per_coil,
            parallel_paths,
            connection,
            end_winding_leakage_coefficient,
            wire,
            coils: Coils::with_capacity((slots * layers / 2).into()),
            concentric_coils,
        };
        winding.create_coils(&winding_table)?;
        return winding.check();
    }

    fn check(self) -> stem_primitives::Result<Self> {
        compare_variables!(0 < self.layers < 3)?;
        compare_variables!(0 < self.slots)?;
        compare_variables!(0 < self.pole_pairs)?;
        compare_variables!(0 < self.parallel_paths)?;
        compare_variables!(0 < self.phases)?;
        compare_variables!(0 < self.layers < 3)?;
        compare_variables!(0.0 <= self.end_winding_leakage_coefficient)?;

        // Check if the number of parallel paths is valid
        let mut valid = false;
        for pp in self.possible_parallel_paths() {
            if pp == self.parallel_paths as usize {
                valid = true
            }
        }
        if !valid {
            return Err(stem_primitives::ErrorType::Other(
                "Given number of parallel paths not possible.".into(),
            )
            .into());
        }

        if self.equal_winding_factors() {
            return Ok(self);
        } else {
            return Err(
                stem_primitives::ErrorType::Other("Winding is not symmetric.".into()).into(),
            );
        }
    }

    /**
    Return the distribution and the pitch factor as `[distribution, pitch]`.
    The product of those two values equals the winding factor calculated from `self.winding_factor()`
     */
    pub fn distribution_and_pitch_factor(&self, phase: u16, ordinal: f64) -> [f64; 2] {
        // If layers==1, the distribution factor equals the winding factor,
        // meaning that the pitch factor is 1 by default. If layers == 2,
        // distribution and pitch factor are calculated by temporarily changing the
        // pitch to zero.
        let coil_span_reduction = self.coil_span_reduction();
        if self.layers() == 1 || coil_span_reduction == 0 {
            return [self.winding_factor(phase, ordinal), 1.0];
        } else {
            // Create a copy of this winding, but without the short pitch
            let temp_winding = Self::new_minimal(
                self.slots(),
                self.pole_pairs(),
                self.phases(),
                self.layers(),
                0,
                self.zone_span_variation,
                WindingTableMethod::DistributionTable,
            )
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
        return self.coil_span_reduction;
    }

    pub fn zone_span_variation(&self) -> u16 {
        return self.zone_span_variation;
    }

    pub fn wire(&self) -> &dyn IsWire {
        return &*self.wire;
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

    /**
    This algorithm creates the coils of self by first grouping the zones into pairs of coil groups and
    then creating coils going from one pair to the other.
     */
    fn create_coils(&mut self, winding_table: &WindingTable) -> stem_primitives::Result<()> {
        let return_layer = if self.layers() == 1 { 0 } else { 1 };

        // The three phases are handled independently from each other, since they don't
        // interact.
        for phase in 1..(self.phases() as i32 + 1) {
            let start_slot = winding_table.start_at_largest_possible_span(phase);
            for slot in start_slot..(self.slots() + start_slot) {
                let slot = slot.rem_euclid(self.slots());

                /*
                It is sufficient to search on layer 0, since all coils in a double-layer winding go
                from layer 0 to layer 1 (return_layer).
                 */
                let zone = Zone::new(slot, 0);

                // Skip all zones which don't belong to the current phase
                if winding_table.get_wrapped(zone).abs() != phase {
                    continue;
                }

                // Identify the coil group pair
                if let Some(cg) =
                    AvailableSlotsForCoilGroup::new(winding_table, &self.coils, zone, self.slots())
                {
                    if let Some((cg_partner, found_partner_while_ascending)) =
                        cg.find_partner(winding_table, &self.coils, self.slots(), return_layer)
                    {
                        // Derive coils from the pair
                        self.populate_from_coil_group_pair(
                            &cg,
                            &cg_partner,
                            found_partner_while_ascending,
                        )?;
                    }
                }
            }
        }

        // Check if all zones are filled.
        for layer in 0..self.layers() {
            for slot in 0..self.slots() {
                // Check if the zone is already occupied
                let zone = Zone::new(slot, layer);
                if !self.coils.0.contains_key(&zone) {
                    return Err(stem_primitives::ErrorType::EmptyZone {
                        slot: zone.slot,
                        layer: zone.layer,
                    }
                    .into());
                }
            }
        }
        return Ok(());
    }

    fn populate_from_coil_group_pair(
        &mut self,
        seed_group: &AvailableSlotsForCoilGroup,
        partner_group: &AvailableSlotsForCoilGroup,
        found_partner_while_ascending: bool,
    ) -> stem_primitives::Result<()> {
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
                let positive_zone = Zone::new(pos_slot.rem_euclid(self.slots()), pos.layer);
                let negative_zone = Zone::new(neg_slot.rem_euclid(self.slots()), neg.layer);
                let coil = CoilFull::new(
                    positive_zone,
                    negative_zone,
                    true,
                    clockwise,
                    self.turns_per_coil,
                    pos.phase as u16,
                    clone_box(&*self.wire),
                )?;
                self.coils
                    .0
                    .insert_many(vec![positive_zone, negative_zone], Coil::Full(coil))
                    .map_err(|err| stem_primitives::ErrorType::Other(err.to_string()))?;
            }
        } else {
            // Connect pos.start to neg.start and pos.stop to neg.stop
            for (pos_slot, neg_slot) in (pos.start..(pos.start + pos.len + 1))
                .into_iter()
                .zip((neg.start..(neg.start + neg.len + 1)).into_iter())
            {
                let positive_zone = Zone::new(pos_slot.rem_euclid(self.slots()), pos.layer);
                let negative_zone = Zone::new(neg_slot.rem_euclid(self.slots()), neg.layer);
                let coil = CoilFull::new(
                    positive_zone,
                    negative_zone,
                    true,
                    clockwise,
                    self.turns_per_coil,
                    pos.phase as u16,
                    clone_box(&*self.wire),
                )?;
                self.coils
                    .0
                    .insert_many(vec![positive_zone, negative_zone], Coil::Full(coil))
                    .map_err(|err| stem_primitives::ErrorType::Other(err.to_string()))?;
            }
        }
        return Ok(());
    }
}

impl Default for WindingDistributed {
    fn default() -> Self {
        return WindingDistributed::new(
            6,
            1,
            3,
            1,
            0,
            0,
            1,
            1,
            Connection::Star,
            0.0,
            Box::new(RoundWire::default()),
            WindingTableMethod::Tingley,
            false,
        )
        .unwrap();
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Winding for WindingDistributed {
    fn phases(&self) -> u16 {
        return self.phases;
    }

    fn slots(&self) -> u16 {
        return self.slots;
    }

    fn pole_pairs(&self) -> u16 {
        return self.pole_pairs;
    }

    fn layers(&self) -> u16 {
        return self.layers;
    }

    /// Calculate the number of basic windings with the formulae from [Pyr08],
    /// section 2.11 (p. 102 ff)
    fn periodicity(&self) -> u16 {
        return periodicity(
            self.slots(),
            self.pole_pairs(),
            self.phases(),
            self.layers(),
        );
    }

    fn turns_at(&self, _zone: Zone) -> usize {
        return self.turns_per_coil;
    }

    fn coil_layout(&self) -> CoilLayout {
        if self.layers() == 1 {
            return CoilLayout::Single;
        } else {
            return CoilLayout::DoubleVertical;
        }
    }

    fn turns_per_phase(&self, _phase: u16) -> Ratio<usize> {
        return Ratio::new_raw(
            usize::from(self.layers() * self.slots()) * self.turns_per_coil
                / usize::from(2 * self.phases() * self.parallel_paths()),
            1,
        );
    }

    fn parallel_paths(&self) -> u16 {
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

    /**
    Returns the number of coil groups per phase. This value is equal to the maximum possible number of parallel paths and can be calculated
    as described in [Seq50], p. 37: First, the number of coils per phase in a basic winding is calculated. Then, it is checked whether this
    number is even or odd. If it is even, the number of coil groups equals twice the number of basic windings (= the periodicity). If it is odd,
    the number of coil groups equals the number of basic windings.
    */
    fn coil_groups_per_phase(&self) -> u16 {
        let t = self.periodicity();
        let number_of_coils_in_basic_winding =
            self.slots() * self.layers() / (t * 2 * self.phases());
        if number_of_coils_in_basic_winding % 2 == 0 {
            // Antiparallel coil groups are possible
            return 2 * t;
        } else {
            // Only parallel coil groups are possible
            return t;
        }
    }

    fn as_dyn(&self) -> &dyn Winding {
        self
    }

    
    fn is_symmetric(&self, _core: CoreRef<'_>, _overrides: &Overrides) -> bool {
        return true;
    }

    /// Phase resistance calculation of a distributed winding according to
    /// [Mat19], eq. (3.48).
    
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

// =================================================================================
// Deserialization

#[derive(constructor)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[constructor(
    target = "WindingDistributed",
    fn_name = "new_with_doubled_zone_span",
    error = "stem_primitives::Error"
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
struct NewWithDoubleZoneSpan {
    slots: u16,
    pole_pairs: u16,
    phases: u16,
    coil_span_reduction: i32,
    turns_per_coil: usize,
    parallel_paths: u16,
    connection: Connection,
    end_winding_leakage_coefficient: f64,
    wire: Box<dyn IsWire>,
    winding_table_method: WindingTableMethod,
    concentric_coils: bool,
}

impl TryFrom<NewWithDoubleZoneSpan> for WindingDistributed {
    type Error = stem_primitives::Error;

    fn try_from(value: NewWithDoubleZoneSpan) -> Result<Self, Self::Error> {
        // Check if an integer slot winding can be constructed from the input
        let ratio = hole_number(value.slots, value.pole_pairs, value.phases);
        let g = ratio.trunc().numer().clone();
        let n = ratio.denom().clone();
        let zone_span_variation = if n == 1 {
            g
        } else {
            return Err(stem_primitives::ErrorType::Other(
                "A double zone span winding can only be constructed from an integer slot winding."
                    .into(),
            )
            .into());
        };

        return Self::new(
            value.slots,
            value.pole_pairs,
            value.phases,
            2, // Default value for integer slot windings with a doubled zone span
            value.coil_span_reduction,
            zone_span_variation.try_into().unwrap(),
            value.turns_per_coil,
            value.parallel_paths,
            value.connection,
            value.end_winding_leakage_coefficient,
            value.wire,
            value.winding_table_method,
            value.concentric_coils,
        );
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
struct NewWindingTableMethod {
    slots: u16,
    pole_pairs: u16,
    phases: u16,
    layers: u16,
    coil_span_reduction: i32, /* Coil span reduction in slots ("Wickelschrittverkürzung aufgrund
                               * der Sehnung") */
    zone_span_variation: u16,
    turns_per_coil: usize,
    parallel_paths: u16,
    connection: Connection, /* Connection type, e.g. star (Y), delta (D) or combinations like YY
                             * (double star) or DY (star-delta) */
    end_winding_leakage_coefficient: f64, // End winding flux leakage coefficient
    wire: Box<dyn IsWire>,                // Wire motor
    winding_table_method: WindingTableMethod,
    concentric_coils: bool,
}

impl TryFrom<NewWindingTableMethod> for WindingDistributed {
    type Error = stem_primitives::Error;

    fn try_from(value: NewWindingTableMethod) -> Result<Self, Self::Error> {
        return Self::new(
            value.slots,
            value.pole_pairs,
            value.phases,
            value.layers,
            value.coil_span_reduction,
            value.zone_span_variation,
            value.turns_per_coil,
            value.parallel_paths,
            value.connection,
            value.end_winding_leakage_coefficient,
            value.wire,
            value.winding_table_method,
            value.concentric_coils,
        );
    }
}

#[derive(constructor)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[constructor(
    target = "WindingDistributed",
    fn_name = "new_minimal",
    error = "stem_primitives::Error"
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
struct NewMinimal {
    slots: u16,
    pole_pairs: u16,
    phases: u16,
    layers: u16,
    coil_span_reduction: i32, /* Coil span reduction in slots ("Wickelschrittverkürzung aufgrund
                               * der Sehnung") */
    zone_span_variation: u16,
    winding_table_method: WindingTableMethod,
}

impl TryFrom<NewMinimal> for WindingDistributed {
    type Error = stem_primitives::Error;

    fn try_from(value: NewMinimal) -> Result<Self, Self::Error> {
        // Definition of default values
        let turns_per_coil = 1;
        let parallel_paths = 1;
        let connection = Connection::Star;
        let end_winding_leakage_coefficient = 0.0;
        let wire = Box::new(RoundWire::default());

        return Self::new(
            value.slots,
            value.pole_pairs,
            value.phases,
            value.layers,
            value.coil_span_reduction,
            value.zone_span_variation,
            turns_per_coil,
            parallel_paths,
            connection,
            end_winding_leakage_coefficient,
            wire,
            value.winding_table_method,
            false,
        );
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;
    use deserialize_untagged_verbose_error::{DeserializeUntaggedVerboseError, TryFromEnum};

    #[derive(TryFromEnum, DeserializeUntaggedVerboseError)]
    #[try_from_enum(target = "WindingDistributed", error = "stem_primitives::Error")]
    pub(super) enum WindingDistributedVariants {
        #[try_from_enum(convert_with = "check")]
        NewFull(NewFull),
        NewWindingTableMethod(NewWindingTableMethod),
        NewWithDoubleZoneSpan(NewWithDoubleZoneSpan),
        NewMinimal(NewMinimal),
    }

    fn check(alias: NewFull) -> stem_primitives::Result<WindingDistributed> {
        return WindingDistributed::from(alias).check();
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

        let phase = winding_table.get_wrapped(zone);

        // Find the start
        let mut start = zone.slot;
        for slot in (zone.slot..(zone.slot + slots)).rev() {
            let new_zone = Zone::new(slot.rem_euclid(slots), zone.layer);
            if coils.0.contains_key(&new_zone) || winding_table.get_wrapped(new_zone) != phase {
                break;
            } else {
                start = new_zone.slot;
            }
        }

        // Find the length
        let mut stop = zone.slot;
        for slot in (zone.slot + 1)..(zone.slot + slots) {
            let new_zone = Zone::new(slot.rem_euclid(slots), zone.layer);
            if coils.0.contains_key(&new_zone) || winding_table.get_wrapped(new_zone) != phase {
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
            phase,
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
                if winding_table.get_wrapped(search_zone) == -self.phase {
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

impl WindingTable {
    /*
    The coil arrangement is very sensitive to the starting location. As an example, let's consider a
    18 slot / 4 pole pairs single layer winding with the following zone plan:
    ```ignore
    1 2 -1 3 -2 -3 2 3 -2 1 -3 -1 3 1 -3 2 -1 -2
    ```
    Starting the coil builder algorithm at the first slot results in the following coil configuration for phase 2:
    ```ignore
    ──┐       ┌────┐    ┌────────────────┐     ┌
    1 2 -1 3 -2 -3 2 3 -2 1 -3 -1 3 1 -3 2 -1 -2
    ──┘       └────┘    └────────────────┘     └
    ```
    The very long third coil is solely a result of the starting location. IF we instead start at the third positive zone,
    the coil configuration looks like this:
      ```ignore
      ┌───────┐    ┌────┐                ┌─────┐
    1 2 -1 3 -2 -3 2 3 -2 1 -3 -1 3 1 -3 2 -1 -2
      └───────┘    └────┘                └─────┘
    ```
    The overall end winding length of this configuration is much shorter. To find the optimal configuration with a
    minimum overall end winding length, a heuristic approach is used: We search for the hypothetical coil
    with the longest span and start the coil creater algorithm at the positive zone of this coil.
     */
    fn start_at_largest_possible_span(&self, phase: i32) -> u16 {
        let mut longest_span = 0;
        let mut span_start_slot = 0;
        let mut positive_slot: u16 = 0;
        let mut last_phase: i32 = 0;

        /*
        In order to find coils which go from the end to the start of the zone plan,
        the algorithm scans the zone plan two times.
         */
        for slot in 0..(2 * self.slots()) {
            /*
            We only need to check layer 0, since in case of a double-layer winding layer 1
            is merely a shifted and mirrored version of layer 0.
             */
            let current_phase = self.get_wrapped(Zone::new(slot, 0));
            if current_phase.abs() == phase {
                if current_phase != last_phase {
                    let current_span = slot - span_start_slot;
                    if current_span > longest_span {
                        longest_span = current_span;
                        if current_phase > 0 {
                            positive_slot = slot
                        } else {
                            positive_slot = span_start_slot;
                        }
                    }
                    last_phase = current_phase;
                }
                span_start_slot = slot;
            }
        }
        return positive_slot;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_coil_group() {
        let winding_table = zones::create_winding_table_by_method(
            WindingTableMethod::Tingley,
            18,
            1,
            3,
            1,
            9,
            false,
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
        let winding_table = zones::create_winding_table_by_method(
            WindingTableMethod::Tingley,
            18,
            1,
            3,
            1,
            9,
            false,
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
