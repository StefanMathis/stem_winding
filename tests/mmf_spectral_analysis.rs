use approxim::assert_abs_diff_eq;
use stem_winding::{prelude::*, winding::MmfSpectralAnalysis};

#[test]
fn test_fec_integer_slot_winding() {
    {
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
        let output = mmf
            .analyze(
                &winding,
                [
                    ElectricCurrent::new::<ampere>(0.5),
                    ElectricCurrent::new::<ampere>(-0.25),
                    ElectricCurrent::new::<ampere>(-0.25),
                ],
            )
            .unwrap();

        // Assert the length of the output
        assert_eq!(output.len(), 513);

        // Convert the output into f64 for evaluation
        let output: Vec<_> = output
            .iter()
            .map(|c| num::Complex::new(c.re.get::<ampere>(), c.im.get::<ampere>()))
            .collect();

        // All even orders are zero. Since the field is a pure 2D field, no constant
        // component exists.
        let (val, _) = output[0].to_polar(); // Constant component
        assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

        let (val, _) = output[1].to_polar(); // First order
        assert_abs_diff_eq!(val, 1.8453, epsilon = 1e-3);

        let (val, _) = output[2].to_polar(); // Second order
        assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

        let (val, _) = output[3].to_polar(); // Third order
        assert_abs_diff_eq!(val, 0.00276, epsilon = 1e-5);

        let (val, _) = output[4].to_polar(); // Fourth order
        assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

        let (val, _) = output[5].to_polar(); // Fifth order
        assert_abs_diff_eq!(val, 0.0967, epsilon = 1e-3);

        // Different current
        let output = mmf
            .analyze(
                &winding,
                [
                    ElectricCurrent::new::<ampere>(-0.25),
                    ElectricCurrent::new::<ampere>(-0.25),
                    ElectricCurrent::new::<ampere>(0.5),
                ],
            )
            .unwrap();

        // Assert the length of the output
        assert_eq!(output.len(), 513);

        let output: Vec<_> = output
            .iter()
            .map(|c| num::Complex::new(c.re.get::<ampere>(), c.im.get::<ampere>()))
            .collect();

        // All even orders are zero. Since the field is a pure 2D field, no constant
        // component exists.
        let (val, _) = output[0].to_polar(); // Constant component
        assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

        let (val, _) = output[1].to_polar(); // First order
        assert_abs_diff_eq!(val, 1.8453, epsilon = 1e-3);

        let (val, _) = output[2].to_polar(); // Second order
        assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

        let (val, _) = output[3].to_polar(); // Third order
        assert_abs_diff_eq!(val, 0.00195, epsilon = 1e-5);

        let (val, _) = output[4].to_polar(); // Fourth order
        assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

        let (val, _) = output[5].to_polar(); // Fifth order
        assert_abs_diff_eq!(val, 0.09997, epsilon = 1e-3);
    }
    {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: 24.try_into().expect("not zero"),
            pole_pairs: 2.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            coil_throw_reduction: 0,
            zone_shift: 0,
            winding_table_constructor: WindingTableConstructor::Tingley,
        }
        .try_into()
        .unwrap();
        let mut mmf = MmfSpectralAnalysis::new(1024);
        let output = mmf
            .analyze(
                &winding,
                [
                    ElectricCurrent::new::<ampere>(0.5),
                    ElectricCurrent::new::<ampere>(-0.25),
                    ElectricCurrent::new::<ampere>(-0.25),
                ],
            )
            .unwrap();

        let output: Vec<_> = output
            .iter()
            .map(|c| num::Complex::new(c.re.get::<ampere>(), c.im.get::<ampere>()))
            .collect();

        // Assert the length of the output
        assert_eq!(output.len(), 513);

        // All even orders are zero. Since the field is a pure 2D field, no constant
        // component exists.
        let (val, _) = output[0].to_polar(); // Constant component
        assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

        let (val, _) = output[1].to_polar(); // First order
        assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

        let (val, _) = output[2].to_polar(); // Second order
        assert_abs_diff_eq!(val, 1.8453, epsilon = 1e-3);

        let (val, _) = output[3].to_polar(); // Third order
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
        coil_throw_reduction: 0,
        zone_shift: 0,
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();
    let trait_object: &dyn Winding = &winding;
    let mut mmf = MmfSpectralAnalysis::new(1024);
    let output = mmf
        .analyze(
            trait_object,
            [
                ElectricCurrent::new::<ampere>(0.5),
                ElectricCurrent::new::<ampere>(-0.25),
                ElectricCurrent::new::<ampere>(-0.25),
            ],
        )
        .unwrap();

    // Assert the length of the output
    assert_eq!(output.len(), 513);

    let output: Vec<_> = output
        .iter()
        .map(|c| num::Complex::new(c.re.get::<ampere>(), c.im.get::<ampere>()))
        .collect();

    // All even orders are zero. Since the field is a pure 2D field, no constant
    // component exists.
    let (val, _) = output[0].to_polar(); // Constant component
    assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

    let (val, _) = output[1].to_polar(); // First order
    assert_abs_diff_eq!(val, 1.8453, epsilon = 1e-3);

    let (val, _) = output[2].to_polar(); // Second order
    assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

    let (val, _) = output[3].to_polar(); // Third order
    assert_abs_diff_eq!(val, 0.0028, epsilon = 1e-3);

    let (val, _) = output[4].to_polar(); // Fourth order
    assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

    let (val, _) = output[5].to_polar(); // Fifth order
    assert_abs_diff_eq!(val, 0.0967, epsilon = 1e-3);
}

