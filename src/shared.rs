use num::{Integer, integer::gcd, rational::Ratio};
use stem_wire::prelude::uom::si::f64::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::f64::consts::{PI, TAU};

/// Connection type used for the winding
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Connection {
    Star,
    Delta,
    // TODO: Implement star-delta and double delta and double star
}

/// Calculate the electrical angle between two slots (2.66 in [1]). In [1] this
/// is expressed by α_z as well: α_u = α_z * p / t
/// However, α_z = 2π * t / Q, therefore this equation can be simplified to the
/// term below
pub fn electrical_slot_angle(slots: u16, p: u16) -> f64 {
    return TAU * (p as f64) / (slots as f64);
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
pub fn hole_number(slots: u16, pole_pairs: u16, phases: u16) -> Ratio<u16> {
    // Get greatest common divisor of numerator and denominator
    let val = num::integer::gcd(slots, 2 * pole_pairs * phases);

    // Calculate smallest integer numerator and denominator
    let n = (2 * pole_pairs * phases) / val;
    let z = slots / val;
    return Ratio::new_raw(z, n);
}

/// Calculate the phase sequence for the given number of phases to generate a
/// rotating field. The sequence is calculated with the star of slots as shown
/// in e.g. [Pyr08] or [Mat20a] The phase sequence star consists of 2*m beams
/// (each phase in positive and negative direction) with the angle (2*pi)/(2*m)
/// between neighboring beams. It is created by creating a positive star first
/// and then inverting it by adding pi to each angle. The positive star has an
/// angle of (2*pi)/m between two neighboring beams.
pub fn phase_sequence(phases: u16) -> (Vec<i32>, Vec<f64>) {
    // Phase angles
    let phase_angle = TAU / (phases as f64);
    let mut angles: Vec<f64> = Vec::with_capacity(2 * phases as usize);
    let mut phase_indices: Vec<i32> = Vec::with_capacity(2 * phases as usize);
    for phase in 0..phases {
        // The angles are normalized by dividing them through 2*pi and just keeping
        // the rest (modulo operation)
        angles.push(phase_angle * phase as f64);
        phase_indices.push(phase as i32 + 1);
        angles.push((phase_angle * phase as f64 + PI) % TAU);
        phase_indices.push(-(phase as i32 + 1));
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
    pole_pairs: u16,
    air_gap_radius_stator: Length,
    air_gap: Length,
    ordinal: f64,
) -> f64 {
    let v_times_p = (ordinal * pole_pairs as f64).abs();
    let a =
        f64::from(air_gap_radius_stator / (air_gap_radius_stator - air_gap)).powf(2.0 * v_times_p);
    return f64::from(air_gap * v_times_p / air_gap_radius_stator * (a + 1.0) / (a - 1.0));
}

/// Calculate the number of basic windings with the formulae from [Pyr08],
/// section 2.11 (p. 102 ff)
///
/// ```
/// use winding::periodicity;
///
/// // 12/4 single-layer integer winding
/// assert_eq!(2, periodicity(12, 2, 3, 1));
///
/// // 12/10 double-layer tooth-coil winding
/// assert_eq!(1, periodicity(12, 5, 3, 2));
///
/// // 12/8 double-layer tooth-coil winding
/// assert_eq!(4, periodicity(12, 4, 3, 2));
///
/// // 36/8 single-layer fractional slot winding
/// assert_eq!(2, periodicity(36, 4, 3, 1));
/// ```
pub fn periodicity(slots: u16, pole_pairs: u16, phases: u16, layers: u16) -> u16 {
    // Inner of basic windings
    let t = gcd(slots, pole_pairs);
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
            if layers == 1 {
                if t.is_odd() {
                    return t;
                } else {
                    return t / 2;
                }

            // Double-layer winding
            } else {
                return t;
            }
        }
    }
}

/// Returns the (electrical) angle between two  slots in radians. In [Pyr08],
/// this value is designated as α_u.
pub fn phasor_angle(slots: u16, pole_pairs: u16) -> f64 {
    return TAU * (pole_pairs as f64) / (slots as f64);
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
