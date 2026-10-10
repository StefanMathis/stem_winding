use std::num::NonZeroU16;

use approxim;
use stem_winding::{iterators::WindingHarmonic, prelude::*};

const ONE: NonZeroU16 = NonZeroU16::MIN;

#[test]
fn test_air_gap_leakage_factor() {
    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 12.try_into().expect("not zero"),
        pole_pairs: 4.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 2.try_into().expect("not zero"),
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();
    approxim::assert_abs_diff_eq!(
        0.46216,
        winding.air_gap_leakage_factor(NonZeroU16::MIN),
        epsilon = 0.0001
    );

    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 12.try_into().expect("not zero"),
        pole_pairs: 5.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 2.try_into().expect("not zero"),
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();
    approxim::assert_abs_diff_eq!(
        0.96835,
        winding.air_gap_leakage_factor(NonZeroU16::MIN),
        epsilon = 0.0001
    );

    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 24.try_into().expect("not zero"),
        pole_pairs: 10.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 2.try_into().expect("not zero"),
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();
    approxim::assert_abs_diff_eq!(
        0.96835,
        winding.air_gap_leakage_factor(NonZeroU16::MIN),
        epsilon = 0.0001
    );

    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 12.try_into().expect("not zero"),
        pole_pairs: 5.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 1.try_into().expect("not zero"),
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();
    approxim::assert_abs_diff_eq!(
        2.67299,
        winding.air_gap_leakage_factor(NonZeroU16::MIN),
        epsilon = 0.0001
    );

    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 9.try_into().expect("not zero"),
        pole_pairs: 5.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 2.try_into().expect("not zero"),
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();
    approxim::assert_abs_diff_eq!(
        2.40953,
        winding.air_gap_leakage_factor(NonZeroU16::MIN),
        epsilon = 0.0001
    );

    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 9.try_into().expect("not zero"),
        pole_pairs: 4.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 2.try_into().expect("not zero"),
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();
    approxim::assert_abs_diff_eq!(
        1.18210,
        winding.air_gap_leakage_factor(NonZeroU16::MIN),
        epsilon = 0.0001
    );

    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 18.try_into().expect("not zero"),
        pole_pairs: 10.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 2.try_into().expect("not zero"),
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();
    approxim::assert_abs_diff_eq!(
        2.40953,
        winding.air_gap_leakage_factor(NonZeroU16::MIN),
        epsilon = 0.0001
    );
}

#[test]
fn test_coil_direction() {
    {
        // 12/10 SL
        let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 5.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            winding_table_constructor: WindingTableConstructor::Tingley,
        }
        .try_into()
        .unwrap();

        for coil in winding.coils_iter() {
            if let Coil::Full(coil) = coil {
                assert_eq!(coil.throw(Some(winding.slots())), 1); // A tooth coil winding always has a throw of 1

                // Exclude corner cases
                if coil.positive_zone().slot > 0 && coil.positive_zone().slot < 11 {
                    if coil.positive_zone() < coil.negative_zone() {
                        assert!(coil.positive_slot_direction());
                    } else {
                        assert!(!coil.positive_slot_direction());
                    }
                }
            }
        }
    }

    {
        // 12/10 DL
        let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 5.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            winding_table_constructor: WindingTableConstructor::Tingley,
        }
        .try_into()
        .unwrap();

        for coil in winding.coils_iter() {
            if let Coil::Full(coil) = coil {
                assert_eq!(coil.throw(Some(winding.slots())), 1); // A tooth coil winding always has a throw of 1

                // Exclude corner cases
                if coil.positive_zone().slot > 0 && coil.positive_zone().slot < 11 {
                    if coil.positive_zone() < coil.negative_zone() {
                        assert!(coil.positive_slot_direction());
                    } else {
                        assert!(!coil.positive_slot_direction());
                    }
                }
            }
        }
    }
}

