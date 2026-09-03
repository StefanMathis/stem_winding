use dyn_clone::clone_box;
use num::Integer;
use num::rational::Ratio;
use slot::CoilLayout;
use wire::{IsWire, RoundWire};

use compare_variables::compare_variables;

#[cfg(feature = "serde")]
use deserialize_untagged_verbose_error::DeserializeAlias;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};


use magnetic_core::CoreRef;


use stem_primitives::Overrides;


use uom::si::f64::*;

use crate::CoilBuilder;
use crate::iterators::HarmonicOrdinalsIterator;
use crate::{Coil, CoilFull, Coils, Connection, Winding, WindingTable, Zone, zones};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, DeserializeAlias))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "serde_impl::WindingDistributedToothCoilVariants")
)]
#[cfg_attr(
    feature = "serde",
    deserialize_alias(name = "NewWithWindingTable", error = "stem_primitives::Error")
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct WindingDistributedToothCoil {
    slots: u16,      // Inner of slots
    pole_pairs: u16, // Inner of pole pairs
    phases: u16,     // Inner of phases
    layers: u16,     // Inner of layers
    coil_group_turns: Vec<usize>, /* Inner of turns per coil, starting with the
                      * innermost coil of a coil group */
    parallel_paths: u16, // Inner of parallel paths
    connection: Connection, /* Connection type, e.g. star (Y), delta (D) or
                          * combinations like YY (double star) or DY
                          * (star-delta) */
    end_winding_leakage_coefficient: f64, // End winding flux leakage coefficient
    wires: Vec<Box<dyn IsWire>>,          // Wire motor
    coils: Coils,
    periodicity: u16,
    double_zone_span: bool,
}

impl WindingDistributedToothCoil {
    /// Create a new distributed tooth-coil winding
    pub fn new(
        slots: u16,
        pole_pairs: u16,
        phases: u16,
        layers: u16,
        coil_group_turns: Vec<usize>,
        parallel_paths: u16,
        connection: Connection,
        end_winding_leakage_coefficient: f64,
        wires: Vec<Box<dyn IsWire>>,
        double_zone_span: bool,
    ) -> stem_primitives::Result<Self> {
        if coil_group_turns.len() == 0 {
            return Err(stem_primitives::ErrorType::EmptyInputCollection(Some(
                "coil_group_turns".into(),
            ))
            .into());
        }

        // Assert that the wire vector and coil_group_turns have the same length
        if coil_group_turns.len() != wires.len() {
            return Err(stem_primitives::ErrorType::Other(
                "Vectors coil_group_turns and wires must have the same length".into(),
            )
            .into());
        }

        // Try creating the zone plan for the basic pole pair number (which is
        // calculated by winding_holes)
        let coils_per_coil_group = coil_group_turns.len() as u16;
        let slots_winding_table = if double_zone_span {
            2 * coils_per_coil_group * phases / layers
        } else {
            4 * coils_per_coil_group * phases / layers
        };

        let periodicity = slots / slots_winding_table;
        if pole_pairs % periodicity != 0 {
            return Err(stem_primitives::ErrorType::Other(
                "Given number of pole pairs does not create a functioning winding".into(),
            )
            .into());
        }

        let mut winding_table = zones::distributed_tooth_coil_assembly(
            slots_winding_table,
            1,
            phases,
            layers,
            coils_per_coil_group,
            double_zone_span,
        )?;

        // Create the winding
        let mut winding = WindingDistributedToothCoil {
            slots,
            pole_pairs,
            phases,
            layers,
            coil_group_turns,
            parallel_paths,
            connection,
            end_winding_leakage_coefficient,
            wires,
            coils: Coils::with_capacity((slots * layers / 2).into()),
            periodicity,
            double_zone_span,
        };

        // Create the coils from the zone plan
        winding.create_coils(&winding_table, true)?;

        // Invert the zone plan if the first harmonic ordinal is negative. The reasoning
        // for this is: The distributed tooth-coil windings are created from
        // very short-pitched integer slot windings. If the pole pair harmonic
        // wave wanders in the opposite direction as the harmonic of the integer
        // winding, the zone plan direction needs to be reversed. For an integer
        // winding, the first harmonic equals the pole pair harmonic.
        // Because distributed tooth-coil windings operate on a super harmonic of the
        // integer winding they are based on, the zone plan direction must be
        // reversed if the first harmonic is negative.
        let harmonic_ordinals = HarmonicOrdinalsIterator::new_unchecked(&winding);
        match harmonic_ordinals.coupling_with_pole_pairs() {
            Some(coupling) => {
                if coupling < 0 {
                    winding_table.reverse();
                    winding_table.shift_layers(2 * coils_per_coil_group as i32);
                }
            }
            None => {
                return Err(stem_primitives::ErrorType::Other(
                    "The winding does not create a harmonic for the given number of pole pairs."
                        .into(),
                )
                .into());
            }
        }

        // Recreate the coils from the adjusted zone plan
        winding.create_coils(&winding_table, true)?;

        // Check if the number of parallel paths is valid
        let mut valid = false;
        for pp in winding.possible_parallel_paths() {
            if pp == parallel_paths as usize {
                valid = true
            }
        }
        if !valid {
            return Err(stem_primitives::ErrorType::Other(
                "Given number of parallel paths not possible.".into(),
            )
            .into());
        }
        return Ok(winding);
    }

