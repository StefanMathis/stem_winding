use approxim;
use num::rational::Ratio;
use std::{f64::consts::TAU, num::NonZeroU16};
use stem_coil_layout::Zone;

use crate::{coils::Coil, winding::Winding};

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

/**
This iterator returns all winding harmonic ordinals of a symmetric winding in ascending order.
As described in [Bin12] p.126ff., the ordinals of the waves created by the winding can be represented by their
integer value v* or the fraction value v, where v = 1 is the ordinal creating the constant torque coomponent.
The `HarmonicOrdinalsIterator` returns the fraction v. The numerator is v*, the denominator is the number of poles in a basic winding.

Deviating from [Bin12], the ordinal of the pole pair harmonic is always positive. In [Bin12], p. 135f., the ordinal signs
show the coupling to the very first ordinal, which can be a sub-ordinal of the pole pair ordinal (and possibly a negative coupling).


# Example 1 (integer-slot winding):
```
use winding::{DistributedWinding, Winding};
use num::rational::Ratio;

let winding = DistributedWinding::default(); // This is a 6/2 single-layer integer-slot winding
let ordinals: Vec<Ratio<i32>> = winding.harmonic_ordinals().take(5).collect();
assert_eq!(ordinals[0], Ratio::new(1, 1));
assert_eq!(ordinals[1], Ratio::new(-5, 1));
assert_eq!(ordinals[2], Ratio::new(7, 1));
assert_eq!(ordinals[3], Ratio::new(-11, 1));
assert_eq!(ordinals[4], Ratio::new(13, 1));
```

# Example 2 (tooth-coil winding):
```
use winding::{ToothCoilWinding, Winding, WindingTableMethod};
use num::rational::Ratio;

let winding = ToothCoilWinding::new_minimal(24, 10, 3, 2, WindingTableMethod::Tingley).unwrap();
let ordinals: Vec<Ratio<i32>> = winding.harmonic_ordinals().take(5).collect();
assert_eq!(ordinals[0], Ratio::new(-1, 5));
assert_eq!(ordinals[1], Ratio::new(5, 5));
assert_eq!(ordinals[2], Ratio::new(-7, 5));
assert_eq!(ordinals[3], Ratio::new(11, 5));
assert_eq!(ordinals[4], Ratio::new(-13, 5));
```
*/
#[derive(Clone)]
pub struct HarmonicOrdinalsIterator<'a> {
    winding: &'a dyn Winding,
    p_bw: u16,
    v_star: u16,
    coupling: i32,
}

impl<'a> HarmonicOrdinalsIterator<'a> {
    pub fn new(winding: &'a dyn Winding) -> HarmonicOrdinalsIterator<'a> {
        // Calculate the number of pole pairs in the basic winding
        let p_bw = winding.pole_pairs().get() / winding.base_winding_count().get();
        let iterator = HarmonicOrdinalsIterator {
            winding,
            p_bw,
            v_star: 0,
            coupling: 1, // Temporary value
        };
        let coupling = iterator.coupling_with_pole_pairs().unwrap_or(1);
        return HarmonicOrdinalsIterator {
            winding,
            p_bw,
            v_star: 0,
            coupling: coupling,
        };
    }

    pub fn coupling(&self) -> i32 {
        self.coupling
    }

    /**
    Identify the coupling the given pole pair number.
    This is done by running a copied iterator which assumes that the coupling is
    positive. If some ordinal equals 1, this assumption was true.
    If some ordinal equals -1, this assumption was false.
     */
    pub(crate) fn coupling_with_pole_pairs(mut self) -> Option<i32> {
        loop {
            let ratio = self.next().expect("this is an infinite iterator");

            // The absolute value of the iterator increases strictly monotonic -> once it is
            // larger than 1, the coupling calculation failed
            if ratio.numer().abs() > ratio.denom().abs() {
                return None;
            }

            // Equal 1 => some coupling has been detected
            if ratio.numer().abs() == ratio.denom().abs() {
                // Find the sign
                if ratio.numer() * ratio.denom() > 0 {
                    return Some(1);
                } else {
                    return Some(-1);
                }
            }
        }
    }
}

impl<'a> Iterator for HarmonicOrdinalsIterator<'a> {
    type Item = Ratio<i32>;
    fn next(&mut self) -> Option<Ratio<i32>> {
        // Stop the iteration if an overflow would occur when increasing v_star
        if self.v_star == u16::MAX {
            return None;
        }

        // Update v_star
        self.v_star = self.v_star + 1;

        // For a symmetric winding, all v* which are multiples of the number of phases
        // are 0, therefore those ordinals can be skipped. [Bin12], p. 132,
        // section c)
        if self.v_star % self.winding.phases() == 0 {
            return self.next();
        }

        /*
        Calculate the winding factor. Since the winding is symmetric by definition, it sufficient
        to calculate the winding factor for a single phase. If the winding factor is zero, the
        current ordinal can be skipped as well.
        */
        let v = self.v_star as f64 / self.p_bw as f64;
        let k_w = self.winding.winding_factor(NonZeroU16::MIN, v); // Any other phase would work as well.
        if approxim::abs_diff_eq!(k_w, 0.0, epsilon = 1e-12) {
            return self.next();
        }

        /*
        The winding factor is not zero, which means that the ordinal v_star occurs. Now it is necessary
        to identify the direction of the ordinal (i.e. if it is positive or negative.).
        The general equation for the magnetic voltage of a phase i is:
        V_i =   V_amp/2*cos(v_star*y - v_star*2*pi/phases*(i-1) + omega*t - 2*pi/phases*(i-1)) +
                V_amp/2*cos(v_star*y - v_star*2*pi/phases*(i-1) - omega*t + 2*pi/phases*(i-1))
        Without a loss of generality, we can set t and y to zero, which simplifies the equation to:
        V_i =   V_amp/2*cos(- v_star*2*pi/phases*(i-1) - 2*pi/phases*(i-1)) +
                V_amp/2*cos(- v_star*2*pi/phases*(i-1) + 2*pi/phases*(i-1))
        If the sum of the first cosine term of all phases is zero, then the ordinal is positive, otherwise
        it is negative according to [Bin12], p. 123.
        */
        let mut sum: f64 = 0.0;
        let phases = self.winding.phases();
        for phase in 0..phases.get() {
            sum = sum
                + (-(phase as f64) * TAU / phases.get() as f64 * (self.v_star as f64 + 1.0)).cos();
        }

        // The signs of the ordinals returned by the iterator are all in relation to the
        // pole pair ordinal, i.e. their corresponding waves are moving in the
        // same direction as the wave of the pole pair ordinal. The algorithm
        // described in [Bin12] however returns the ordinals in relation to the
        // lowest-order ordinal, which may be a subharmonic of the pole pair
        // harmonic. Therefore, this coupling factor is used to adjust
        // the sign of the ordinal.
        if approxim::abs_diff_eq!(sum, 0.0, epsilon = 1e-12) {
            return Some(Ratio::new(
                i32::from(self.v_star) * self.coupling,
                i32::from(self.p_bw),
            ));
        } else {
            return Some(Ratio::new(
                -(i32::from(self.v_star)) * self.coupling,
                i32::from(self.p_bw),
            ));
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.v_star += n as u16;
        return self.next();
    }
}

pub struct NormalizedInductionIterator<'a>(pub HarmonicOrdinalsIterator<'a>);

impl<'a> NormalizedInductionIterator<'a> {
    pub fn new(winding: &'a dyn Winding) -> Self {
        return HarmonicOrdinalsIterator::new(winding).into();
    }
}

impl<'a> Iterator for NormalizedInductionIterator<'a> {
    type Item = (Ratio<i32>, f64);

