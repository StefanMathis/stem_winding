use std::sync::Arc;

use num::Complex;
use realfft::{RealFftPlanner, RealToComplex};

use crate::{error::Error, winding::Winding};

/**
Interprets the input iterator as an equidistant step function and sample it into the output vector.

# Panic
Panics if the given number of steps is larger than the number of iterator elements.

# Examples
```
use stem_winding::fec_spectral_analysis::sample_as_equidistant_step_function;

let iterator = [1.0, 2.0, 3.0, 1.0].iter().cloned();
let number_steps = 4;

let mut sampling_buffer = vec![0.0; 5];
sample_as_equidistant_step_function(iterator, number_steps, &mut sampling_buffer);
assert_eq!(
    sampling_buffer.as_slice(),
    &[1.0, 1.0, 2.0, 3.0, 1.0]
);
```
*/
pub fn sample_as_equidistant_step_function<I: Iterator<Item = f64>>(
    mut step_function: I,
    number_steps: usize,
    buffer_samples: &mut [f64],
) {
    let mut step = 0.0;
    let mut stored_value = step_function.next().unwrap();
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
                *sample_value = stored_value;
                break;
            } else {
                step += 1.0;
                stored_value = step_function.next().unwrap();
            }
        }
    }
}

pub struct FecSpectralAnalysis {
    step_function: Vec<f64>,
    sampled_step_function: Vec<f64>,
    fft_output: Vec<Complex<f64>>,
    fft_scratch_buffer: Vec<Complex<f64>>,
    fft_planer: Arc<dyn RealToComplex<f64>>,
}

impl FecSpectralAnalysis {
    pub fn new(samples: usize) -> Self {
        // This preallocates a buffer for a 100-slot-winding, which should be enough in
        // most cases. If it is not, a reallocation is necessary (but that is
        // usually no big deal)
        let step_function = Vec::with_capacity(100);

        // Create the FFT planner
        let mut real_planner = RealFftPlanner::<f64>::new();
        let fft_planer = real_planner.plan_fft_forward(samples);
        let sampled_step_function = fft_planer.make_input_vec();
        let fft_output = fft_planer.make_output_vec();
        let fft_scratch_buffer = fft_planer.make_scratch_vec();

        return Self {
            step_function,
            sampled_step_function,
            fft_output,
            fft_scratch_buffer,
            fft_planer,
        };
    }

    pub fn step_function(&self) -> &[f64] {
        return self.step_function.as_slice();
    }

    pub fn len(&self) -> usize {
        return self.step_function.len();
    }

    pub fn fft_output(&self) -> &[Complex<f64>] {
        return self.fft_output.as_slice();
    }

    /**
    Calculate the field excitation curve spectrum for the given currents. The results are stored in `self`
    and can be accessed via `self.fft_output()`. The calculation fails if either the current slice length
    is not equal to the number of phases or if the FFT fails.

    ```
    use winding::{DistributedWinding, Winding, WindingTableMethod, FecSpectralAnalysis};
    use approxim::assert_abs_diff_eq;

    let winding = DistributedWinding::new_minimal(12, 1, 3, 1, 0, 0, WindingTableMethod::Tingley).unwrap();
    let mut fec = FecSpectralAnalysis::new(1024);
    let output = fec.analyze(&winding, &[1.0, -0.5, -0.5]).unwrap();

    // Assert the length of the output
    assert_eq!(output.len(), 513);

    // All even ordinals are zero. Since the field is a pure 2D field, no constant component exists.
    let (val, _) = output[0].to_polar(); // Constant component
    assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

    let (val, _) = output[1].to_polar(); // First ordinal
    assert_abs_diff_eq!(val, 1.8453, epsilon = 1e-3);

    let (val, _) = output[2].to_polar(); // Second ordinal
    assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

    let (val, _) = output[3].to_polar(); // Third ordinal
    assert_abs_diff_eq!(val, 0.0028, epsilon = 1e-3);

    let (val, _) = output[4].to_polar(); // Fourth ordinal
    assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

    let (val, _) = output[5].to_polar(); // Fifth ordinal
    assert_abs_diff_eq!(val, 0.0967, epsilon = 1e-3);

    ```
     */
    pub fn analyze<W: Winding + ?Sized>(
        &mut self,
        winding: &W,
        currents: &[f64],
    ) -> Result<&[Complex<f64>], Error> {
        // Prepare the step function
        self.step_function.clear();
        for _ in 0..winding.slots().get() {
            self.step_function.push(0.0); // Dummy value, will be overwritten in winding.field_excitation_curve anyway.
        }

        // Calculate the field excitation curve (FEC)
        winding.field_excitation_curve(self.step_function.as_mut_slice(), currents)?;

        // Switch to non-generic version to reduce code bloat
        return self.analyze_priv();
    }