#[test]
fn test_fec_short_pitched_integer_slot_winding() {
    let winding: DistributedWinding = DistributedMinimalBuilder {
        slots: 12.try_into().expect("not zero"),
        pole_pairs: 1.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 2.try_into().expect("not zero"),
        coil_throw_reduction: 1,
        zone_shift: 0,
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();
    let mut mmf = MmfSpectralAnalysis::new(1024);
    let output = mmf
        .analyze(
            &winding,
            [
                ElectricCurrent::new::<ampere>(0.5),
                ElectricCurrent::new::<ampere>(-0.25),
                ElectricCurrent::new::<ampere>(-0.25),
            ],
        )
        .unwrap();

    // Assert the length of the output
    assert_eq!(output.len(), 513);

    let output: Vec<_> = output
        .iter()
        .map(|c| num::Complex::new(c.re.get::<ampere>(), c.im.get::<ampere>()))
        .collect();

    // All even orders are zero. Since the field is a pure 2D field, no constant
    // component exists.
    let (val, _) = output[0].to_polar(); // Constant component
    assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

    let (val, _) = output[1].to_polar(); // First order
    assert_abs_diff_eq!(val, 1.7820, epsilon = 1e-3);

    let (val, _) = output[2].to_polar(); // Second order
    assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

    let (val, _) = output[3].to_polar(); // Third order
    assert_abs_diff_eq!(val, 0.0028, epsilon = 1e-3);

    let (val, _) = output[4].to_polar(); // Fourth order
    assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

    let (val, _) = output[5].to_polar(); // Fifth order
    assert_abs_diff_eq!(val, 0.0246, epsilon = 1e-3);
}

#[test]
fn test_fec_tooth_coil_assembly() {
    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 12.try_into().expect("not zero"),
        pole_pairs: 5.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 2.try_into().expect("not zero"),
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();

    let mut mmf = MmfSpectralAnalysis::new(1024);
    let output = mmf
        .analyze(
            &winding,
            [
                ElectricCurrent::new::<ampere>(1.0),
                ElectricCurrent::new::<ampere>(-0.5),
                ElectricCurrent::new::<ampere>(-0.5),
            ],
        )
        .unwrap();

    // Assert the length of the output
    assert_eq!(output.len(), 513);

    let output: Vec<_> = output
        .iter()
        .map(|c| num::Complex::new(c.re.get::<ampere>(), c.im.get::<ampere>()))
        .collect();

    // All even orders are zero. Since the field is a pure 2D field, no constant
    // component exists.
    let (val, _) = output[0].to_polar(); // Constant component
    assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

    let (val, _) = output[1].to_polar(); // First order
    assert_abs_diff_eq!(val, 0.2580, epsilon = 1e-3);

    let (val, _) = output[2].to_polar(); // Second order
    assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

    let (val, _) = output[3].to_polar(); // Third order
    assert_abs_diff_eq!(val, 0.0043, epsilon = 1e-3);

    let (val, _) = output[4].to_polar(); // Fourth order
    assert_abs_diff_eq!(val, 0.0, epsilon = 1e-3);

    let (val, _) = output[5].to_polar(); // Fifth order
    assert_abs_diff_eq!(val, 0.7126, epsilon = 1e-3);
}

#[test]
fn test_dq_coefficients() {
    {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 2.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_throw_reduction: 0,
            zone_shift: 0,
            winding_table_constructor: WindingTableConstructor::Tingley,
        }
        .try_into()
        .unwrap();
        let mut mmf = MmfSpectralAnalysis::new(1024);

        {
            let dq0 = Dq0Current {
                direct: ElectricCurrent::new::<ampere>(1.0),
                quadrature: ElectricCurrent::new::<ampere>(0.0),
                zero: ElectricCurrent::new::<ampere>(0.0),
            };
            let currents: Vec<_> = dq0
                .winding_phase_currents(
                    &winding,
                    SpatialOrder::Electrical(1),
                    winding.magnetic_axis(1.try_into().unwrap(), SpatialOrder::Electrical(1)),
                )
                .collect();
            assert_abs_diff_eq!(currents[0].get::<ampere>(), 1.0, epsilon = 1e-3);
            assert_abs_diff_eq!(currents[1].get::<ampere>(), -0.5, epsilon = 1e-3);
            assert_abs_diff_eq!(currents[2].get::<ampere>(), -0.5, epsilon = 1e-3);

            mmf.analyze(&winding, currents).unwrap();

            let fundamental = mmf
                .coefficient_dq(
                    SpatialOrder::Electrical(1),
                    winding.magnetic_axis(1.try_into().unwrap(), SpatialOrder::Electrical(1)),
                )
                .unwrap();
            assert_abs_diff_eq!(fundamental.re.get::<ampere>(), 0.95379, epsilon = 1e-3);
            assert_abs_diff_eq!(fundamental.im.get::<ampere>(), -0.003906, epsilon = 1e-3);
        }
        {
            let dq0 = Dq0Current {
                direct: ElectricCurrent::new::<ampere>(0.0),
                quadrature: ElectricCurrent::new::<ampere>(1.0),
                zero: ElectricCurrent::new::<ampere>(0.0),
            };
            let currents: Vec<_> = dq0
                .winding_phase_currents(
                    &winding,
                    SpatialOrder::Electrical(1),
                    winding.magnetic_axis(1.try_into().unwrap(), SpatialOrder::Electrical(1)),
                )
                .collect();
            assert_abs_diff_eq!(currents[0].get::<ampere>(), 0.0, epsilon = 1e-3);
            assert_abs_diff_eq!(currents[1].get::<ampere>(), 0.866, epsilon = 1e-3);
            assert_abs_diff_eq!(currents[2].get::<ampere>(), -0.866, epsilon = 1e-3);

            mmf.analyze(&winding, currents).unwrap();

            let fundamental = mmf
                .coefficient_dq(
                    SpatialOrder::Electrical(1),
                    winding.magnetic_axis(1.try_into().unwrap(), SpatialOrder::Electrical(1)),
                )
                .unwrap();
            assert_abs_diff_eq!(fundamental.re.get::<ampere>(), 0.0, epsilon = 1e-3);
            assert_abs_diff_eq!(fundamental.im.get::<ampere>(), 0.95606, epsilon = 1e-3);
        }
    }
}
