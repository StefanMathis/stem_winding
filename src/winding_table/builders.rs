use std::num::NonZeroU16;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use num::Integer;
use stem_coil_layout::Zone;

use crate::{
    error::WindingTableCreationError,
    winding::{hole_number, phase_sequence},
    winding_table::WindingTable,
};

/**
This enum contains the options available for the zone plan creation of a single- or double-layer winding.
The following methods are currently implemented:
* `Tingley`: Method for single- and double layer windings with two or three phases according to [Seq50], p.186f.
* `Zone`: Method for distributed single layer windings. Further details in [Hut20a]
* `AlgebraicAlgorithm`: Method for single- and double layer windings. This method is taken from [Kre88].
Be aware that this method may result in incorrect configurations, so check the zone plan before proceeding.
* `StarOfSlots`: Method for single- and double layer windings according to [Bia06].
* `DistributionTable`: Method for single- and double layer windings according to [Car18].
 */

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum WindingTableMethod {
    /// Seq50
    Tingley,
    CoilSide,
    AlgebraicAlgorithm,
    StarOfSlots,
    DistributionTable,
}

impl WindingTableMethod {
    /// Return a string representation of the zone plan method
    pub fn as_str(&self) -> &str {
        match self {
            WindingTableMethod::Tingley => return "Tingley",
            WindingTableMethod::CoilSide => return "Coil side",
            WindingTableMethod::AlgebraicAlgorithm => return "Algebraic algorithm",
            WindingTableMethod::StarOfSlots => return "Star of slots",
            WindingTableMethod::DistributionTable => return "Winding distribution table",
        }
    }

    pub fn from_str(string: &str) -> Option<Self> {
        match string {
            "Tingley" => return Some(WindingTableMethod::Tingley),
            "Coil side" => return Some(WindingTableMethod::CoilSide),
            "Algebraic algorithm" => return Some(WindingTableMethod::AlgebraicAlgorithm),
            "Star of slots" => return Some(WindingTableMethod::StarOfSlots),
            "Winding distribution table" => {
                return Some(WindingTableMethod::DistributionTable);
            }
            _ => return None,
        }
    }

    pub fn iter() -> impl Iterator<Item = WindingTableMethod> {
        return [
            Self::Tingley,
            Self::CoilSide,
            Self::AlgebraicAlgorithm,
            Self::StarOfSlots,
            Self::DistributionTable,
        ]
        .into_iter();
    }
}

impl WindingTable {
    pub fn with_method(
        winding_table_method: &WindingTableMethod,
        slots: NonZeroU16,
        layers: NonZeroU16,
        pole_pairs: NonZeroU16,
        phases: NonZeroU16,
        span: i32,
    ) -> Result<Self, WindingTableCreationError> {
        return match winding_table_method {
            WindingTableMethod::Tingley => {
                if u16::from(layers) == 1 && span.abs() == 1 {
                    Self::tingley_single_layer_tooth_coil(slots, pole_pairs, phases)
                } else {
                    Self::tingley(slots, layers, pole_pairs, phases, span)
                }
            }
            WindingTableMethod::CoilSide => {
                Self::coil_side(slots, layers, pole_pairs, phases, span)
            }
            WindingTableMethod::AlgebraicAlgorithm => {
                Self::algebraic_algorithm(slots, layers, pole_pairs, phases, span)
            }
            WindingTableMethod::StarOfSlots => {
                Self::star_of_slots(slots, layers, pole_pairs, phases, span)
            }
            WindingTableMethod::DistributionTable => {
                Self::distribution_table(slots, layers, pole_pairs, phases, span)
            }
        };
    }

