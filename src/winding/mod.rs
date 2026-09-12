use dyn_clone::DynClone;
use num::{Complex, Integer, integer::gcd};
use std::{
    any::Any,
    f64::consts::{PI, TAU},
    num::{NonZeroU16, NonZeroUsize},
};

use rayon::prelude::*;

#[cfg(feature = "stem_core")]
use stem_core::prelude::*;

#[cfg(feature = "stem_core")]
use crate::overrides::Overrides;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use stem_coil_layout::{CoilLayout, Zone};
use stem_wire::{stem_material::si::Length, wire::Wire};

use crate::{
    coils::{Coil, CoilExt},
    iterators::*,
    winding_table::WindingTable,
};

pub mod coil_assembly;
pub mod distributed;
pub mod distributed_tooth_coil;
pub mod quadruple_layer_tooth_coil;
pub mod squirrel_cage;
pub mod tooth_coil;

pub use coil_assembly::*;
pub use distributed::*;
pub use distributed_tooth_coil::*;
pub use quadruple_layer_tooth_coil::{
    QuadrupleLayerToothCoilBuilder, QuadrupleLayerToothCoilMinimalBuilder,
    QuadrupleLayerToothCoilWinding,
};
pub use squirrel_cage::*;
pub use tooth_coil::*;

#[cfg_attr(feature = "serde", typetag::serde)]
pub trait Winding: Sync + Send + Any + DynClone + std::fmt::Debug + 'static {
    /// Return the number of winding phases.
    fn phases(&self) -> NonZeroU16;

    /// Return the number of winding slots.
    fn slots(&self) -> NonZeroU16;

    /// Return the number of pole pairs.
    fn pole_pairs(&self) -> NonZeroU16;

    /// Return the number of parallel paths of the winding
    fn parallel_paths(&self) -> NonZeroU16;

    /// Return the connection type (star, delta, star-delta, double star, double
    /// delta, ...)
    fn connection(&self) -> Connection;

    /// Return the leakage coefficient of the end winding.
    fn end_winding_leakage_coefficient(&self) -> f64;

    /// Returns the coil layout of the winding.
    fn coil_layout(&self) -> CoilLayout;

    // Return the coil at the given slot / layer position, if the position contains
    // a coil.
    fn coil_at(&self, zone: Zone) -> Option<&Coil>;

    fn as_dyn(&self) -> &dyn Winding;

    // =========================================================================
    // These functions are likely to be overloaded

    /// Return the number of winding layers.
    fn layers(&self) -> NonZeroU16 {
        return self.coil_layout().layers();
    }

    /// Returns the number of basic windings contained in the winding
    /// TODO: Recommend overloading, default impl is O(n), where n is the number
    /// of zones (it scans the entire winding table for repetitions)
    fn base_winding_count<'a>(&'a self) -> NonZeroU16 {
        // Wrapper structure which makes the winding indexable by an usize (slot-major)
        struct IndexWrapper<'a, W: ?Sized>(&'a W);

        impl<'a, W: Winding + ?Sized> RandomAccess for IndexWrapper<'a, W> {
            type Item = i32;

            fn get(&self, index: usize) -> Self::Item {
                let layers = usize::from(self.0.layers().get());

                let slot = index / layers;
                let layer = index % layers;

                self.0
                    .phase_at(Zone {
                        slot: slot as u16,
                        layer: layer as u16,
                    })
                    .unwrap_or(0)
            }
        }

        let num_zones = usize::from(self.layers().get()) * usize::from(self.slots().get());
        let value = repeating_pattern_count(
            &IndexWrapper(self),
            NonZeroUsize::new(num_zones).expect("cannot be zero"),
        )
        .get();
        // We don't need to worry about the as-cast, because values can at most be
        // "num_zones".
        return NonZeroU16::new(value as u16).unwrap_or(NonZeroU16::MIN);
    }

    /// Return the angle covered by one phase zone
    fn phase_zone_angle(&self) -> f64 {
        return TAU / (self.phases().get() as f64);
    }

    /// Return the pole pitch in slots.
    fn pole_pitch(&self) -> f64 {
        return (self.slots().get() as f64) / (2.0 * self.pole_pairs().get() as f64);
    }

    /// Return the number of turns at the given slot / layer position, if the
    /// position contains a coil. If the zone does not contain a coil, this
    /// value is zero. The counting of slot and layer starts at zero, so the
    /// upper layer of a double layer winding at slot 2 is indexed as
    /// `self.turns_at(Zone::new(1, 0))`.
    fn turns_at(&self, zone: Zone) -> usize {
        if let Some(coil) = self.coil_at(zone) {
            return coil.turns().into();
        } else {
            return 0;
        }
    }

    /// Return the number of turns per phase as given in [MVP08].
    fn turns_per_phase(&self, phase: NonZeroU16) -> num::rational::Ratio<usize> {
        // Loop through all coils and add up the number of turns
        let mut turns = 0;

        for slot in 0..self.slots().get() {
            for layer in 0..self.layers().get() {
                if let Some(coil) = self.coil_at(Zone::new(slot, layer)) {
                    if coil.phase() == phase {
                        // Check if the current winding zone is the first occurence of the coil
                        let first_zone = coil
                            .zones()
                            .next()
                            .expect("A coil must occupy at least one zone");
                        if first_zone == (Zone { slot, layer }) {
                            turns = turns + usize::from(coil.turns());
                        }
                    }
                }
            }
        }
        return num::rational::Ratio::new(turns, 1);
    }

    /**
    End winding leakage inductance.
     */
    #[cfg(feature = "stem_core")]
    fn end_winding_leakage_inductance(
        &self,
        phase: NonZeroU16,
        core: CoreRef<'_>,
        overrides: &Overrides,
    ) -> Inductance {
        return Default::default();
    }

    /**
    Return the end winding length of a half-turn. A half turn starts in the middle of the end winding
    on one side and ends in the middle of the end winding of the other side.

    If the space of a single character in the ASCII drawing below equals one mm, the return value of
    this function would be 7 mm (│ + ┌ + 4*─ + ┐).

    ```text
       ┌──────┐
    ┌──│      │──┐
    │  │ Core │  │ <-- Coil
    └──│      │──┘
       └──────┘
    ```
     */
    #[cfg(feature = "stem_core")]
    fn end_winding_half_turn_length(
        &self,
        core: CoreRef<'_>,
        zone: Zone,
        overrides: &Overrides,
    ) -> Option<Length> {
        return Default::default();
    }

    // =========================================================================
    // These functions usually don't need to be overloaded.

    /// Return the wire at the given slot / layer position, if the position
    /// contains a coil.
    fn wire_at(&self, zone: Zone) -> Option<&dyn Wire> {
        let coil = self.coil_at(zone)?;
        return Some(coil.wire());
    }

    /// Returns the number of turns at the given slot / layer position, if the
    /// position contains a coil. The counting of slot and layer starts at
    /// zero, so the upper layer of a double layer winding at slot 2 is indexed
    /// as `self.turns_at(1, 0)`. If the slot or layer doesn't exist, return
    /// an error instead.
    fn phase_at(&self, zone: Zone) -> Option<i32> {
        let coil = self.coil_at(zone)?;
        let is_positive = match coil {
            Coil::Full(coil) => zone == coil.positive_zone(),
            Coil::Half(coil) => coil.first_zone_is_positive(),
        };
        let phase = i32::from(u16::from(coil.phase()));
        if is_positive {
            return Some(phase);
        } else {
            return Some(-phase);
        }
    }

    // Return the number of coils of the winding
    fn number_coils(&self) -> usize {
        let mut counter = 0;

        for slot in 0..self.slots().get() {
            for layer in 0..self.layers().get() {
                if let Some(coil) = self.coil_at(Zone::new(slot, layer)) {
                    // Check if the current winding zone is the first occurence of the coil
                    let first_zone = coil
                        .zones()
                        .next()
                        .expect("A coil must occupy at least one zone");
                    if first_zone == (Zone { slot, layer }) {
                        counter += 1;
                    }
                }
            }
        }
        return counter;
    }

    /// Create a matrix representing the zone plan of the winding.
    /// If `full` is set to `true`, return the zone plan over all slots.
    /// If `full` is set to `false`, return the zone plan of the underlying
    /// basic winding instead.
    fn winding_table(&self, full: bool) -> WindingTable {
        let slots = if full {
            self.slots()
        } else {
            NonZeroU16::new(self.slots().get() / self.base_winding_count().get()).expect("not zero")
        };
        let layers = self.layers();

        let mut winding_table = WindingTable::new(slots, layers);

        for slot in 0..slots.get() {
            for layer in 0..layers.get() {
                let zone = Zone::new(slot, layer);
                if let Some(phase) = self.phase_at(zone) {
                    winding_table[zone] = phase;
                }
            }
        }

        return winding_table;
    }

    /// Returns the (electrical) angle between two  slots in radians. In
    /// [Pyr08], this value is designated as α_u.
    fn phasor_angle(&self) -> f64 {
        phasor_angle(self.slots(), self.pole_pairs())
    }

    /// Returns the hole number represented as q = z/n.
    fn hole_number(&self) -> num::rational::Ratio<u16> {
        hole_number(self.slots(), self.pole_pairs(), self.phases())
    }

    /**
    Return the hole number as f64.
     */
    fn hole_number_float(&self) -> f64 {
        let hole_number = self.hole_number();
        return f64::from(*hole_number.numer()) / f64::from(*hole_number.denom());
    }

    // Returns the number of turns at a given slot.
    fn turns_in_slot(&self, slot: u16) -> usize {
        let mut turns: usize = 0;
        for layer in 0..self.layers().get() {
            let turns_at = self.turns_at(Zone::new(slot, layer));
            turns = turns + turns_at;
        }
        return turns;
    }

    /// Calculate the number of phasors skipped in the numbering of the voltage
    /// phasor star.
    fn skipped_phasors(&self) -> usize {
        return usize::from(self.pole_pairs().get() / self.base_winding_count().get()) - 1;
    }

    /// Vibration mode: The number equals the number of constant, rotating
    /// attraction forces which deforms the stator [Pol12]. With
    /// o_vibration=1, a constant force in one direction exists which leads
    /// to a dynamic load on the bearing. o_vibration=2 results
    /// in an] ovalization of the stator, o_vibration=3 leads to a rounded
    /// triangle and so on.
    fn lowest_vibration_mode(&self) -> usize {
        return num::integer::gcd(
            2 * self.pole_pairs().get(),
            self.slots().get() / self.phases().get(),
        ) as usize;
    }

    /**
    Torque ripple order: The number equals the lowest order of sinusoidal torque waves caused by current.
    E.g. 24 means that 24 sinusoidal waves occur during one full turn of the rotor.
    To put it in other terms: During one full turn of the rotor, the positive peak of the sine occurs 24 times.
    Beside the lowest order, higher orders may occur, whose order is g*o_ripple with g being an integer.

    ```
    use winding::{ToothCoilWinding, Winding, WindingTableMethod};

    let winding = ToothCoilWinding::new_minimal(12, 5, 3, 2, WindingTableMethod::Tingley).unwrap();
    assert_eq!(winding.lowest_torque_ripple_ordinal(), 30); // Poles times phases: 10 * 3 = 60
    ```
     */
    fn lowest_torque_ripple_ordinal(&self) -> usize {
        return 2 * (self.pole_pairs().get() * self.phases().get()) as usize;
    }

    /**
    Torque cogging order: The number equals the lowest order of sinusoidal torque waves
    caused by reluctance effects on the teeth. The meaning of the number is the
    same as that of the torque ripple order.

    ```
    use winding::{ToothCoilWinding, Winding, WindingTableMethod};

    let winding = ToothCoilWinding::new_minimal(12, 5, 3, 2, WindingTableMethod::Tingley).unwrap();
    assert_eq!(winding.lowest_cogging_torque_ordinal(), 60); // Least common multiple of poles and slots: lcm(12, 10) = 60
    ```
     */
    fn lowest_cogging_torque_ordinal(&self) -> usize {
        return num::integer::lcm(2 * self.pole_pairs().get(), self.slots().get()) as usize;
    }

    /// Returns the winding grade (first grade or second grade) according to
    /// [Phy08]. If the denominator of q is odd, then the winding is of
    /// first grade, otherwise of second grade. Integer windings are always
    /// of first grade, since the denominator is always 1.
    fn winding_grade(&self) -> usize {
        let n = self.hole_number().denom().clone();
        return usize::from(2 - n % 2);
    }

    /// Returns the number of wound coils per phase (equals winding_holes in
    /// case of single-layer winding).
    fn coils_per_phase(&self) -> u16 {
        return self.layers().get() * self.slots().get() / (2 * self.phases().get());
    }

    /**
    Returns the number of coil groups per phase. This value is equal to the maximum possible number of parallel paths and can be calculated
    as described in [Seq50], p. 37: First, the number of coils per phase in a basic winding is calculated. Then, it is checked whether this
    number is even or odd. If it is even, the number of coil groups equals twice the number of basic windings (= the base_winding_count). If it is odd,
    the number of coil groups equals the number of basic windings.

    At least one coiö group per phase is always possible
    */
    fn coil_groups_per_phase(&self) -> NonZeroU16 {
        let t = self.base_winding_count();
        let number_of_coils_in_basic_winding =
            self.slots().get() * self.layers().get() / (t.get() * 2 * self.phases().get());
        if number_of_coils_in_basic_winding % 2 == 0 {
            // Antiparallel coil groups are possible
            return NonZeroU16::new(2 * t.get()).expect("t is not zero");
        } else {
            // Only parallel coil groups are possible
            return t;
        }
    }

    /**
    Return the number of coils in a coil group
     */
    fn coils_per_coil_group(&self) -> u16 {
        return self.coils_per_phase() / self.coil_groups_per_phase();
    }

    /// Returns an iterator for all possible numbers of parallel paths, starting
    /// from 1.
    fn possible_parallel_paths(&self) -> crate::iterators::ParallelPathIterator {
        return crate::iterators::ParallelPathIterator::new(self.coil_groups_per_phase());
    }

    /// Returns the winding factor for the given phase and harmonic ordinal.
    /// Note that the phase counting starts with 1 as it usually does in
    /// scientific literature regarding electrical machines. If the phase is
    /// set to 0 or to a number larger than the number of phases, this functions
    /// returns zero. The winding factor is calculated with the voltage
    /// phasor method as described in e.g. [Pyr08].
    fn winding_factor(&self, phase: NonZeroU16, ordinal: f64) -> f64 {
        if phase > self.phases() && phase.get() == 0 {
            return 0.0;
        }

        // The winding factor is formally calculated as k_w =
        // sin(ν*π/2)/Z*Σ_ρ=1^Z(cos(α_ρ)) The angle α_ρ can be derivatived from
        // the phasor star. To find the beams of a phase and their respective
        // sign, the zone plan is used.

        // It is sufficient to calculate the phasor star for the basic winding
        let slots_basic = self.slots().get() / self.base_winding_count().get();
        let layers = self.layers();
        let alpha_u = self.phasor_angle();

        let (phasor_sum_geo, phasor_sum_abs) = (0..slots_basic)
            .into_par_iter()
            .map(|slot| {
                let mut phasor_sum_geo = Complex::new(0.0f64, 0.0);
                let mut phasor_sum_abs = 0;

                let slot_angle = slot as f64 * alpha_u * ordinal;

                for layer in 0..layers.get() {
                    // Check if the current slot and layer is assigned to the phase
                    if let Some(current_phase) = self.phase_at(Zone::new(slot, layer)) {
                        if current_phase.abs() == i32::from(u16::from(phase)) {
                            // Get the angle of the ρ-th zone. The reference is the first zone
                            // of phase 1, which always equals the first beam of the phasor star
                            // (alpha_rho=0 = 0°) If the zone is
                            // negative, the beam direction needs to be inverted (sign-function)
                            let dir_angle = if current_phase < 0 { PI } else { 0.0 };
                            let phasor_length = self.turns_at(Zone::new(slot, layer));
                            phasor_sum_geo = phasor_sum_geo
                                + (phasor_length as f64)
                                    * (Complex::new(0.0, slot_angle + dir_angle)).exp();
                            phasor_sum_abs = phasor_sum_abs + phasor_length;
                        }
                    }
                }

                (phasor_sum_geo, phasor_sum_abs)
            })
            .reduce(
                || (Complex::new(0.0, 0.0), 0),
                |a, b| (a.0 + b.0, a.1 + b.1),
            );

        // Sum up the phasors and get the absolute value, which equals the resulting
        // phasor length. Calculate winding factor as quotient of the absolute sum of
        // all phasors and the geometrical sum of all phasors.
        return phasor_sum_geo.norm_sqr().sqrt() / (phasor_sum_abs as f64);
    }

    /// Returns the angle between two neighbouring phases.
    fn phase_angle_difference(&self) -> f64 {
        return TAU / self.phases().get() as f64;
    }

    /// Calculate the vertices of the Görges polygon for the winding `obj`
    /// according to [MVP08], p. 97ff. as a vector of complex coordinates.
    fn calculate_goerges_polygon(&self, full: bool) -> Vec<Complex<f64>> {
        let slots = if full {
            self.slots()
        } else {
            NonZeroU16::new(self.slots().get() / self.base_winding_count().get()).expect("not zero")
        };

        let mut verts = vec![Complex::new(0.0, 0.0); usize::from(slots.get())];

        // Angle between two phases
        let delta_phase_angle = self.phase_angle_difference();

        for slot in 0..slots.get() {
            for layer in 0..self.layers().get() {
                // The new polygon edge starts at the end of the previous polygon edge
                if layer == 0 && slot > 0 {
                    verts[slot as usize] = verts[slot as usize - 1].clone()
                }

                if let Some(phase) = self.phase_at(Zone::new(slot, layer)) {
                    // Add the magnetic voltage to the current polygon node
                    if phase == 0 {
                        continue;
                    } else {
                        let turns = self.turns_at(Zone::new(slot, layer));
                        let phase_angle = ((phase as f64).abs() - 1.0) * delta_phase_angle;

                        verts[slot as usize] = verts[slot as usize]
                            + (turns as f64 * num::signum(phase) as f64)
                                * (Complex::new(0.0, phase_angle)).exp();
                    }
                }
            }
        }

        // Calculate the focal point and center the polygon
        let sum = verts.as_slice().iter().sum::<Complex<f64>>() as Complex<f64>;
        let focal_point = sum / verts.len() as f64;
        for ii in 0..verts.len() {
            verts[ii] = verts[ii] - focal_point;
        }

        return verts;
    }

    /// Check if the winding factors of all phases are identical.
    /// A default implementation exists.
    fn equal_winding_factors(&self) -> bool {
        let ref_winding_factor = self.winding_factor(NonZeroU16::MIN, 1.0);
        for phase in 2..(self.phases().get() + 1) {
            let winding_factor =
                self.winding_factor(NonZeroU16::new(phase).expect("is not zero"), 1.0);
            if approxim::abs_diff_ne!(winding_factor, ref_winding_factor, epsilon = 1e-15) {
                return false;
            }
        }
        return true;
    }

    /// Conversion of the voltage between two symmetric power grid phases to the
    /// voltage drop over a winding phase.
    fn line_to_phase_voltage(&self) -> num::Complex<f64> {
        // Assumption of a symmetric grid
        let a = Complex::new(0.0, TAU / (self.phases().get() as f64)).exp();
        match self.connection() {
            Connection::Star => return Complex::new(1.0, 0.0) / (Complex::new(1.0, 0.0) - a),
            Connection::Delta => return Complex::new(1.0, 0.0),
        }
    }

    /// Returns the change in amplitude from line voltage to phase voltage.
    fn ratio_line_to_phase_voltage(&self) -> f64 {
        return self.line_to_phase_voltage().norm();
    }

    /// Returns the phase angle between line voltage and phase voltage.
    fn angle_line_to_phase_voltage(&self) -> f64 {
        return -self.line_to_phase_voltage().arg();
    }

    /// Conversion of the voltage between two symmetric power grid phases to the
    /// voltage drop over a winding phase. The voltages are given as complex
    /// numbers.
    fn line_to_phase_current(&self) -> num::Complex<f64> {
        // Assumption of a symmetric grid
        let a = Complex::new(0.0, TAU / (self.phases().get() as f64)).exp();

        match self.connection() {
            Connection::Star => return Complex::new(1.0, 0.0) / (Complex::new(1.0, 0.0) * a),
            Connection::Delta => return Complex::new(1.0, 0.0),
        }
    }

    /// Returns the change in amplitude from line current to phase current.
    fn ratio_line_to_phase_current(&self) -> f64 {
        return self.line_to_phase_current().norm();
    }

    /// Returns the phase angle between line current and phase current.
    fn angle_line_to_phase_current(&self) -> f64 {
        return -self.line_to_phase_current().arg();
    }

    /// Calculate the air gap leakage factor from the Görges diagram ([MVP08],
    /// p. 97 ff.) A default implementation exists.
    fn air_gap_leakage_factor(&self) -> f64 {
        let verts = self.calculate_goerges_polygon(false);
        let slots = self.slots().get() / self.base_winding_count();
        let phase = NonZeroU16::MIN;
        let k_w = self.winding_factor(phase, 1.0);

        // Calculate the "Trägheitsradius" (inertia radius) squared according to , eq.
        // (1.2.85)
        let mut rg2: f64 = verts.iter().map(|v| v.norm_sqr()).sum();

        // For the magnetic slot voltage, the following holds true:
        // Θ_n = obj.layers*Θ_sp,
        // which means that the inertia radius has to be corrected accordingly here
        rg2 = rg2 / ((slots * self.layers().get().pow(2)) as f64);

        // Calculate the "Trägheitsradius der Hauptwelle" squared according to [MVP08],
        // eq. (1.2.86)

        // Current is arbitrarily set to 1 A (as it is in
        // self.calculate_goerges_polygon()) Correction by the number of winding
        // layers is necessary due to the same reason as for rg2
        let turns_per_phase = *self.turns_per_phase(phase).numer() as f64
            / *self.turns_per_phase(phase).denom() as f64;
        let rp2 = (self.phases().get() as f64 * turns_per_phase * k_w
            / (PI * (self.pole_pairs().get() * self.layers().get()) as f64))
            .powi(2);

        // [MVP08], eq. (1.2.87)
        return rg2 / rp2 - 1.0;
    }

    /**
    Calculates the field excitation curve (FEC) of `self` for the given phase currents into the provided buffer.
    The buffer length must be equal to the number of winding slots and can therefore be created by `vec![0.0; winding.slots() as usize]`.
    The current slice length must be equal to the number of phases.

    # Example
    ```
    use winding::{DistributedWinding, Winding, WindingTableMethod};

    let winding = DistributedWinding::new_minimal(12, 1, 3, 1, 0, 0, WindingTableMethod::Tingley).unwrap();
    let mut buffer = vec![0.0; winding.slots() as usize];
    let currents = [1.0, -0.5, -0.5];

    winding.field_excitation_curve(&mut buffer, currents.as_slice()).unwrap();

    assert_eq!(buffer.as_slice(), &[0.0, 1.0, 1.5, 2.0, 1.5, 1.0, 0.0, -1.0, -1.5, -2.0, -1.5, -1.0]);
    ```
     */
    fn field_excitation_curve(
        &self,
        buffer: &mut [f64],
        currents: &[f64],
    ) -> Result<(), compare_variables::Comparison<usize>> {
        // Assert that the input slices have the right length
        let buffer_len = buffer.len();
        let slots = self.slots().get() as usize;
        compare_variables::compare_variables!(slots == buffer_len)?;

        let currents_len = currents.len();
        let phases = self.phases().get() as usize;
        compare_variables::compare_variables!(phases == currents_len)?;

        let mut ampere_turns_slot = 0.0;
        for (slot, buffer) in (0..self.slots().get()).zip(buffer.iter_mut()) {
            *buffer = ampere_turns_slot;
            for layer in 0..self.layers().get() {
                if let Some(coil) = self.coil_at(Zone::new(slot, layer)) {
                    // Get the direction of the coil
                    let is_positive = match coil {
                        Coil::Full(coil) => coil.positive_zone() == Zone { slot, layer },
                        Coil::Half(coil) => coil.first_zone_is_positive(),
                    };

                    let current = currents
                        .get(usize::from(coil.phase().get()) - 1)
                        .expect("The current vector should have a value for each winding phase.");
                    let ampere_turns = usize::from(coil.turns()) as f64 * current;
                    if is_positive {
                        *buffer = *buffer + ampere_turns;
                    } else {
                        *buffer = *buffer - ampere_turns;
                    }
                }
            }

            // Store the previous ampere turns value as starting point for the next
            // iteration
            ampere_turns_slot = *buffer;
        }

        /*
        The FEC is now an undefined integral with the value of the constant C
        depending on the starting slot. C is equal to the a0 coefficient (constant)
        of the Fourier series of the FEC. Therefore, a Fourier analysis of the
        FEC is performed to identify C. Afterwards, C is set to 0 by substracting
        a0 (the offset) from the FEC. The face integral of the FEC over all slots
        then equals 0, meaning that no unipolar flux exists (pure 2D-flux in
        non-axial orientation).
        To avoid performing the FFT, the mean value is calculated and used for the
        correction
         */
        let sum: f64 = buffer.iter().sum();
        let mean: f64 = sum / buffer.len() as f64;
        buffer.iter_mut().for_each(|val| *val = *val - mean);

        return Ok(());
    }

    /**
    Returns a `HarmonicOrdinalsIterator` which gives the harmonic ordinals of the field excitation curve created by the winding.
    For further details, see the documentation on `HarmonicOrdinalsIterator`.

    The returned iterator has an infinite length, because the number of harmonics is infinite as well.
    Therefore, this iterator should not be used directly in e.g. a loop. Instead, a subset of the iterator
    can be used with the `take()` method (see example).

    ```
    use winding::{DistributedWinding, Winding};
    use num::rational::Ratio;

    let winding = DistributedWinding::default(); // This is a 6/2 single-layer integer-slot winding
    let ho_iter = winding.harmonic_ordinals();

    let ordinals: Vec<Ratio<i32>> = winding.harmonic_ordinals().take(5).collect();
    assert_eq!(ordinals[0], Ratio::new(1, 1));
    assert_eq!(ordinals[1], Ratio::new(-5, 1));
    assert_eq!(ordinals[2], Ratio::new(7, 1));
    assert_eq!(ordinals[3], Ratio::new(-11, 1));
    assert_eq!(ordinals[4], Ratio::new(13, 1));
    ```
    A default implementation exists.
    */
    fn harmonic_ordinals(&self) -> HarmonicOrdinalsIterator<'_> {
        return HarmonicOrdinalsIterator::new(self.as_dyn());
    }

    /**
    Return an iterator over the normalized air gap flux density / induction |B_v / B_p| and the
    associated harmonic ordinal (calculated by [`harmonic_ordinals`](Winding::harmonic_ordinals)).
    for each harmonic ordinal returned by [`harmonic_ordinals`](Winding::harmonic_ordinals)
    The formula is based on [Hut18a], page 25.x
     */
    fn harmonic_inductions(&self) -> NormalizedInductionIterator<'_> {
        return NormalizedInductionIterator::new(self.as_dyn());
    }

    /// Returns an iterator over all coils of the winding.
    fn coils(&self) -> CoilsIterator<'_> {
        return CoilsIterator::new(self.as_dyn());
    }

    /**
    Return the volume of a single half turn of the wire in the specified zone.
     */
    #[cfg(feature = "stem_core")]
    fn end_winding_half_turn_volume(
        &self,
        core: CoreRef<'_>,
        zone: Zone,
        overrides: &Overrides,
    ) -> Option<Volume> {
        let coil = self.coil_at(zone)?;
        let zone_area = core.zone_area();
        let cross_section = coil.wire().cross_section(zone_area, coil.turns());
        let length = self.end_winding_half_turn_length(core, zone, overrides)?;
        return Some(cross_section * length);
    }

    /**
    Returns the total axial coil overhang on boths sides of the magnetic core.

    The ASCII art below visualizes the axial coil overhang with equal signs (=).
    If = equals one mm, the return value of `axial_coil_overhang` would be 3 mm.

    ```text
         ┌──────┐
    ┌──==│      │=──┐
    │    │ Core │   │ <-- Coil
    └──==│      │=──┘
         └──────┘
    ```
    Axial overhang can be caused by e.g. the end winding insulation. While this overhang is part of the end winding,
    it is not included in the calculation of the end winding length of the `Winding` trait method `end_winding_half_turn_length`.
    Therefore, in the calculation of the end winding inductance, the coil length is calculated as `axial_coil_overhang` + `end_winding_half_turn_length`.
    This length is not considered in the main inductance calculation.
     */
    #[cfg(feature = "stem_core")]
    fn axial_coil_overhang(&self, core: CoreRef<'_>, _zone: Zone) -> Option<Length> {
        return Some(core.axial_coil_overhang());
    }

    /// Assert the symmetry of the winding. If true, the following is also true:
    /// * The winding factor is identical for all phases (this holds true for
    ///   all ordinals individually).
    /// * The phase resistance of all phases is identical
    /// * The number of turns per phase is identical for all phases.
    #[cfg(feature = "stem_core")]
    fn is_symmetric(&self, core: CoreRef<'_>, overrides: &Overrides) -> bool {
        use uom::si::electrical_resistance::ohm;

        // Condition 1: Comparison of winding factors
        if !self.equal_winding_factors() {
            return false;
        }

        // Condition 2: Calculate the phase resistance for some assumed conditions
        let resistance_1 = self.resistance(1, core, &[], overrides).get::<ohm>();
        for phase in 2..(self.phases() + 1) {
            if approxim::abs_diff_ne!(
                self.resistance(phase, core, &[], overrides).get::<ohm>(),
                resistance_1,
                epsilon = 1e-10
            ) {
                return false;
            }
        }

        // Condition 3: Add the number of turns of phase 1 and compare it to the number
        // of turns of all phases
        let turns_phase_1 = self.turns_per_phase(1);
        for phase in 2..(self.phases() + 1) {
            if self.turns_per_phase(phase) != turns_phase_1 {
                return false;
            }
        }

        return true;
    }

    /**
    If the phase resistance of the winding can be represented as `resistance = resistance_constant * electric_resistivity`,
    this function returns the `resistance_constant` as well as the material from which the electric resistivity can be taken.
    */
    #[cfg(feature = "stem_core")]
    fn resistance_components(
        &self,
        _core: CoreRef<'_>,
        _overrides: &Overrides,
    ) -> Option<crate::core_integration::ResistanceComponents> {
        return None;
    }

    /**
    Calculate the slot leakage inductance for a symmetric winding

    Panics if the winding is not symmetric
     */
    #[cfg(feature = "stem_core")]
    fn slot_leakage_inductance(
        &self,
        phase: NonZeroU16,
        core: CoreRef<'_>,
        effective_air_gap: Length,
        _conditions: &[DynQuantity<f64>],
        overrides: &Overrides,
    ) -> Inductance {
        if let Some(slot_leakage_inductance) = overrides.slot_leakage_inductance {
            return slot_leakage_inductance;
        }
        assert!(self.is_symmetric(core, overrides));

        if let Some(slot) = core.slot() {
            // Opening and tooth tip inductance are independent of the layer configuration
            let lambda_opening_and_tooth_tip = slot.leakage_coefficient_opening()
                + slot.leakage_coefficient_tooth_tip(effective_air_gap);

            // Calculate the total slot leakage inductance by iterating over all slots of a
            // basic winding and calculating the sum of the slot flux leakage inductance.
            let number_basic_slots = self.slots() / self.base_winding_count();
            let slots = 0..number_basic_slots;
            let number_layers = self.layers();
            let number_layers_squared = number_layers.pow(2);

            // Calculate the normalized current of all phases
            let normalized_current: Vec<f64> = multiphase_system(
                Time::new::<second>(0.0),
                Frequency::new::<hertz>(0.0),
                0.0,
                self.phases(),
            )
            .collect();

            // Precalculate the leakage coefficient matrix
            let lambda_slot = slot.leakage_coefficient_matrix(&self.coil_layout());

            // Precalculate the product of axial length and vacuum permeability
            let single_turn_inductance =
                (*VACUUM_PERMEABILITY * core.axial_coil_length()).get::<henry>();

            // Calculate the total slot inductance for the selected phase
            let basic_winding_slot_inductance: f64 = slots
                .into_par_iter()
                .map(|slot_idx| {
                    // Calculate the self-inductance of a single slot according to [MVP08], section
                    // 3.5.2.1 (p. 311)
                    return (0..number_layers_squared)
                        .into_iter()
                        .map(|lin_idx| {
                            let [linked_layer, excitation_layer] = cart_lin::lin_to_cart_unchecked(
                                lin_idx.into(),
                                &[number_layers.into(), number_layers.into()],
                            );
                            let linked_layer = u16::try_from(linked_layer)
                                .expect("the input values to lin_to_cart must be in the u16 range");
                            let excitation_layer = u16::try_from(excitation_layer)
                                .expect("the input values to lin_to_cart must be in the u16 range");

                            if let Some(linked_coil) =
                                self.coil_at(Zone::new(slot_idx, linked_layer))
                            {
                                // Only consider the selected phase
                                if linked_coil.phase() != phase {
                                    return 0.0;
                                }

                                if let Some(excitation_coil) =
                                    self.coil_at(Zone::new(slot_idx, excitation_layer))
                                {
                                    // Calculate the coupling direction between the linked coil and
                                    // the excitation coil
                                    let coupling = 1
                                        * self
                                            .phase_at(Zone::new(slot_idx, linked_layer))
                                            .expect("the link coil exists")
                                            .signum()
                                        * self
                                            .phase_at(Zone::new(slot_idx, excitation_layer))
                                            .expect("the excitation coil exists")
                                            .signum();

                                    // Get the normalized excitation current and modify its
                                    // direction according to the coupling calculated above
                                    let excitation_current = normalized_current
                                        [excitation_coil.phase() as usize - 1]
                                        * coupling as f64;

                                    // Calculate the inductance
                                    return single_turn_inductance
                                        * linked_coil.turns() as f64
                                        * excitation_coil.turns() as f64
                                        * excitation_current
                                        * (lambda_slot
                                            [(linked_layer as usize, excitation_layer as usize)]
                                            + lambda_opening_and_tooth_tip);
                                } else {
                                    return 0.0;
                                }
                            } else {
                                return 0.0;
                            }
                        })
                        .sum::<f64>();
                })
                .sum();

            // Scale with the winding base_winding_count and the number of parallel paths
            return Inductance::new::<henry>(basic_winding_slot_inductance)
                * self.base_winding_count() as f64
                / self.parallel_paths() as f64;
        } else {
            return Inductance::new::<henry>(0.0);
        }
    }

    /// Return an iterator over the coils of the winding and their properties
    #[cfg(feature = "stem_core")]
    fn coil_properties<'a>(
        &'a self,
        core: CoreRef<'a>,
        overrides: &'a Overrides,
    ) -> crate::coils::CoilPropertyIterator<'a> {
        return crate::coils::CoilPropertyIterator {
            coils: self.coils(),
            winding: self.as_dyn(),
            core,
            overrides,
        };
    }

    #[cfg(feature = "stem_core")]
    fn coil_properties_at<'a>(
        &'a self,
        core: CoreRef<'a>,
        zone: Zone,
        overrides: &'a Overrides,
    ) -> Option<crate::coils::CoilProperties<'a>> {
        let coil = self.coil_at(zone)?;
        return Some(crate::coils::CoilProperties {
            coil,
            winding: self.as_dyn(),
            core,
            overrides,
        });
    }

    #[cfg(feature = "stem_core")]
    fn resistance(
        &self,
        phase: NonZeroU16,
        core: CoreRef<'_>,
        conditions: &[DynQuantity<f64>],
        overrides: &Overrides,
    ) -> ElectricalResistance {
        // Calculate the phase resistance by calculating the phase resistance of each
        // individual coil and then summing them up
        let mut resistance = ElectricalResistance::new::<ohm>(0.0);
        let zone_area = core.zone_area() / self.layers() as f64;

        for coil in self.coils() {
            let multiplier = match coil {
                Coil::Full(_) => 2.0,
                Coil::Half(_) => 1.0,
            };
            if coil.phase() == phase {
                let length = multiplier
                    * (core.axial_coil_length()
                        + self
                            .axial_coil_overhang(core, coil.first_zone())
                            .expect("must contain a coil")
                        + self
                            .end_winding_half_turn_length(core, coil.first_zone(), overrides)
                            .expect("at this zone, there must be a coil"));
                resistance += coil.resistance(zone_area, length, conditions);
            }
        }
        return resistance;
    }

    #[cfg(all(feature = "cairo", feature = "stem_core"))]
    fn drawables(&self, core: CoreRef<'_>, zone_config: ZoneConfig) -> WindingZoneDrawables {
        WindingZoneDrawables::new(self.as_dyn(), core, zone_config)
    }
}

