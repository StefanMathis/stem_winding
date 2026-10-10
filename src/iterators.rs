use approxim;
use std::{f64::consts::TAU, num::NonZeroU16};
use stem_types::{SpatialOrder, Zone};

use crate::{
    coils::Coil,
    winding::{WINDING_FACTOR_ZERO_THRESHOLD, Winding},
};

/// This iterator returns all possible parallel path configurations, starting
/// from 1. It is created by the `parallel_paths()` method of a winding which
/// implements `is_winding`.
pub struct ParallelPathIterator {
    a_max: NonZeroU16,
    index: u16,
}
impl ParallelPathIterator {
    pub fn new(a_max: NonZeroU16) -> ParallelPathIterator {
        return ParallelPathIterator { a_max, index: 0 };
    }
}
impl Iterator for ParallelPathIterator {
    type Item = NonZeroU16;
    fn next(&mut self) -> Option<NonZeroU16> {
        if self.index <= self.a_max.get() {
            for _ in self.index..(self.a_max.get() + 1) {
                self.index = self.index + 1;
                if (self.a_max.get()) % self.index == 0 {
                    return Some(
                        NonZeroU16::new(self.index).expect("has been incremented at least once"),
                    );
                }
            }
        }
        return None;
    }
}

/// A nonzero spatial harmonic present in a winding.
///
/// `spatial_order` specifies the harmonic's spatial order, while `is_positive`
/// indicates its direction of rotation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindingHarmonic {
    /// Spatial order of the harmonic.
    pub spatial_order: SpatialOrder,
    /// Sign of the order: positive for a wave co-rotating with the current
    /// system, negative for a wave counter-rotating relative to it.
    pub is_positive: bool,
}

/// An iterator over the [`WindingHarmonic`]s of a winding.
///
/// This struct is created by [`Winding::harmonics`], see its documentation
/// for details and examples.
#[derive(Clone)]
pub struct WindingHarmonicsIterator<'a> {
    winding: &'a dyn Winding,
    phases: u16,
    base_winding_count: u16,
    equal_winding_factors: bool,
    mech_order: u32,
}

impl<'a> WindingHarmonicsIterator<'a> {
    /// Creates a new [`WindingHarmonicsIterator`] for the given [`Winding`].
    pub fn new(winding: &'a dyn Winding) -> WindingHarmonicsIterator<'a> {
        let equal_winding_factors = winding.equal_winding_factors();
        let base_winding_count = winding.base_winding_count().get();
        let phases = winding.phases().get();
        return WindingHarmonicsIterator {
            winding,
            phases,
            base_winding_count,
            equal_winding_factors,
            mech_order: 0,
        };
    }
}

impl<'a> Iterator for WindingHarmonicsIterator<'a> {
    type Item = WindingHarmonic;

    fn next(&mut self) -> Option<WindingHarmonic> {
        // Stop the iteration if an overflow would occur when increasing v_star
        if self.mech_order == u32::MAX {
            return None;
        }

        // Update v_star
        self.mech_order = self.mech_order + 1;

        let winding_factor_all_phases = if self.equal_winding_factors {
            Some(
                self.winding
                    .winding_factor(NonZeroU16::MIN, SpatialOrder::Mechanical(self.mech_order)),
            )
        } else {
            None
        };

        match is_harmonic(
            self.winding,
            winding_factor_all_phases,
            self.phases,
            self.base_winding_count,
            self.mech_order,
        ) {
            Some(is_positive) => Some(WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(self.mech_order),
                is_positive,
            }),
            None => self.next(),
        }
    }
}

/// Implementation of the [`Winding::is_harmonic`] method which is also reused
/// in the [`WindingHarmonicsIterator`]. In that iterator, some information such
/// as `equal_winding_factors` or `base_winding_count` is cached for efficiency
/// reasons and handed to this function as arguments.
pub(crate) fn is_harmonic<W: Winding + ?Sized>(
    winding: &W,
    winding_factor_all_phases: Option<f64>,
    phases: u16,
    base_winding_count: u16,
    mech_order: u32,
) -> Option<bool> {
    // Check if the waves cancel each other or a resulting wave exists.
    let mut sum_corotating: f64 = 0.0;
    let mut sum_counterrotating: f64 = 0.0;
    for phase in 1..(phases + 1) {
        let nonzero_phase = NonZeroU16::new(phase).expect("not zero");
        let k_w = winding_factor_all_phases.unwrap_or_else(|| {
            winding.winding_factor(nonzero_phase, SpatialOrder::Mechanical(mech_order))
        });

        // Reduced wave equation from the struct docstring (t = 0, γ = 0):
        // V_i,red = n_i * k_w / 2 * (cos(2π * i / m (1 - v_mech*) + cos(2π * i / m (- 1
        // - v_mech*))).

        // All other parts are identical for each phase and therefore do not influence
        // the signum (which is the aspect we're interested in!). Calculate
        // the sums of both components. One of them will cancel out, the
        // other one will be positive.
        let pi_term = f64::from(phase) * TAU / f64::from(phases);
        let tpp = winding.series_turns_per_phase(nonzero_phase);
        let turns_per_phase = *tpp.numer() as f64 / *tpp.denom() as f64;
        let nu_star = f64::from(mech_order) / f64::from(base_winding_count);
        sum_corotating += turns_per_phase * k_w * (pi_term * (1.0 - nu_star)).cos();
        sum_counterrotating += turns_per_phase * k_w * (pi_term * (-1.0 - nu_star)).cos();
    }
    if approxim::abs_diff_eq!(
        sum_corotating + sum_counterrotating,
        0.0,
        epsilon = WINDING_FACTOR_ZERO_THRESHOLD
    ) {
        return None;
    }
    Some(sum_corotating > sum_counterrotating)
}