    /// Returns a zone plan created with the tingley pattern for a single-layer
    /// tooth coil In the case of single-layer tooth-coil windings, the number
    /// of teeth is halfed for the calculation and a double-layer zone plan
    /// is created with the Tingley pattern. Then, additional teeth are
    /// inserted between the coils to get the original number of teeth back
    pub(crate) fn tingley_single_layer_tooth_coil(
        slots: NonZeroU16,
        pole_pairs: NonZeroU16,
        phases: NonZeroU16,
    ) -> Result<Self, WindingTableCreationError> {
        let slots_num = u16::from(slots);

        if slots_num.is_odd() {
            return Err(WindingTableCreationError::SingleLayerOddSlotNumber);
        }

        // Half the number of slots and double the number of layers
        // This works because a single-layer winding needs an even number of slots.
        // Create the zone plan with the adjusted slots (with the Tingley-pattern)
        if let Some(halfed_slots) = NonZeroU16::new(slots_num / 2) {
            match Self::tingley(
                halfed_slots,
                NonZeroU16::new(2).expect("is not zero"),
                pole_pairs,
                phases,
                1,
            ) {
                Ok(mut winding_table_dl) => {
                    // "Flatten" the tingley pattern, doubling the number of
                    // slots and halfing the number of layers in the process
                    // This compensates the aforementioned halfing of the slots.
                    winding_table_dl.layers = NonZeroU16::MIN;
                    return winding_table_dl.check(phases);
                }
                Err(msg) => return Err(msg),
            }
        } else {
            return Err(WindingTableCreationError::EmptyZone(None));
        }
    }

    /**
    Returns a zone plan created with the Tingley pattern according to [Seq50], p.186f.
    The coil span is ignored for single-layer windings.
     */
    fn tingley(
        slots: NonZeroU16,
        layers: NonZeroU16,
        pole_pairs: NonZeroU16,
        phases: NonZeroU16,
        span: i32,
    ) -> Result<Self, WindingTableCreationError> {
        // Check whether the Tingley pattern is applicable to the given winding
        // configuration or not
        let slots_num = u16::from(slots);
        let layers_num = u16::from(layers);
        let phases_num = u16::from(phases);
        let pole_pairs_num = u16::from(pole_pairs);

        // The Tingley pattern only works for two- and three-phase AC windings
        if phases_num != 2 && phases_num != 3 {
            return Err(WindingTableCreationError::TingleyInvalidNumberPhases);
        }

        // Winding must be symmetric (e.g. no empty slots)
        if (layers_num * slots_num) % (2 * phases_num) != 0 {
            return Err(WindingTableCreationError::EmptyZone(None));
        }

        // Create the phase sequence
        let (phase_sequence, _) = phase_sequence(phases);

        // Initialize empty zone plan.
        let mut winding_table = WindingTable::new(slots, layers);

        // Calculate the number of winding holes to determine whether the winding in
        // question is a fractional slot or an integer slot winding
        let flag_frac = *hole_number(slots, pole_pairs, phases).denom() != 1;

        // First layer of the zone plan
        // *******************************************************************

        // The following steps are taken directly from the rules for the graphical
        // creation of the Tingley pattern with squared paper as described in [Seq50],
        // p. 186f. However, the intermediate slot plan is not created explicitely
        // to avoid the allocations.

        // Step 1:
        // Calculate the greatest common divider (gcd) for the number of slots and pole
        // pairs
        let t_dash = num::integer::gcd(slots_num, 2 * pole_pairs_num);

        // Step 2:
        // Calculate the number of "squares" representing a slot division. The first
        // square is the slot, all other square are part of the tooth.
        let num_squares = 2 * pole_pairs_num / t_dash; // Since t_dash is the gcd, no truncating occurs in this integer division.

        // Step 3:
        // Calculate the number of pole divisions ("Polteilung")
        let squares_per_pole_division = slots_num / t_dash;

        // Step 4:
        // Virtual "drawing" of the intermediate array with assignment of the slots to
        // the phases.
        let mut slot = 0;
        let mut square_counter = 0;
        let block_length = squares_per_pole_division / phases;
        for pole_division in 0..(2 * pole_pairs_num) {
            for square in 0..squares_per_pole_division {
                // If the square counter is 0, the current square represents a slot, otherwise
                // it represents a tooth.
                if square_counter == 0 {
                    // Offset for the negative phases
                    let offset = if pole_division.is_odd() {
                        phases_num as usize
                    } else {
                        0
                    };

                    /*
                    Determine the phase by separating "squares_per_pole_division" in m blocks, where m is the total number of phases.
                    "squares_per_pole_division" is always divisable by "phases". Find the block which contains the current square and select the corresponding phase.
                    It is sufficient to only check the upper limit of the block in each loop iteration.
                    */
                    for phase in 0..phases_num {
                        // Special case of slots directly on the block border: If the slot number is
                        // even, assign them to the next phase, otherwise assign
                        // them to the current phase
                        if u16::from(layers) == 1 && square == block_length * (phase + 1) - 1 {
                            let phase_mod = if flag_frac && slot.is_even() {
                                (phase + 1).rem_euclid(2 * phases_num) as usize
                            } else {
                                phase as usize
                            };
                            let total_idx =
                                (phase_mod + offset).rem_euclid(2 * phases_num as usize);
                            winding_table[Zone::new(slot, 0)] = phase_sequence[total_idx];
                            break;
                        }

                        if square < block_length * (phase + 1) {
                            winding_table[Zone::new(slot, 0)] =
                                phase_sequence[phase as usize + offset];
                            break;
                        }
                    }

                    // Increase the slot counter
                    slot = slot + 1;
                }

                // Increase the square counter and reset it, if square_counter equals
                // num_squares
                square_counter = square_counter + 1;
                if square_counter == num_squares {
                    square_counter = 0;
                }
            }
        }

        if u16::from(layers) == 2 {
            // Invert layers
            for slot in 0..slots_num {
                winding_table[Zone::new(slot, 1)] = winding_table[Zone::new(slot, 0)];
            }

            // Add the return conductors
            for slot in 0..slots_num {
                let return_slot = (slot as i32 + span).rem_euclid(slots_num as i32) as u16;
                winding_table[Zone::new(return_slot, 0)] = -1 * winding_table[Zone::new(slot, 1)];
            }
        }
        return winding_table.check(phases);
    }