dyn_clone::clone_trait_object!(Winding);

/// Connection type used for the winding
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Connection {
    Star,
    Delta,
}

/// Calculate the electrical angle between two slots (2.66 in [1]). In [1] this
/// is expressed by α_z as well: α_u = α_z * p / t
/// However, α_z = 2π * t / Q, therefore this equation can be simplified to the
/// term below
pub fn electrical_slot_angle(slots: NonZeroU16, p: u16) -> f64 {
    return TAU * (p as f64) / (slots.get() as f64);
}

/// Returns the hole number representation q = g + z/n where g, z and n are
/// integers in order [g, z, n].
///
/// ```
/// use winding::hole_number;
///
/// // A 12/10 three-phase winding has a hole number of 2/5, which means that g = 0, z = 2, n = 5
/// let ratio = hole_number(12, 5, 3);
/// let g = ratio.trunc().numer().clone();
/// let z = ratio.fract().numer().clone();
/// let n = ratio.denom().clone();
/// assert_eq!(g, 0);
/// assert_eq!(z, 2);
/// assert_eq!(n, 5);
///
/// // A 36/10 three-phase winding has a hole number of 6/5, which means that g = 1, z = 1, n = 5
/// let ratio = hole_number(36, 5, 3);
/// let g = ratio.trunc().numer().clone();
/// let z = ratio.fract().numer().clone();
/// let n = ratio.denom().clone();
/// assert_eq!(g, 1);
/// assert_eq!(z, 1);
/// assert_eq!(n, 5);
///
/// // A 36/4 three-phase winding has a hole number of 3, which means that g = 3, z = 0, n = 1
/// let ratio = hole_number(36, 2, 3);
/// let g = ratio.trunc().numer().clone();
/// let z = ratio.fract().numer().clone();
/// let n = ratio.denom().clone();
/// assert_eq!(g, 3);
/// assert_eq!(z, 0);
/// assert_eq!(n, 1);
/// ```
pub fn hole_number(
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    phases: NonZeroU16,
) -> num::rational::Ratio<u16> {
    // Get greatest common divisor of numerator and denominator
    let val = num::integer::gcd(slots.get(), 2 * pole_pairs.get() * phases.get());

    // Calculate smallest integer numerator and denominator
    let n = (2 * pole_pairs.get() * phases.get()) / val;
    let z = slots.get() / val;
    return num::rational::Ratio::new_raw(z, n);
}

