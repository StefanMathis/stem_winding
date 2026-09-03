use dyn_clone::clone_box;
use nalgebra::DMatrix;
use num::rational::Ratio;
use num::Integer;
use slot::CoilLayout;
use wire::{IsWire, RoundWire};


use magnetic_core::CoreRef;


use stem_primitives::Overrides;


use uom::si::f64::*;

use compare_variables::compare_variables;

#[cfg(feature = "serde")]
use deserialize_untagged_verbose_error::DeserializeAlias;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::CoilBuilder;
use crate::{
    hole_number, periodicity, zones, Coil, CoilFull, Coils, Connection, Winding, Zone, WindingTable,
    WindingTableMethod,
};

// Shorter aliases
use slot::coil_layout;
pub const LL: u16 = coil_layout::QUADRUPLE_LAYER_BOTTOM_LEFT;
pub const UL: u16 = coil_layout::QUADRUPLE_LAYER_TOP_LEFT;
pub const UR: u16 = coil_layout::QUADRUPLE_LAYER_TOP_RIGHT;
pub const LR: u16 = coil_layout::QUADRUPLE_LAYER_BOTTOM_RIGHT;

/**
Representation of a quadruple-layer tooth coil winding as presented in [Kim14] and [Alb11].

The setup of a quadruple layer winding is explained below with the example of a 9/8 configuration (see [Alb11]).
The following schematic shows the first four slots of a non-shifted configuration, in which case the electromagnetic properties are identical to those of a 9/8 double-layer tooth-coil winding:

 +1  +1 | -1  +2 | -2  -2 | +2  +2   <- upper layer (at slot top, next to slot opening)
 +1  +1 | -1  +2 | -2  -2 | +2  +2   <- lower layer (at slot bottom)
        |        |        |
 slot 0 | slot 1 | slot 2 | slot 3

The layers are indexed as follows:

  1  2  |  1  2  |
  0  3  |  0  3  |
        |        |
 slot 0 | slot 1 | ...

The sum of turns in layer 0 and layer 1 must be equal to `turns_slot_side`. The same is true for layer 2 and 3.
Therefore, the sum of all turns in a slot is always equal to 2*`turns_slot_side`.

By defining a vector of shifted upper layer turns (`turns_upper_layer_coils`), the upper layer is shifted against the lower layer by the vector length
and the number of turns in the upper layer are defined by the respective values in this vector.
The maximum vector length must be smaller than z, where z is the numerator of the number of slots per pole and phase q = z/n.
It is also possible to have an empty vector as an input, which results in no layer shift.

For further explanation, consider the two examples below:

#Example 1: 9/8 with a one-slot shift of the upper layer.

`let turns_slot_side = 9;`
`let turns_upper_layer_coils = vec![3];`
This input gives the constructor the following information: Shift the upper layer by 1 (since `turns_upper_layer_coils.len()` equals 1)
and use three turns in the shifted layer. The number of turns in the lower layer is adjusted correspondingly.

Phases

 +1  +1 | -1  +2 | -2  -2 | +2  +2
 +1  +1 | -1  -1 | +1  -2 | +2  +2
        |        |        |
 slot 0 | slot 1 | slot 2 | slot 3

Turns per coil

  5  5  |  5  6  |  6  5  |  5  5
  4  4  |  4  3  |  3  4  |  4  4
        |        |        |
 slot 0 | slot 1 | slot 2 | slot 3

#Example 2: 9/8 with a two-slot shift of the upper layer.

`let turns_slot_side = 10;`
`let turns_upper_layer_coils = vec![4,3]`

This input gives the constructor the following information: Shift the upper layer by 2 (since `turns_upper_layer_coils.len()` equals 2)
and use four turns in the first coil of the shifted layer, three coils in the second coil. Counting is started from the non-shifted lower layer.

Phases

 +1  +1 | -1  +2 | -2  -2 | +2  +2
 -3  +1 | -1  -1 | +1  +1 | -1  +2
        |        |        |
 slot 0 | slot 1 | slot 2 | slot 3

Turns per coil

  7  5  |  5  6  |  6  7  |  7  5
  3  5  |  5  4  |  4  3  |  3  5
        |        |        |
 slot 0 | slot 1 | slot 2 | slot 3
 */
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, DeserializeAlias))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "serde_impl::WindingQuadrupleLayerToothCoilVariants")
)]
#[cfg_attr(
    feature = "serde",
    deserialize_alias(name = "NewWithWindingTable", error = "stem_primitives::Error")
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct WindingQuadrupleLayerToothCoil {
    slots: u16,                 // Inner of slots
    pole_pairs: u16,            // Inner of pole pairs
    phases: u16,                // Inner of phases
    turns_per_slot_side: usize, // Inner of turns per slot side
    turns_upper_layer_coils: Vec<usize>, /* Inner of turns per coil for the shifted coils,
                                 * starting with the innermost coil of the winding
                                 * zone */
    parallel_paths: u16, // Inner of parallel paths
    connection: Connection, /* Connection type, e.g. star (Y), delta (D) or
                          * combinations like YY (double star) or DY
                          * (star-delta) */
    end_winding_leakage_coefficient: f64, // End winding flux leakage coefficient
    wire: Box<dyn IsWire>,                // Wire motor
    coils: Coils,
}

