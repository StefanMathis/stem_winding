//! Types and functions for calculating the Fourier coefficients of the field
//! excitation curve of a winding.
//!
//! This module contains the [`MmfSpectralAnalysis`] struct for calculating the
//! complex Fourier coefficients of a [`Winding::magnetomotive_force`] and
//! the [`sample_mmf_values`] function for sampling it. Its
//! public items are re-exported from the parent module.

use std::sync::Arc;

use num::Complex;
use realfft::{FftError, RealFftPlanner, RealToComplex};

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
/// use stem_winding::winding::sample_mmf_values;
///
/// let step_values = [1.0, 2.0, 3.0, 1.0];
/// let mut samples = [0.0; 8];
///
/// sample_mmf_values(step_values.iter().cloned(), &mut samples);
///
/// assert_eq!(samples, [1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 1.0, 1.0]);
/// ```
pub fn sample_mmf_values<T: Clone, I: ExactSizeIterator<Item = T>>(
    mut mmf_values: I,
    buffer_samples: &mut [T],
) {
    if buffer_samples.is_empty() {
        return;
    }
    let number_steps = mmf_values.len();
    let mut step = 0.0;
    let mut stored_value = match mmf_values.next() {
        Some(v) => v,
        None => return,
    };
    let delta_sample_position = number_steps as f64 / buffer_samples.len() as f64;

    for (idx, sample_value) in buffer_samples.iter_mut().enumerate() {
        let sample_position = delta_sample_position * idx as f64;

        /*
        Find the interval where the current sample position is located.
        */
        loop {
            // If the sample position is within the current bounds, assign the stored value
            // to the sampling element
            if sample_position >= step && sample_position < step + 1.0 {
                *sample_value = stored_value.clone();
                break;
            } else {
                step += 1.0;
                stored_value = match mmf_values.next() {
                    Some(v) => v,
                    None => return,
                };
            }
        }
    }
}

/**
Computes the complex Fourier coefficients of the magnetomotive force (MMF)
distribution produced by a [`Winding`].

The MMF distribution is obtained from the winding's
[`magnetomotive_force`](Winding::magnetomotive_force), which is sampled
at equidistant positions around the air gap and transformed using a
real-to-complex fast Fourier transform (FFT).

The index of each coefficient corresponds to its mechanical spatial order,
starting with the constant component (order 0) at index 0.

# Examples

```
use approxim::assert_abs_diff_eq;
use stem_winding::prelude::*;
use stem_winding::winding::MmfSpectralAnalysis;

let winding: DistributedWinding = DistributedMinimalBuilder {
    slots: 12.try_into().expect("not zero"),
    pole_pairs: 1.try_into().expect("not zero"),
    phases: 3.try_into().expect("not zero"),
    layers: 2.try_into().expect("not zero"),
    coil_throw_reduction: 0,
    zone_shift: 0,
    winding_table_constructor: WindingTableConstructor::Tingley,
}
.try_into()
.unwrap();
let mut mmf = MmfSpectralAnalysis::new(1024);
let output = mmf.analyze(&winding, [
    ElectricCurrent::new::<ampere>(1.0),
    ElectricCurrent::new::<ampere>(-0.5),
    ElectricCurrent::new::<ampere>(-0.5),
]).unwrap();

// Convert the output into f64 for evaluation
let output: Vec<_> = output
    .iter()
    .map(|c| num::Complex::new(c.re.get::<ampere>(), c.im.get::<ampere>()))
    .collect();

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
relative to the first slot (where the mmf starts). For example, if the origin of
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

This implementation uses [realfft](https://crates.io/crates/realfft), which is
based on [rustfft](https://crates.io/crates/rustfft). For best FFT performance,
a power-of-two sample count is a good default; see the
[RustFFT documentation](https://docs.rs/rustfft/latest/rustfft/) for further
details.

The analysis buffers are reused between calls. Repeatedly analyzing different
current configurations of the same winding therefore requires no further
allocations once the buffers have sufficient capacity. The same
`MmfSpectralAnalysis` can also be used with different windings; its mmf buffer
is resized as necessary and reallocates only if its capacity is insufficient.
*/
#[derive(Clone)]
pub struct MmfSpectralAnalysis {
    pole_pairs: NonZeroU16,
    mmf_values: Vec<f64>,
    sampled_fec: Vec<f64>,
    phase_currents: Vec<f64>,
    coefficients: Vec<Complex<f64>>,
    fft_scratch_buffer: Vec<Complex<f64>>,
    fft_plan: Arc<dyn RealToComplex<f64>>,
}