/// Calculate the phase sequence for the given number of phases to generate a
/// rotating field. The sequence is calculated with the star of slots as shown
/// in e.g. [Pyr08] or [Mat20a] The phase sequence star consists of 2*m beams
/// (each phase in positive and negative direction) with the angle (2*pi)/(2*m)
/// between neighboring beams. It is created by creating a positive star first
/// and then inverting it by adding pi to each angle. The positive star has an
/// angle of (2*pi)/m between two neighboring beams.
pub fn phase_sequence(phases: NonZeroU16) -> (Vec<i32>, Vec<f64>) {
    // Phase angles
    let phase_angle = TAU / (phases.get() as f64);
    let mut angles: Vec<f64> = Vec::with_capacity(2 * usize::from(phases.get()));
    let mut phase_indices: Vec<i32> = Vec::with_capacity(2 * usize::from(phases.get()));
    for phase in 0..phases.get() {
        // The angles are normalized by dividing them through 2*pi and just keeping
        // the rest (modulo operation)
        angles.push(phase_angle * phase as f64);
        phase_indices.push(i32::from(phase) + 1);
        angles.push((phase_angle * phase as f64 + PI) % TAU);
        phase_indices.push(-(i32::from(phase) + 1));
    }
    // The phase sequence is now created by going through the star in mathematical
    // positive (counter-clockwise) direction and noting the beam indices in order
    // of appearance.
    let mut permutation = permutation::sort_unstable_by(angles.as_slice(), |a, b| a.total_cmp(&b));
    permutation.apply_slice_in_place(&mut phase_indices);
    permutation.apply_slice_in_place(&mut angles);

    return (phase_indices, angles);
}

