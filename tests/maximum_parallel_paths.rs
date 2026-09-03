use winding::*;

#[test]
fn test_various_windings_parallel_paths() {
    let wdg = WindingDistributed::new_minimal(6, 1, 3, 1, 0, 0, WindingTableMethod::CoilSide).unwrap();
    assert_eq!(wdg.periodicity(), 1);
    assert_eq!(wdg.coil_groups_per_phase(), 1);

    let wdg = WindingToothCoil::new_minimal(12, 5, 3, 2, WindingTableMethod::Tingley).unwrap();
    assert_eq!(wdg.periodicity(), 1);
    assert_eq!(wdg.coil_groups_per_phase(), 2);

    let wdg = WindingToothCoil::new_minimal(12, 5, 3, 1, WindingTableMethod::Tingley).unwrap();
    assert_eq!(wdg.periodicity(), 1);
    assert_eq!(wdg.coil_groups_per_phase(), 2);

    let wdg = WindingToothCoil::new_minimal(12, 4, 3, 2, WindingTableMethod::Tingley).unwrap();
    assert_eq!(wdg.periodicity(), 4);
    assert_eq!(wdg.coil_groups_per_phase(), 4);

    let winding =
        WindingDistributed::new_minimal(18, 2, 3, 2, 0, 0, WindingTableMethod::Tingley).unwrap();
    assert_eq!(winding.coil_groups_per_phase(), 2);
    assert_eq!(winding.coils_per_phase(), 6);
    assert_eq!(winding.coils_per_coil_group(), 3);

    let winding =
        WindingDistributed::new_minimal(18, 2, 3, 1, 0, 0, WindingTableMethod::CoilSide).unwrap();
    assert_eq!(winding.coil_groups_per_phase(), 1);
    assert_eq!(winding.coils_per_phase(), 3);
    assert_eq!(winding.coils_per_coil_group(), 3);

    let winding =
        WindingDistributed::new_minimal(12, 1, 3, 1, 0, 0, WindingTableMethod::Tingley).unwrap();
    assert_eq!(winding.coil_groups_per_phase(), 2);
    assert_eq!(winding.coils_per_phase(), 2);
    assert_eq!(winding.coils_per_coil_group(), 1);
}