    /// Returns a zone plan created with the coil side pattern
    fn coil_side(
        slots: NonZeroU16,
        layers: NonZeroU16,
        pole_pairs: NonZeroU16,
        phases: NonZeroU16,
        _span: i32,
    ) -> Result<WindingTable, WindingTableCreationError> {
        let slots_num = u16::from(slots);
        let layers_num = u16::from(layers);
        let phases_num = u16::from(phases);

        // Check whether the Coil side pattern is applicable to the given
        // winding configuration or not.

        // Can only be applied to single-layer windings
        if layers_num != 1 {
            return Err(WindingTableCreationError::CoilSideInvalidNumberLayers);
        }

        // Winding must be symmetric (e.g. no empty slots)
        if (layers_num * slots_num) % (2 * phases_num) != 0 {
            return Err(WindingTableCreationError::EmptyZone(None));
        }

        // Calculate the number of slots per pole and phase
        let ratio = hole_number(slots, pole_pairs, phases);
        let g = ratio.trunc().numer().clone();
        let z = ratio.fract().numer().clone();
        let n = ratio.denom().clone();

        //  `*******************************************************************
        // Distribute q1 and q2 along a vector qk which has n entries. This
        // operation is equal to step 2 in [Hut20]

        // The two possible integer numbers of qk are given here explicitly
        let q1 = g + 1; // occurs z times
        let q2 = g; // occurs n-z' times

        // Vector with q2 in each entry of length n
        let mut qk = vec![q2 as i32; n as usize];

        // The number of slots which need to be distributed is n*q. Calculate the number
        // of slots which are still "free"
        let free_slots = (n * g + z) as i32 - qk.iter().sum::<i32>();

        // The first k entries of q_k are replaced by q_1. k equals "free_slots".
        // By this operation, all slots are distributed, since sum(q_k) = n*q.
        // For an integer winding, "free_slots" always equals 0 => q_1 is not assigned
        // to any entry at all.
        for ii in 0..free_slots {
            qk[ii as usize] = q1 as i32;
        }

        // Repeat the vector for all phases
        let qk_org = qk.clone();
        for _ in 1..phases_num {
            for elem in &qk_org {
                qk.push(*elem)
            }
        }

        // Check if something went wrong during the calculation of qk
        if 2 * qk.iter().sum::<i32>() != slots_num as i32 {
            return Err(WindingTableCreationError::CoilSideCouldNotDistribute);
        }

        // Create the phase vector
        let mut phase_vector: Vec<i32> = Vec::with_capacity(n as usize);
        for _ in 0..n {
            for phase in 1..(phases_num + 1) {
                phase_vector.push(phase as i32);
            }
        }

        // *********************************************************************
        // Create the whole coil side pattern (step 4 in [Hut20])

        // Create the phase sequence vector
        let (phase_sequence, _) = phase_sequence(phases);
        let mut phase_sequence_vector: Vec<i32> =
            Vec::with_capacity(2 * phases_num as usize * n as usize);
        for _ in 0..n {
            for phase in &phase_sequence {
                phase_sequence_vector.push(*phase);
            }
        }

        // Initialize the whole coil side pattern matrix
        let mut coil_distribution = vec![0; phase_sequence_vector.len()];

        // Assign the slot numbers to the phases
        // The result is equal to step 4 in [Hut20] w/ negative phases
        let mut coil_counter: usize = 0;

        // All rows of winding_table_coil_side
        for idx in 0..phase_sequence_vector.len() {
            // Is the current phase positive? If yes, assign the slot number from
            // winding_table_CS_pos
            if phase_sequence_vector[idx] > 0 {
                // Positive conductor
                coil_distribution[idx] = qk[coil_counter];

                // Move to the next positive phase in coils_per_phase
                coil_counter += 1;
            }
        }

        // Add the return conductors by "rolling" (circular shifting) the lower
        // row of winding_table_coil_side by the number of phases m
        let mut return_conductor_slots = coil_distribution.clone();

        return_conductor_slots.rotate_right(phases_num as usize);
        for ii in 0..phase_sequence_vector.len() {
            coil_distribution[ii] = coil_distribution[ii] + return_conductor_slots[ii];
        }

        //  `*******************************************************************
        // Create the zone plan with the coil side pattern (equals step 5 in [Hut20])

        // Initialize empty zone plan
        let mut winding_table = WindingTable::new(slots, layers);

        let mut counter_filled_slots: usize = 0; // Counter for the slots in the zone plan
        for ii in 0..phase_sequence_vector.len() {
            // Assign k slots to the phase winding_table_coil_side[1,index] (first row)
            // where k is the number in the second row winding_table_coil_side[2,index]
            // The slot counter is then raised by k-1
            let k = coil_distribution[ii] as usize;
            if k != 0 {
                let phase = phase_sequence_vector[ii];
                for kk in counter_filled_slots..(counter_filled_slots + k) {
                    winding_table[Zone::new(kk as u16, 0)] = phase;
                }
            }
            // Raise the slot counter
            counter_filled_slots = counter_filled_slots + k;
        }

        return winding_table.check(phases);
    }