impl WindingQuadrupleLayerToothCoil {
    pub fn new(
        slots: u16,
        pole_pairs: u16,
        phases: u16,
        turns_per_slot_side: usize,
        turns_upper_layer_coils: Vec<usize>,
        parallel_paths: u16,
        connection: Connection,
        end_winding_leakage_coefficient: f64,
        wire: Box<dyn IsWire>,
        winding_table_method: WindingTableMethod,
    ) -> stem_primitives::Result<Self> {
        // Sanity checks
        compare_variables!(0 < slots)?;
        compare_variables!(0 < pole_pairs)?;
        compare_variables!(0 < parallel_paths)?;
        compare_variables!(0 < phases)?;
        compare_variables!(0 < phases)?;
        compare_variables!(1 < turns_per_slot_side)?;
        compare_variables!(0.0 <= end_winding_leakage_coefficient)?;

        let upper_layer_shift = turns_upper_layer_coils.len();

        // Check if the upper layer shift is smaller than the numerator of the number of
        // slots per pole and phase.
        let hole_number_val = *hole_number(slots, pole_pairs, phases).numer() as usize;
        if upper_layer_shift >= hole_number_val {
            compare_variables!(upper_layer_shift < hole_number_val as hole_number)?;
        }

        for turn in turns_upper_layer_coils.iter() {
            if *turn >= turns_per_slot_side {
                return Err(stem_primitives::ErrorType::Other(
                    "Inner of turns per coil must be smaller than number of turns per slot side."
                        .into(),
                )
                .into());
            }
        }

        // Calculate with 2 layers
        let layers = 2;

        // Calculate the basic winding parameters
        let t = periodicity(slots, pole_pairs, phases, layers);
        let slots_basic = slots / t;
        let pole_pairs_basic = pole_pairs / t;

        // Create the zone plan by method
        let winding_table_dl = zones::create_winding_table_by_method(
            winding_table_method,
            slots_basic,
            pole_pairs_basic,
            phases,
            layers,
            1, // Tooth coil winding => throw is 1
            true,
        )?;

        // Add a new third and fourth layer to the zone plan. The third layer is above
        // the first layer (slot side) in the slot, while the fourth layer is above the
        // second layer (slot side)
        let mut winding_table = WindingTable(DMatrix::repeat(4, winding_table_dl.slots().into(), 0));
        for slot in 0..winding_table_dl.slots() {
            for layer in 0..4 {
                winding_table[Zone::new(slot.into(), layer.into())] =
                    winding_table_dl[Zone::new(slot, layer / 2)];
            }
        }

        // Shift the upper layers according to upper_layer_shift. If upper_layer_shift
        // is odd, the layers also need to be inverted, because otherwise the
        // 1st layer would be cancelled by the third layer (same for 2nd and 4th).
        winding_table.shift_layer(upper_layer_shift as i32, UL);
        winding_table.shift_layer(upper_layer_shift as i32, UR);

        if upper_layer_shift.is_odd() {
            winding_table.invert_coils_in_layer(UL);
            winding_table.invert_coils_in_layer(UR);
        }

        let mut winding = WindingQuadrupleLayerToothCoil {
            slots,
            pole_pairs,
            phases,
            turns_per_slot_side,
            turns_upper_layer_coils,
            parallel_paths,
            connection,
            end_winding_leakage_coefficient,
            wire,
            coils: Coils::with_capacity((slots * layers / 2).into()),
        };

        /*
        Assert that all coils of a coil group are located next to each other.
         */
        let coils_per_coil_group = winding.coils_per_coil_group() / 2;
        let mut phase_counter = coils_per_coil_group;
        let mut previous_phase = 0;
        let mut first_coil_group = true;
        for slot in 0..slots_basic {
            // Select the current phase of the first layer, if none is selected
            let current_phase = winding_table_dl.get_wrapped(Zone::new(slot, 0));
            if phase_counter == coils_per_coil_group {
                phase_counter = 1;
            } else {
                if current_phase == -previous_phase {
                    // Current coil belongs to the same coil group
                    phase_counter += 1;
                } else {
                    // Current coil does not belong to the same coil group. If we are in the first
                    // coil group, the loop continues. Otherwise, this is an
                    // indicator of a failed zone plan and the winding
                    // creation is aborted
                    if first_coil_group {
                        first_coil_group = false;
                        phase_counter = 1;
                    } else {
                        return Err(stem_primitives::ErrorType::Other(
                            "All coils of a coil group must be positioned next to each other and have the same polarity inside a slot.".into(),
                        )
                        .into());
                    }
                }
            }
            previous_phase = current_phase;
        }

        winding.create_coils(&winding_table, true)?;
        return winding.check();
    }