    pub fn coils_per_coil_group(&self) -> u16 {
        return self.coil_group_turns.len() as u16;
    }

    pub fn parallel_paths(&self) -> u16 {
        return self.parallel_paths;
    }

    pub fn double_zone_span(&self) -> bool {
        return self.double_zone_span;
    }

    pub fn coil_group_turns(&self) -> &[usize] {
        return self.coil_group_turns.as_slice();
    }

    /**
    Create a turn distribution for a double-layer distributed winding which keeps the number of turns per slot constant.

    # Example
    ```
    use winding::WindingDistributedToothCoil;

    // Three coils per coil group, 10 turns per slot (5 per coil). The inner coil gets 5 - 2 = 3 turns, the middle coil gets 5 turns, the outer coil gets 5 + 2 = 7 turns.
    let turns = WindingDistributedToothCoil::double_layer_turn_distribution(3, 10, 2).unwrap();
    assert_eq!(turns[0], 3);
    assert_eq!(turns[1], 5);
    assert_eq!(turns[2], 7);

    // Four coils per coil group, 10 turns per slot. The inner coils get 3 turns, the outer coils get 7 turns.
    let turns = WindingDistributedToothCoil::double_layer_turn_distribution(4, 10, 2).unwrap();
    assert_eq!(turns[0], 3);
    assert_eq!(turns[1], 3);
    assert_eq!(turns[2], 7);
    assert_eq!(turns[3], 7);

    // Function errors for the following cases: turns per slot is zero or odd, absolute value of turn_difference equal to or greater than turns_per_slot
    assert!(WindingDistributedToothCoil::double_layer_turn_distribution(3, 0, 3).is_err());
    assert!(WindingDistributedToothCoil::double_layer_turn_distribution(3, 9, 3).is_err());
    assert!(WindingDistributedToothCoil::double_layer_turn_distribution(3, 10, 5).is_err());
    ```
     */
    pub fn double_layer_turn_distribution(
        coils_per_coil_group: u16,
        turns_per_slot: usize,
        turn_difference: i32,
    ) -> stem_primitives::Result<Vec<usize>> {
        compare_variables!(0 < coils_per_coil_group)?;
        compare_variables!(0 < turns_per_slot)?;

        if turns_per_slot.is_odd() {
            return Err(stem_primitives::ErrorType::Other(
                "The number of turns per slot must be even".into(),
            )
            .into());
        }

        if !(turns_per_slot as i32 > 2 * turn_difference.abs()) {
            return Err(stem_primitives::ErrorType::Other(
                "The turns_per_slot must be greater than twice the absolute value of turn_difference.".into(),
            )
            .into());
        }

        // Create the turns per coil vector from coil_group_turns and turn_difference as
        // described in [Hut19], p. 137.
        let mut coil_group_turns: Vec<usize> =
            vec![turns_per_slot / 2; coils_per_coil_group as usize];
        for hole in 0..coils_per_coil_group {
            // If q_star is odd, the center value of the vector does not change
            if 2 * hole == coils_per_coil_group - 1 {
                continue;
            // Inner coils: Reduce the number of turns per coil
            } else if 2 * hole < coils_per_coil_group - 1 {
                coil_group_turns[hole as usize] =
                    (turns_per_slot as i32 / 2 - turn_difference) as usize;
            // Outer coils: Increase the number of turns per coil
            } else {
                coil_group_turns[hole as usize] =
                    (turns_per_slot as i32 / 2 + turn_difference) as usize;
            }
        }
        return Ok(coil_group_turns);
    }