    fn analyze_priv(&mut self) -> Result<&[Complex<f64>], Error> {
        // Upsample the FEC
        sample_as_equidistant_step_function(
            self.step_function.iter().cloned(),
            self.step_function.len(),
            &mut self.sampled_step_function,
        );

        // Perform the FFT on the buffer
        self.fft_planer.process_with_scratch(
            &mut self.sampled_step_function,
            &mut self.fft_output,
            &mut self.fft_scratch_buffer,
        )?;

        // Normalize the output: Divide it by N (number of input elements) and multiply
        // by 2 to account for the negative side of the spectrum
        let inverse = 1.0 / (self.sampled_step_function.len() as f64);
        let inverse_doubled = 2.0 * inverse;
        self.fft_output
            .iter_mut()
            .skip(1)
            .for_each(|value| *value = *value * inverse_doubled);

        // The first value equals the constant part and does not occur twice, but still
        // needs to be normalized
        self.fft_output[0] = self.fft_output[0] * inverse;

        return Ok(self.fft_output.as_slice());
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        winding::{
            ToothCoilWinding,
            distributed::{DistributedMinimalBuilder, DistributedWinding},
            tooth_coil::ToothCoilMinimalBuilder,
        },
        winding_table::WindingTableMethod,
    };

    use super::*;
    use approxim::assert_abs_diff_eq;