/// An iterator over the [`Coil`]s of a winding.
///
/// This struct is created by [`Winding::coils_iter`], see its documentation
/// for details and examples.
pub struct CoilsIterator<'a> {
    winding: &'a dyn Winding,
    slots: NonZeroU16,
    layers: NonZeroU16,
    slot_counter: u16,
    layer_counter: u16,
}
impl<'a> CoilsIterator<'a> {
    /// Creates a new [`CoilsIterator`] for the given [`Winding`].
    pub fn new(winding: &'a dyn Winding) -> CoilsIterator<'a> {
        return CoilsIterator {
            winding,
            slots: winding.slots(),
            layers: winding.layers(),
            slot_counter: 0,
            layer_counter: 0,
        };
    }

    fn increment(&mut self) {
        // If all layers of the current slot have been exhausted, increment the slot.
        // Otherwise, increment the layer.
        if self.layer_counter + 1 == self.layers.get() {
            // Reset the layer counter
            self.layer_counter = 0;
            self.slot_counter = self.slot_counter + 1;
        } else {
            self.layer_counter = self.layer_counter + 1;
        }
    }
}

impl<'a> Iterator for CoilsIterator<'a> {
    type Item = &'a Coil;
    fn next(&mut self) -> Option<&'a Coil> {
        // Check if the iterator has been exhausted
        if self.slot_counter == self.slots.get() {
            return None;
        }

        // Return the coil if the current position is the positive conductor.
        // Otherwise, increment and try the next position.
        match self
            .winding
            .coil_at(Zone::new(self.slot_counter, self.layer_counter))
        {
            Some(coil) => {
                let return_this = match coil {
                    Coil::Full(coil) => {
                        coil.positive_zone() == Zone::new(self.slot_counter, self.layer_counter)
                    }
                    Coil::Half(_) => true,
                };
                if return_this {
                    self.increment();
                    return Some(&coil);
                } else {
                    self.increment();
                    return self.next();
                }
            }
            None => {
                self.increment();
                return self.next();
            }
        }
    }
}

/**
This iterator returns all possible configurations for the number of coils in a coil group of an distributed tooth-coil winding.
```
use winding::CoilsPerCoilGroupIterator;
let mut iter = CoilsPerCoilGroupIterator::new(12, 3, 2, false);
assert_eq!(iter.next().unwrap(), 1);
assert_eq!(iter.next().unwrap(), 2);
assert!(iter.next().is_none());
```
 */
pub struct CoilsPerCoilGroupIterator {
    slots: u16,
    phases: u16,
    layers: u16,
    coils_per_coil_group: u16,
    max_pole_pairs: u16,
    multiplier: u16,
    iterator_is_exhausted: bool,
}

impl CoilsPerCoilGroupIterator {
    pub fn new(
        slots: NonZeroU16,
        phases: NonZeroU16,
        layers: NonZeroU16,
        double_zone_span: bool,
    ) -> Self {
        let slots = slots.get();
        let phases = phases.get();
        let layers = layers.get();

        // Check if the current number of turns per coil would lead to a valid winding
        let multiplier = if double_zone_span { 1 } else { 2 };

        let max_pole_pairs = multiplier * slots / phases * layers;
        Self {
            slots,
            phases,
            layers,
            coils_per_coil_group: 0,
            max_pole_pairs,
            multiplier,
            iterator_is_exhausted: false,
        }
    }
}

impl Iterator for CoilsPerCoilGroupIterator {
    type Item = u16;
    fn next(&mut self) -> Option<u16> {
        // Check if the iterator is exhausted
        if self.iterator_is_exhausted {
            return None;
        }

        // Exhaust the iterator
        if 2 * self.phases * self.coils_per_coil_group > self.layers * self.slots {
            self.iterator_is_exhausted = true;
            return None;
        }

        // Increase the iterator
        self.coils_per_coil_group = self.coils_per_coil_group + 1;

        for pole_pairs in 1..(self.max_pole_pairs + 1) {
            if self.layers * self.slots
                == self.multiplier * 2 * self.phases * self.coils_per_coil_group * pole_pairs
            {
                return Some(self.coils_per_coil_group);
            }
        }

        return self.next();
    }
}