    /// Returns a zone plan created with the algebraic algorithm
    fn algebraic_algorithm(
        slots: NonZeroU16,
        layers: NonZeroU16,
        pole_pairs: NonZeroU16,
        phases: NonZeroU16,
        span: i32,
    ) -> Result<WindingTable, WindingTableCreationError> {
        let slots_num = u16::from(slots);
        let layers_num = u16::from(layers);
        let phases_num = u16::from(phases);
        let pole_pairs_num = u16::from(pole_pairs);

        // Winding must be symmetric (e.g. no empty slots)
        if (layers_num * slots_num) % (2 * phases_num) != 0 {
            return Err(WindingTableCreationError::EmptyZone(None));
        }

        // Number of phases must be greater than two, only works for odd phases
        if phases_num <= 2 || phases_num.is_even() {
            return Err(WindingTableCreationError::AlgebraicAlgorithmInvalidNumberPhases);
        }

        // Calculate the number of steps between two electrically neighboring slots
        // ("Kollektorschritt" in [Kre88]). This is accomplished by finding the smallest
        // positive integer value of g (or g = 0), where Y_k = (g*N'+1)/p'
        let mut yk: u16 = 0;
        let mut g_val: i32 = -1;
        for g in 0..1000 {
            // Calculate Y_k with the given g. Equivalent to (3.2) and (3.7) in [Kre88]
            let yk_float = (g as f64 * slots_num as f64 * layers_num as f64 / 2.0 + 1.0)
                / (pole_pairs_num as f64);

            // Check if Y_k is an integer value
            if yk_float.fract() == 0.0 {
                yk = yk_float as u16;
                g_val = g;
                break;
            }
        }

        if g_val == -1 {
            return Err(WindingTableCreationError::AlgebraicAlgorithmInvalidStep);
        }

        // Double Y_k for single-layer windings
        if layers_num == 1 {
            yk = 2 * yk
        }

        // The "first winding zone under positive poles" has q_1 coils [Kre88].
        // The "second winding zone under negative poles" has q_2 coils [Kre88].
        // The number of coils q_1 and q_2 can be chosen in any way as long as they
        // fulfill the condition: q_1+q_2 = N'/(2*m)*[Number of winding layers].
        // The best windings (with respect to the winding factor) are created when
        // q_1 = q_2 (N' being even) or q_1 = q_2+1 (N' being odd)
        let q12 = slots_num * layers_num / (2 * phases_num); // q_12 = q_1+q_2
        let q1: u16;
        let q2: u16;
        if q12.is_even() {
            q1 = q12 / 2; // q_1 + q_2 = 2*q_1 = q_12
            q2 = q1;
        } else {
            q1 = (q12 + 1) / 2; // q_1 + q_2 = 2*q_1 - 1 = q_12
            q2 = q1 - 1;
        }

        // Initialize empty zone plan
        let mut winding_table = WindingTable::new(slots, layers);

        // First layer of the zone plan (Double-layer winding)/ Left coil side
        // (Single-layer winding)  `************************************************
        // *******************
        for phase in 1..(phases_num + 1) {
            // Loop for each phase
            for g in 0..q1 {
                // Populate zone 1, which goes from 0 <= g <= q_1-1
                let zone_1: u16 = (q12 * yk * (phase - 1) + g * yk).rem_euclid(slots_num);
                winding_table[Zone::new(zone_1, 0)] = phase as i32;

                // Populate zone 2, which goes from 0 <= g <= q_2-1
                if g + 1 <= q2 {
                    let zone_2: u16 = ((q1 + q2) * yk * (phase - 1)
                        + g * yk
                        + ((phases_num + 1) / 2 * q1 + (phases_num - 1) / 2 * q2) * yk)
                        .rem_euclid(slots_num);
                    winding_table[Zone::new(zone_2, 0)] = -(phase as i32);
                }
            }
        }

        // Right coil side (Single-layer winding)
        if layers_num == 1 {
            // If the winding is a distributed winding, the return conductors are
            // assigned after all conductors of a phase sequence have been assigned.
            // The coil span can be every positive odd number. Therefore it is tested
            // if W_sp is odd. If yes, the value of W_sp is used for shifting.
            // Otherwise, W_sp is raised by 1 and then used for shifting (see [Kre88], p.20)
            let cp: i32;
            if span.is_even() {
                cp = span + 1;
            } else {
                cp = span;
            }

            // Since every second slot is left empty by the "Left coil side"
            // algorithm, the return conductor can be assigned by inverting the
            // pattern, shifting it by an odd coil span and then combining both
            // patterns via element-wise addition
            let mut right_coil_side: Vec<i32> = winding_table
                .iter_layers()
                .filter_map(
                    |(zone, phase)| {
                        if zone.layer == 0 { Some(*phase) } else { None }
                    },
                )
                .collect();
            if span > 0 {
                right_coil_side.rotate_right(cp as usize);
            } else {
                right_coil_side.rotate_left(cp.abs() as usize);
            }
            for slot in 0..winding_table.slots() {
                if winding_table[Zone::new(slot, 0)] == 0 {
                    winding_table[Zone::new(slot, 0)] = -right_coil_side[usize::from(slot)]
                }
            }

        // Second layer of the zone plan (Double-layer winding)
        } else {
            // The second layer contains the return conductors of the first layer.
            // Therefore, the zone plan of the first layer is inverted (multiplied by -1)
            // and circularly shifted ("rolled") by the realized coil span W_sp.
            for slot in 0..winding_table.slots() {
                let shifted = slot as i32 - span;
                let shifted_slot = shifted.rem_euclid(slots_num as i32) as u16;
                winding_table[Zone::new(slot, 1)] = -winding_table[Zone::new(shifted_slot, 0)];
            }
        }

        return winding_table.check(phases);
    }