    fn calculated_turns_at(
        &self,
        zone: Zone,
        _winding_table: &WindingTable,
    ) -> stem_primitives::Result<usize> {
        // The number of turns is encoded in the vector self.coil_group_turns as
        // follows: First entry is the innermost coil, second entry is the
        // second-innermost coil and so on.
        let offset = if zone.layer == 0 {
            0
        } else {
            self.coils_per_coil_group()
        };
        let normalized_slot = (zone.slot + offset) % (2 * self.coils_per_coil_group());
        if normalized_slot < self.coils_per_coil_group() {
            return Ok(
                self.coil_group_turns[(self.coils_per_coil_group() - 1 - normalized_slot) as usize]
            );
        } else {
            return Ok(
                self.coil_group_turns[(normalized_slot % (2 * self.coils_per_coil_group())
                    - self.coils_per_coil_group()) as usize],
            );
        }
    }
}

impl Clone for WindingDistributedToothCoil {
    fn clone(&self) -> Self {
        // Clone the wires vector manually
        let mut wires = Vec::with_capacity(self.wires.len());
        for wire in &self.wires {
            wires.push(clone_box(&**wire));
        }
        Self {
            slots: self.slots.clone(),
            pole_pairs: self.pole_pairs.clone(),
            phases: self.phases.clone(),
            layers: self.layers.clone(),
            coil_group_turns: self.coil_group_turns.clone(),
            parallel_paths: self.parallel_paths.clone(),
            connection: self.connection.clone(),
            end_winding_leakage_coefficient: self.end_winding_leakage_coefficient.clone(),
            wires,
            coils: self.coils.clone(),
            periodicity: self.periodicity.clone(),
            double_zone_span: self.double_zone_span.clone(),
        }
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Winding for WindingDistributedToothCoil {
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

    fn periodicity(&self) -> u16 {
        return self.periodicity;
    }

    fn coil_layout(&self) -> CoilLayout {
        if self.layers() == 1 {
            return CoilLayout::Single;
        } else {
            return CoilLayout::DoubleVertical;
        }
    }

    /// Returns the hole number representation q = g + z/n where g, z and n are
    /// integers in order [g, z, n].
    fn hole_number(&self) -> Ratio<u16> {
        return Ratio::new(2 * self.coils_per_coil_group(), 1);
    }

    fn turns_per_phase(&self, _phase: u16) -> Ratio<usize> {
        let all_turns: usize = self.coil_group_turns.iter().sum();
        return Ratio::new(
            (all_turns * usize::from(self.coil_groups_per_phase()))
                / usize::from(self.parallel_paths()),
            1,
        );
    }

    fn connection(&self) -> Connection {
        return self.connection;
    }

    fn end_winding_leakage_coefficient(&self) -> f64 {
        return self.end_winding_leakage_coefficient;
    }

    /// The number of parallel paths of this winding type is alwaystwice the
    /// number of basic windings.
    fn parallel_paths(&self) -> u16 {
        return self.periodicity() * 2;
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

impl CoilBuilder for WindingDistributedToothCoil {
    fn coil_map(&self) -> &Coils {
        return &self.coils;
    }

    fn coil_map_mut(&mut self) -> &mut Coils {
        return &mut self.coils;
    }

    fn create_single_coil(&self, zone: Zone, winding_table: &WindingTable) -> Option<Coil> {
        let phase = winding_table.get_wrapped(zone);
        let slot = zone.slot;
        let layer = zone.layer;

        let mut return_slot: Option<u16> = None;
        let mut clockwise: Option<bool> = None;
        let mut index: Option<u16> = None;

        // Search through the neighboring slots, but stay in the same layer
        for k in 1..(self.coils_per_coil_group() + 1) {
            let search_slot = (k + slot).rem_euclid(self.slots());

            if self
                .coil_map()
                .0
                .contains_key(&Zone::new(search_slot, layer))
            {
                break;
            }

            // Detect direction change in search_slot: Use the slot in position q-k+1
            if winding_table.get_wrapped(Zone::new(search_slot, layer)) == -phase {
                return_slot = Some((slot + 2 * k - 1).rem_euclid(self.slots()));
                clockwise = Some(true);
                index = Some(k - 1);
                break;
            }

            // Detect phase change in search_slot: Use the slot in position k
            if winding_table
                .get_wrapped(Zone::new(search_slot, layer))
                .abs()
                != phase.abs()
            {
                return_slot = Some(
                    (slot as i32 - 2 * self.coils_per_coil_group() as i32 + 2 * k as i32 - 1)
                        .rem_euclid(self.slots() as i32) as u16,
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
                if phase > 0 {
                    return CoilFull::new(
                        Zone::new(slot, layer),
                        Zone::new(return_slot, layer),
                        true,
                        clockwise,
                        self.calculated_turns_at(Zone::new(slot, layer), winding_table)
                            .unwrap(),
                        phase.try_into().unwrap(),
                        clone_box(&*self.wires[index as usize]),
                    )
                    .ok()
                    .map(|coil| Coil::Full(coil));
                } else {
                    return CoilFull::new(
                        Zone::new(slot, layer),
                        Zone::new(return_slot, layer),
                        false,
                        !clockwise,
                        self.calculated_turns_at(Zone::new(slot, layer), winding_table)
                            .unwrap(),
                        phase.abs().try_into().unwrap(),
                        clone_box(&*self.wires[index as usize]),
                    )
                    .ok()
                    .map(|coil| Coil::Full(coil));
                }
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
    target = "WindingDistributedToothCoil",
    fn_name = "new_minimal",
    error = "stem_primitives::Error"
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
struct NewMinimal {
    slots: u16,
    pole_pairs: u16,
    phases: u16,
    layers: u16,
    coil_group_turns: Vec<usize>,
    parallel_paths: u16,
    double_zone_span: bool,
}

impl TryFrom<NewMinimal> for WindingDistributedToothCoil {
    type Error = stem_primitives::Error;

    fn try_from(value: NewMinimal) -> Result<Self, Self::Error> {
        let coils_per_coil_group = value.coil_group_turns.len();
        let mut wires: Vec<Box<dyn IsWire>> = Vec::with_capacity(coils_per_coil_group);
        for _ in 0..coils_per_coil_group {
            wires.push(Box::new(RoundWire::default()));
        }

        return WindingDistributedToothCoil::new(
            value.slots,
            value.pole_pairs,
            value.phases,
            value.layers,
            value.coil_group_turns,
            value.parallel_paths,
            Connection::Star,
            0.0,
            wires,
            value.double_zone_span,
        );
    }
}

#[derive(constructor)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[constructor(
    target = "WindingDistributedToothCoil",
    fn_name = "new_minimal_double_layer_with_turn_difference",
    error = "stem_primitives::Error"
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
struct NewMinimalDoubleLayerTurnDifference {
    slots: u16,
    pole_pairs: u16,
    phases: u16,
    mean_coil_group_turns: usize,
    turn_difference: i32,
    coils_per_coil_group: u16,
    parallel_paths: u16,
    double_zone_span: bool,
}

impl TryFrom<NewMinimalDoubleLayerTurnDifference> for WindingDistributedToothCoil {
    type Error = stem_primitives::Error;

    fn try_from(value: NewMinimalDoubleLayerTurnDifference) -> Result<Self, Self::Error> {
        let layers = 2;
        let coil_group_turns = WindingDistributedToothCoil::double_layer_turn_distribution(
            value.coils_per_coil_group,
            value.mean_coil_group_turns,
            value.turn_difference,
        )?;
        return Self::new_minimal(
            value.slots,
            value.pole_pairs,
            value.phases,
            layers,
            coil_group_turns,
            value.parallel_paths,
            value.double_zone_span,
        );
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;
    use deserialize_untagged_verbose_error::{DeserializeUntaggedVerboseError, TryFromEnum};

    #[derive(TryFromEnum, DeserializeUntaggedVerboseError)]
    #[try_from_enum(
        target = "WindingDistributedToothCoil",
        error = "stem_primitives::Error"
    )]
    pub(super) enum WindingDistributedToothCoilVariants {
        #[try_from_enum(into)]
        NewWithWindingTable(NewWithWindingTable),
        NewMinimal(NewMinimal),
        NewMinimalDoubleLayerTurnDifference(NewMinimalDoubleLayerTurnDifference),
    }
}

/// Returns a zone plan for a single-layer distributed tooth-coil winding.
/// The algorithm is taken from [Hut21].
pub(crate) fn distributed_tooth_coil_assembly(
    slots: u16,
    pole_pairs: u16,
    phases: u16,
    layers: u16,
    coils_per_coil_group: u16,
    double_zone_span: bool,
) -> Result<WindingTable, Error> {
    if double_zone_span {
        if layers * slots != 2 * phases * pole_pairs * coils_per_coil_group {
            return Err(stem_primitives::ErrorType::WindingTableCreationFailed(
                "The number of slots must be a multiple of the phases times the pole pairs times the winding holes (for a double zone-span winding)."
                    .into(),
            )
            .into());
        }
    } else {
        if layers * slots != 4 * phases * pole_pairs * coils_per_coil_group {
            return Err(stem_primitives::ErrorType::WindingTableCreationFailed(
                "The number of slots must be a multiple of 2 times the phases times the pole pairs times the winding holes (for a normal winding)."
                    .into(),
            )
            .into());
        }
    }

    let span: i32 = if layers == 1 {
        coils_per_coil_group as i32 * (phases as i32 + 1)
    } else {
        coils_per_coil_group as i32
    };

    // Create a double-layer zone plan with 2*m zones and a single pole pair
    let mut winding_table = match winding_distribution_table(slots, pole_pairs, phases, 2, span) {
        Ok(zp) => zp,
        Err(msg) => return Err(msg),
    };

    let winding_table = if layers == 1 {
        // Winding with m zones as discussed in [Hut19], section 2.1
        if double_zone_span {
            // Convert to m zones
            winding_table.shift_zones(coils_per_coil_group as i32);
        }

        // Derived the distributed tooth-coil winding from the integer-slot winding as
        // shown in [Hut21], fig. 1
        for slot in 0..slots {
            if (slot).rem_euclid(2 * coils_per_coil_group) >= coils_per_coil_group {
                winding_table[Zone::new(slot, 0)] = -winding_table[Zone::new(slot, 0)]
            }
        }

        // Remove the lower layer (second row)
        WindingTable(winding_table.0.remove_row(1))
    } else {
        // Move the return conductors of the distributed tooth-coil winding in the
        // same layer as the original conductors of each phase. This means exchanging
        // upper and lower layers for every second winding zone defined by the number
        // of winding holes q.
        for slot in 0..slots {
            if (slot).rem_euclid(2 * coils_per_coil_group) >= coils_per_coil_group {
                let ul = winding_table[Zone::new(slot, 0)];
                let ll = winding_table[Zone::new(slot, 1)];
                winding_table[Zone::new(slot, 0)] = ll;
                winding_table[Zone::new(slot, 1)] = ul;
            }
        }

        if double_zone_span {
            // Convert to m zones by inverting every phase in the second layer
            for slot in 0..slots {
                winding_table[Zone::new(slot, 1)] = -winding_table[Zone::new(slot, 1)];
            }
        }
        winding_table
    };
    return Ok(winding_table);
}

#[test]
fn test_single_layer_distributed_tooth_coil_assembly() {
    {
        let winding_table = distributed_tooth_coil_assembly(12, 1, 3, 1, 2, true).unwrap();
        let expected_result = WindingTable(DMatrix::from_row_slice(
            1,
            12,
            &[1, 1, -1, -1, 2, 2, -2, -2, 3, 3, -3, -3],
        ));
        assert_eq!(winding_table, expected_result);
    }

    {
        let winding_table = distributed_tooth_coil_assembly(18, 1, 3, 1, 3, true).unwrap();
        let expected_result = WindingTable(DMatrix::from_row_slice(
            1,
            18,
            &[
                1, 1, 1, -1, -1, -1, 2, 2, 2, -2, -2, -2, 3, 3, 3, -3, -3, -3,
            ],
        ));
        assert_eq!(winding_table, expected_result);
    }

    {
        let winding_table = distributed_tooth_coil_assembly(24, 1, 3, 1, 2, false).unwrap();
        let expected_result = WindingTable(DMatrix::from_row_slice(
            1,
            24,
            &[
                1, 1, -1, -1, -3, -3, 3, 3, 2, 2, -2, -2, -1, -1, 1, 1, 3, 3, -3, -3, -2, -2, 2, 2,
            ],
        ));
        assert_eq!(winding_table, expected_result);
    }

    {
        // This should fail because the number of slots and winding holes does not
        // match.
        let winding_table = distributed_tooth_coil_assembly(12, 1, 3, 1, 2, false);
        assert!(winding_table.is_err());
    }

    {
        // This should fail because the number of slots is not applicable
        let winding_table = distributed_tooth_coil_assembly(13, 1, 3, 1, 2, true);
        assert!(winding_table.is_err());
    }

    {
        // This should fail because the number of pole pairs is not applicable
        let winding_table = distributed_tooth_coil_assembly(12, 2, 3, 1, 2, true);
        assert!(winding_table.is_err());
    }
}

#[test]
fn test_double_layer_distributed_tooth_coil_assembly() {
    {
        let winding_table = distributed_tooth_coil_assembly(12, 1, 3, 2, 2, false).unwrap();
        let expected_result = WindingTable(DMatrix::from_row_slice(
            2,
            12,
            &[
                1, 1, -1, -1, 2, 2, -2, -2, 3, 3, -3, -3, 2, 2, -3, -3, 3, 3, -1, -1, 1, 1, -2, -2,
            ],
        ));
        assert_eq!(winding_table, expected_result);
    }

    {
        let winding_table = distributed_tooth_coil_assembly(18, 1, 3, 2, 3, false).unwrap();
        let expected_result = WindingTable(DMatrix::from_row_slice(
            2,
            18,
            &[
                1, 1, 1, -1, -1, -1, 2, 2, 2, -2, -2, -2, 3, 3, 3, -3, -3, -3, 2, 2, 2, -3, -3, -3,
                3, 3, 3, -1, -1, -1, 1, 1, 1, -2, -2, -2,
            ],
        ));
        assert_eq!(winding_table, expected_result);
    }

    {
        // This should fail because the number of pole pairs is not applicable
        let winding_table = distributed_tooth_coil_assembly(12, 4, 3, 2, 3, true);
        assert!(winding_table.is_err());
    }
}