/// The curvature factor accounts for the curvature of the air gap field due to
/// the stator roundness (see [Hut04])
pub fn curvature_factor(
    pole_pairs: NonZeroU16,
    air_gap_radius: Length,
    air_gap_width: Length,
    ordinal: f64,
    is_outer: bool,
) -> f64 {
    let other_radius = air_gap_radius
        + if is_outer {
            -air_gap_width
        } else {
            air_gap_width
        };
    let v_times_p = (ordinal * pole_pairs.get() as f64).abs();
    let a = f64::from(air_gap_radius / other_radius).powf(2.0 * v_times_p);
    return f64::from(air_gap_width * v_times_p / air_gap_radius * (a + 1.0) / (a - 1.0));
}

/// Provides random access to the values of a collection.
///
/// Unlike [`std::ops::Index`], `RandomAccess` does not require the value to be
/// stored in the collection or to have an addressable location. Implementations
/// may calculate the value on demand.
///
/// The index is expected to be valid for the collection, and implementations
/// should provide constant-time access.
///
/// A blanket implementation is provided for all types implementing
/// [`std::ops::Index<usize>`] whose [`std::ops::Index::Output`] is [`Copy`].
/// This allows ordinary indexable collections to be used as `RandomAccess`
/// collections without any additional implementation.
///
/// # Examples
///
/// A collection may calculate its values on demand rather than storing them.
/// This would not work with the `Index` trait, because that trait requires
/// returning a reference to a stored item, which simply won't exist.
///
/// ```
/// use stem_winding::winding::RandomAccess;
///
/// struct Calculated;
/// impl Calculated {
///     fn value_at(&self, index: usize) -> i32 {
///         index as i32 * 2
///     }
/// }
///
/// impl RandomAccess for Calculated {
///     type Item = i32;
///
///     fn get(&self, index: usize) -> Self::Item {
///         self.value_at(index)
///     }
/// }
///
/// assert_eq!(Calculated {}.get(2), 4);
/// ```
///
/// [`Copy`]: std::marker::Copy
pub trait RandomAccess<I = usize> {
    type Item;

