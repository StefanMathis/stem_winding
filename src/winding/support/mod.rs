//! Supporting types and iterators used by [`Winding`](super::Winding).
//!
//! This module contains implementation details and auxiliary types related
//! to winding analysis. Its public items are re-exported from the parent
//! module.

use approxim;
use std::{f64::consts::TAU, num::NonZeroU16};
use stem_types::{SpatialOrder, Zone};

use crate::coils::Coil;

use super::*;

mod fec_spectral_analysis;
pub use fec_spectral_analysis::*;

/// Electrical connection of the phases of a [`Winding`].
///
/// The connection determines how the winding phase voltages and currents
/// relate to the line voltages and currents of a symmetric multi-phase power
/// supply.
///
/// [`Connection::Star`] connects one terminal of each phase to a common
/// neutral point. [`Connection::Delta`] connects the phases in a closed
/// loop, with the line terminals connected at the phase junctions.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum Connection {
    /// Star (Y) connection with a common neutral point.
    Star,
    /// Delta (Δ) connection.
    Delta,
}

impl Connection {
    /// Returns the complex conversion factor from line voltage to phase voltage
    /// for a symmetric multi-phase power supply.
    ///
    /// The returned value `k` relates the line voltage `U_line` to the winding
    /// phase voltage `U_phase` as
    ///
    /// `U_phase = k * U_line`.
    ///
    /// For a [`Connection::Star`] connection, the phase voltage is the voltage
    /// between a phase terminal and the neutral point. For a
    /// [`Connection::Delta`] connection, the phase voltage is equal to the line
    /// voltage.
    ///
    /// The conversion assumes a symmetric power supply with `phases` equally
    /// spaced phase voltages.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let phases = NonZeroU16::new(3).unwrap();
    ///
    /// let factor = Connection::Star.line_to_phase_voltage(phases);
    /// assert_abs_diff_eq!(0.57735, factor.norm(), epsilon = 0.0001);
    /// assert_abs_diff_eq!(0.5, factor.re, epsilon = 0.0001);
    /// assert_abs_diff_eq!(0.288675, factor.im, epsilon = 0.0001);
    ///
    /// let factor = Connection::Delta.line_to_phase_voltage(phases);
    /// assert_abs_diff_eq!(1.0, factor.norm(), epsilon = 0.0001);
    /// assert_abs_diff_eq!(1.0, factor.re, epsilon = 0.0001);
    /// assert_abs_diff_eq!(0.0, factor.im, epsilon = 0.0001);
    /// ```
    pub fn line_to_phase_voltage(&self, phases: NonZeroU16) -> num::Complex<f64> {
        // Assumption of a symmetric grid
        let a = Complex::new(0.0, TAU / (phases.get() as f64)).exp();
        match self {
            Connection::Star => return Complex::new(1.0, 0.0) / (Complex::new(1.0, 0.0) - a),
            Connection::Delta => return Complex::new(1.0, 0.0),
        }
    }

    /// Returns the ratio of the phase-voltage magnitude to the line-voltage
    /// magnitude for a symmetric multi-phase power supply.
    ///
    /// This is the magnitude of [`Connection::line_to_phase_voltage`]:
    ///
    /// `|U_phase| / |U_line|`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let phases = NonZeroU16::new(3).unwrap();
    ///
    /// assert_abs_diff_eq!(0.57735, Connection::Star.ratio_line_to_phase_voltage(phases), epsilon = 0.0001);
    /// assert_abs_diff_eq!(1.0, Connection::Delta.ratio_line_to_phase_voltage(phases), epsilon = 0.0001);
    /// ```
    pub fn ratio_line_to_phase_voltage(&self, phases: NonZeroU16) -> f64 {
        self.line_to_phase_voltage(phases).norm()
    }