    /// Returns a zone plan created with the star of slots algorithm. This
    /// algorithm is taken from [Bia06].
    fn star_of_slots(
        slots: NonZeroU16,
        layers: NonZeroU16,
        pole_pairs: NonZeroU16,
        phases: NonZeroU16,
        span: i32,
    ) -> Result<Self, WindingTableCreationError> {
        use std::f64::consts::TAU;

        let slots_num = u16::from(slots);
        let layers_num = u16::from(layers);
        let phases_num = u16::from(phases);
        let pole_pairs_num = u16::from(pole_pairs);

        // Only works for odd phases
        if phases_num.is_even() {
            return Err(WindingTableCreationError::StarOfSlotsInvalidNumberPhases);
        }

        if layers_num != 1 && layers_num != 2 {
            return Err(WindingTableCreationError::StarOfSlotsInvalidNumberLayers);
        }

        // Winding must be symmetric (e.g. no empty slots)
        if (layers_num * slots_num) % (2 * phases_num) != 0 {
            return Err(WindingTableCreationError::EmptyZone(None));
        }

        // Create a two-layer zone plan by dividing the phasor star in 2*m sectors
        // of width π/m. For each slot, find the corresponding sector (and therefore
        // the corresponding phase) by comparing maximum and minium angle of the sector
        // (sector borders) to the phasor angle.
        let mut winding_table = WindingTable::new(slots, NonZeroU16::new(2).expect("not zero"));
        let sector_width = std::f64::consts::PI / phases_num as f64;
        let (ps, _) = phase_sequence(phases);

        // Get the slot phasors for the pole pair harmonic (ν = 1). They are described
        // by their angle
        let slot_angle = TAU * (pole_pairs_num as f64) / (slots_num as f64);
        for (idx, phase) in ps.iter().enumerate() {
            // The small offset is necessary to avoid numerical rounding errors when doing
            // the "<=" comparison below.
            let angle_lower_layer = sector_width * (idx as f64 - 0.5) - 1e-10;
            let angle_upper_layer = sector_width * (idx as f64 + 0.5) - 1e-10;

            // Loop through all slots
            for slot in 0..slots_num {
                let phasor_angle = (slot as f64 * slot_angle) % TAU;

                if (angle_lower_layer <= phasor_angle && phasor_angle <= angle_upper_layer)
                    || (angle_lower_layer + TAU <= phasor_angle
                        && phasor_angle <= angle_upper_layer + TAU)
                {
                    winding_table[Zone::new(slot, 0)] = *phase; // First conductor

                    // Return conductor is in the current slot + coil / slot pitch
                    // rem_euclid-operator for circular indexing
                    let return_slot = (slot as i32 + span).rem_euclid(slots_num as i32) as u16;
                    winding_table[Zone::new(return_slot, 1)] = -phase;
                }
            }
        }

        if layers_num == 1 {
            // In case of a single-layer winding, the double-layer zone plan is reduced to
            // a single-layer winding according to [Bia06], section 6.
            let mut sl_winding_table = WindingTable::new(slots, NonZeroU16::MIN);
            for slot in 0..slots_num {
                if slot.is_odd() {
                    sl_winding_table[Zone::new(slot, 0)] = winding_table[Zone::new(slot, 1)];
                } else {
                    sl_winding_table[Zone::new(slot, 0)] = winding_table[Zone::new(slot, 0)];
                }
            }
            return sl_winding_table.check(phases);
        } else {
            return winding_table.check(phases);
        }
    }

