//! Types and functions for calculating the Fourier coefficients of the field
//! excitation curve of a winding.
//!
//! This module contains the [`FecSpectralAnalysis`] struct for calculating the
//! complex Fourier coefficients of a [`Winding::field_excitation_curve`] and
//! the [`sample_fec_values`] function for sampling it. Its
//! public items are re-exported from the parent module.

use std::sync::Arc;

use num::Complex;
use realfft::{RealFftPlanner, RealToComplex};

use crate::error::Error;

use super::*;

/// Samples a step function at equidistant positions into the provided slice.
///
/// The input iterator yields the values of a piecewise-constant function
/// over equally sized intervals. The output slice determines the number
/// of samples. The function is sampled over one period, excluding the
/// endpoint, so the samples can be used as input to a discrete Fourier
/// transform.
///
/// If the input iterator is empty or the output slice is empty, the
/// output slice is left unchanged.
///
/// # Examples
/// ```
/// use stem_winding::prelude::sample_fec_values;
///
/// let step_values = [1.0, 2.0, 3.0, 1.0];
/// let mut samples = [0.0; 8];
///
/// sample_fec_values(step_values.iter().cloned(), &mut samples);
///
/// assert_eq!(samples, [1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 1.0, 1.0]);
/// ```
pub fn sample_fec_values<T: Clone, I: ExactSizeIterator<Item = T>>(
    mut fec_values: I,
    buffer_samples: &mut [T],
) {
    if buffer_samples.len() == 0 {
        return;
    }
    let number_steps = fec_values.len();
    let mut step = 0.0;
    let mut stored_value = match fec_values.next() {
        Some(v) => v,
        None => return,
    };
    let delta_sample_position = number_steps as f64 / buffer_samples.len() as f64;

    for (idx, sample_value) in buffer_samples.iter_mut().enumerate() {
        let sample_position = delta_sample_position * idx as f64;

        /*
        Find the intervall where the current sample position is located.
        */
        loop {
            // If the sample position is within the current bounds, assign the stored value
            // to the sampling element
            if sample_position >= step && sample_position < step + 1.0 {
                *sample_value = stored_value.clone();
                break;
            } else {
                step += 1.0;
                stored_value = match fec_values.next() {
                    Some(v) => v,
                    None => return,
                };
            }
        }
    }
}

/**
Computes the complex Fourier coefficients of the field excitation curve (FEC)
of a [`Winding`].

This struct computes the Fourier spectrum of the
[`field_excitation_curve`](Winding::field_excitation_curve) (FEC) of a winding.
The FEC is sampled at equidistant positions around the air gap and transformed
using a real-to-complex fast Fourier transform (FFT). The indices of the
returned coefficients correspond to the corresponding mechanical spatial order,
starting with the constant component at index 0.

# Examples

```
use approxim::assert_abs_diff_eq;
use stem_winding::prelude::*;

let winding: DistributedWinding = DistributedMinimalBuilder {
    slots: 12.try_into().expect("not zero"),
    pole_pairs: 1.try_into().expect("not zero"),
    phases: 3.try_into().expect("not zero"),
    layers: 2.try_into().expect("not zero"),
    coil_span_reduction: 0,
    zone_span_variation: 0,
    winding_table_constructor: WindingTableConstructor::Tingley,
}
.try_into()
.unwrap();
let mut fec = FecSpectralAnalysis::new(1024);
let output = fec.analyze(&winding, &[1.0, -0.5, -0.5]).unwrap();

// The constant component vanishes for this winding and current configuration.
let (val, _) = output[0].to_polar(); // Constant component
assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

let (val, _) = output[1].to_polar(); // First order
assert_abs_diff_eq!(val, 3.6907, epsilon = 1e-3);

// The even spatial orders vanish for this winding and current configuration.
let (val, _) = output[2].to_polar(); // Second order
assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

let (val, _) = output[3].to_polar(); // Third order
assert_abs_diff_eq!(val, 0.0055, epsilon = 1e-3);

let (val, _) = output[4].to_polar(); // Fourth order
assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

let (val, _) = output[5].to_polar(); // Fifth order
assert_abs_diff_eq!(val, 0.1933, epsilon = 1e-3);
```

# Interpreting the coefficients

Let `N` be the number of samples used for the FFT, and let `C[k]` denote the
coefficient at index `k`. The coefficients are normalized such that, for an
ordinary spatial order `k`, the corresponding sinusoidal component is

`F_k(γ) = A_k * cos(k * γ + φ_k - k * φ_o)`

where `γ` is the mechanical angle, `A_k` is the amplitude, `φ_k` is the
phase angle in radians and `φ_o` is the offset from the origin of `γ`
relative to the first slot (where the FEC starts). For example, if the origin of
`γ` is in the middle of the tooth before the first slot, `φ_o` becomes
`π / number_slots`.

The amplitude and phase are obtained directly from the complex coefficient
(which has already been normalized):

`A_k = |C[k]|, φ_k = arg(C[k])`.

In Rust, these quantities can be obtained using [`Complex::norm`] and
[`Complex::arg`].

The index `k` corresponds to the mechanical spatial order: the associated
sinusoid completes `k` periods over one mechanical revolution. For a machine
with `p` pole pairs, the fundamental field harmonic therefore occurs at
index `p`, not necessarily at index 1.

# Identifying spatial harmonics

If only the nonzero spatial orders are of interest, consider
[`Winding::is_harmonic`] or [`Winding::harmonics`]. To obtain relative
induction amplitudes, use [`Winding::relative_induction_amplitude`].

# Performance

This implementation uses [`realfft`], which is based on
[`rustfft`](https://crates.io/crates/rustfft). For best FFT performance, a
power-of-two sample count is a good default; see the
[RustFFT documentation](https://docs.rs/rustfft/latest/rustfft/) for further
details.

The analysis buffers are reused between calls. Repeatedly analyzing different
current configurations of the same winding therefore requires no further
allocations once the buffers have sufficient capacity. The same
`FecSpectralAnalysis` can also be used with different windings; its FEC buffer
is resized as necessary and reallocates only if its capacity is insufficient.
*/
#[derive(Clone)]
pub struct FecSpectralAnalysis {
    fec_values: Vec<f64>,
    sampled_fec: Vec<f64>,
    coefficients: Vec<Complex<f64>>,
    fft_scratch_buffer: Vec<Complex<f64>>,
    fft_plan: Arc<dyn RealToComplex<f64>>,
}