/**
A magnetic field which is symmetric over a single pole pair always creates uneven orders:
`o = 1, 3, 5, 7, ...`
This iterator returns these orders:
```
use winding::iterators::PolePairFieldOrders;

let mut iter = PolePairFieldOrders::new();
assert_eq!(iter.next(), Some(1));
assert_eq!(iter.next(), Some(3));
assert_eq!(iter.next(), Some(5));
assert_eq!(iter.next(), Some(7));
```
*/
pub struct PolePairFieldOrders(usize);

impl PolePairFieldOrders {
    pub fn new() -> Self {
        return PolePairFieldOrders(0);
    }
}

impl Iterator for PolePairFieldOrders {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let order = self.nth(self.0);
        self.0 += 1;
        return order;
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        return Some(1 + 2 * n);
    }
}

/**
Return an iterator over a normalized, symmetric multiphase system with a "cos" formulation.

# Examples

```
use em_basic_equations::multiphase_system;
use uom::si::f64::*;
use uom::si::{frequency::hertz, time::second};
use approx;

// Create a three-phase system iterator
let mut iter = multiphase_system(Time::new::<second>(0.0), Frequency::new::<hertz>(50.0), 0.0, 3);

approx::assert_abs_diff_eq!(iter.next().unwrap(), 1.0);
approx::assert_abs_diff_eq!(iter.next().unwrap(), -0.5, epsilon = 1e-15);
approx::assert_abs_diff_eq!(iter.next().unwrap(), -0.5, epsilon = 1e-15);
assert!(iter.next().is_none());
```
 */
pub fn multiphase_system(
    time: stem_wire::prelude::Time,
    frequency: stem_wire::prelude::Frequency,
    offset: f64,
    phases: NonZeroU16,
) -> impl Iterator<Item = f64> + Clone + Send + Sync {
    return (0..phases.get()).map(move |phase| {
        (TAU * f64::from(frequency * time) + offset - TAU * phase as f64 / phases.get() as f64)
            .cos()
    });
}

/// An iterator over the phase sequence that creates a rotating magnetic field.
///
/// The phase sequence can be determined from the voltage phasor star
/// [\(1\)](#phase_sequence_1), section 2.2. The phasor star consists of
/// `2 * phases` beams, representing the positive and negative directions of
/// each phase, with an angular separation of `π / phases` between neighboring
/// beams.
///
/// The positive-phase beams are separated by `2 * π / phases` and are
/// enumerated counter-clockwise. The negative-phase beams are obtained by
/// reversing this enumeration and rotating it by `π`. Superimposing the two
/// stars produces the complete phasor star. For a counter-clockwise rotating
/// magnetic field, the phase sequence is given by the resulting sequence of
/// beam indices.
#[doc = ""]
#[cfg_attr(feature = "doc-images", doc = "![Phase sequence][phase_sequence]")]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("phase_sequence", "docs/img/cad_phase_sequence.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
///
/// # Literature
/// <a id="phase_sequence_1">\(1\)</a>
/// Pyrhönen, J., Jokinen, T., Hrabovcová, V.:
/// *Design of Rotating Electrical Machines*, 1st edition, John Wiley &
/// Sons, 2008
///
/// # Examples
///
/// ```
/// use stem_winding::iterators::PhaseSequence;
///
/// // 2-phase winding
/// let sequence: Vec<_> = PhaseSequence::new(2.try_into().expect("not zero")).collect();
/// assert_eq!(sequence, vec![1, -2, 2, -1]);
///
/// // 3-phase winding
/// let sequence: Vec<_> = PhaseSequence::new(3.try_into().expect("not zero")).collect();
/// assert_eq!(sequence, vec![1, -3, 2, -1, 3, -2]);
///
/// // 5-phase winding
/// let sequence: Vec<_> = PhaseSequence::new(5.try_into().expect("not zero")).collect();
/// assert_eq!(sequence, vec![1, -4, 2, -5, 3, -1, 4, -2, 5, -3]);
/// ```
pub struct PhaseSequence {
    phases: i32,
    index: i32,
    prev_pos: i32,
    prev_neg: i32,
}

impl PhaseSequence {
    /// Returns a new [`PhaseSequence`] for the given number of `phases`.
    pub fn new(phases: NonZeroU16) -> Self {
        PhaseSequence {
            phases: i32::from(phases.get()),
            index: 0,
            prev_pos: i32::from(phases.get()),
            prev_neg: (i32::from(phases.get()) + 1) / 2,
        }
    }
}

impl Iterator for PhaseSequence {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == 2 * self.phases {
            return None;
        }
        self.index += 1;

        if self.index % 2 == 0 {
            let next = self.prev_neg % self.phases + 1;
            self.prev_neg = next;
            Some(-next)
        } else {
            let next = self.prev_pos % self.phases + 1;
            self.prev_pos = next;
            Some(next)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = (2 * self.phases - self.index).abs() as usize;
        (len, Some(len))
    }
}

impl ExactSizeIterator for PhaseSequence {}
