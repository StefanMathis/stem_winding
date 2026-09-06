use std::num::NonZeroU16;

use stem_winding::stem_material::prelude::*;
use stem_winding::winding::*;

#[test]
fn test_periodicity() {
    assert_eq!(
        2,
        periodicity(
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            NonZeroU16::new(1).expect("not zero")
        )
        .get()
    );
    assert_eq!(
        5,
        periodicity(
            NonZeroU16::new(15).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            NonZeroU16::new(2).expect("not zero")
        )
        .get()
    );
    assert_eq!(
        2,
        periodicity(
            NonZeroU16::new(18).expect("not zero"),
            NonZeroU16::new(10).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            NonZeroU16::new(2).expect("not zero")
        )
        .get()
    );
    assert_eq!(
        1,
        periodicity(
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
    approxim::assert_abs_diff_eq!(
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
    approxim::assert_abs_diff_eq!(
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
    approxim::assert_abs_diff_eq!(
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

    approxim::assert_abs_diff_eq!(
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
fn test_phase_sequence() {
    let (ps, a) = phase_sequence(NonZeroU16::new(3).expect("not zero"));
    assert_eq!(ps, vec![1i32, -3, 2, -1, 3, -2]);
    approxim::assert_abs_diff_eq!(a[0], 0.0, epsilon = 0.001);
    approxim::assert_abs_diff_eq!(a[1], 1.047197, epsilon = 0.001);
    approxim::assert_abs_diff_eq!(a[2], 2.094395, epsilon = 0.001);
    approxim::assert_abs_diff_eq!(a[3], 3.141592, epsilon = 0.001);
    approxim::assert_abs_diff_eq!(a[4], 4.188790, epsilon = 0.001);
    approxim::assert_abs_diff_eq!(a[5], 5.235988, epsilon = 0.001);
}
