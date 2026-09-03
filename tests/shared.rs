use stem_winding::prelude::*;
use stem_winding::shared::*;

#[test]
fn test_periodicity() {
    assert_eq!(2, periodicity(12, 2, 3, 1));
    assert_eq!(5, periodicity(15, 5, 3, 2));
    assert_eq!(2, periodicity(18, 10, 3, 2));
    assert_eq!(1, periodicity(19, 4, 3, 1));
}

#[test]
fn test_curvature_factor() {
    approxim::assert_abs_diff_eq!(
        curvature_factor(
            2,
            Length::new::<millimeter>(55.0),
            Length::new::<millimeter>(0.6),
            1.0
        ),
        0.994695,
        epsilon = 0.001
    );
    approxim::assert_abs_diff_eq!(
        curvature_factor(
            2,
            Length::new::<millimeter>(55.0),
            Length::new::<millimeter>(0.6),
            7.0
        ),
        1.002341,
        epsilon = 0.001
    );
    approxim::assert_abs_diff_eq!(
        curvature_factor(
            10,
            Length::new::<millimeter>(55.0),
            Length::new::<millimeter>(0.6),
            1.0
        ),
        0.998521,
        epsilon = 0.001
    );

    approxim::assert_abs_diff_eq!(
        curvature_factor(
            5,
            Length::new::<millimeter>(37.5),
            Length::new::<millimeter>(0.6),
            1.0
        ),
        0.9941281,
        epsilon = 1e-6
    );
}

#[test]
fn test_phase_sequence() {
    let (ps, a) = phase_sequence(3);
    assert_eq!(ps, vec![1i32, -3, 2, -1, 3, -2]);
    approxim::assert_abs_diff_eq!(a[0], 0.0, epsilon = 0.001);
    approxim::assert_abs_diff_eq!(a[1], 1.047197, epsilon = 0.001);
    approxim::assert_abs_diff_eq!(a[2], 2.094395, epsilon = 0.001);
    approxim::assert_abs_diff_eq!(a[3], 3.141592, epsilon = 0.001);
    approxim::assert_abs_diff_eq!(a[4], 4.188790, epsilon = 0.001);
    approxim::assert_abs_diff_eq!(a[5], 5.235988, epsilon = 0.001);
}