    /// Returns the value at `index`.
    ///
    /// The value may be retrieved from stored data or calculated on demand.
    /// Implementations should provide constant-time access.
    ///
    /// The behavior for an out-of-bounds `index` is implementation-defined.
    /// Implementations should document whether such an index panics, returns a
    /// sentinel value, or is otherwise handled.
    fn get(&self, index: I) -> Self::Item;
}

impl<C> RandomAccess for C
where
    C: std::ops::Index<usize>,
    C::Output: Copy,
{
    type Item = C::Output;

    fn get(&self, index: usize) -> Self::Item {
        self[index]
    }
}

pub fn repeating_pattern_count<C>(collection: &C, collection_len: NonZeroUsize) -> NonZeroUsize
where
    C: RandomAccess,
    C::Item: PartialEq,
{
    struct PatternLength {
        len: NonZeroUsize,
        divisor: usize,
    }

    impl PatternLength {
        fn new(len: NonZeroUsize) -> Self {
            Self {
                len,
                divisor: len.get(),
            }
        }
    }

    impl Iterator for PatternLength {
        type Item = usize;

        fn next(&mut self) -> Option<Self::Item> {
            while self.divisor > 0 {
                if self.len.get() % self.divisor == 0 {
                    let divisor = self.divisor;
                    self.divisor -= 1;
                    return Some(self.len.get() / divisor);
                }
                self.divisor -= 1;
            }
            return None;
        }
    }

    let mut c1 = 0;
    let mut c2 = 1;
    let mut pattern_len_iter = PatternLength::new(collection_len);

    // Candidate for the pattern length. We start with the smallest possible pattern
    // length, which is always 1.
    let mut pattern_len_cand = match pattern_len_iter.next() {
        Some(l) => l,
        None => unreachable!(
            "collection_len cannot be zero, hence the iterator will always return at least one item"
        ),
    };
    while c2 < collection_len.get() {
        if collection.get(c1) == collection.get(c2) {
            c1 = (c1 + 1) % pattern_len_cand;
            c2 += 1;
        } else {
            // If the new pattern length is not larger than c2, we must exclude it.
            // because we already skipped the possibility to validate that pattern
            // (since c2 already went too far). Furthermore, this pattern length
            // isn't valid anyway because we just encountered a case invalidating
            // a pattern length of at least c2, hence pattern_len_cand <= c2 cannot
            // be a valid pattern.
            loop {
                pattern_len_cand = match pattern_len_iter.next() {
                    Some(l) => l,
                    None => unreachable!("the last iterator item is always collection_len"),
                };

                if pattern_len_cand >= c2 {
                    break;
                }
            }

            // We can jump ahead to the next multiple of the pattern_len_cand
            c2 = ((c2 + pattern_len_cand - 1) / pattern_len_cand) * pattern_len_cand;

            // We start checking the new pattern length from the beginning again
            c1 = 0;
        }
    }

    return NonZeroUsize::new(c2 / pattern_len_cand).unwrap_or(NonZeroUsize::MIN);
}

