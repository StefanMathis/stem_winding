use approxim::assert_abs_diff_eq;
use stem_winding::{prelude::*, winding::FecSpectralAnalysis};

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
        let mut fec = FecSpectralAnalysis::new(1024);
        let output = fec.analyze(&winding, &[0.5, -0.25, -0.25]).unwrap();

        // Assert the length of the output
        assert_eq!(output.len(), 513);

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
        let output = fec.analyze(&winding, &[-0.25, -0.25, 0.5]).unwrap();

        // Assert the length of the output
        assert_eq!(output.len(), 513);

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
        let mut fec = FecSpectralAnalysis::new(1024);
        let output = fec.analyze(&winding, &[0.5, -0.25, -0.25]).unwrap();

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
    let mut fec = FecSpectralAnalysis::new(1024);
    let output = fec.analyze(trait_object, &[0.5, -0.25, -0.25]).unwrap();

    // Assert the length of the output
    assert_eq!(output.len(), 513);

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
    let mut fec = FecSpectralAnalysis::new(1024);
    let output = fec.analyze(&winding, &[0.5, -0.25, -0.25]).unwrap();

    // Assert the length of the output
    assert_eq!(output.len(), 513);

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

    let mut fec = FecSpectralAnalysis::new(1024);
    let output = fec.analyze(&winding, &[1.0, -0.5, -0.5]).unwrap();

    // Assert the length of the output
    assert_eq!(output.len(), 513);

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