impl MmfSpectralAnalysis {
    /// Returns a new [`MmfSpectralAnalysis`] that samples the magnetomotive
    /// force (MMF) distribution curve of a winding using `samples` samples.
    ///
    /// The number of samples determines the highest mechanical spatial order
    /// that can be resolved without aliasing. According to the
    /// Nyquist–Shannon sampling theorem, this limit is samples / 2 (rounded
    /// down). Spatial orders above this limit cannot be distinguished from
    /// lower orders and therefore alias into the spectrum. In
    /// practice, increasing the number of samples generally improves the
    /// accuracy of the numerical approximation even for lower orders.
    ///
    /// The number of samples affects FFT performance. When in doubt, use a
    /// power of two. See the [RustFFT documentation](https://docs.rs/rustfft/latest/rustfft/) for details.
    pub fn new(samples: usize) -> Self {
        // This preallocates a buffer for a 100-slot-winding, which should be enough in
        // most cases. If it is not, a reallocation is necessary (but that is
        // usually no big deal)
        let mmf_values = Vec::with_capacity(100);

        // Create the FFT planner
        let mut real_planner = RealFftPlanner::<f64>::new();
        let fft_plan = real_planner.plan_fft_forward(samples);
        let sampled_fec = fft_plan.make_input_vec();
        let coefficients = fft_plan.make_output_vec();
        let fft_scratch_buffer = fft_plan.make_scratch_vec();

        Self {
            mmf_values,
            sampled_fec,
            phase_currents: Vec::new(),
            coefficients,
            fft_scratch_buffer,
            fft_plan,
            pole_pairs: NonZeroU16::MIN,
        }
    }

    /// Returns the number of pole pairs of the winding used in the most recent
    /// [`analyze`](MmfSpectralAnalysis::analyze) call, to which the stored
    /// [`coefficients`](MmfSpectralAnalysis::coefficients) belong.
    ///
    /// If no winding has been analyzed yet, returns [`NonZeroU16::MIN`].
    pub fn pole_pairs(&self) -> NonZeroU16 {
        self.pole_pairs
    }

    /// Returns the magnetomotive force (MMF) distribution from the most recent
    /// [`analyze`](MmfSpectralAnalysis::analyze) call.
    ///
    /// Each index represents a slot: The value at index 0 is the MMF at the
    /// middle of slot 0, the value at index 1 that at the middle of slot 1,
    /// and so on.
    pub fn mmf_values(&self) -> &[ElectricCurrent] {
        // SAFETY: ElectricCurrent is a uom::Quantity with #[repr(transparent)]
        // over f64. Therefore, it has the same memory layout and ABI as f64.
        // The stored values represent amperes, as ensured by analyze().
        // https://docs.rs/uom/latest/uom/si/struct.Quantity.html
        unsafe { std::mem::transmute(self.mmf_values.as_slice()) }
    }