    pub fn new_minimal(
        slots: u16,
        pole_pairs: u16,
        phases: u16,
        turns_per_slot_side: usize,
        turns_upper_layer_coils: Vec<usize>,
        winding_table_method: WindingTableMethod,
    ) -> stem_primitives::Result<Self> {
        let parallel_paths = 1;
        let connection = Connection::Star;
        let end_winding_leakage_coefficient = 0.0;

        return WindingQuadrupleLayerToothCoil::new(
            slots,
            pole_pairs,
            phases,
            turns_per_slot_side,
            turns_upper_layer_coils,
            parallel_paths,
            connection,
            end_winding_leakage_coefficient,
            Box::new(RoundWire::default()),
            winding_table_method,
        );
    }

    pub fn upper_layer_shift(&self) -> usize {
        return self.turns_upper_layer_coils.len();
    }

    pub fn turns_per_slot_side(&self) -> usize {
        return self.turns_per_slot_side;
    }

    pub fn turns_upper_layer_coils(&self) -> &[usize] {
        return self.turns_upper_layer_coils.as_slice();
    }

    fn check(self) -> stem_primitives::Result<Self> {
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
    Calculate the number of turns in the given zone
     */
    fn calculated_turns_at(&self, zone: Zone, winding_table: &WindingTable) -> usize {
        /*
        Get the second layer which is on the same slot side as the zone
         */
        let second_layer_on_zone_side = match zone.layer {
            LL => UL,
            UL => LL,
            UR => LR,
            LR => UR,
            _ => panic!("a quadruple layer winding must have 4 layers"),
        };

        /*
        Check if both upper and lower layer have the same phase.
        If yes, this means that the entire side has the same phase and therefore `self.turns_per_slot_side()`
        can be splitted arbitrarily between the two coils.
        If no, the distance of this coil to the next "full" side with the same phase is calculated
        and the number of coils is determined from `self.turns_per_slot_side()` and `self.turns_per_coil()`.
        */
        if winding_table.get_wrapped(zone)
            == winding_table.get_wrapped(Zone::new(zone.slot, second_layer_on_zone_side))
        {
            // Arbitrary split of the number of turns per slot side
            // => Lower layer gets half of the number of turns per slot side (rounded up),
            // upper layer gets half of the number of turns per slot side (rounded down)
            if zone.layer == LL || zone.layer == LR {
                return num::Integer::div_ceil(&self.turns_per_slot_side, &2);
            } else {
                return num::Integer::div_floor(&self.turns_per_slot_side, &2);
            }
        } else {
            let upper_layer_shift = self.upper_layer_shift() as u16;

            // Check if we're in the upper or the lower layer. If we're in the lower layer,
            // we need to search "forwards" for the next full side. If we're in the lower
            // layer, we need to go back.
            let search_forward = zone.layer == LL || zone.layer == LR;

            for idx in 0..upper_layer_shift {
                let offset = idx + 1;

                // Calculate the current "search slot"
                let search_slot = if search_forward {
                    zone.slot + offset
                } else {
                    zone.slot
                        .checked_sub(offset)
                        .unwrap_or(zone.slot + self.slots() - offset)
                };

                /*
                Check if the search slot contains a full half of a single phase
                 */
                // If true, we've found the next full side and can therefore derive the correct
                // value from self.turns_per_coil().
                if winding_table.get_wrapped(Zone::new(search_slot, zone.layer))
                    == winding_table.get_wrapped(Zone::new(search_slot, second_layer_on_zone_side))
                {
                    // For the lower layer, the number of turns per coil is the total number of
                    // turns of the slot side minus the number of turns of the
                    // corresponding upper layer.
                    if search_forward {
                        let turns_in_upper_layer_coil = self.turns_upper_layer_coils()
                            [usize::from(upper_layer_shift - idx - 1)];
                        return self.turns_per_slot_side - turns_in_upper_layer_coil;
                    } else {
                        let turns_in_upper_layer_coil =
                            self.turns_upper_layer_coils()[usize::from(idx)];
                        return turns_in_upper_layer_coil;
                    }
                }
            }
            unreachable!()
        }
    }
}

impl Clone for WindingQuadrupleLayerToothCoil {
    fn clone(&self) -> Self {
        Self {
            slots: self.slots.clone(),
            pole_pairs: self.pole_pairs.clone(),
            phases: self.phases.clone(),
            turns_per_slot_side: self.turns_per_slot_side.clone(),
            turns_upper_layer_coils: self.turns_upper_layer_coils.clone(),
            parallel_paths: self.parallel_paths.clone(),
            connection: self.connection.clone(),
            end_winding_leakage_coefficient: self.end_winding_leakage_coefficient.clone(),
            wire: clone_box(&*self.wire),
            coils: self.coils.clone(),
        }
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Winding for WindingQuadrupleLayerToothCoil {
    fn phases(&self) -> u16 {
        return self.phases;
    }

    fn slots(&self) -> u16 {
        return self.slots;
    }

    fn pole_pairs(&self) -> u16 {
        return self.pole_pairs;
    }

    /// The number of winding layers is always 4 for a quadruple-layer winding
    fn layers(&self) -> u16 {
        return 4;
    }

    fn periodicity(&self) -> u16 {
        return periodicity(self.slots(), self.pole_pairs(), self.phases(), 2);
    }

    fn coil_layout(&self) -> CoilLayout {
        return CoilLayout::Quadruple;
    }

    fn coil_groups_per_phase(&self) -> u16 {
        // Antiparallel coil groups are possible
        let t = self.periodicity();

        if (self.slots() / (t * self.phases())) % 2 == 0 {
            return 2 * t;
        // Only parallel coil groups are possible
        } else {
            return t;
        }
    }

    fn parallel_paths(&self) -> u16 {
        return self.parallel_paths;
    }

    fn turns_per_phase(&self, _phase: u16) -> Ratio<usize> {
        return Ratio::new(
            usize::from(self.layers() * self.slots()) * self.turns_per_slot_side
                / usize::from(2 * self.phases() * self.parallel_paths()),
            1,
        );
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

impl CoilBuilder for WindingQuadrupleLayerToothCoil {
    fn coil_map(&self) -> &Coils {
        return &self.coils;
    }

    fn coil_map_mut(&mut self) -> &mut Coils {
        return &mut self.coils;
    }

    fn create_single_coil(&self, zone: Zone, winding_table: &WindingTable) -> Option<Coil> {
        // Only create a coil if the phase is positive. Since each coil has a positive
        // and a negative phase, in the end all coils will be created anyway.
        // However, only considering positive phases makes the function much
        // simpler.
        let phase = winding_table.get_wrapped(zone);
        if phase < 0 {
            return None;
        }
        let phase_abs: u16 = phase.try_into().expect("the number of phases cannot exceed the maximum value of an u16 (this is guaranteed by the winding constructor)");
        let slot = zone.slot;
        let layer = zone.layer;

        // Search in the opposite layer => See the documentation of `CoilLayout`
        let search_layer = match zone.layer {
            LL => LR,
            UL => UR,
            UR => UL,
            LR => LL,
            _ => panic!("a quadruple layer winding must have 4 layers"),
        };

        let (negative_zone, clockwise) = if layer == LL || layer == UL {
            // Coil must start at the previous slot
            (
                Zone::new(
                    slot.checked_sub(1).unwrap_or(self.slots() - 1),
                    search_layer,
                ),
                false,
            )
        } else {
            // Coil must stop in the next slot
            (
                Zone::new((slot + 1).rem_euclid(self.slots()), search_layer),
                true,
            )
        };

        let turns = self.calculated_turns_at(Zone::new(slot, layer), winding_table);

        return CoilFull::new(
            Zone::new(slot, layer),
            negative_zone,
            true,
            clockwise,
            turns,
            phase_abs,
            clone_box(&*self.wire),
        )
        .ok()
        .map(|coil| Coil::Full(coil));
    }
}

// =================================================================================
// Deserialization

#[cfg_attr(feature = "serde", derive(Deserialize))]
struct NewWindingTableMethod {
    slots: u16,                 // Inner of slots
    pole_pairs: u16,            // Inner of pole pairs
    phases: u16,                // Inner of phases
    turns_per_slot_side: usize, // Inner of turns per slot side
    turns_upper_layer_coils: Vec<usize>, /* Inner of turns per coil for the shifted coils,
                                 * starting with the innermost coil of the winding
                                 * zone */
    parallel_paths: u16, // Inner of parallel paths
    connection: Connection, /* Connection type, e.g. star (Y), delta (D) or
                          * combinations like YY (double star) or DY
                          * (star-delta) */
    end_winding_leakage_coefficient: f64, // End winding flux leakage coefficient
    wire: Box<dyn IsWire>,                // Wire motor
    winding_table_method: WindingTableMethod,
}

impl TryFrom<NewWindingTableMethod> for WindingQuadrupleLayerToothCoil {
    type Error = stem_primitives::Error;

    fn try_from(value: NewWindingTableMethod) -> Result<Self, Self::Error> {
        return Self::new(
            value.slots,
            value.pole_pairs,
            value.phases,
            value.turns_per_slot_side,
            value.turns_upper_layer_coils,
            value.parallel_paths,
            value.connection,
            value.end_winding_leakage_coefficient,
            value.wire,
            value.winding_table_method,
        );
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
struct NewMinimal {
    slots: u16,
    pole_pairs: u16,
    phases: u16,
    turns_per_slot_side: usize,
    turns_upper_layer_coils: Vec<usize>,
    winding_table_method: WindingTableMethod,
}

impl TryFrom<NewMinimal> for WindingQuadrupleLayerToothCoil {
    type Error = stem_primitives::Error;

    fn try_from(value: NewMinimal) -> Result<Self, Self::Error> {
        return Self::new_minimal(
            value.slots,
            value.pole_pairs,
            value.phases,
            value.turns_per_slot_side,
            value.turns_upper_layer_coils,
            value.winding_table_method,
        );
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;
    use deserialize_untagged_verbose_error::{DeserializeUntaggedVerboseError, TryFromEnum};

    #[derive(TryFromEnum, DeserializeUntaggedVerboseError)]
    #[try_from_enum(
        target = "WindingQuadrupleLayerToothCoil",
        error = "stem_primitives::Error"
    )]
    pub(super) enum WindingQuadrupleLayerToothCoilVariants {
        #[try_from_enum(convert_with = "check")]
        NewWithWindingTable(NewWithWindingTable),
        NewWindingTableMethod(NewWindingTableMethod),
        NewMinimal(NewMinimal),
    }

    fn check(alias: NewWithWindingTable) -> stem_primitives::Result<WindingQuadrupleLayerToothCoil> {
        return WindingQuadrupleLayerToothCoil::from(alias).check();
    }
}
