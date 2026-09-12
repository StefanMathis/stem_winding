use std::num::{NonZeroU16, NonZeroUsize};

use compare_variables::compare_variables;

use dyn_clone::clone_box;
use num::Integer;
use stem_coil_layout::{CoilLayout, Zone};
use stem_wire::{round::RoundWire, wire::Wire};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    coils::{Coil, CoilFull, Coils},
    error::{Error, WindingTableCreationError},
    iterators::HarmonicOrdinalsIterator,
    winding::{Connection, Winding},
    winding_table::{WindingTable, WindingTableMethod},
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct DistributedToothCoilWinding {
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    phases: NonZeroU16,
    layers: NonZeroU16,
    parallel_paths: NonZeroU16,
    connection: Connection,
    end_winding_leakage_coefficient: f64,
    wires: Vec<(NonZeroUsize, Box<dyn Wire>)>,
    #[cfg_attr(feature = "serde", serde(skip))]
    coils: Coils,
    #[cfg_attr(feature = "serde", serde(skip))]
    base_winding_count: NonZeroU16,
    double_zone_span: bool,
}

impl DistributedToothCoilWinding {
    pub fn new<W>(builder: W) -> Result<Self, Error>
    where
        W: TryInto<DistributedToothCoilWinding>,
        W::Error: Into<Error>,
    {
        builder.try_into().map_err(Into::into)
    }

    pub fn coils_per_coil_group(&self) -> u16 {
        return self.wires.len() as u16;
    }

    pub fn double_zone_span(&self) -> bool {
        self.double_zone_span
    }