    /// Returns the phase-angle difference from line voltage to phase voltage
    /// for a symmetric multi-phase power supply.
    ///
    /// The returned angle is the phase-voltage angle relative to the
    /// line-voltage angle, in radians.
    ///
    /// This is the argument of [`Connection::line_to_phase_voltage`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let phases = NonZeroU16::new(3).unwrap();
    ///
    /// assert_abs_diff_eq!(0.52359, Connection::Star.angle_line_to_phase_voltage(phases), epsilon = 0.0001);
    /// assert_abs_diff_eq!(0.0, Connection::Delta.angle_line_to_phase_voltage(phases), epsilon = 0.0001);
    /// ```
    pub fn angle_line_to_phase_voltage(&self, phases: NonZeroU16) -> f64 {
        self.line_to_phase_voltage(phases).arg()
    }

    /// Returns the complex conversion factor from line current to phase current
    /// for a symmetric multi-phase power supply.
    ///
    /// The returned value `k` relates the line current `I_line` to the winding
    /// phase current `I_phase` as
    ///
    /// `I_phase = k * I_line`.
    ///
    /// For a [`Connection::Star`] connection, the phase current is equal to the
    /// line current. For a [`Connection::Delta`] connection, the phase current
    /// is the current through a winding phase between two line terminals.
    ///
    /// The conversion assumes a symmetric power supply with `phases` equally
    /// spaced phase currents.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let phases = NonZeroU16::new(3).unwrap();
    ///
    /// let factor = Connection::Star.line_to_phase_current(phases);
    /// assert_abs_diff_eq!(1.0, factor.norm(), epsilon = 0.0001);
    /// assert_abs_diff_eq!(1.0, factor.re, epsilon = 0.0001);
    /// assert_abs_diff_eq!(0.0, factor.im, epsilon = 0.0001);
    ///
    /// let factor = Connection::Delta.line_to_phase_current(phases);
    /// assert_abs_diff_eq!(0.57735, factor.norm(), epsilon = 0.0001);
    /// assert_abs_diff_eq!(0.5, factor.re, epsilon = 0.0001);
    /// assert_abs_diff_eq!(-0.288675, factor.im, epsilon = 0.0001);
    /// ```
    pub fn line_to_phase_current(&self, phases: NonZeroU16) -> num::Complex<f64> {
        // Assumption of a symmetric grid
        let a = Complex::new(0.0, -TAU / (phases.get() as f64)).exp();
        match self {
            Connection::Star => Complex::new(1.0, 0.0),
            Connection::Delta => Complex::new(1.0, 0.0) / (Complex::new(1.0, 0.0) - a),
        }
    }

    /// Returns the ratio of the phase-current magnitude to the line-current
    /// magnitude for a symmetric multi-phase power supply.
    ///
    /// This is the magnitude of [`Connection::line_to_phase_current`]:
    ///
    /// `|I_phase| / |I_line|`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let phases = NonZeroU16::new(3).unwrap();
    ///
    /// assert_abs_diff_eq!(1.0, Connection::Star.ratio_line_to_phase_current(phases), epsilon = 0.0001);
    /// assert_abs_diff_eq!(0.57735, Connection::Delta.ratio_line_to_phase_current(phases), epsilon = 0.0001);
    /// ```
    pub fn ratio_line_to_phase_current(&self, phases: NonZeroU16) -> f64 {
        self.line_to_phase_current(phases).norm()
    }

    /// Returns the phase-angle difference from line current to phase current
    /// for a symmetric multi-phase power supply.
    ///
    /// The returned angle is the phase-current angle relative to the
    /// line-current angle, in radians.
    ///
    /// This is the argument of [`Connection::line_to_phase_current`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let phases = NonZeroU16::new(3).unwrap();
    ///
    /// assert_abs_diff_eq!(0.0, Connection::Star.angle_line_to_phase_current(phases), epsilon = 0.0001);
    /// assert_abs_diff_eq!(-0.52359, Connection::Delta.angle_line_to_phase_current(phases), epsilon = 0.0001);
    /// ```
    pub fn angle_line_to_phase_current(&self, phases: NonZeroU16) -> f64 {
        self.line_to_phase_current(phases).arg()
    }
}