    /// Like [`MmfSpectralAnalysis::mmf_values`], but returns the underlying
    /// [`f64`] values without dimensional information.
    ///
    /// The FFT algorithms operate directly on raw [`f64`] values, which are
    /// also used for internal storage. This method provides zero-copy access
    /// to the underlying MMF distribution.
    pub fn mmf_values_raw(&self) -> &[f64] {
        self.mmf_values.as_slice()
    }
    /// Returns the complex Fourier coefficients from the most recent
    /// [`analyze`](MmfSpectralAnalysis::analyze) call.
    ///
    /// The phase angles of the coefficients are given in mechanical degrees.
    /// To convert the phase angle at index / mechanical spatial order `k` into
    /// electrical degrees, the following formula can be used:
    ///
    /// `α_electrical = k / p * α_mechanical`
    ///
    /// where `p` is the number of pole pairs.
    pub fn coefficients(&self) -> &[Complex<ElectricCurrent>] {
        // SAFETY: Complex<T> has #[repr(C)] and ElectricCurrent has
        // #[repr(transparent)] over f64. Consequently, Complex<ElectricCurrent>
        // has the same memory layout as Complex<f64>. The stored values
        // represent amperes, as ensured by analyze().
        // https://docs.rs/uom/latest/uom/si/struct.Quantity.html
        // https://docs.rs/num-complex/latest/num_complex/struct.Complex.html
        unsafe { std::mem::transmute(self.coefficients.as_slice()) }
    }

    /// Like [`MmfSpectralAnalysis::coefficients`], but returns the underlying
    /// [`f64`] values without dimensional information.
    ///
    /// The FFT algorithms operate directly on raw [`f64`] values, which are
    /// also used for internal storage. This method provides zero-copy access
    /// to the underlying Fourier coefficients.
    pub fn coefficients_raw(&self) -> &[Complex<f64>] {
        self.coefficients.as_slice()
    }

    /// Returns the complex fourier coefficient from the most recent
    /// [`analyze`](MmfSpectralAnalysis::analyze) call for the specified
    /// `spatial_order`.
    ///
    /// If the order does not exist, this function returns `None`. See
    /// [`MmfSpectralAnalysis::new`] on how the number of samples influences the
    /// maximum spatial order.
    ///
    /// The phase angles of the coefficient is given in mechanical degrees.
    /// To convert the phase angle into electrical degrees, the following
    /// formula can be used:
    ///
    /// `α_electrical = k / p * α_mechanical`
    ///
    /// where `p` is the number of pole pairs and `k` is the mechanical
    /// `spatial_order`.
    pub fn coefficient(&self, spatial_order: SpatialOrder) -> Option<Complex<ElectricCurrent>> {
        let idx = spatial_order.to_mechanical(self.pole_pairs);
        let val = self.coefficients.get(idx as usize)?;
        Some(Complex::new(
            ElectricCurrent::new::<ampere>(val.re),
            ElectricCurrent::new::<ampere>(val.im),
        ))
    }

