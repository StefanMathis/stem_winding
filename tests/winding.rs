use std::num::{NonZeroU16, NonZeroUsize};

use approxim::assert_abs_diff_eq;
use stem_winding::stem_material::prelude::*;
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
fn test_base_winding_count_repeating_coil_groups() {
    assert_eq!(
        2,
        base_winding_count_repeating_coil_groups(
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            NonZeroU16::new(1).expect("not zero")
        )
        .get()
    );
    assert_eq!(
        5,
        base_winding_count_repeating_coil_groups(
            NonZeroU16::new(15).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            NonZeroU16::new(2).expect("not zero")
        )
        .get()
    );
    assert_eq!(
        2,
        base_winding_count_repeating_coil_groups(
            NonZeroU16::new(18).expect("not zero"),
            NonZeroU16::new(10).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            NonZeroU16::new(2).expect("not zero")
        )
        .get()
    );
    assert_eq!(
        1,
        base_winding_count_repeating_coil_groups(
            NonZeroU16::new(19).expect("not zero"),
            NonZeroU16::new(4).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            NonZeroU16::new(1).expect("not zero")
        )
        .get()
    );
}

#[test]
fn test_curvature_factor() {
    assert_abs_diff_eq!(
        curvature_factor(
            NonZeroU16::new(2).expect("not zero"),
            Length::new::<millimeter>(55.0),
            Length::new::<millimeter>(0.6),
            1.0,
            true
        ),
        0.994695,
        epsilon = 0.001
    );
    assert_abs_diff_eq!(
        curvature_factor(
            NonZeroU16::new(2).expect("not zero"),
            Length::new::<millimeter>(55.0),
            Length::new::<millimeter>(0.6),
            7.0,
            true
        ),
        1.002341,
        epsilon = 0.001
    );
    assert_abs_diff_eq!(
        curvature_factor(
            NonZeroU16::new(10).expect("not zero"),
            Length::new::<millimeter>(55.0),
            Length::new::<millimeter>(0.6),
            1.0,
            true
        ),
        0.998521,
        epsilon = 0.001
    );

    assert_abs_diff_eq!(
        curvature_factor(
            NonZeroU16::new(5).expect("not zero"),
            Length::new::<millimeter>(37.5),
            Length::new::<millimeter>(0.6),
            1.0,
            true
        ),
        0.9941281,
        epsilon = 1e-6
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