/// Returns the hole number `q` of a winding as a reduced fraction.
///
/// The hole number is the number of slots per pole and phase:
///
/// `q = slots / (2 * pole_pairs * phases)`.
///
/// It can be represented as
///
/// `q = g + z / n`,
///
/// where `g` is the integer part and `z` and `n` are integers with
/// `0 <= z < n`. The returned [`Ratio`] is reduced, so its numerator and
/// denominator directly provide `z` and `n` for the fractional part.
///
/// ```
/// use std::num::NonZeroU16;
/// use stem_winding::winding::hole_number;
///
/// const fn nz(value: u16) -> NonZeroU16 {
///    NonZeroU16::new(value).expect("not zero")
/// }
///
/// // A 12/10 three-phase winding has a hole number of 2/5, which means that g = 0, z = 2, n = 5
/// let ratio = hole_number(nz(12), nz(5), nz(3));
/// let g = ratio.trunc().numer().clone();
/// let z = ratio.fract().numer().clone();
/// let n = ratio.denom().clone();
/// assert_eq!(g, 0);
/// assert_eq!(z, 2);
/// assert_eq!(n, 5);
///
/// // A 36/10 three-phase winding has a hole number of 6/5, which means that g = 1, z = 1, n = 5
/// let ratio = hole_number(nz(36), nz(5), nz(3));
/// let g = ratio.trunc().numer().clone();
/// let z = ratio.fract().numer().clone();
/// let n = ratio.denom().clone();
/// assert_eq!(g, 1);
/// assert_eq!(z, 1);
/// assert_eq!(n, 5);
///
/// // A 36/4 three-phase winding has a hole number of 3, which means that g = 3, z = 0, n = 1
/// let ratio = hole_number(nz(36), nz(2), nz(3));
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
    /// The item returned with the random access.
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

/// Returns the number of repetitions of the smallest pattern in a collection.
///
/// The collection must consist entirely of repetitions of the same pattern.
/// The smallest pattern is the shortest prefix of the collection whose
/// repetitions make up the entire collection.
///
/// The collection is accessed through the [`RandomAccess`] trait, allowing
/// values to be retrieved from stored data or calculated on demand. The
/// pattern has a length of `collection_len / count`, where `count` is the value
/// returned by this function.
///
/// This algorithm has a time complexity of O(n) and a space complexity of O(1).
///
/// Within stem_winding, this function is used to implement
/// [Winding::base_winding_count].
///
/// # Examples
///
/// ```
/// use std::num::NonZeroUsize;
///
/// use stem_winding::winding::repeating_pattern_count;
///
/// let collection = [1, 0, 0, 0, 1, 0, 0, 0];
/// let len = NonZeroUsize::new(collection.len()).expect("not zero");
/// let expected = NonZeroUsize::new(2).expect("not zero");
/// assert_eq!(expected, repeating_pattern_count(&collection, len));
///
/// let collection = [1, 1, 0, 0, 0, 1, 0, 0, 0];
/// let len = NonZeroUsize::new(collection.len()).expect("not zero");
/// let expected = NonZeroUsize::new(1).expect("not zero");
/// assert_eq!(expected, repeating_pattern_count(&collection, len));
///
/// let collection = [1, 2, 1, 2, 1, 2, 1, 2];
/// let len = NonZeroUsize::new(collection.len()).expect("not zero");
/// let expected = NonZeroUsize::new(4).expect("not zero");
/// assert_eq!(expected, repeating_pattern_count(&collection, len));
/// ```
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

