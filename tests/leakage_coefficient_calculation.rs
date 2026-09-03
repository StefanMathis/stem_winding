use winding::*;

#[test]
fn test_various_windings_air_gap_leakage_factor() {
    let wdg = WindingToothCoil::new_minimal(12, 5, 3, 1, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(wdg.air_gap_leakage_factor(), 2.67298, epsilon = 1e-3);

    let wdg = WindingToothCoil::new_minimal(12, 5, 3, 2, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(wdg.air_gap_leakage_factor(), 0.96834, epsilon = 1e-3);

    let wdg = WindingToothCoil::new_minimal(15, 5, 3, 2, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(wdg.air_gap_leakage_factor(), 0.46216, epsilon = 1e-3);

    let wdg = WindingDistributed::new_minimal(24, 5, 3, 1, 0, 0, WindingTableMethod::CoilSide).unwrap();
    approxim::assert_abs_diff_eq!(wdg.air_gap_leakage_factor(), 0.75215, epsilon = 1e-3);

    let wdg = WindingDistributed::new_minimal(24, 5, 3, 2, 0, 0, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(wdg.air_gap_leakage_factor(), 0.25154, epsilon = 1e-3);

    let wdg = WindingDistributed::new_minimal(30, 5, 3, 1, 0, 0, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(wdg.air_gap_leakage_factor(), 0.09662, epsilon = 1e-3);

    let wdg = WindingDistributed::new_minimal(36, 5, 3, 1, 0, 0, WindingTableMethod::CoilSide).unwrap();
    approxim::assert_abs_diff_eq!(wdg.air_gap_leakage_factor(), 0.35994, epsilon = 1e-3);

    let wdg = WindingDistributed::new_minimal(36, 5, 3, 2, 0, 0, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(wdg.air_gap_leakage_factor(), 0.11601, epsilon = 1e-3);

    // =====================================================

    let wdg = WindingDistributed::new_minimal(60, 10, 3, 1, 0, 0, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(wdg.air_gap_leakage_factor(), 0.09662, epsilon = 1e-3);

    let wdg = WindingDistributed::new_minimal(60, 5, 3, 1, 0, 0, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(wdg.air_gap_leakage_factor(), 0.02842, epsilon = 1e-3);
}