    fn next(&mut self) -> Option<Self::Item> {
        let ratio = self.0.next()?;
        let ordinal = *ratio.numer() as f64 / *ratio.denom() as f64;
        let winding_factor = self.0.winding.winding_factor(NonZeroU16::MIN, ordinal);
        let amp = (winding_factor / ordinal).abs();
        return Some((ratio, amp));
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.0.v_star += n as u16;
        return self.next();
    }
}

impl<'a> From<HarmonicOrdinalsIterator<'a>> for NormalizedInductionIterator<'a> {
    fn from(value: HarmonicOrdinalsIterator<'a>) -> Self {
        Self(value)
    }
}

impl<'a> From<NormalizedInductionIterator<'a>> for HarmonicOrdinalsIterator<'a> {
    fn from(value: NormalizedInductionIterator<'a>) -> Self {
        value.0
    }
}

/// This iterator returns all coils of the coil motor starting in the first
/// slot, going through all layers and then proceeding to the next slot. Each
/// coil is only returned once.
pub struct CoilsIterator<'a> {
    winding: &'a dyn Winding,
    slots: NonZeroU16,
    layers: NonZeroU16,
    slot_counter: u16,
    layer_counter: u16,
}
impl<'a> CoilsIterator<'a> {
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
A magnetic field which is symmetric over a single pole pair always creates uneven ordinals:
`o = 1, 3, 5, 7, ...`
This iterator returns these ordinals:
```
use winding::iterators::PolePairFieldOrdinals;

let mut iter = PolePairFieldOrdinals::new();
assert_eq!(iter.next(), Some(1));
assert_eq!(iter.next(), Some(3));
assert_eq!(iter.next(), Some(5));
assert_eq!(iter.next(), Some(7));
```
*/
pub struct PolePairFieldOrdinals(usize);

impl PolePairFieldOrdinals {
    pub fn new() -> Self {
        return PolePairFieldOrdinals(0);
    }
}

impl Iterator for PolePairFieldOrdinals {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let ordinal = self.nth(self.0);
        self.0 += 1;
        return ordinal;
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

#[cfg(feature = "cairo")]
#[derive(Clone, Debug)]
pub struct WindingZoneDrawables {
    pub(crate) winding_zones: WindingZones,
    pub(crate) zone_config: crate::ZoneConfig,
}

#[cfg(feature = "cairo")]
impl WindingZoneDrawables {
    pub(crate) fn new(winding_zones: WindingZones, zone_config: crate::ZoneConfig) {
        return WindingZoneDrawables {};
    }
}

#[cfg(feature = "cairo")]
impl Iterator for WindingZoneDrawables {
    type Item = (Zone, Drawable);

    fn next(&mut self) -> Option<Self::Item> {
        let zone = self.winding_zones.next()?;
        return Some(zone.into_drawable());
    }
}