/// Returns the number of base windings for a symmetric winding.
///
/// For symmetric windings ([`winding_factor`](Winding::winding_factor) and
/// [`series_turns_per_phase`](Winding::series_turns_per_phase) identical for
/// each phase), the number of base windings can be determined in constant time
/// from the winding parameters. This function implements the algorithm
/// presented in [\[1\]](#base_winding_count_symmetric_1), section 2.11.
///
/// It is a specialized implementation of [`Winding::base_winding_count`] with
/// O(1) time and O(1) space complexity and is used by the symmetric winding
/// types provided by stem_winding, such as [`DistributedWinding`]. The generic
/// implementation of [`Winding::base_winding_count`] uses
/// [`repeating_pattern_count`], which has O(n) time and O(1) space complexity.
///
/// # Literature
/// <a id="base_winding_count_symmetric_1">\[1\]</a>
/// Pyrhönen, J., Jokinen, T., Hrabovcová, V.: Design of rotating electrical
/// machines, 1st edition, John Wiley & Sons, 2008
///
/// # Examples
///
/// ```
/// use std::num::NonZeroU16;
///
/// use stem_winding::winding::base_winding_count_symmetric;
///
/// const fn nz(value: u16) -> NonZeroU16 {
///    NonZeroU16::new(value).expect("not zero")
/// }
///
/// // 12/4 single-layer integer winding
/// assert_eq!(nz(2), base_winding_count_symmetric(nz(12), nz(2), nz(3), nz(1)));
///
/// // 12/10 double-layer tooth-coil winding
/// assert_eq!(nz(1), base_winding_count_symmetric(nz(12), nz(5), nz(3), nz(2)));
///
/// // 12/8 double-layer tooth-coil winding
/// assert_eq!(nz(4), base_winding_count_symmetric(nz(12), nz(4), nz(3), nz(2)));
///
/// // 36/8 single-layer fractional-slot winding
/// assert_eq!(nz(2), base_winding_count_symmetric(nz(36), nz(4), nz(3), nz(1)));
/// ```
pub fn base_winding_count_symmetric(
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    phases: NonZeroU16,
    layers: NonZeroU16,
) -> NonZeroU16 {
    // Inner of basic windings
    let t = NonZeroU16::new(gcd(slots.get(), pole_pairs.get()))
        .expect("cannot be zero, because gcd inputs are not zero");
    let r = hole_number(slots, pole_pairs, phases);
    let n = r.denom().clone();

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

/// Returns the maximum number of parallel paths for the topology parameters of
/// a symmetric winding.
///
/// Assumes that each phase has the same winding factor and the same number
/// of series turns per phase.
///
/// The underlying algorithm is described in
/// [\[1\]](#max_parallel_paths_symmetric_1), pp. 37: First, the number of coils
/// per phase in a base winding is calculated using
/// [`base_winding_count_symmetric`]. If this number is even, the maximum number
/// of parallel paths is twice the base winding count . Otherwise, it equals the
/// base winding count. At least one coil group per phase is always possible
///
/// # Literature
///
/// <a id="max_parallel_paths_symmetric_1">\[1\]</a>
/// Sequenz, H.: Die Wicklungen elektrischer Maschinen -
/// Wechselstrom-Ankerwicklungen 1st edition, Springer-Verlag Wien, 1950
///
/// # Examples
///
/// ```
/// use std::num::NonZeroU16;
///
/// use stem_winding::winding::max_parallel_paths_symmetric;
///
/// const fn nz(value: u16) -> NonZeroU16 {
///    NonZeroU16::new(value).expect("not zero")
/// }
///
/// // 12/4 single-layer integer winding (equal base winding count)
/// assert_eq!(nz(2), max_parallel_paths_symmetric(nz(12), nz(2), nz(3), nz(1)));
///
/// // 12/10 double-layer tooth-coil winding (twice base winding count)
/// assert_eq!(nz(2), max_parallel_paths_symmetric(nz(12), nz(5), nz(3), nz(2)));
///
/// // 12/8 double-layer tooth-coil winding (equal base winding count)
/// assert_eq!(nz(4), max_parallel_paths_symmetric(nz(12), nz(4), nz(3), nz(2)));
///
/// // 36/8 single-layer fractional-slot winding (equal base winding count)
/// assert_eq!(nz(2), max_parallel_paths_symmetric(nz(36), nz(4), nz(3), nz(1)));
/// ```
pub fn max_parallel_paths_symmetric(
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    phases: NonZeroU16,
    layers: NonZeroU16,
) -> NonZeroU16 {
    let t = base_winding_count_symmetric(slots, pole_pairs, phases, layers);
    let number_of_coils_in_base_winding = slots.get() * layers.get() / (t.get() * 2 * phases.get());
    if number_of_coils_in_base_winding % 2 == 0 {
        // Antiparallel coil groups are possible
        return NonZeroU16::new(2 * t.get()).expect("t is not zero");
    } else {
        // Only parallel coil groups are possible
        return t;
    }
}

/// Returns the electrical angle between the phasors of two neighboring
/// slots.
///
/// The angle is calculated as
///
/// `phasor_angle = 2π * pole_pairs / slots`.
///
/// This relation is given by Pyrhönen, J., Jokinen, T., Hrabovcová, V.:
/// Design of Rotating Electrical Machines, 1st edition, John Wiley &
/// Sons, 2008, eq. (2.66).
pub fn phasor_angle(slots: NonZeroU16, pole_pairs: NonZeroU16) -> f64 {
    return TAU * (pole_pairs.get() as f64) / (slots.get() as f64);
}

/// An iterator over the possible numbers of parallel paths.
///
/// The possible numbers of parallel paths are the divisors of the maximum
/// number of parallel paths. The iterator yields these numbers in ascending
/// order, starting with 1 and ending with the maximum.
///
/// # Examples
///
/// ```
/// use std::num::NonZeroU16;
///
/// use stem_winding::winding::ParallelPathIterator;
///
/// let max_par_paths = NonZeroU16::new(4).expect("not zero");
/// let par_paths: Vec<u16> = ParallelPathIterator::new(max_par_paths)
///     .map(|pp|pp.get())
///     .collect();
/// assert_eq!(par_paths, vec![1, 2, 4]);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ParallelPathIterator {
    max_parallel_paths: NonZeroU16,
    index: u16,
}