    /// Returns the complex Fourier coefficient from the most recent
    /// [`analyze`](MmfSpectralAnalysis::analyze) call for the specified
    /// `spatial_order`, transformed into dq-coordinates.
    ///
    /// The MMF distribution created by [`Winding::magnetomotive_force`]
    /// starts in the middle of the first slot of the winding. Consequently,
    /// the spatial reference of the Fourier coefficients is also the middle
    /// of the first slot.
    ///
    /// This function retrieves the coefficient from
    /// [`MmfSpectralAnalysis::coefficient`] and transforms it into
    /// dq-coordinates using:
    ///
    /// `c_k_dq = conj(c_k * e^(j * k * ν * θ))`
    ///
    /// where `ν` is the mechanical spatial order. `θ` is the
    /// `winding_reference_to_d_axis` which describes the offset of the
    /// d-axis to the reference axis of the winding (which is in the middle
    /// of the first slot) in mechanical radians. See [`Winding::magnetic_axis`]
    /// for an illustration.
    ///
    /// The complex conjugation `conj` accounts for the negative-exponent
    /// convention of the forward Fourier transform used by RustFFT. It ensures
    /// that the imaginary component follows the conventional dq sign
    /// convention, where positive q-axis excitation is represented by a
    /// positive imaginary component.
    ///
    /// # Examples
    ///
    /// In the following example, the d-axis coincides with the magnetic axis
    /// of phase 2.
    ///
    /// ```
    /// use std::f64::consts::PI;
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let dq0 = Dq0Current {
    ///     direct: ElectricCurrent::new::<ampere>(0.0),
    ///     quadrature: ElectricCurrent::new::<ampere>(1.0),
    ///     zero: ElectricCurrent::new::<ampere>(0.0),
    /// };
    ///
    /// let winding: DistributedWinding = DistributedMinimalBuilder {
    ///     slots: 36.try_into().expect("not zero"),
    ///     pole_pairs: 2.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 1.try_into().expect("not zero"),
    ///     coil_throw_reduction: 0,
    ///     zone_shift: 0,
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    /// }
    /// .try_into()
    /// .unwrap();
    ///
    /// let fundamental = SpatialOrder::Electrical(1);
    ///
    /// // Magnetic axis of phase 2 = position of the d-axis
    /// let winding_reference_to_d_axis = winding.magnetic_axis(
    ///     2.try_into().expect("not zero"),
    ///     fundamental
    /// );
    ///
    /// // Derive the current system
    /// let currents_from_dq = dq0
    ///     .winding_phase_currents(&winding, fundamental, winding_reference_to_d_axis);
    ///
    /// let mut analysis = MmfSpectralAnalysis::new(1024);
    /// analysis.analyze(&winding, currents_from_dq).expect("succeeds");
    ///
    /// let dq_fundamental = analysis.coefficient_dq(
    ///     fundamental,
    ///     winding_reference_to_d_axis
    /// ).expect("exists");
    /// assert_abs_diff_eq!(dq_fundamental.re.get::<ampere>(), 0.0037, epsilon = 1e-3);
    /// assert_abs_diff_eq!(dq_fundamental.im.get::<ampere>(), 2.746, epsilon = 1e-3);
    /// ```
    pub fn coefficient_dq(
        &self,
        spatial_order: SpatialOrder,
        winding_reference_to_d_axis: f64,
    ) -> Option<Complex<ElectricCurrent>> {
        let k = spatial_order.to_mechanical(self.pole_pairs);
        let coefficient = self.coefficients.get(k as usize)?;
        let theta = f64::from(k) * winding_reference_to_d_axis;
        let val = coefficient * Complex::from_polar(1.0, theta);

        /*
        The imaginary component follows RustFFT's Fourier transform convention,
        which uses a negative complex exponential (`e^(-j * ω * t)`).
        Consequently, positive q-axis excitation corresponds to a negative
        imaginary component. This differs from the conventional dq
        representation, where the q-component is represented by the positive
        imaginary part. Hence, the sign of val is inverted.
        */
        Some(Complex::new(
            ElectricCurrent::new::<ampere>(val.re),
            ElectricCurrent::new::<ampere>(-val.im),
        ))
    }

    /// Calculates the magnetomotive force (MMF) spectrum for the given
    /// instantaneous currents.
    ///
    /// The current vector used to determine the MMF has the size
    /// [`winding.phases()`](Winding::phases) and is populated from the
    /// given `currents` iterator. If the iterator does not yield enough
    /// elements, the remaining currents are set to 0. Any additional values
    /// beyond the number of phases created by the iterator are discarded.
    ///
    /// As shown in the example below, it is also possible to use a
    /// [`Dq0Current`] as an input by deriving the phase current iterator via
    /// [`Dq0Current::winding_phase_currents`]. For retrieving the MMF in
    /// dq-coordinates, see [`MmfSpectralAnalysis::coefficient_dq`].
    ///
    /// The results are stored internally and can be accessed via
    /// [`coefficients`](MmfSpectralAnalysis::coefficients).
    ///
    /// Returns an error if the FFT fails.
    ///
    /// # Examples
    ///
    /// This example shows how to calculate the MMF coefficients both from the
    /// phase currents directly and from the dq-currents.
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
    ///     coil_throw_reduction: 0,
    ///     zone_shift: 0,
    ///     winding_table_constructor: WindingTableConstructor::CoilSide,
    /// }
    /// .try_into()
    /// .unwrap();
    ///
    /// let mut analysis = MmfSpectralAnalysis::new(1024);
    ///
    /// // Get the fundamental for the given phase currents.
    /// let currents = [
    ///     ElectricCurrent::new::<ampere>(1.0),
    ///     ElectricCurrent::new::<ampere>(-0.5),
    ///     ElectricCurrent::new::<ampere>(-0.5),
    /// ];
    /// analysis.analyze(&winding, currents).expect("succeeds");
    /// let fundamental_from_phases = analysis.coefficients_raw()[1].norm();
    /// assert_abs_diff_eq!(fundamental_from_phases, 1.8453, epsilon = 1e-3);
    ///
    /// // Reuse the same analyzer for a dq-current where the d-axis
    /// // coincides with the magnetic axis of phase 1:
    /// let dq0 = Dq0Current {
    ///     direct: ElectricCurrent::new::<ampere>(1.0),
    ///     quadrature: ElectricCurrent::new::<ampere>(0.0),
    ///     zero: ElectricCurrent::new::<ampere>(0.0),
    /// };
    /// let winding_reference_to_d_axis = winding.magnetic_axis(
    ///     1.try_into().expect("not zero"),
    ///     SpatialOrder::Electrical(1)
    /// );
    /// let currents_from_dq = dq0
    ///     .winding_phase_currents(&winding, SpatialOrder::Electrical(1), winding_reference_to_d_axis);
    /// analysis.analyze(&winding, currents_from_dq).expect("succeeds");
    /// let fundamental_from_dq = analysis.coefficients_raw()[1].norm();
    /// assert_abs_diff_eq!(fundamental_from_dq, fundamental_from_phases, epsilon = 1e-3);
    /// ```
    pub fn analyze<W: Winding + ?Sized>(
        &mut self,
        winding: &W,
        currents: impl IntoIterator<Item = ElectricCurrent>,
    ) -> Result<&[Complex<ElectricCurrent>], FftError> {
        self.mmf_values.resize(winding.slots().get().into(), 0.0);

        // Populate the current vector with all given currents
        self.phase_currents
            .resize(winding.phases().get().into(), 0.0);
        self.phase_currents.iter_mut().for_each(|v| {
            *v = 0.0;
        });
        for (phase_curr, curr) in self.phase_currents.iter_mut().zip(currents) {
            *phase_curr = curr.get::<ampere>();
        }

        // Calculate the MMF
        winding
            .magnetomotive_force(self.mmf_values.as_mut_slice(), &self.phase_currents)
            .expect("buffers have a valid length");

        // Switch to non-generic version to reduce code bloat
        self.analyze_priv(winding.pole_pairs())
    }

    fn analyze_priv(
        &mut self,
        pole_pairs: NonZeroU16,
    ) -> Result<&[Complex<ElectricCurrent>], FftError> {
        // Upsample the mmf
        sample_mmf_values(self.mmf_values.iter().cloned(), &mut self.sampled_fec);

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
        let has_nyquist = self.sampled_fec.len().is_multiple_of(2);
        let ordinary_end = if has_nyquist { end - 1 } else { end };

        self.coefficients[1..ordinary_end]
            .iter_mut()
            .for_each(|value| *value *= 2.0 * inverse);

        if has_nyquist {
            self.coefficients[end - 1] *= inverse;
        }

        // The first value equals the constant part and does not occur twice, but still
        // needs to be normalized
        self.coefficients[0] *= inverse;

        // Store the pole pairs of the winding
        self.pole_pairs = pole_pairs;

        Ok(self.coefficients())
    }
}
