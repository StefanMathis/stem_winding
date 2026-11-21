use std::num::{NonZeroU16, NonZeroUsize};

use approxim::assert_abs_diff_eq;
use stem_winding::prelude::*;
use stem_winding::winding::*;

#[test]
fn test_repeating_pattern_count() {
    {
        let collection = [1, 0, 0, 0, 1, 0, 0, 0];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(2).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [0, 1, 0, 0, 1, 0];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(2).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [0, 1, 0, 0, 1, 0, 0, 1, 0];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(3).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [1, 1, 1, 7, 1, 1, 1, 7];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(2).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [1, 1, 1, 1, 1, 7];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(1).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [1, 1, 1, 1, 1, 1, 1, 7];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(1).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [1, 1, 1, 1, 1, 1, 7];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(1).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [1, 1, 1];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(3).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [1, 2, 1, 2];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(2).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [1, 2, 1, 2, 1, 2];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(3).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [1, 2, 1, 2, 1, 2, 1, 2, 1, 2];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(5).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [1, 2, 1, 2, 1, 2, 3];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(1).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [1, 2, 3, 1, 2, 3];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(2).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [1, 1, 1, 1, 1, 1];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(6).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(1).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(4).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(3).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 7, 4];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(1).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [1, 2, 3, 4, 5, 6, 7, 1, 2, 3, 4, 5, 6, 7];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(2).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [
            1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4,
        ];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(6).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [
            1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3,
        ];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(8).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [
            1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1,
            2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4,
        ];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(12).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        let collection = [
            1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2,
            3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3,
        ];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(16).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        // Note the stray 5, which completely kills the symmetry.
        let collection = [
            1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 5, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2,
            3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3,
        ];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(1).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
    {
        // Here, the two 5 result in two repeating patterns
        let collection = [
            1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 5, 2, 3, 1, 2, 3, 1, 2,
            3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 5, 2, 3,
        ];
        let len = NonZeroUsize::new(collection.len()).expect("not zero");
        let expected = NonZeroUsize::new(2).expect("not zero");
        assert_eq!(expected, repeating_pattern_count(&collection, len));
    }
}

#[test]
fn test_base_winding_count_symmetric() {
    assert_eq!(
        2,
        base_winding_count_symmetric(
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            NonZeroU16::new(1).expect("not zero")
        )
        .get()
    );
    assert_eq!(
        5,
        base_winding_count_symmetric(
            NonZeroU16::new(15).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            NonZeroU16::new(2).expect("not zero")
        )
        .get()
    );
    assert_eq!(
        2,
        base_winding_count_symmetric(
            NonZeroU16::new(18).expect("not zero"),
            NonZeroU16::new(10).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            NonZeroU16::new(2).expect("not zero")
        )
        .get()
    );
    assert_eq!(
        1,
        base_winding_count_symmetric(
            NonZeroU16::new(19).expect("not zero"),
            NonZeroU16::new(4).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            NonZeroU16::new(1).expect("not zero")
        )
        .get()
    );
}

#[test]
fn test_line_to_phase_voltage() {
    let phases = NonZeroU16::new(3).unwrap();

    {
        let factor = Connection::Star.line_to_phase_voltage(phases);
        assert_abs_diff_eq!(0.57735, factor.norm(), epsilon = 0.0001);
        assert_abs_diff_eq!(0.5, factor.re, epsilon = 0.0001);
        assert_abs_diff_eq!(0.288675, factor.im, epsilon = 0.0001);
        assert_abs_diff_eq!(
            factor.norm(),
            Connection::Star.ratio_line_to_phase_voltage(phases),
        );
        assert_abs_diff_eq!(
            factor.arg(),
            Connection::Star.angle_line_to_phase_voltage(phases),
            epsilon = 0.0001
        );
    }
    {
        let factor = Connection::Delta.line_to_phase_voltage(phases);
        assert_abs_diff_eq!(1.0, factor.norm(), epsilon = 0.0001);
        assert_abs_diff_eq!(1.0, factor.re, epsilon = 0.0001);
        assert_abs_diff_eq!(0.0, factor.im, epsilon = 0.0001);
        assert_abs_diff_eq!(
            factor.norm(),
            Connection::Delta.ratio_line_to_phase_voltage(phases),
        );
        assert_abs_diff_eq!(
            factor.arg(),
            Connection::Delta.angle_line_to_phase_voltage(phases),
            epsilon = 0.0001
        );
    }
}

#[test]
fn test_line_to_phase_current() {
    let phases = NonZeroU16::new(3).unwrap();

    {
        let factor = Connection::Star.line_to_phase_current(phases);
        assert_abs_diff_eq!(1.0, factor.norm(), epsilon = 0.0001);
        assert_abs_diff_eq!(1.0, factor.re, epsilon = 0.0001);
        assert_abs_diff_eq!(0.0, factor.im, epsilon = 0.0001);
        assert_abs_diff_eq!(
            factor.norm(),
            Connection::Star.ratio_line_to_phase_current(phases),
        );
        assert_abs_diff_eq!(
            factor.arg(),
            Connection::Star.angle_line_to_phase_current(phases),
            epsilon = 0.0001
        );
    }
    {
        let factor = Connection::Delta.line_to_phase_current(phases);
        assert_abs_diff_eq!(0.57735, factor.norm(), epsilon = 0.0001);
        assert_abs_diff_eq!(0.5, factor.re, epsilon = 0.0001);
        assert_abs_diff_eq!(-0.288675, factor.im, epsilon = 0.0001);
        assert_abs_diff_eq!(
            factor.norm(),
            Connection::Delta.ratio_line_to_phase_current(phases),
        );
        assert_abs_diff_eq!(
            factor.arg(),
            Connection::Delta.angle_line_to_phase_current(phases),
            epsilon = 0.0001
        );
    }
}