impl ParallelPathIterator {
    /// Creates a new [`ParallelPathIterator`].
    pub fn new(max_parallel_paths: NonZeroU16) -> ParallelPathIterator {
        return ParallelPathIterator {
            max_parallel_paths,
            index: 0,
        };
    }

    /// Creates a [`ParallelPathIterator`] from the topology parameters of a
    /// symmetric winding.
    ///
    /// Assumes that each phase has the same winding factor and the same number
    /// of series turns per phase.
    ///
    /// This is a convenience wrapper around [`max_parallel_paths_symmetric`]
    /// and [`ParallelPathIterator::new`].
    pub fn from_symmetric_winding_topology(
        slots: NonZeroU16,
        pole_pairs: NonZeroU16,
        phases: NonZeroU16,
        layers: NonZeroU16,
    ) -> Self {
        Self::new(max_parallel_paths_symmetric(
            slots, pole_pairs, phases, layers,
        ))
    }

    /// Creates a [`ParallelPathIterator`] from a symmetric winding.
    ///
    /// Assumes that each phase has the same winding factor and the same number
    /// of series turns per phase.
    ///
    /// This is a convenience wrapper around
    /// [`ParallelPathIterator::from_symmetric_winding_topology`].
    pub fn from_winding<W: Winding + ?Sized>(winding: &W) -> Self {
        Self::from_symmetric_winding_topology(
            winding.slots(),
            winding.pole_pairs(),
            winding.phases(),
            winding.layers(),
        )
    }
}

impl<W: Winding + ?Sized> From<&W> for ParallelPathIterator {
    fn from(value: &W) -> Self {
        Self::from_winding(value)
    }
}