impl FecSpectralAnalysis {
    /// Returns a new [`FecSpectralAnalysis`] that samples the field excitation
    /// curve of a winding using `samples` samples.
    ///
    /// The number of samples affects FFT performance. When in doubt, use a
    /// power of two.
    pub fn new(samples: usize) -> Self {
        // This preallocates a buffer for a 100-slot-winding, which should be enough in
        // most cases. If it is not, a reallocation is necessary (but that is
        // usually no big deal)
        let fec_values = Vec::with_capacity(100);

        // Create the FFT planner
        let mut real_planner = RealFftPlanner::<f64>::new();
        let fft_plan = real_planner.plan_fft_forward(samples);
        let sampled_fec = fft_plan.make_input_vec();
        let coefficients = fft_plan.make_output_vec();
        let fft_scratch_buffer = fft_plan.make_scratch_vec();

        return Self {
            fec_values,
            sampled_fec,
            coefficients,
            fft_scratch_buffer,
            fft_plan,
        };
    }

    /// Returns the field excitation curve from the most recent
    /// [`analyze`](FecSpectralAnalysis::analyze) call.
    pub fn fec_values(&self) -> &[f64] {
        self.fec_values.as_slice()
    }

    /// Returns the complex Fourier coefficients from the most recent
    /// [`analyze`](FecSpectralAnalysis::analyze) call.
    pub fn coefficients(&self) -> &[Complex<f64>] {
        self.coefficients.as_slice()
    }

    /// Calculates the field excitation curve spectrum for the given currents.
    ///
    /// The results are stored internally and can be accessed via
    /// [`coefficients`](FecSpectralAnalysis::coefficients). Returns an error if
    /// the number of currents does not match the number of phases or if the FFT
    /// fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let winding: DistributedWinding = DistributedMinimalBuilder {
    ///     slots: 12.try_into().expect("not zero"),
    ///     pole_pairs: 1.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 1.try_into().expect("not zero"),
    ///     coil_span_reduction: 0,
    ///     zone_span_variation: 0,
    ///     winding_table_constructor: WindingTableConstructor::CoilSide,
    /// }
    /// .try_into()
    /// .unwrap();
    ///
    /// let mut analysis = FecSpectralAnalysis::new(1024);
    ///
    /// let fundamental_1 = analysis.analyze(&winding, &[1.0, -0.5, -0.5]).expect("succeeds")[1].norm();
    /// assert_abs_diff_eq!(fundamental_1, 1.8453, epsilon = 1e-3);
    ///
    /// // Reuse the same analyzer for a different current configuration.
    /// let fundamental_2 = analysis.analyze(&winding, &[0.0, 2.0, -2.0]).expect("succeeds")[1].norm();
    /// assert_abs_diff_eq!(fundamental_2, 4.2589, epsilon = 1e-3);
    /// ```
    pub fn analyze<W: Winding + ?Sized>(
        &mut self,
        winding: &W,
        currents: &[f64],
    ) -> Result<&[Complex<f64>], Error> {
        self.fec_values.resize(winding.slots().get().into(), 0.0);

        // Calculate the field excitation curve (FEC)
        winding.field_excitation_curve(self.fec_values.as_mut_slice(), currents)?;

        // Switch to non-generic version to reduce code bloat
        self.analyze_priv()
    }

    fn analyze_priv(&mut self) -> Result<&[Complex<f64>], Error> {
        // Upsample the FEC
        sample_fec_values(self.fec_values.iter().cloned(), &mut self.sampled_fec);

        // Perform the FFT on the buffer
        self.fft_plan.process_with_scratch(
            &mut self.sampled_fec,
            &mut self.coefficients,
            &mut self.fft_scratch_buffer,
        )?;

        // Normalize the output: Divide it by N (number of input elements) and multiply
        // by 2 to account for the negative side of the spectrum
        let inverse = 1.0 / self.sampled_fec.len() as f64;

        self.coefficients[0] *= inverse;

        let end = self.coefficients.len();
        let has_nyquist = self.sampled_fec.len() % 2 == 0;
        let ordinary_end = if has_nyquist { end - 1 } else { end };

        self.coefficients[1..ordinary_end]
            .iter_mut()
            .for_each(|value| *value *= 2.0 * inverse);

        if has_nyquist {
            self.coefficients[end - 1] *= inverse;
        }

        // The first value equals the constant part and does not occur twice, but still
        // needs to be normalized
        self.coefficients[0] = self.coefficients[0] * inverse;

        return Ok(self.coefficients.as_slice());
    }
}
