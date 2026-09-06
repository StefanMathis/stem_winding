use std::num::{NonZeroU16, NonZeroUsize};

use compare_variables::compare_variables;
use dyn_clone::clone_box;
use stem_coil_layout::{CoilLayout, Zone};
use stem_wire::{round::RoundWire, wire::Wire};

use crate::{
    coils::{Coil, CoilFull, Coils},
    error::{Error, WindingTableCreationError},
    winding::{Connection, Winding, periodicity},
    winding_table::{WindingTable, WindingTableMethod},
};

// Shorter aliases
const LL: u16 = stem_coil_layout::QUADRUPLE_LAYER_BOTTOM_LEFT;
const UL: u16 = stem_coil_layout::QUADRUPLE_LAYER_TOP_LEFT;
const UR: u16 = stem_coil_layout::QUADRUPLE_LAYER_TOP_RIGHT;
const LR: u16 = stem_coil_layout::QUADRUPLE_LAYER_BOTTOM_RIGHT;

/**
Representation of a quadruple-layer tooth coil winding as presented in [Kim14] and [Alb11].

The setup of a quadruple layer winding is explained below with the example of a 9/8 configunum::rational::Ration (see [Alb11]).
The following schematic shows the first four slots of a non-shifted configunum::rational::Ration, in which case the electromagnetic properties are identical to those of a 9/8 double-layer tooth-coil winding:

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
#[derive(Debug, Clone)]
pub struct QuadrupleLayerToothCoilWinding {
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    phases: NonZeroU16,
    turns_per_slot_side: NonZeroUsize,
    turns_upper_layer_coils: Vec<NonZeroUsize>,
    parallel_paths: NonZeroU16,
    connection: Connection,
    end_winding_leakage_coefficient: f64,
    wire: Box<dyn Wire>,
    coils: Coils,
    winding_table_method: WindingTableMethod,
}

impl QuadrupleLayerToothCoilWinding {
    pub fn upper_layer_shift(&self) -> usize {
        self.turns_upper_layer_coils.len()
    }

    pub fn turns_per_slot_side(&self) -> NonZeroUsize {
        self.turns_per_slot_side
    }

    pub fn turns_upper_layer_coils(&self) -> &[NonZeroUsize] {
        self.turns_upper_layer_coils.as_slice()
    }

    /**
    Calculate the number of turns in the given zone
     */
    fn calculated_turns_at(&self, zone: Zone, winding_table: &WindingTable) -> NonZeroUsize {
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
        if winding_table.get_cyclic(zone)
            == winding_table.get_cyclic(Zone::new(zone.slot, second_layer_on_zone_side))
        {
            // Arbitrary split of the number of turns per slot side
            // => Lower layer gets half of the number of turns per slot side (rounded up),
            // upper layer gets half of the number of turns per slot side (rounded down)
            if zone.layer == LL || zone.layer == LR {
                return num::Integer::div_ceil(&self.turns_per_slot_side.get(), &2)
                    .try_into()
                    .unwrap_or(NonZeroUsize::MIN);
            } else {
                return num::Integer::div_floor(&self.turns_per_slot_side.get(), &2)
                    .try_into()
                    .unwrap_or(NonZeroUsize::MIN);
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
                        .unwrap_or(zone.slot + self.slots().get() - offset)
                };

                /*
                Check if the search slot contains a full half of a single phase
                 */
                // If true, we've found the next full side and can therefore derive the correct
                // value from self.turns_per_coil().
                if winding_table.get_cyclic(Zone::new(search_slot, zone.layer))
                    == winding_table.get_cyclic(Zone::new(search_slot, second_layer_on_zone_side))
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

    fn create_coils(
        &mut self,
        winding_table: &WindingTable,
        all_zones_must_be_used: bool,
    ) -> Result<(), Error> {
        for slot in 0..self.slots.get() {
            for layer in 0..self.layers().get() {
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

        // Check if all zones are occupied
        if all_zones_must_be_used {
            for (zone, _) in winding_table.iter_slots() {
                // Check if the zone is already occupied
                if !self.coils.0.contains_key(&zone) {
                    return Err(WindingTableCreationError::EmptyZone(Some(zone)).into());
                }
            }
        }

        return Ok(());
    }

    fn create_single_coil(&self, zone: Zone, winding_table: &WindingTable) -> Option<Coil> {
        // Only create a coil if the phase is positive. Since each coil has a positive
        // and a negative phase, in the end all coils will be created anyway.
        // However, only considering positive phases makes the function much
        // simpler.
        let phase = *winding_table.get_cyclic(zone);
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
                    slot.checked_sub(1).unwrap_or(self.slots().get() - 1),
                    search_layer,
                ),
                false,
            )
        } else {
            // Coil must stop in the next slot
            (
                Zone::new((slot + 1).rem_euclid(self.slots().get()), search_layer),
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

#[cfg_attr(feature = "serde", typetag::serde)]
impl Winding for QuadrupleLayerToothCoilWinding {
    fn phases(&self) -> NonZeroU16 {
        self.phases
    }

    fn slots(&self) -> NonZeroU16 {
        self.slots
    }

    fn pole_pairs(&self) -> NonZeroU16 {
        self.pole_pairs
    }

    /// The number of winding layers is always 4 for a quadruple-layer winding
    fn layers(&self) -> NonZeroU16 {
        return NonZeroU16::new(4).expect("not zero");
    }

    fn periodicity(&self) -> NonZeroU16 {
        periodicity(
            self.slots(),
            self.pole_pairs(),
            self.phases(),
            NonZeroU16::new(2).expect("not zero"),
        )
    }

    fn coil_layout(&self) -> CoilLayout {
        CoilLayout::Quadruple
    }

    fn coil_groups_per_phase(&self) -> NonZeroU16 {
        // Antiparallel coil groups are possible
        let t = self.periodicity();

        if (self.slots().get() / (t.get() * self.phases().get())) % 2 == 0 {
            return NonZeroU16::new(2 * t.get()).expect("not zero");
        // Only parallel coil groups are possible
        } else {
            return t;
        }
    }

    fn parallel_paths(&self) -> NonZeroU16 {
        self.parallel_paths
    }

    fn turns_per_phase(&self, _phase: NonZeroU16) -> num::rational::Ratio<usize> {
        return num::rational::Ratio::new(
            usize::from(self.layers().get() * self.slots().get()) * self.turns_per_slot_side.get()
                / usize::from(2 * self.phases().get() * self.parallel_paths().get()),
            1,
        );
    }

    fn connection(&self) -> Connection {
        self.connection
    }

    fn end_winding_leakage_coefficient(&self) -> f64 {
        self.end_winding_leakage_coefficient
    }

    fn coil_at(&self, zone: Zone) -> Option<&Coil> {
        self.coils.0.get(&zone)
    }

    fn as_dyn(&self) -> &dyn Winding {
        self
    }

    #[cfg(feature = "stem_core")]
    fn is_symmetric(&self, _core: CoreRef<'_>, _overrides: &Overrides) -> bool {
        return true;
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

// =============================================================================
// Builder

#[cfg_attr(feature = "serde", deserialize(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(skip))]
pub struct QuadrupleLayerToothCoilBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub turns_per_slot_side: NonZeroUsize,
    pub turns_upper_layer_coils: Vec<NonZeroUsize>,
    pub parallel_paths: NonZeroU16,
    pub connection: Connection,
    pub end_winding_leakage_coefficient: f64,
    pub wire: Box<dyn Wire>,
    pub winding_table_method: WindingTableMethod,
}

impl TryFrom<QuadrupleLayerToothCoilBuilder> for QuadrupleLayerToothCoilWinding {
    type Error = Error;

    fn try_from(value: QuadrupleLayerToothCoilBuilder) -> Result<Self, Self::Error> {
        // Sanity checks
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
        let mut winding_table =
            WindingTable(DMatrix::repeat(4, winding_table_dl.slots().into(), 0));
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

        let mut winding = QuadrupleLayerToothCoilWinding {
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
            let current_phase = winding_table_dl.get_cyclic(Zone::new(slot, 0));
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

#[cfg_attr(feature = "serde", deserialize(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(skip))]
pub struct QuadrupleLayerToothCoilMinimalBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub turns_per_slot_side: NonZeroUsize,
    pub turns_upper_layer_coils: Vec<NonZeroUsize>,
    pub winding_table_method: WindingTableMethod,
}

impl TryFrom<QuadrupleLayerToothCoilMinimalBuilder> for QuadrupleLayerToothCoilWinding {
    type Error = Error;

    fn try_from(builder: QuadrupleLayerToothCoilMinimalBuilder) -> Result<Self, Self::Error> {
        QuadrupleLayerToothCoilBuilder {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            turns_per_slot_side: builder.turns_per_slot_side,
            turns_upper_layer_coils: builder.turns_upper_layer_coils,
            parallel_paths: NonZeroU16::MIN,
            connection: Connection::Star,
            end_winding_leakage_coefficient: 0.0,
            wire: Box::new(RoundWire::default()),
            winding_table_method: builder.winding_table_method,
        }
        .try_into()
    }
}