    pub fn coil_group_turns<'a>(&'a self) -> impl Iterator<Item = NonZeroUsize> + 'a {
        self.wires.iter().map(|(turns, _)| *turns)
    }

    /**
    Create a turn distribution for a double-layer distributed winding which keeps the number of turns per slot constant.

    # Example
    ```
    use winding::DistributedToothCoilWinding;

    // Three coils per coil group, 10 turns per slot (5 per coil). The inner coil gets 5 - 2 = 3 turns, the middle coil gets 5 turns, the outer coil gets 5 + 2 = 7 turns.
    let turns = DistributedToothCoilWinding::double_layer_turn_distribution(3, 10, 2).unwrap();
    assert_eq!(turns[0], 3);
    assert_eq!(turns[1], 5);
    assert_eq!(turns[2], 7);

    // Four coils per coil group, 10 turns per slot. The inner coils get 3 turns, the outer coils get 7 turns.
    let turns = DistributedToothCoilWinding::double_layer_turn_distribution(4, 10, 2).unwrap();
    assert_eq!(turns[0], 3);
    assert_eq!(turns[1], 3);
    assert_eq!(turns[2], 7);
    assert_eq!(turns[3], 7);

    // Function errors for the following cases: turns per slot is zero or odd, absolute value of turn_difference equal to or greater than turns_per_slot
    assert!(DistributedToothCoilWinding::double_layer_turn_distribution(3, 0, 3).is_err());
    assert!(DistributedToothCoilWinding::double_layer_turn_distribution(3, 9, 3).is_err());
    assert!(DistributedToothCoilWinding::double_layer_turn_distribution(3, 10, 5).is_err());
    ```
     */
    pub fn double_layer_turn_distribution(
        coils_per_coil_group: u16,
        turns_per_slot: NonZeroUsize,
        turn_difference: i32,
    ) -> Result<Vec<NonZeroUsize>, Error> {
        let turns_per_slot = turns_per_slot.get();
        compare_variables!(0 < coils_per_coil_group)?;
        compare_variables!(0 < turns_per_slot)?;

        let twice_turn_difference = 2 * turn_difference.abs() as usize;
        compare_variables!(turns_per_slot > twice_turn_difference)?;

        if turns_per_slot.is_odd() {
            return Err(Error::OddNumberOfTurnsPerSlot);
        }

        // Create the turns per coil vector from coil_group_turns and turn_difference as
        // described in [Hut19], p. 137.
        let mut coil_group_turns: Vec<NonZeroUsize> = vec![
            NonZeroUsize::new(turns_per_slot / 2)
                .unwrap_or(NonZeroUsize::MIN);
            coils_per_coil_group as usize
        ];
        for hole in 0..coils_per_coil_group {
            // If q_star is odd, the center value of the vector does not change
            if 2 * hole == coils_per_coil_group - 1 {
                continue;
            // Inner coils: Reduce the number of turns per coil
            } else if 2 * hole < coils_per_coil_group - 1 {
                let turns =
                    NonZeroUsize::new((turns_per_slot as i32 / 2 - turn_difference) as usize)
                        .unwrap_or(NonZeroUsize::MIN);
                coil_group_turns[hole as usize] = turns;
            // Outer coils: Increase the number of turns per coil
            } else {
                let turns =
                    NonZeroUsize::new((turns_per_slot as i32 / 2 + turn_difference) as usize)
                        .unwrap_or(NonZeroUsize::MIN);
                coil_group_turns[hole as usize] = turns;
            }
        }
        return Ok(coil_group_turns);
    }

    fn calculated_turns_at(&self, zone: Zone, _winding_table: &WindingTable) -> NonZeroUsize {
        // The number of turns is encoded in the vector self.coil_group_turns as
        // follows: First entry is the innermost coil, second entry is the
        // second-innermost coil and so on.
        let offset = if zone.layer == 0 {
            0
        } else {
            self.coils_per_coil_group()
        };
        let normalized_slot = (zone.slot + offset) % (2 * self.coils_per_coil_group());
        let idx = if normalized_slot < self.coils_per_coil_group() {
            (self.coils_per_coil_group() - 1 - normalized_slot) as usize
        } else {
            (normalized_slot % (2 * self.coils_per_coil_group()) - self.coils_per_coil_group())
                as usize
        };
        self.wires[idx].0
    }

    fn create_coils(&mut self, winding_table: &WindingTable) -> Result<(), Error> {
        for slot in 0..self.slots.get() {
            for layer in 0..self.layers.get() {
                let zone = Zone { slot, layer };

                // Check if the zone is already occupied
                if self.coils.0.contains_key(&zone) {
                    continue;
                }

                // Insert the coil
                if let Some(coil) = self.create_single_coil(zone, winding_table) {
                    let zones: Vec<Zone> = coil.zones().collect();
                    self.coils.0.insert_many(zones, coil).map_err(Error::from)?;
                }
            }
        }

        return Ok(());
    }

    fn create_single_coil(&self, zone: Zone, winding_table: &WindingTable) -> Option<Coil> {
        let phase = *winding_table.get_cyclic(zone);
        let slot = zone.slot;
        let layer = zone.layer;

        let mut return_slot: Option<u16> = None;
        let mut clockwise: Option<bool> = None;
        let mut index: Option<u16> = None;

        // Search through the neighboring slots, but stay in the same layer
        for k in 1..(self.coils_per_coil_group() + 1) {
            let search_slot = (k + slot).rem_euclid(self.slots().get());

            if self.coils.0.contains_key(&Zone::new(search_slot, layer)) {
                break;
            }

            // Detect direction change in search_slot: Use the slot in position q-k+1
            if *winding_table.get_cyclic(Zone::new(search_slot, layer)) == -phase {
                return_slot = Some((slot + 2 * k - 1).rem_euclid(self.slots().get()));
                clockwise = Some(true);
                index = Some(k - 1);
                break;
            }

            // Detect phase change in search_slot: Use the slot in position k
            if winding_table
                .get_cyclic(Zone::new(search_slot, layer))
                .abs()
                != phase.abs()
            {
                return_slot = Some(
                    (slot as i32 - 2 * self.coils_per_coil_group() as i32 + 2 * k as i32 - 1)
                        .rem_euclid(self.slots().get() as i32) as u16,
                );
                clockwise = Some(false);
                index = Some(k - 1);
                break;
            }
        }

        match return_slot {
            Some(return_slot) => {
                let clockwise = clockwise.unwrap();
                let index = index.unwrap();
                let phase_abs = NonZeroU16::new(phase.abs() as u16)
                    .expect("phase cannot be zero for this winding type");
                let wire = clone_box(&*self.wires[index as usize].1);
                if phase > 0 {
                    return CoilFull::new(
                        Zone::new(slot, layer),
                        Zone::new(return_slot, layer),
                        true,
                        clockwise,
                        self.calculated_turns_at(Zone::new(slot, layer), winding_table),
                        phase_abs,
                        wire,
                    )
                    .ok()
                    .map(|coil| Coil::Full(coil));
                } else {
                    return CoilFull::new(
                        Zone::new(slot, layer),
                        Zone::new(return_slot, layer),
                        false,
                        !clockwise,
                        self.calculated_turns_at(Zone::new(slot, layer), winding_table),
                        phase_abs,
                        wire,
                    )
                    .ok()
                    .map(|coil| Coil::Full(coil));
                }
            }
            None => return None,
        }
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Winding for DistributedToothCoilWinding {
    fn phases(&self) -> NonZeroU16 {
        self.phases
    }

    fn slots(&self) -> NonZeroU16 {
        self.slots
    }

    fn pole_pairs(&self) -> NonZeroU16 {
        self.pole_pairs
    }

    fn layers(&self) -> NonZeroU16 {
        self.layers
    }

    fn base_winding_count(&self) -> NonZeroU16 {
        self.base_winding_count
    }

    fn coil_layout(&self) -> CoilLayout {
        if self.layers().get() == 1 {
            return CoilLayout::Single;
        } else {
            return CoilLayout::DoubleVertical;
        }
    }

    /// Returns the hole number representation q = g + z/n where g, z and n are
    /// integers in order [g, z, n].
    fn hole_number(&self) -> num::rational::Ratio<u16> {
        return num::rational::Ratio::new(2 * self.coils_per_coil_group(), 1);
    }

    fn turns_per_phase(&self, _phase: NonZeroU16) -> num::rational::Ratio<usize> {
        let all_turns: usize = self.coil_group_turns().map(usize::from).sum();
        return num::rational::Ratio::new(
            (all_turns * usize::from(u16::from(self.coil_groups_per_phase())))
                / usize::from(u16::from(self.parallel_paths())),
            1,
        );
    }

    fn connection(&self) -> Connection {
        self.connection
    }

    fn end_winding_leakage_coefficient(&self) -> f64 {
        self.end_winding_leakage_coefficient
    }

    /// The number of parallel paths of this winding type is alwaystwice the
    /// number of basic windings.
    fn parallel_paths(&self) -> NonZeroU16 {
        return NonZeroU16::new(self.base_winding_count().get() * 2).expect("not zero");
    }

    fn coil_at(&self, zone: Zone) -> Option<&Coil> {
        return self.coils.0.get(&zone);
    }

    /**
    Returns the number of coil groups per phase. This value is equal to the maximum possible number of parallel paths and can be calculated
    as described in [Seq50], p. 37: First, the number of coils per phase in a basic winding is calculated. Then, it is checked whether this
    number is even or odd. If it is even, the number of coil groups equals twice the number of basic windings (= the base_winding_count). If it is odd,
    the number of coil groups equals the number of basic windings.
    */
    fn coil_groups_per_phase(&self) -> NonZeroU16 {
        let t = self.base_winding_count();
        let number_of_coils_in_basic_winding =
            self.slots().get() * self.layers().get() / (t.get() * 2 * self.phases().get());
        if number_of_coils_in_basic_winding % 2 == 0 {
            // Antiparallel coil groups are possible
            return NonZeroU16::new(2 * t.get()).expect("cannot be zero, since t is NonZeroU16");
        } else {
            // Only parallel coil groups are possible
            return t;
        }
    }

    fn as_dyn(&self) -> &dyn Winding {
        self
    }

    #[cfg(feature = "stem_core")]
    fn is_symmetric(&self, _core: CoreRef<'_>, _overrides: &Overrides) -> bool {
        true
    }

    #[cfg(feature = "stem_core")]
    fn end_winding_leakage_inductance(
        &self,
        _phase: u16,
        _core: CoreRef<'_>,
        overrides: &Overrides,
    ) -> Inductance {
        if let Some(end_winding_leakage_inductance) = overrides.end_winding_leakage_inductance {
            return end_winding_leakage_inductance;
        }

        todo!()
    }

    #[cfg(feature = "stem_core")]
    fn end_winding_half_turn_length(
        &self,
        _core: CoreRef<'_>,
        _zone: Zone,
        overrides: &Overrides,
    ) -> Option<Length> {
        if let Some(end_winding_half_turn_length) = overrides.end_winding_half_turn_length {
            return Some(end_winding_half_turn_length);
        }

        todo!();
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DistributedToothCoilBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub layers: NonZeroU16,
    pub parallel_paths: NonZeroU16,
    pub connection: Connection,
    pub end_winding_leakage_coefficient: f64,
    pub wires: Vec<(NonZeroUsize, Box<dyn Wire>)>,
    pub double_zone_span: bool,
}

impl TryFrom<DistributedToothCoilBuilder> for DistributedToothCoilWinding {
    type Error = Error;

    fn try_from(builder: DistributedToothCoilBuilder) -> Result<Self, Self::Error> {
        let wires_len = builder.wires.len() as u16;
        compare_variables!(wires_len != 0)?;

        // Try creating the zone plan for the basic pole pair number (which is
        // calculated by winding_holes)
        let slots_winding_table = if builder.double_zone_span {
            2 * wires_len * builder.phases.get() / builder.layers.get()
        } else {
            4 * wires_len * builder.phases.get() / builder.layers.get()
        };

        let base_winding_count = builder.slots.get() / slots_winding_table;
        if builder.pole_pairs.get() % base_winding_count != 0 {
            return Err(Error::InvalidPolePairNumber);
        }

        let mut winding_table = distributed_tooth_coil_assembly(
            NonZeroU16::new(slots_winding_table).expect("not zero"),
            NonZeroU16::MIN,
            builder.phases,
            builder.layers,
            builder.wires.len(),
            builder.double_zone_span,
        )?;

        // Create the winding
        let mut winding = DistributedToothCoilWinding {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            layers: builder.layers,
            parallel_paths: builder.parallel_paths,
            connection: builder.connection,
            end_winding_leakage_coefficient: builder.end_winding_leakage_coefficient,
            wires: builder.wires,
            coils: Coils::with_capacity((builder.slots.get() * builder.layers.get() / 2).into()),
            base_winding_count: NonZeroU16::new(base_winding_count).expect("not zero"),
            double_zone_span: builder.double_zone_span,
        };

        // Create the coils from the zone plan
        winding.create_coils(&winding_table)?;

        // Invert the zone plan if the first harmonic ordinal is negative. The reasoning
        // for this is: The distributed tooth-coil windings are created from
        // very short-pitched integer slot windings. If the pole pair harmonic
        // wave wanders in the opposite direction as the harmonic of the integer
        // winding, the zone plan direction needs to be reversed. For an integer
        // winding, the first harmonic equals the pole pair harmonic.
        // Because distributed tooth-coil windings operate on a super harmonic of the
        // integer winding they are based on, the zone plan direction must be
        // reversed if the first harmonic is negative.
        let harmonic_ordinals = HarmonicOrdinalsIterator::new(&winding);
        match harmonic_ordinals.coupling_with_pole_pairs() {
            Some(coupling) => {
                if coupling < 0 {
                    for layer in 0..usize::from(winding_table.layers()) {
                        winding_table.reverse_layer_range(layer, 0, winding_table.slots().into());
                    }
                    winding_table.shift_layers(2 * wires_len as i32);
                }
            }
            None => {
                return Err(Error::InvalidPolePairNumber);
            }
        }

        // Recreate the coils from the adjusted zone plan
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
pub struct DistributedToothCoilMinimalBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub layers: NonZeroU16,
    pub coil_group_turns: Vec<NonZeroUsize>,
    pub parallel_paths: NonZeroU16,
    pub double_zone_span: bool,
}

impl TryFrom<DistributedToothCoilMinimalBuilder> for DistributedToothCoilWinding {
    type Error = Error;

    fn try_from(builder: DistributedToothCoilMinimalBuilder) -> Result<Self, Self::Error> {
        let mut wires: Vec<(NonZeroUsize, Box<dyn Wire>)> =
            Vec::with_capacity(builder.coil_group_turns.len());
        for turns in builder.coil_group_turns {
            wires.push((turns, Box::new(RoundWire::default())));
        }

        DistributedToothCoilBuilder {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            layers: builder.layers,
            parallel_paths: builder.parallel_paths,
            connection: Connection::Star,
            end_winding_leakage_coefficient: 0.0,
            wires,
            double_zone_span: builder.double_zone_span,
        }
        .try_into()
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DistributedToothCoilMinimalBuilderDoubleLayerTurnDifference {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub mean_coil_group_turns: NonZeroUsize,
    pub turn_difference: i32,
    pub coils_per_coil_group: u16,
    pub parallel_paths: NonZeroU16,
    pub double_zone_span: bool,
}

impl TryFrom<DistributedToothCoilMinimalBuilderDoubleLayerTurnDifference>
    for DistributedToothCoilWinding
{
    type Error = Error;

    fn try_from(
        builder: DistributedToothCoilMinimalBuilderDoubleLayerTurnDifference,
    ) -> Result<Self, Self::Error> {
        let coil_group_turns = DistributedToothCoilWinding::double_layer_turn_distribution(
            builder.coils_per_coil_group,
            builder.mean_coil_group_turns,
            builder.turn_difference,
        )?;

        DistributedToothCoilMinimalBuilder {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            layers: NonZeroU16::new(2).expect("not zero"),
            coil_group_turns,
            parallel_paths: builder.parallel_paths,
            double_zone_span: builder.double_zone_span,
        }
        .try_into()
    }
}

/// Returns a zone plan for a single-layer distributed tooth-coil winding.
/// The algorithm is taken from [Hut21].
fn distributed_tooth_coil_assembly(
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    phases: NonZeroU16,
    layers: NonZeroU16,
    coils_per_coil_group: usize,
    double_zone_span: bool,
) -> Result<WindingTable, Error> {
    let layers_times_slot = usize::from(layers.get() * slots.get());
    if double_zone_span {
        let poles_times_phases_times_coils_per_coil_group =
            usize::from(2 * phases.get() * pole_pairs.get()) * coils_per_coil_group;
        compare_variables!(layers_times_slot == poles_times_phases_times_coils_per_coil_group)?;
    } else {
        let two_times_poles_times_phases_times_coils_per_coil_group =
            usize::from(4 * phases.get() * pole_pairs.get()) * coils_per_coil_group;
        compare_variables!(
            layers_times_slot == two_times_poles_times_phases_times_coils_per_coil_group
        )?;
    }

    let span: i32 = if layers.get() == 1 {
        coils_per_coil_group as i32 * (phases.get() as i32 + 1)
    } else {
        coils_per_coil_group as i32
    };

    // Create a double-layer zone plan with 2*m zones and a single pole pair
    let mut winding_table = WindingTable::with_method(
        &WindingTableMethod::DistributionTable,
        slots,
        NonZeroU16::new(2).expect("not zero"),
        pole_pairs,
        phases,
        span,
    )?;

    let winding_table = if layers.get() == 1 {
        // Winding with m zones as discussed in TODO [Hut19], section 2.1
        if double_zone_span {
            // Convert to m zones
            winding_table.shift_zones(coils_per_coil_group as i32);
        }

        // Derived the distributed tooth-coil winding from the integer-slot winding as
        // shown in TODO [Hut21], fig. 1
        let mut wt = WindingTable::new(slots, NonZeroU16::MIN);
        for slot in 0..slots.get() {
            if usize::from(slot).rem_euclid(2 * coils_per_coil_group) >= coils_per_coil_group {
                wt[Zone::new(slot, 0)] = -winding_table[Zone::new(slot, 0)]
            } else {
                wt[Zone::new(slot, 0)] = winding_table[Zone::new(slot, 0)]
            }
        }
        wt
    } else {
        // Move the return conductors of the distributed tooth-coil winding in the
        // same layer as the original conductors of each phase. This means exchanging
        // upper and lower layers for every second winding zone defined by the number
        // of winding holes q.
        for slot in 0..slots.get() {
            if usize::from(slot).rem_euclid(2 * coils_per_coil_group) >= coils_per_coil_group {
                let ul = winding_table[Zone::new(slot, 0)];
                let ll = winding_table[Zone::new(slot, 1)];
                winding_table[Zone::new(slot, 0)] = ll;
                winding_table[Zone::new(slot, 1)] = ul;
            }
        }

        if double_zone_span {
            // Convert to m zones by inverting every phase in the second layer
            for slot in 0..slots.get() {
                winding_table[Zone::new(slot, 1)] = -winding_table[Zone::new(slot, 1)];
            }
        }
        winding_table
    };
    return Ok(winding_table);
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for DistributedToothCoilWinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(deserialize_untagged_verbose_error::DeserializeUntaggedVerboseError)]
        enum DistributedToothCoilEnum {
            DistributedToothCoilBuilder(DistributedToothCoilBuilder),
            DistributedToothCoilMinimalBuilder(DistributedToothCoilMinimalBuilder),
            DistributedToothCoilMinimalBuilderDoubleLayerTurnDifference(
                DistributedToothCoilMinimalBuilderDoubleLayerTurnDifference,
            ),
        }
        let w = DistributedToothCoilEnum::deserialize(deserializer)?;
        match w {
            DistributedToothCoilEnum::DistributedToothCoilBuilder(w) => {
                w.try_into().map_err(serde::de::Error::custom)
            }
            DistributedToothCoilEnum::DistributedToothCoilMinimalBuilder(w) => {
                w.try_into().map_err(serde::de::Error::custom)
            }
            DistributedToothCoilEnum::DistributedToothCoilMinimalBuilderDoubleLayerTurnDifference(w) => {
                w.try_into().map_err(serde::de::Error::custom)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_layer_distributed_tooth_coil_assembly() {
        {
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(12).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                2,
                true,
            )
            .unwrap();

            let expected_result = WindingTable::from_slot_major(
                [1, 1, -1, -1, 2, 2, -2, -2, 3, 3, -3, -3].into_iter(),
                NonZeroU16::new(12).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
            );
            assert_eq!(winding_table, expected_result);
        }

        {
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(18).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                3,
                true,
            )
            .unwrap();

            let expected_result = WindingTable::from_slot_major(
                [
                    1, 1, 1, -1, -1, -1, 2, 2, 2, -2, -2, -2, 3, 3, 3, -3, -3, -3,
                ]
                .into_iter(),
                NonZeroU16::new(18).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
            );
            assert_eq!(winding_table, expected_result);
        }

        {
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(24).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                2,
                false,
            )
            .unwrap();

            let expected_result = WindingTable::from_slot_major(
                [
                    1, 1, -1, -1, -3, -3, 3, 3, 2, 2, -2, -2, -1, -1, 1, 1, 3, 3, -3, -3, -2, -2,
                    2, 2,
                ]
                .into_iter(),
                NonZeroU16::new(24).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
            );
            assert_eq!(winding_table, expected_result);
        }

        {
            // This should fail because the number of slots and winding holes does not
            // match.
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(12).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                2,
                false,
            );
            assert!(winding_table.is_err());
        }

        {
            // This should fail because the number of slots is not applicable
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(13).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                2,
                true,
            );
            assert!(winding_table.is_err());
        }

        {
            // This should fail because the number of pole pairs is not applicable
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(12).expect("not zero"),
                NonZeroU16::new(2).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                2,
                true,
            );
            assert!(winding_table.is_err());
        }
    }

    #[test]
    fn test_double_layer_distributed_tooth_coil_assembly() {
        {
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(12).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(2).expect("not zero"),
                2,
                false,
            )
            .unwrap();

            let expected_result = WindingTable::from_layer_major(
                [
                    1, 1, -1, -1, 2, 2, -2, -2, 3, 3, -3, -3, 2, 2, -3, -3, 3, 3, -1, -1, 1, 1, -2,
                    -2,
                ]
                .into_iter(),
                NonZeroU16::new(12).expect("not zero"),
                NonZeroU16::new(2).expect("not zero"),
            );
            assert_eq!(winding_table, expected_result);
        }

        {
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(18).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(2).expect("not zero"),
                3,
                false,
            )
            .unwrap();

            let expected_result = WindingTable::from_layer_major(
                [
                    1, 1, 1, -1, -1, -1, 2, 2, 2, -2, -2, -2, 3, 3, 3, -3, -3, -3, 2, 2, 2, -3, -3,
                    -3, 3, 3, 3, -1, -1, -1, 1, 1, 1, -2, -2, -2,
                ]
                .into_iter(),
                NonZeroU16::new(18).expect("not zero"),
                NonZeroU16::new(2).expect("not zero"),
            );
            assert_eq!(winding_table, expected_result);
        }

        {
            // This should fail because the number of pole pairs is not applicable
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(12).expect("not zero"),
                NonZeroU16::new(4).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(2).expect("not zero"),
                3,
                true,
            );
            assert!(winding_table.is_err());
        }
    }
}