/// Calculate the number of basic windings with the formulae from [Pyr08],
/// section 2.11 (p. 102 ff)
///
/// TODO: every winding contains at least one base winding
///
/// What is a base winding: Repeating winding. This formula assumes symmetric
/// repetition of coils (like DistributedWinding or ToothCoilWinding) Will
/// return wrong values for arbitrary winding such as e.g. a CoilAssembly,
/// consider using repeating_pattern_count via the
/// [`Winding::base_winding_count`] wrapper instead, which can deal with
/// arbitrary windings. Default impl of [`Winding::base_winding_count`] wraps
/// repeating_pattern_count, which can deal with arbitrary windings. So this
/// method is just an optimization for particular windings.
///
/// ```
/// use winding::base_winding_count;
///
/// // 12/4 single-layer integer winding
/// assert_eq!(2, base_winding_count(12, 2, 3, 1));
///
/// // 12/10 double-layer tooth-coil winding
/// assert_eq!(1, base_winding_count(12, 5, 3, 2));
///
/// // 12/8 double-layer tooth-coil winding
/// assert_eq!(4, base_winding_count(12, 4, 3, 2));
///
/// // 36/8 single-layer fractional slot winding
/// assert_eq!(2, base_winding_count(36, 4, 3, 1));
/// ```
pub fn base_winding_count_repeating_coil_groups(
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    phases: NonZeroU16,
    layers: NonZeroU16,
) -> NonZeroU16 {
    // Inner of basic windings
    let t = NonZeroU16::new(gcd(slots.get(), pole_pairs.get()))
        .expect("cannot be zero, because gcd inputs are not zero");
    let ratio = hole_number(slots, pole_pairs, phases);
    let n = ratio.denom().clone();

    // Check if the winding is an integer slot winding
    if n == 1 {
        return t;

    // Check whether the fractional slot winding is of first or of second grade
    } else {
        // First grade fractional slot winding
        if n.is_odd() {
            return t;
        } else {
            // Single-layer winding
            if layers.get() == 1 {
                if t.get().is_odd() {
                    return t;
                } else {
                    return NonZeroU16::new(t.get() / 2).unwrap_or(NonZeroU16::MIN);
                }

            // Multi-layer winding
            } else {
                return t;
            }
        }
    }
}