#[test]
fn test_winding_12_8_dl() {
    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 12.try_into().expect("not zero"),
        pole_pairs: 4.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 2.try_into().expect("not zero"),
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();

    assert_eq!(4, winding.base_winding_count().get());

    // Check the number of parallel paths
    assert_eq!(4, winding.coil_groups_per_phase().get());
    let parallel_paths: Vec<u16> = winding.possible_parallel_paths().map(|v| v.get()).collect();
    assert_eq!(parallel_paths, vec![1, 2, 4]);

    // Assert that the winding is symmetric
    assert!(winding.equal_winding_factors());

    let harmonics: Vec<WindingHarmonic> = winding.harmonics().take(5).collect();
    assert_eq!(
        harmonics[0],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(4),
            is_positive: true
        }
    );
    assert_eq!(
        harmonics[1],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(8),
            is_positive: false
        }
    );
    assert_eq!(
        harmonics[2],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(16),
            is_positive: true
        }
    );
    assert_eq!(
        harmonics[3],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(20),
            is_positive: false
        }
    );
    assert_eq!(
        harmonics[4],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(28),
            is_positive: true
        }
    );

    // Check the winding factor
    approxim::assert_abs_diff_eq!(
        0.866,
        winding.winding_factor(ONE, SpatialOrder::Electrical(1)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.866,
        winding.winding_factor(ONE, SpatialOrder::Electrical(2)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.866,
        winding.winding_factor(ONE, SpatialOrder::Electrical(4)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.866,
        winding.winding_factor(ONE, SpatialOrder::Electrical(5)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.866,
        winding.winding_factor(ONE, SpatialOrder::Electrical(7)),
        epsilon = 0.0001
    );
}

#[test]
fn test_winding_12_10_dl() {
    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 12.try_into().expect("not zero"),
        pole_pairs: 5.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 2.try_into().expect("not zero"),
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();

    assert_eq!(1, winding.base_winding_count().get());

    // Check the number of parallel paths
    assert_eq!(2, winding.coil_groups_per_phase().get());
    let parallel_paths: Vec<u16> = winding.possible_parallel_paths().map(|v| v.get()).collect();
    assert_eq!(parallel_paths, vec![1, 2]);

    // Assert that the winding is symmetric
    assert!(winding.equal_winding_factors());

    let harmonics: Vec<WindingHarmonic> = winding.harmonics().take(5).collect();
    assert_eq!(
        harmonics[0],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(1),
            is_positive: true
        }
    );
    assert_eq!(
        harmonics[1],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(5),
            is_positive: false
        }
    );
    assert_eq!(
        harmonics[2],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(7),
            is_positive: true
        }
    );
    assert_eq!(
        harmonics[3],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(11),
            is_positive: false
        }
    );
    assert_eq!(
        harmonics[4],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(13),
            is_positive: true
        }
    );

    // Check the winding factor
    approxim::assert_abs_diff_eq!(
        0.067,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(1)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.933,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(5)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.933,
        winding.winding_factor(ONE, SpatialOrder::Electrical(1)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.933,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(7)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.067,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(11)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.067,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(13)),
        epsilon = 0.0001
    );
}

#[test]
fn test_winding_24_20_dl() {
    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 24.try_into().expect("not zero"),
        pole_pairs: 10.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 2.try_into().expect("not zero"),
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();

    assert_eq!(2, winding.base_winding_count().get());

    // Check the number of parallel paths
    assert_eq!(4, winding.coil_groups_per_phase().get());
    let parallel_paths: Vec<u16> = winding.possible_parallel_paths().map(|v| v.get()).collect();
    assert_eq!(parallel_paths, vec![1, 2, 4]);

    // Assert that the winding is symmetric
    assert!(winding.equal_winding_factors());

    let harmonics: Vec<WindingHarmonic> = winding.harmonics().take(5).collect();
    assert_eq!(
        harmonics[0],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(2),
            is_positive: true
        }
    );
    assert_eq!(
        harmonics[1],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(10),
            is_positive: false
        }
    );
    assert_eq!(
        harmonics[2],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(14),
            is_positive: true
        }
    );
    assert_eq!(
        harmonics[3],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(22),
            is_positive: false
        }
    );
    assert_eq!(
        harmonics[4],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(26),
            is_positive: true
        }
    );

    // Check the winding factor
    approxim::assert_abs_diff_eq!(
        0.067,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(2)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.933,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(10)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.933,
        winding.winding_factor(ONE, SpatialOrder::Electrical(1)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.933,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(14)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.067,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(22)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.067,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(26)),
        epsilon = 0.0001
    );
}