impl Iterator for ParallelPathIterator {
    type Item = NonZeroU16;
    fn next(&mut self) -> Option<NonZeroU16> {
        if self.index <= self.max_parallel_paths.get() {
            for _ in self.index..(self.max_parallel_paths.get() + 1) {
                self.index = self.index + 1;
                if (self.max_parallel_paths.get()) % self.index == 0 {
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
pub(super) fn is_harmonic<W: Winding + ?Sized>(
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

/// Returns an iterator over the instantaneous values of a normalized,
/// symmetric multiphase system using a cosine formulation.
///
/// Each phase is shifted by `2π / phases` relative to the preceding phase.
/// `offset` specifies an electrical phase angle offset in radians.
///
/// # Examples
///
/// ```
/// use approxim::assert_abs_diff_eq;
/// use stem_winding::prelude::*;
/// use stem_winding::winding::*;
///
/// // Create a three-phase system iterator
/// let mut iter = multiphase_system(
///     Time::new::<second>(0.0),
///     Frequency::new::<hertz>(50.0),
///     0.0,
///     3.try_into().expect("not zero")
/// );
///
/// assert_abs_diff_eq!(iter.next().unwrap(), 1.0);
/// assert_abs_diff_eq!(iter.next().unwrap(), -0.5, epsilon = 1e-15);
/// assert_abs_diff_eq!(iter.next().unwrap(), -0.5, epsilon = 1e-15);
/// assert!(iter.next().is_none());
/// ```
pub fn multiphase_system(
    time: stem_wire::prelude::Time,
    frequency: stem_wire::prelude::Frequency,
    offset: f64,
    phases: NonZeroU16,
) -> impl Iterator<Item = f64> + Clone + Send + Sync {
    (0..phases.get()).map(move |phase| {
        (TAU * f64::from(frequency * time) + offset
            - TAU * f64::from(phase) / f64::from(phases.get()))
        .cos()
    })
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
/// use stem_winding::winding::PhaseSequence;
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

/// Returns the conversion factor that relates RMS phase current to the
/// electric loading of the machine.
///
/// Electric loading describes the magnetic loading per unit air-gap
/// circumference and is an important design parameter for evaluating
/// the utilization of an electric machine. This function calculates the
/// factor `k` in
///
/// `A = k * I`
///
/// where `A` is the electric loading and `I` is the RMS phase current.
/// The calculation follows
/// [\[1\]](#current_to_electric_loading_conversion_factor_1),
/// eq. (9.1.23c), and is valid only for
/// [symmetric](Winding::is_symmetric) windings. The `series_turns_per_phase`
/// argument can be obtained from [`Winding::series_turns_per_phase_float`],
/// `air_gap_length` from [`CoreExt::air_gap_length`].
///
/// For small, air-cooled induction and permanent-magnet machines, electric
/// loading is typically in the range of 20–120 A/mm. For electrically
/// excited synchronous machines, the ranges in
/// [\[1\]](#current_to_electric_loading_conversion_factor_1),
/// table 9.1.4, provide a reference (all values in A/mm):
///
/// | Indirect air cooling | Indirect H² cooling | Direct air cooling | Direct H² cooling |
/// | -------------------- | ------------------- | ------------------ | ----------------- |
/// | 30 ... 120           | 90 ... 150          | 120 ... 200        | 160 ... 300       |
///
/// These values can be useful in the rough design phase to evaluate initial
/// candidates. They do not replace a detailed thermal simulation.
///
/// # Literature
/// <a id="current_to_electric_loading_conversion_factor_1">\[1\]</a>
/// Müller, G., Vogt, K. and Ponick, B.: Berechnung elektrischer Maschinen,
/// 6th edition, Wiley-VCH, 2008
pub fn current_to_electric_loading_conversion_factor(
    series_turns_per_phase: f64,
    phases: u16,
    air_gap_length: Length,
) -> ReciprocalLength {
    return 2.0 * series_turns_per_phase * f64::from(phases) / air_gap_length;
}