/// Returns the (electrical) angle between two  slots in radians. In [Pyr08],
/// this value is designated as α_u.
pub fn phasor_angle(slots: NonZeroU16, pole_pairs: NonZeroU16) -> f64 {
    return TAU * (pole_pairs.get() as f64) / (slots.get() as f64);
}

/**
Calculate the staggering angle for a given segment of a staggered component (stator or rotor).
Each segment in a staggered component has the same angular offset to its neighbors, which
is calculated from the total number of segments and the resulting skew angle:
`offset_angle = skew_angle / num_segments`.

The segment count starts at zero, the last segment of a staggered component has therefore the index `num_segments-1`.

The reference axis lies on the cross section of skew line and axial stack length, therefore the sum of all segment angles is always zero.

# Panics
Panics if the number of segments (`num_segments`) is zero.

```
use winding::common::segment_angle;
use approxim::assert_abs_diff_eq;

// Two segments
assert_abs_diff_eq!(segment_angle(0, 6.0, 2), -1.5);
assert_abs_diff_eq!(segment_angle(1, 6.0, 2), 1.5);

// Three segments
assert_abs_diff_eq!(segment_angle(0, 6.0, 3), -2.0);
assert_abs_diff_eq!(segment_angle(1, 6.0, 3), 0.0);
assert_abs_diff_eq!(segment_angle(2, 6.0, 3), 2.0);
```
 */
pub fn segment_angle(segment: usize, skew_angle: f64, num_segments: usize) -> f64 {
    let beta = skew_angle / num_segments as f64;
    return (0.5 + segment as f64) * beta - 0.5 * skew_angle;
}