#[test]
fn test_phase_currents() {
    let phases = NonZeroU16::new(3).expect("not zero");
    let angle_between_phases = std::f64::consts::TAU / f64::from(phases.get());
    {
        let dq0 = Dq0Current {
            direct: ElectricCurrent::new::<ampere>(1.0),
            quadrature: ElectricCurrent::new::<ampere>(0.0),
            zero: ElectricCurrent::new::<ampere>(0.0),
        };
        let currents: Vec<_> = dq0
            .phase_currents(3.try_into().expect("not zero"), angle_between_phases, 0.0)
            .collect();
        assert_abs_diff_eq!(currents[0].get::<ampere>(), 1.0, epsilon = 1e-3);
        assert_abs_diff_eq!(currents[1].get::<ampere>(), -0.5, epsilon = 1e-3);
        assert_abs_diff_eq!(currents[2].get::<ampere>(), -0.5, epsilon = 1e-3);
    }
    {
        let dq0 = Dq0Current {
            direct: ElectricCurrent::new::<ampere>(1.0),
            quadrature: ElectricCurrent::new::<ampere>(0.0),
            zero: ElectricCurrent::new::<ampere>(0.0),
        };
        let currents: Vec<_> = dq0
            .phase_currents(
                3.try_into().expect("not zero"),
                angle_between_phases,
                angle_between_phases,
            )
            .collect();
        assert_abs_diff_eq!(currents[0].get::<ampere>(), -0.5, epsilon = 1e-3);
        assert_abs_diff_eq!(currents[1].get::<ampere>(), 1.0, epsilon = 1e-3);
        assert_abs_diff_eq!(currents[2].get::<ampere>(), -0.5, epsilon = 1e-3);
    }
    {
        let dq0 = Dq0Current {
            direct: ElectricCurrent::new::<ampere>(1.0),
            quadrature: ElectricCurrent::new::<ampere>(0.0),
            zero: ElectricCurrent::new::<ampere>(0.0),
        };
        let currents: Vec<_> = dq0
            .phase_currents(
                3.try_into().expect("not zero"),
                angle_between_phases,
                2.0 * angle_between_phases,
            )
            .collect();
        assert_abs_diff_eq!(currents[0].get::<ampere>(), -0.5, epsilon = 1e-3);
        assert_abs_diff_eq!(currents[1].get::<ampere>(), -0.5, epsilon = 1e-3);
        assert_abs_diff_eq!(currents[2].get::<ampere>(), 1.0, epsilon = 1e-3);
    }
}

#[test]
fn test_winding_phase_currents() {
    let wdg1: DistributedWinding = DistributedMinimalBuilder {
        slots: 12.try_into().expect("not zero"),
        pole_pairs: 1.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 1.try_into().expect("not zero"),
        coil_throw_reduction: 0,
        zone_shift: 0,
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();

    let wdg2: DistributedWinding = DistributedMinimalBuilder {
        slots: 24.try_into().expect("not zero"),
        pole_pairs: 2.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 1.try_into().expect("not zero"),
        coil_throw_reduction: 0,
        zone_shift: 0,
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();

    for winding in [wdg1, wdg2] {
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
        }
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
                    winding.magnetic_axis(2.try_into().unwrap(), SpatialOrder::Electrical(1)),
                )
                .collect();
            assert_abs_diff_eq!(currents[0].get::<ampere>(), -0.5, epsilon = 1e-3);
            assert_abs_diff_eq!(currents[1].get::<ampere>(), 1.0, epsilon = 1e-3);
            assert_abs_diff_eq!(currents[2].get::<ampere>(), -0.5, epsilon = 1e-3);
        }
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
                    winding.magnetic_axis(3.try_into().unwrap(), SpatialOrder::Electrical(1)),
                )
                .collect();
            assert_abs_diff_eq!(currents[0].get::<ampere>(), -0.5, epsilon = 1e-3);
            assert_abs_diff_eq!(currents[1].get::<ampere>(), -0.5, epsilon = 1e-3);
            assert_abs_diff_eq!(currents[2].get::<ampere>(), 1.0, epsilon = 1e-3);
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
                    winding.magnetic_axis(2.try_into().unwrap(), SpatialOrder::Electrical(1)),
                )
                .collect();
            assert_abs_diff_eq!(currents[0].get::<ampere>(), -0.866, epsilon = 1e-3);
            assert_abs_diff_eq!(currents[1].get::<ampere>(), 0.0, epsilon = 1e-3);
            assert_abs_diff_eq!(currents[2].get::<ampere>(), 0.866, epsilon = 1e-3);
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
                    winding.magnetic_axis(3.try_into().unwrap(), SpatialOrder::Electrical(1)),
                )
                .collect();
            assert_abs_diff_eq!(currents[0].get::<ampere>(), 0.866, epsilon = 1e-3);
            assert_abs_diff_eq!(currents[1].get::<ampere>(), -0.866, epsilon = 1e-3);
            assert_abs_diff_eq!(currents[2].get::<ampere>(), 0.0, epsilon = 1e-3);
        }
    }
}