    /// Returns a zone plan created with the winding distribution table. This
    /// algorithm is taken from [Car18].
    fn distribution_table(
        slots: NonZeroU16,
        layers: NonZeroU16,
        pole_pairs: NonZeroU16,
        phases: NonZeroU16,
        span: i32,
    ) -> Result<Self, WindingTableCreationError> {
        fn wdt_fill_next_slot(
            wdt: &mut Vec<Vec<i32>>,
            index: usize,
            slot: u16,
            number_rows: usize,
            num_elements: usize,
        ) -> () {
            // Check if the WDT is fully populated. This is the case when WDT does not
            // contains any zeros anymore. In this case, idx is increased until it is
            // equal to the number of elements in the WDT.
            if index == num_elements {
                return ();
            }

            let row = index.rem_euclid(number_rows);
            let phase = index / number_rows;

            // Recursive filling of the next free slot
            if wdt[phase][row] == -1 {
                wdt[phase][row] = slot as i32;
                return ();
            } else {
                wdt_fill_next_slot(wdt, index + 1, slot, number_rows, num_elements)
            }
        }

        let slots_num = u16::from(slots);
        let layers_num = u16::from(layers);
        let phases_num = u16::from(phases);
        let pole_pairs_num = u16::from(pole_pairs);

        if layers_num != 1 && layers_num != 2 {
            return Err(WindingTableCreationError::DistributionTableInvalidNumberLayers);
        }

        // Winding must be symmetric (e.g. no empty slots)
        if (layers_num * slots_num) % (2 * phases_num) != 0 {
            return Err(WindingTableCreationError::EmptyZone(None));
        }

        // Create empty winding distribution table (WDT). Since nalgebra is
        // column-major, the table rows and columns are exchanged compared to the
        // ordering in table 1 of [Car18]. This allows for easy indexing when
        // populating the table.
        let number_rows = slots_num / phases_num;
        let mut wdt = Vec::with_capacity(phases_num.into());
        for _ in 0..phases_num {
            wdt.push(vec![-1; usize::from(number_rows)]);
        }

        // Distribute all slots to the cells of the WDT
        for slot in 0..slots_num {
            // First cell to be populated is the first one, second one has the distance
            // pole_pairs from the first one, third one has the distance pole_pairs from the
            // second one and so on ...
            let index = (pole_pairs_num * slot).rem_euclid(slots_num) as usize;
            wdt_fill_next_slot(
                &mut wdt,
                index,
                slot,
                number_rows.into(),
                usize::from(number_rows * phases_num),
            );
        }

        // Defining the border row between first and second half of the WDT. In figure 3
        // of [Car18], it is shown that the border row itself belongs to the first half
        // (n_c is rounded up / ceiled!)

        // TODO: empty slots are currently not implemented, this code is kept in case
        // empty slots will be implemented later.
        let empty_slots = 0;
        let halfpoint;
        if empty_slots == 0 {
            halfpoint = number_rows / 2;

        // In table 7 of [Car18], it can be seen that the border rows belongs to
        // the second half if empty slots occurr
        } else {
            halfpoint = number_rows / 2;
        }

        // In case of a reduced system, swap quadrant 1 and 3 according to figure 4 in
        // [Car18]
        let reduced = false; // TODO Left here in case the reduced system will be implemented later
        if reduced {
            for col in 0..number_rows {
                for row in 0..(phases_num / 2) {
                    let col_with_offset = number_rows + col;
                    let row_with_offset = phases_num / 2 + 1 + row;
                    let tmp = wdt[row_with_offset as usize][col as usize];
                    wdt[row_with_offset as usize][col as usize] =
                        wdt[row as usize][col_with_offset as usize];
                    wdt[row as usize][col_with_offset as usize] = tmp;
                }
            }

        // Shift the second WDT half (named ζ in [Car18]) for non-reduced
        // systems
        } else {
            // More than one row (n_c > 1)
            if number_rows > 1 {
                let ζ: i32 = if phases_num.is_even() {
                    (phases_num as i32) / 2 - 1
                } else {
                    (phases_num as i32 - 1) / 2
                };

                let wdt_tmp = wdt.clone();

                for col in 0..phases_num {
                    for row in 0..halfpoint {
                        let col_with_offset =
                            (col as i32 - ζ - 1).rem_euclid(phases_num as i32) as usize;
                        let row_with_offset = (number_rows - halfpoint + row) as usize;

                        wdt[col as usize][row_with_offset] =
                            wdt_tmp[col_with_offset][row_with_offset];
                    }
                }
                drop(wdt_tmp);
            }
        }

        // Remove the empty slots from the first row by marking them with "-1"
        if empty_slots != 0 {
            for row in wdt.iter_mut() {
                for idx in 0..empty_slots {
                    row[idx] = -1;
                }
            }
        }

        // Populate the zone plan from the WDT
        let mut winding_table = WindingTable::new(slots, layers);

        for phase_idx in 0..usize::from(phases_num) {
            for row_idx in 0..usize::from(number_rows) {
                // Which slot is going to be filled?
                let slot = wdt[phase_idx][row_idx];

                // Positive or negative phase? If coil_number is in the first half
                // of the WDT, then the phase is positive, otherwise it is negative
                // and must be shifted by 1 step.

                // Empty slots are marked by a -1
                if slot != -1 {
                    let phase = if row_idx + 1 <= usize::from(number_rows - halfpoint) {
                        phase_idx as i32 + 1
                    } else {
                        -(phase_idx as i32 + 1)
                    };
                    winding_table[Zone::new(slot as u16, 0)] = phase;
                }
            }
        }

        // Second layer of the zone plan (if double-layer winding)
        // *********************************************************************
        if layers_num == 2 {
            // The second layer contains the return conductors of the first layer.
            // Therefore, the zone plan of the first layer is inverted (multiplied by -1)
            // and circularly shifted ("rolled") by the realized coil span.
            for slot in 0..winding_table.slots() {
                let shifted = slot as i32 - span;
                let shifted_slot = shifted.rem_euclid(slots_num as i32) as u16;
                winding_table[Zone::new(slot, 1)] = -winding_table[Zone::new(shifted_slot, 0)];
            }
        }

        return winding_table.check(phases);
    }
}