#[test]
fn test_winding_12_10_sl() {
    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 12.try_into().expect("not zero"),
        pole_pairs: 5.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 1.try_into().expect("not zero"),
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();

    assert_eq!(1, winding.base_winding_count().get());

    // Check the number of parallel paths
    assert_eq!(2, winding.coil_groups_per_phase().get());
    let parallel_paths: Vec<u16> = winding.possible_parallel_paths().map(|v| v.get()).collect();
    assert_eq!(parallel_paths, vec![1, 2]);

    // Assert that the winding is symmetric
    assert!(winding.equal_winding_factors());

    let harmonics: Vec<WindingHarmonic> = winding.harmonics().take(5).collect();
    assert_eq!(
        harmonics[0],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(1),
            is_positive: true
        }
    );
    assert_eq!(
        harmonics[1],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(5),
            is_positive: false
        }
    );
    assert_eq!(
        harmonics[2],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(7),
            is_positive: true
        }
    );
    assert_eq!(
        harmonics[3],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(11),
            is_positive: false
        }
    );
    assert_eq!(
        harmonics[4],
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(13),
            is_positive: true
        }
    );

    // Check the winding factor
    approxim::assert_abs_diff_eq!(
        0.2588,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(1)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.9659,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(5)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.9659,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(7)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.2588,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(11)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.2588,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(13)),
        epsilon = 0.0001
    );
}

#[test]
fn test_default_tooth_coil_assembly() {
    let winding = ToothCoilWinding::default();

    // Compare the zone plan
    let winding_table = winding.winding_table(true);
    let expected_result = WindingTable::from_layer_major(
        [-3, 1, -1, 2, -2, 3].into_iter(),
        winding.slots(),
        winding.layers(),
    );
    assert_eq!(winding_table, expected_result);
    assert_eq!(1, winding.base_winding_count().get());
}

#[test]
fn test_derive_coil_assembly_12_10_dl() {
    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 12.try_into().expect("not zero"),
        pole_pairs: 5.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 2.try_into().expect("not zero"),
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();
    let coil_assembly = CoilAssembly::from(&winding);

    assert_eq!(coil_assembly.phases(), winding.phases());
    assert_eq!(coil_assembly.slots(), winding.slots());
    assert_eq!(coil_assembly.layers(), winding.layers());

    let coils: Vec<Coil> = coil_assembly.coils_iter().cloned().collect();
    assert_eq!(coils.len(), 12);

    // Check the first coil
    match &coils[0] {
        Coil::Full(coil) => {
            assert_eq!(coil.positive_zone(), Zone::new(0, 0));
            assert_eq!(coil.negative_zone(), Zone::new(11, 1));
            assert_eq!(coil.phase().get(), 1);
        }
        _ => panic!("Must be full coil"),
    }

    // Compare the zone plans
    assert_eq!(
        winding.winding_table(true),
        coil_assembly.winding_table(true)
    );

    // Compare the winding factor of phase 1
    approxim::assert_abs_diff_eq!(
        coil_assembly.winding_factor(ONE, SpatialOrder::Electrical(1)),
        winding.winding_factor(ONE, SpatialOrder::Electrical(1)),
        epsilon = 0.0001
    );
}

#[test]
fn test_derive_coil_assembly_12_10_sl() {
    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 12.try_into().expect("not zero"),
        pole_pairs: 5.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 1.try_into().expect("not zero"),
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();
    let coil_assembly = CoilAssembly::from(&winding);

    assert_eq!(coil_assembly.phases(), winding.phases());
    assert_eq!(coil_assembly.slots(), winding.slots());
    assert_eq!(coil_assembly.layers(), winding.layers());

    let coils: Vec<Coil> = coil_assembly.coils_iter().cloned().collect();
    assert_eq!(coils.len(), 6);

    // Compare the zone plans
    assert_eq!(
        winding.winding_table(true),
        coil_assembly.winding_table(true)
    );

    // Compare the winding factor of phase 1
    approxim::assert_abs_diff_eq!(
        coil_assembly.winding_factor(ONE, SpatialOrder::Electrical(1)),
        winding.winding_factor(ONE, SpatialOrder::Electrical(1)),
        epsilon = 0.0001
    );
}

#[test]
fn test_field_excitation_curve() {
    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 12.try_into().expect("not zero"),
        pole_pairs: 5.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 2.try_into().expect("not zero"),
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();
    let mut buffer = vec![0.0; winding.slots().get() as usize];
    let currents = [1.0, -0.5, -0.5];

    winding
        .field_excitation_curve(&mut buffer, currents.as_slice())
        .unwrap();

    assert_eq!(
        buffer.as_slice(),
        &[
            1.0, -0.5, 0.5, 0.5, -0.5, 1.0, -1.0, 0.5, -0.5, -0.5, 0.5, -1.0
        ]
    );
}

#[cfg(feature = "serde")]
mod serde_tests {

    use super::*;
    use indoc::indoc;
    use serde_mosaic::{DatabaseManager, SerdeYaml};

    fn create_dbm() -> DatabaseManager {
        return DatabaseManager::open("tests", SerdeYaml).expect("must exist");
    }