    #[test]
    fn test_fec_integer_slot_winding() {
        {
            let winding: DistributedWinding = DistributedMinimalBuilder {
                slots: 12.try_into().expect("not zero"),
                pole_pairs: 1.try_into().expect("not zero"),
                phases: 3.try_into().expect("not zero"),
                layers: 2.try_into().expect("not zero"),
                coil_span_reduction: 0,
                zone_span_variation: 0,
                winding_table_method: WindingTableMethod::Tingley,
            }
            .try_into()
            .unwrap();
            let mut fec = FecSpectralAnalysis::new(1024);
            let output = fec.analyze(&winding, &[0.5, -0.25, -0.25]).unwrap();

            // Assert the length of the output
            assert_eq!(output.len(), 513);

            // All even ordinals are zero. Since the field is a pure 2D field, no constant
            // component exists.
            let (val, _) = output[0].to_polar(); // Constant component
            assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

            let (val, _) = output[1].to_polar(); // First ordinal
            assert_abs_diff_eq!(val, 1.8453, epsilon = 1e-3);

            let (val, _) = output[2].to_polar(); // Second ordinal
            assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

            let (val, _) = output[3].to_polar(); // Third ordinal
            assert_abs_diff_eq!(val, 0.00276, epsilon = 1e-5);

            let (val, _) = output[4].to_polar(); // Fourth ordinal
            assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

            let (val, _) = output[5].to_polar(); // Fifth ordinal
            assert_abs_diff_eq!(val, 0.0967, epsilon = 1e-3);
        }
        {
            let winding: DistributedWinding = DistributedMinimalBuilder {
                slots: 24.try_into().expect("not zero"),
                pole_pairs: 2.try_into().expect("not zero"),
                phases: 3.try_into().expect("not zero"),
                layers: 2.try_into().expect("not zero"),
                coil_span_reduction: 0,
                zone_span_variation: 0,
                winding_table_method: WindingTableMethod::Tingley,
            }
            .try_into()
            .unwrap();
            let mut fec = FecSpectralAnalysis::new(1024);
            let output = fec.analyze(&winding, &[0.5, -0.25, -0.25]).unwrap();

            // Assert the length of the output
            assert_eq!(output.len(), 513);

            // All even ordinals are zero. Since the field is a pure 2D field, no constant
            // component exists.
            let (val, _) = output[0].to_polar(); // Constant component
            assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

            let (val, _) = output[1].to_polar(); // First ordinal
            assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

            let (val, _) = output[2].to_polar(); // Second ordinal
            assert_abs_diff_eq!(val, 1.8453, epsilon = 1e-3);

            let (val, _) = output[3].to_polar(); // Third ordinal
            assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);
        }
    }

    #[test]
    fn test_fec_trait_object() {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 1.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            coil_span_reduction: 0,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();
        let trait_object: &dyn Winding = &winding;
        let mut fec = FecSpectralAnalysis::new(1024);
        let output = fec.analyze(trait_object, &[0.5, -0.25, -0.25]).unwrap();

        // Assert the length of the output
        assert_eq!(output.len(), 513);

        // All even ordinals are zero. Since the field is a pure 2D field, no constant
        // component exists.
        let (val, _) = output[0].to_polar(); // Constant component
        assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

        let (val, _) = output[1].to_polar(); // First ordinal
        assert_abs_diff_eq!(val, 1.8453, epsilon = 1e-3);

        let (val, _) = output[2].to_polar(); // Second ordinal
        assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

        let (val, _) = output[3].to_polar(); // Third ordinal
        assert_abs_diff_eq!(val, 0.0028, epsilon = 1e-3);

        let (val, _) = output[4].to_polar(); // Fourth ordinal
        assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

        let (val, _) = output[5].to_polar(); // Fifth ordinal
        assert_abs_diff_eq!(val, 0.0967, epsilon = 1e-3);
    }

    #[test]
    fn test_fec_short_pitched_integer_slot_winding() {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 1.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            coil_span_reduction: 1,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();
        let mut fec = FecSpectralAnalysis::new(1024);
        let output = fec.analyze(&winding, &[0.5, -0.25, -0.25]).unwrap();

        // Assert the length of the output
        assert_eq!(output.len(), 513);

        // All even ordinals are zero. Since the field is a pure 2D field, no constant
        // component exists.
        let (val, _) = output[0].to_polar(); // Constant component
        assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

        let (val, _) = output[1].to_polar(); // First ordinal
        assert_abs_diff_eq!(val, 1.7820, epsilon = 1e-3);

        let (val, _) = output[2].to_polar(); // Second ordinal
        assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

        let (val, _) = output[3].to_polar(); // Third ordinal
        assert_abs_diff_eq!(val, 0.0028, epsilon = 1e-3);

        let (val, _) = output[4].to_polar(); // Fourth ordinal
        assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

        let (val, _) = output[5].to_polar(); // Fifth ordinal
        assert_abs_diff_eq!(val, 0.0246, epsilon = 1e-3);
    }

    #[test]
    fn test_fec_tooth_coil_assembly() {
        let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 5.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let mut fec = FecSpectralAnalysis::new(1024);
        let output = fec.analyze(&winding, &[1.0, -0.5, -0.5]).unwrap();

        // Assert the length of the output
        assert_eq!(output.len(), 513);

        // All even ordinals are zero. Since the field is a pure 2D field, no constant
        // component exists.
        let (val, _) = output[0].to_polar(); // Constant component
        assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

        let (val, _) = output[1].to_polar(); // First ordinal
        assert_abs_diff_eq!(val, 0.2580, epsilon = 1e-3);

        let (val, _) = output[2].to_polar(); // Second ordinal
        assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

        let (val, _) = output[3].to_polar(); // Third ordinal
        assert_abs_diff_eq!(val, 0.0043, epsilon = 1e-3);

        let (val, _) = output[4].to_polar(); // Fourth ordinal
        assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

        let (val, _) = output[5].to_polar(); // Fifth ordinal
        assert_abs_diff_eq!(val, 0.7126, epsilon = 1e-3);
    }
}