    #[test]
    fn test_load_wire_with_database_material() {
        // Read from the database
        let yaml = indoc! {"
        ---
        slots: 12
        pole_pairs: 5
        phases: 3
        layers: 2
        turns_per_coil: 50
        parallel_paths: 1
        connection: Star
        end_winding_leakage_coefficient: 0.25
        wire:
            SffWire:
                conductor_material:
                    name: Copper
                slot_fill_factor_conductor: 0.375
                slot_fill_factor_overall: 0.4
        winding_table_constructor: Tingley
        "};

        let winding: ToothCoilWinding = create_dbm()
            .from_str::<ToothCoilWinding, SerdeYaml>(yaml)
            .unwrap();
        assert_eq!(winding.turns_in_slot(1), 100);
    }

    #[test]
    fn test_deserialize_failed_to_create_winding_table() {
        // Fails because a 12/12 single layer winding is not possible
        let yaml = indoc! {"
        ---
        slots: 12
        pole_pairs: 6
        phases: 3
        layers: 1
        winding_table_constructor: Tingley
        "};

        let maybe_winding: Result<ToothCoilWinding, _> = yaml_serde::from_str(yaml);
        assert!(maybe_winding.is_err());
    }

    #[test]
    fn test_deserialize_min() {
        // Minimal winding
        let yaml = indoc! {"
            ---
            slots: 12
            pole_pairs: 5
            phases: 3
            layers: 1
            winding_table_constructor: Tingley
            "};

        let winding: ToothCoilWinding = yaml_serde::from_str(yaml).unwrap();

        // Compare the winding factor
        approxim::assert_abs_diff_eq!(
            winding.winding_factor(ONE, SpatialOrder::Electrical(1)),
            0.966,
            epsilon = 1e-3
        );
    }

    #[test]
    fn test_deserialize_full() {
        // Full winding
        let yaml = indoc! {"
            ---
            slots: 12
            pole_pairs: 5
            phases: 3
            layers: 2
            turns_per_coil: 10
            parallel_paths: 2
            connection: Star
            end_winding_leakage_coefficient: 0.0
            wire:
                RoundWire:
                    outer_diameter: 1e-3
                    inner_diameter: 0.0
                    insulation_thickness: 0.0
                    conductor_material:
                        name: Copper
                        relative_permeability: 1.0
            winding_table_constructor: Tingley
            "};

        let winding: ToothCoilWinding = yaml_serde::from_str(yaml).unwrap();

        assert_eq!(winding.turns_in_slot(0), 20);
        assert_eq!(winding.series_turns_per_phase(ONE).numer().clone(), 20);
        assert_eq!(winding.parallel_paths(ONE).get(), 2);
    }

    #[test]
    fn test_serialize_and_deserialize() {
        let winding = ToothCoilWinding::default();
        let string = yaml_serde::to_string(&winding).expect("can be serialized");
        let winding_de: ToothCoilWinding =
            yaml_serde::from_str(&string).expect("can be deserialized");

        assert_eq!(winding.slots(), winding_de.slots());
        assert_eq!(winding.layers(), winding_de.layers());
        assert_eq!(winding.pole_pairs(), winding_de.pole_pairs());
        assert_eq!(winding.winding_table(true), winding_de.winding_table(true));
    }
}

#[cfg(feature = "stem_core")]
mod stem_core_tests {
    use super::*;

    use std::{f64::consts::PI, sync::Arc};

    use serde_mosaic::{DatabaseManager, SerdeYaml};
    use si::inductance::henry;

    fn create_core_meas_servo() -> RotCore {
        let slot_angle = PI / 6.0;
        let slot: OpenTrapezoidSlot = OpenTrapezoidWithoutSlopesBuilder {
            opening_width: Length::new::<millimeter>(9.6),
            side_height: Length::new::<millimeter>(19.0),
            opening_height: Length::new::<millimeter>(1.0),
            slot_angle,
            bottom_radius: Length::new::<millimeter>(0.0),
            consider_tooth_tip_leakage: false,
        }
        .try_into()
        .unwrap();

        return RotCoreBuilder {
            air_gap_radius: Length::new::<millimeter>(37.5),
            yoke_radius: Length::new::<millimeter>(63.0),
            axial_length: Length::new::<millimeter>(60.0),
            axial_coil_overhang: Length::new::<millimeter>(0.0),
            iron_fill_factor: 0.95,
            material: Arc::new(Material::default()),
            pole_pairs: 5.try_into().expect("not zero"),
            skew_angle: 0.0,
            air_gap: Box::new(SlottedAirGap::new(
                12.try_into().expect("not zero"),
                true,
                CarterFactorModel::Bin12,
                Box::new(slot),
            )),
            flux_barrier: None,
        }
        .try_into()
        .expect("valid magnetic core");
    }

    fn create_core_1210_sl() -> RotCore {
        let slot_angle = PI / 6.0;
        let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
            bottom_width: Length::new::<millimeter>(21.5),
            opening_width: Length::new::<millimeter>(4.0),
            height: Length::new::<millimeter>(19.35),
            opening_height: Length::new::<millimeter>(1.0),
            slot_angle,
            bottom_radius: Length::new::<millimeter>(2.0),
            top_radius: Length::new::<millimeter>(2.0),
            opening_radius: Length::new::<millimeter>(0.5),
            consider_tooth_tip_leakage: false,
        }
        .try_into()
        .unwrap();

        return RotCoreBuilder {
            air_gap_radius: Length::new::<millimeter>(38.2),
            yoke_radius: Length::new::<millimeter>(63.0),
            axial_length: Length::new::<millimeter>(60.0),
            axial_coil_overhang: Length::new::<millimeter>(0.0),
            iron_fill_factor: 0.95,
            material: Arc::new(Material::default()),
            pole_pairs: 5.try_into().expect("not zero"),
            skew_angle: 0.0,
            air_gap: Box::new(SlottedAirGap::new(
                12.try_into().expect("not zero"),
                true,
                CarterFactorModel::Bin12,
                Box::new(slot),
            )),
            flux_barrier: None,
        }
        .try_into()
        .expect("valid magnetic core");
    }

    fn create_winding_meas_servo() -> ToothCoilWinding {
        let copper: Material = create_dbm().read("Copper").unwrap();
        let copper = Arc::new(copper);
        let wire = SffWire::new(copper, 0.375, 0.4).unwrap();
        ToothCoilBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 5.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            turns_per_coil: 50.try_into().expect("not zero"),
            parallel_paths: 1.try_into().expect("not zero"),
            connection: Connection::Star,
            end_winding_leakage_coefficient: 0.25,
            wire: Box::new(wire),
            winding_table_constructor: WindingTableConstructor::Tingley,
        }
        .try_into()
        .unwrap()
    }

    fn create_winding_1210_sl() -> ToothCoilWinding {
        let copper: Material = create_dbm().read("Copper").unwrap();
        let copper = Arc::new(copper);
        let wire = SffWire::new(copper, 0.375, 0.4).unwrap();
        ToothCoilBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 5.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            turns_per_coil: 140.try_into().expect("not zero"),
            parallel_paths: 1.try_into().expect("not zero"),
            connection: Connection::Star,
            end_winding_leakage_coefficient: 0.25,
            wire: Box::new(wire),
            winding_table_constructor: WindingTableConstructor::Tingley,
        }
        .try_into()
        .unwrap()
    }

    fn create_dbm() -> DatabaseManager {
        return DatabaseManager::open("tests", SerdeYaml).expect("must exist");
    }

    #[test]
    fn test_concentrated_winding_meas_servo() {
        let core = create_core_meas_servo();
        let winding = create_winding_meas_servo();

        // Check the mean end winding length
        approxim::assert_abs_diff_eq!(
            winding
                .end_winding_half_turn_length(CoreRef::Rot(&core), Zone::new(0, 0),)
                .get::<meter>(),
            0.027807,
            epsilon = 0.00001
        );

        // Slot leakage inductance
        approxim::assert_abs_diff_eq!(
            winding
                .slot_leakage_inductance(
                    CoreRef::Rot(&core),
                    NonZeroU16::MIN,
                    Length::new::<millimeter>(1.0),
                    &[],
                    &Default::default(),
                )
                .get::<henry>(),
            0.0018424,
            epsilon = 1e-6
        );
    }

    #[test]
    fn test_concentrated_winding_1210_sl() {
        let core = create_core_1210_sl();
        let winding = create_winding_1210_sl();

        // Check the mean end winding length
        approxim::assert_abs_diff_eq!(
            winding
                .end_winding_half_turn_length(CoreRef::Rot(&core), Zone::new(0, 0))
                .get::<meter>(),
            0.0399880,
            epsilon = 0.00001
        );

        // Slot leakage inductance
        approxim::assert_abs_diff_eq!(
            winding
                .slot_leakage_inductance(
                    CoreRef::Rot(&core),
                    NonZeroU16::MIN,
                    Length::new::<millimeter>(1.0),
                    &[],
                    &Default::default(),
                )
                .get::<henry>(),
            0.0044860,
            epsilon = 1e-6
        );
    }
}
