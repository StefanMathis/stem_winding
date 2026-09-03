use approxim;
use indoc::indoc;
use magnetic_core::{AirGapSlotted, CarterFactorModel, CoreRot, CoreRotBuilder, IsCoreRef};
use material::Material;
use nalgebra::DMatrix;
use slot::{SlotRectangular, SlotTrapezoidSemi};
use std::f64::consts::PI;
use std::sync::Arc;
use test_database::create_dbm;
use uom::si::electrical_resistance::ohm;
use uom::si::f64::*;
use uom::si::inductance::henry;
use uom::si::length::{meter, millimeter};
use uom::si::volume::cubic_millimeter;
use winding::*;
use winding::{Connection, WindingTableMethod};
use wire::{WireGroup, RoundWire, StrandedWire};

#[test]
fn test_deserialize_failed_to_create_winding_table() {
    // Minimal winding
    let yaml = indoc! {"
          ---
          slots: 19
          pole_pairs: 4
          phases: 3
          layers: 1
          coil_span_reduction: 0
          zone_span_variation: 0
          winding_table_method: Zone
          "};

    let maybe_winding: std::io::Result<WindingDistributed> = create_dbm().from_str(yaml);
    assert!(maybe_winding.is_err());
}

#[test]
fn test_deserialize_min() {
    // Minimal winding
    let yaml = indoc! {"
            ---
            slots: 18
            pole_pairs: 4
            phases: 3
            layers: 1
            coil_span_reduction: 0
            zone_span_variation: 0
            winding_table_method: CoilSide
            "};

    let winding: WindingDistributed = create_dbm().from_str(yaml).unwrap();

    // Compare the zone plan
    let winding_table = winding.winding_table(true);
    let expected_result = WindingTable(DMatrix::from_column_slice(
        1,
        18,
        &[
            1, 2, -1, 3, -2, -3, 2, 3, -2, 1, -3, -1, 3, 1, -3, 2, -1, -2,
        ],
    ));
    assert_eq!(winding_table, expected_result);
}

#[test]
fn test_deserialize_double_zone_span() {
    // Full winding
    let yaml = indoc! {"
    ---
    slots: 18
    pole_pairs: 1
    phases: 3
    coil_span_reduction: 0
    turns_per_coil: 1
    parallel_paths: 1
    connection: Star
    end_winding_leakage_coefficient: 0.0
    wire:
        RoundWire:
            outer_diameter: 1e-3
            inner_diameter: 0.0
            insulation_thickness: 0.0
            material_conductor:
                file_name: Copper
    winding_table_method: Tingley
    concentric_coils: false
    "};

    let winding: WindingDistributed = create_dbm().from_str(yaml).unwrap();

    // Compare the zone plan
    let winding_table = winding.winding_table(false);
    let expected_result = WindingTable(DMatrix::from_row_slice(
        2,
        18,
        &[
            1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, -2, -2, -2, -3, -3, -3, -3, -3,
            -3, -1, -1, -1, -1, -1, -1, -2, -2, -2,
        ],
    ));
    assert_eq!(winding_table, expected_result);
}

#[test]
fn test_deserialize_full() {
    // Full winding
    let yaml = indoc! {"
            ---
            slots: 36
            pole_pairs: 2
            phases: 3
            layers: 2
            coil_span_reduction: 0
            zone_span_variation: 0
            turns_per_coil: 31
            parallel_paths: 1
            connection: Star
            end_winding_leakage_coefficient: 0.0
            wire:
                RoundWire:
                    outer_diameter: 1e-3
                    inner_diameter: 0.0
                    insulation_thickness: 0.0
                    material_conductor:
                        file_name: Copper
            winding_table_method: Tingley
            concentric_coils: false
            "};

    let winding: WindingDistributed = create_dbm().from_str(yaml).unwrap();

    assert_eq!(winding.turns_in_slot(0), 62);
    assert_eq!(winding.turns_per_phase(1).numer().clone(), 372);
}

#[test]
fn test_winding_6_1_dl_coil_span_reduction() {
    let winding =
        WindingDistributed::new_minimal(6, 1, 3, 2, 0, 0, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(1.0, winding.winding_factor(1, 1.0), epsilon = 0.0001);

    let winding =
        WindingDistributed::new_minimal(6, 1, 3, 2, 1, 0, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(0.8660254, winding.winding_factor(1, 1.0), epsilon = 0.0001);

    let winding =
        WindingDistributed::new_minimal(6, 1, 3, 2, 2, 0, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(0.5, winding.winding_factor(1, 1.0), epsilon = 0.0001);

    let winding =
        WindingDistributed::new_minimal(6, 1, 3, 2, 3, 0, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(0.0, winding.winding_factor(1, 1.0), epsilon = 0.0001);

    let winding =
        WindingDistributed::new_minimal(6, 1, 3, 2, 4, 0, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(0.5, winding.winding_factor(1, 1.0), epsilon = 0.0001);
}

#[test]
fn test_winding_18_4_sl() {
    let winding =
        WindingDistributed::new_minimal(18, 4, 3, 1, 0, 0, WindingTableMethod::CoilSide).unwrap();

    // Check the number of parallel paths
    assert_eq!(1, winding.coil_groups_per_phase());

    // Assert that the winding is symmetric
    assert!(winding.equal_winding_factors());

    // Check the winding ordinals
    let ordinals: Vec<num::rational::Ratio<i32>> = winding.harmonic_ordinals().take(10).collect();
    assert_eq!(ordinals[0], num::rational::Ratio::new_raw(1, 4));
    assert_eq!(ordinals[1], num::rational::Ratio::new_raw(-2, 4));
    assert_eq!(ordinals[2], num::rational::Ratio::new_raw(4, 4));
    assert_eq!(ordinals[3], num::rational::Ratio::new_raw(-5, 4));
    assert_eq!(ordinals[4], num::rational::Ratio::new_raw(7, 4));
    assert_eq!(ordinals[5], num::rational::Ratio::new_raw(-8, 4));
    assert_eq!(ordinals[6], num::rational::Ratio::new_raw(10, 4));
    assert_eq!(ordinals[7], num::rational::Ratio::new_raw(-11, 4));
    assert_eq!(ordinals[8], num::rational::Ratio::new_raw(13, 4));
    assert_eq!(ordinals[9], num::rational::Ratio::new_raw(-14, 4));

    // Check the winding factor
    approxim::assert_abs_diff_eq!(0.9452, winding.winding_factor(1, 1.0), epsilon = 0.0001);
}

#[test]
fn test_set_concentric() {
    let wdg_1 = WindingDistributed::new(
        18,
        1,
        3,
        2,
        0,
        0,
        1,
        1,
        Connection::Star,
        0.0,
        Box::new(RoundWire::default()),
        WindingTableMethod::Tingley,
        true,
    )
    .unwrap();

    let mut wdg_2 = WindingDistributed::new(
        18,
        1,
        3,
        2,
        0,
        0,
        1,
        1,
        Connection::Star,
        0.0,
        Box::new(RoundWire::default()),
        WindingTableMethod::Tingley,
        false,
    )
    .unwrap();
    wdg_2.set_concentric_coils(true);

    for (coil_1, coil_2) in wdg_1.coils().zip(wdg_2.coils()) {
        assert_eq!(
            coil_1.zones().collect::<Vec<_>>(),
            coil_2.zones().collect::<Vec<_>>()
        );
        assert_eq!(coil_1.phase(), coil_2.phase());
    }
}

#[test]
fn test_coil_span() {
    {
        let winding =
            WindingDistributed::new_minimal(18, 1, 3, 1, 0, 0, WindingTableMethod::Tingley).unwrap();
        for coil in winding.coils() {
            if let Coil::Full(coil) = coil {
                assert_eq!(coil.span(winding.slots()), 9); // Coil span is always 9
            }
        }
    }

    {
        let winding =
            WindingDistributed::new_minimal(36, 2, 3, 2, 0, 0, WindingTableMethod::Tingley).unwrap();

        for coil in winding.coils() {
            if let Coil::Full(coil) = coil {
                assert_eq!(coil.span(winding.slots()), 9); // Coil span is always 9
                assert_ne!(coil.negative_zone().layer, coil.positive_zone().layer);
            }
        }
    }

    {
        let winding =
            WindingDistributed::new_minimal(24, 5, 3, 1, 0, 0, WindingTableMethod::Tingley).unwrap();

        for coil in winding.coils() {
            if let Coil::Full(coil) = coil {
                let span = coil.span(winding.slots());
                assert!(span == 1 || span == 2); // Coil span is either 2 or 1
            }
        }
    }

    {
        let winding =
            WindingDistributed::new_minimal(24, 5, 3, 2, 0, 0, WindingTableMethod::Tingley).unwrap();

        for coil in winding.coils() {
            if let Coil::Full(coil) = coil {
                let span = coil.span(winding.slots());
                assert!(span == 2); // Coil span is always 2
                assert_ne!(coil.negative_zone().layer, coil.positive_zone().layer);
            }
        }
    }

    {
        let winding =
            WindingDistributed::new_minimal(36, 5, 3, 1, 0, 0, WindingTableMethod::Tingley).unwrap();

        for coil in winding.coils() {
            if let Coil::Full(coil) = coil {
                let span = coil.span(winding.slots());
                assert!(span == 3 || span == 4); // Coil span is either 3 or 4
            }
        }
    }

    {
        let winding =
            WindingDistributed::new_minimal(36, 5, 3, 2, 0, 0, WindingTableMethod::Tingley).unwrap();

        for coil in winding.coils() {
            if let Coil::Full(coil) = coil {
                let span = coil.span(winding.slots());
                assert!(span == 3); // Coil span is always 3
                assert_ne!(coil.negative_zone().layer, coil.positive_zone().layer);
            }
        }
    }

    // Concentric winding
    {
        let winding = WindingDistributed::new(
            18,
            1,
            3,
            1,
            0,
            0,
            1,
            1,
            Connection::Star,
            0.0,
            Box::new(RoundWire::default()),
            WindingTableMethod::Tingley,
            true,
        )
        .unwrap();
        let s = winding.slots();

        // In a concentric winding, the coil is reduced to the middle of a coil group
        assert_eq!(winding.coil_at(Zone::new(0, 0)).unwrap().span(s), 11);
        assert_eq!(winding.coil_at(Zone::new(1, 0)).unwrap().span(s), 9);
        assert_eq!(winding.coil_at(Zone::new(2, 0)).unwrap().span(s), 7);
        assert_eq!(winding.coil_at(Zone::new(3, 0)).unwrap().span(s), 7);
        assert_eq!(winding.coil_at(Zone::new(4, 0)).unwrap().span(s), 9);
        assert_eq!(winding.coil_at(Zone::new(5, 0)).unwrap().span(s), 11);
        assert_eq!(winding.coil_at(Zone::new(6, 0)).unwrap().span(s), 11);
        assert_eq!(winding.coil_at(Zone::new(7, 0)).unwrap().span(s), 9);
        assert_eq!(winding.coil_at(Zone::new(8, 0)).unwrap().span(s), 7);
    }

    // Concentric winding
    {
        let winding = WindingDistributed::new(
            18,
            1,
            3,
            2,
            0,
            0,
            1,
            1,
            Connection::Star,
            0.0,
            Box::new(RoundWire::default()),
            WindingTableMethod::Tingley,
            true,
        )
        .unwrap();
        let s = winding.slots();

        // In a concentric winding, the coil is reduced to the middle of a coil group
        assert_eq!(winding.coil_at(Zone::new(0, 0)).unwrap().span(s), 11);
        assert_eq!(winding.coil_at(Zone::new(1, 0)).unwrap().span(s), 9);
        assert_eq!(winding.coil_at(Zone::new(2, 0)).unwrap().span(s), 7);
        assert_eq!(winding.coil_at(Zone::new(3, 0)).unwrap().span(s), 11);
        assert_eq!(winding.coil_at(Zone::new(4, 0)).unwrap().span(s), 9);
        assert_eq!(winding.coil_at(Zone::new(5, 0)).unwrap().span(s), 7);
        assert_eq!(winding.coil_at(Zone::new(6, 0)).unwrap().span(s), 11);
        assert_eq!(winding.coil_at(Zone::new(7, 0)).unwrap().span(s), 9);
        assert_eq!(winding.coil_at(Zone::new(8, 0)).unwrap().span(s), 7);
    }
}

#[test]
fn test_winding_36_4() {
    {
        let winding =
            WindingDistributed::new_minimal(36, 2, 3, 1, 0, 0, WindingTableMethod::Tingley).unwrap();

        // Compare the zone plan
        let winding_table = winding.winding_table(false);
        let expected_result = WindingTable(DMatrix::from_column_slice(
            1,
            18,
            &[
                1, 1, 1, -3, -3, -3, 2, 2, 2, -1, -1, -1, 3, 3, 3, -2, -2, -2,
            ],
        ));
        assert_eq!(winding_table, expected_result);

        // Each coil has a slot pitch of 9
        for coil in winding.coils() {
            match coil {
                Coil::Full(coil) => {
                    assert_eq!(coil.span(winding.slots()), 9);
                }
                Coil::Half(_) => unreachable!(),
            }
        }

        // Check the number of parallel paths
        assert_eq!(2, winding.coil_groups_per_phase());
        let parallel_paths: Vec<usize> = winding.possible_parallel_paths().collect();
        assert_eq!(parallel_paths, vec![1, 2]);

        // Assert that the winding is symmetric
        assert!(winding.equal_winding_factors());

        // Check the winding ordinals
        let ordinals: Vec<num::rational::Ratio<i32>> =
            winding.harmonic_ordinals().take(5).collect();
        assert_eq!(ordinals[0], num::rational::Ratio::new(1, 1));
        assert_eq!(ordinals[1], num::rational::Ratio::new(-5, 1));
        assert_eq!(ordinals[2], num::rational::Ratio::new(7, 1));
        assert_eq!(ordinals[3], num::rational::Ratio::new(-11, 1));
        assert_eq!(ordinals[4], num::rational::Ratio::new(13, 1));

        // Check the winding factor
        approxim::assert_abs_diff_eq!(0.9598, winding.winding_factor(1, 1.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.2176, winding.winding_factor(1, -5.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.1774, winding.winding_factor(1, 7.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.1774, winding.winding_factor(1, -11.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.2176, winding.winding_factor(1, 13.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.9598, winding.winding_factor(1, -17.0), epsilon = 0.0001);
    }

    {
        let winding =
            WindingDistributed::new_minimal(36, 2, 3, 2, 1, 0, WindingTableMethod::Tingley).unwrap();

        // Each coil must go from the upper to the lower layer and have a span of 8
        for coil in winding.coils() {
            match coil {
                Coil::Full(coil) => {
                    assert_ne!(coil.positive_zone().layer, coil.negative_zone().layer);
                    assert_eq!(coil.span(winding.slots()), 8);
                }
                Coil::Half(_) => unreachable!(),
            }
        }

        // Check the winding factor
        let [w_d, w_q] = winding.distribution_and_pitch_factor(1, 1.0);
        approxim::assert_abs_diff_eq!(0.9598, w_d, epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.9848, w_q, epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.9452, winding.winding_factor(1, 1.0), epsilon = 0.0001);

        // ====================================================================================
        let winding =
            WindingDistributed::new_minimal(36, 4, 3, 2, 0, 0, WindingTableMethod::Tingley).unwrap();
        approxim::assert_abs_diff_eq!(0.9452, winding.winding_factor(1, 1.0), epsilon = 0.0001);
    }

    {
        // This fails because the number of parallel paths is not possible
        let failed_winding = WindingDistributed::new(
            36,
            2,
            3,
            2,
            0,
            0,
            1,
            3,
            Connection::Star,
            0.0,
            Box::new(RoundWire::default()),
            WindingTableMethod::Tingley,
            false,
        );
        assert!(failed_winding.is_err())
    }
}

#[test]
fn test_failed_creation() {
    assert!(WindingDistributed::new_minimal(18, 5, 3, 1, 0, 0, WindingTableMethod::Tingley).is_err());
}

#[test]
fn test_air_gap_leakage_factor() {
    let winding =
        WindingDistributed::new_minimal(18, 2, 3, 2, 0, 0, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(0.045589, winding.air_gap_leakage_factor(), epsilon = 0.0001);

    let winding =
        WindingDistributed::new_minimal(18, 2, 3, 1, 0, 0, WindingTableMethod::CoilSide).unwrap();
    approxim::assert_abs_diff_eq!(0.181971, winding.air_gap_leakage_factor(), epsilon = 0.0001);

    let winding =
        WindingDistributed::new_minimal(36, 2, 3, 2, 0, 0, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(0.014061, winding.air_gap_leakage_factor(), epsilon = 0.0001);

    let winding =
        WindingDistributed::new_minimal(36, 2, 3, 2, 1, 0, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(0.011494, winding.air_gap_leakage_factor(), epsilon = 0.0001);

    let winding =
        WindingDistributed::new_minimal(36, 2, 3, 2, 2, 0, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(0.011090, winding.air_gap_leakage_factor(), epsilon = 0.0001);

    let winding =
        WindingDistributed::new_minimal(36, 2, 3, 2, 0, 1, WindingTableMethod::Tingley).unwrap();
    approxim::assert_abs_diff_eq!(0.011494, winding.air_gap_leakage_factor(), epsilon = 0.0001);
}

#[test]
fn test_turns_per_phase() {
    let winding = WindingDistributed::new(
        36,
        2,
        3,
        2,
        0,
        0,
        31,
        1,
        Connection::Star,
        0.0,
        Box::new(RoundWire::default()),
        WindingTableMethod::Tingley,
        false,
    )
    .unwrap();

    assert_eq!(winding.turns_in_slot(0), 62);
    assert_eq!(winding.turns_per_phase(1).numer().clone(), 372);

    let winding = WindingDistributed::new(
        36,
        2,
        3,
        1,
        0,
        0,
        31,
        1,
        Connection::Star,
        0.0,
        Box::new(RoundWire::default()),
        WindingTableMethod::Tingley,
        false,
    )
    .unwrap();

    assert_eq!(winding.turns_in_slot(0), 31);
    assert_eq!(winding.turns_per_phase(1).numer().clone(), 186);
}

// Create windings with a doubled zone span
#[test]
fn test_doubled_zone_span() {
    let failed_winding_initialization = WindingDistributed::new_with_doubled_zone_span(
        18,
        2,
        3,
        0,
        1,
        1,
        Connection::Star,
        0.0,
        Box::new(RoundWire::default()),
        WindingTableMethod::Tingley,
        false,
    );
    assert!(failed_winding_initialization.is_err());

    let winding = WindingDistributed::new_with_doubled_zone_span(
        18,
        1,
        3,
        0,
        1,
        1,
        Connection::Star,
        0.0,
        Box::new(RoundWire::default()),
        WindingTableMethod::Tingley,
        false,
    )
    .unwrap();

    // Compare the zone plan
    let winding_table = winding.winding_table(false);
    let expected_result = WindingTable(DMatrix::from_row_slice(
        2,
        18,
        &[
            1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, -2, -2, -2, -3, -3, -3, -3, -3,
            -3, -1, -1, -1, -1, -1, -1, -2, -2, -2,
        ],
    ));
    assert_eq!(winding_table, expected_result);

    // Check the winding ordinals
    let ordinals: Vec<num::rational::Ratio<i32>> = winding.harmonic_ordinals().take(5).collect();
    assert_eq!(ordinals[0], num::rational::Ratio::new_raw(1, 1));
    assert_eq!(ordinals[1], num::rational::Ratio::new_raw(-5, 1));
    assert_eq!(ordinals[2], num::rational::Ratio::new_raw(7, 1));
    assert_eq!(ordinals[3], num::rational::Ratio::new_raw(-11, 1));
    assert_eq!(ordinals[4], num::rational::Ratio::new_raw(13, 1));

    // Check the winding factor
    approxim::assert_abs_diff_eq!(0.8312, winding.winding_factor(1, 1.0), epsilon = 0.0001);
}

#[test]
fn test_derive_coil_assembly() {
    let winding =
        WindingDistributed::new_minimal(18, 4, 3, 1, 0, 0, WindingTableMethod::CoilSide).unwrap();
    let mut coil_assembly = CoilAssembly::from(&winding);

    assert_eq!(coil_assembly.phases(), winding.phases());
    assert_eq!(coil_assembly.slots(), winding.slots());
    assert_eq!(coil_assembly.layers(), winding.layers());

    let coils: Vec<Coil> = coil_assembly.coils().cloned().collect();
    assert_eq!(coils.len(), 9);

    // Compare the zone plans
    assert_eq!(winding.winding_table(true), coil_assembly.winding_table(true));

    // Compare the winding factor of phase 1
    approxim::assert_abs_diff_eq!(
        coil_assembly.winding_factor(1, 1.0),
        winding.winding_factor(1, 1.0),
        epsilon = 0.0001
    );

    // Now remove the first coil (phase 1)
    let removed_coil = coil_assembly.remove(Zone::new(0, 0)).unwrap();
    let removed_coil_full = match &removed_coil {
        Coil::Full(coil) => coil,
        Coil::Half(_) => panic!("Test failed"),
    };
    let coils: Vec<Coil> = coil_assembly.coils().cloned().collect();
    assert_eq!(coils.len(), 8);

    // Evaluate the coil
    assert_eq!(removed_coil_full.phase(), 1);
    assert_eq!(removed_coil_full.turns(), 1);
    assert_eq!(removed_coil_full.positive_zone(), Zone::new(0, 0));
    assert_eq!(removed_coil_full.negative_zone(), Zone::new(2, 0));

    // Check that the winding factor of phase 2 is still identical for both windings
    approxim::assert_abs_diff_eq!(
        coil_assembly.winding_factor(2, 1.0),
        winding.winding_factor(2, 1.0),
        epsilon = 0.0001
    );

    // Check that the winding factor of phase 1 has changed
    approxim::assert_abs_diff_ne!(
        coil_assembly.winding_factor(1, 1.0),
        winding.winding_factor(1, 1.0),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        coil_assembly.winding_factor(1, 1.0),
        0.9254,
        epsilon = 0.0001
    );

    // Push the coil again to the coil motor and assert that the winding factor of phase 1 is now identical again.
    coil_assembly.insert(removed_coil.clone()).unwrap();
    approxim::assert_abs_diff_eq!(
        coil_assembly.winding_factor(1, 1.0),
        winding.winding_factor(1, 1.0),
        epsilon = 0.0001
    );

    // Assert that pushing the removed coil again fails.
    assert!(coil_assembly.insert(removed_coil.clone()).is_err());
}

#[test]
fn test_line_to_phase_voltage() {
    // Star
    let winding = WindingDistributed::new(
        18,
        2,
        3,
        2,
        0,
        0,
        1,
        1,
        Connection::Star,
        0.0,
        Box::new(RoundWire::default()),
        WindingTableMethod::Tingley,
        false,
    )
    .unwrap();
    approxim::assert_abs_diff_eq!(
        0.57735,
        winding.line_to_phase_voltage().norm(),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(0.5, winding.line_to_phase_voltage().re, epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(
        0.288675,
        winding.line_to_phase_voltage().im,
        epsilon = 0.0001
    );

    // Delta
    let winding = WindingDistributed::new(
        18,
        2,
        3,
        2,
        0,
        0,
        1,
        1,
        Connection::Delta,
        0.0,
        Box::new(RoundWire::default()),
        WindingTableMethod::Tingley,
        false,
    )
    .unwrap();
    approxim::assert_abs_diff_eq!(
        1.0,
        winding.line_to_phase_voltage().norm(),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(1.0, winding.line_to_phase_voltage().re, epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(0.0, winding.line_to_phase_voltage().im, epsilon = 0.0001);
}

fn create_core_rect() -> CoreRot {
    let opening_height = Length::new::<millimeter>(2.0);
    let opening_width = Length::new::<millimeter>(3.0);
    let width = Length::new::<millimeter>(3.0);
    let height = Length::new::<millimeter>(20.0);
    let slot =
        SlotRectangular::new(width, opening_width, height, opening_height, true, false).unwrap();

    return CoreRotBuilder {
        air_gap_radius: Length::new::<millimeter>(55.0),
        yoke_radius: Length::new::<millimeter>(85.0),
        axial_length: Length::new::<millimeter>(165.0),
        axial_coil_overhang: Length::new::<millimeter>(0.0),
        iron_fill_factor: 0.95,
        material: Arc::new(Material::default()),
        pole_pairs: 2,
        skew_angle: 0.0,
        air_gap: Box::new(AirGapSlotted {
            slots: 36,
            starts_in_slot_middle: true,
            carter_factor_model: CarterFactorModel::Bin12,
            slot: Box::new(slot),
        }),
        flux_barrier: None,
    }
    .try_into()
    .expect("valid magnetic core");
}

fn create_core_trap() -> CoreRot {
    let slot = SlotTrapezoidSemi::new(
        Length::new::<millimeter>(9.21),
        Length::new::<millimeter>(6.2353854401185835),
        Length::new::<millimeter>(2.0),
        Length::new::<millimeter>(17.75),
        Length::new::<millimeter>(17.0),
        Length::new::<millimeter>(0.75),
        0.17453292519943295,
        1.4835298641951802,
        1.658062789394613,
        Length::new::<millimeter>(0.5),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(0.0),
        true,
    )
    .unwrap();
    return CoreRotBuilder {
        air_gap_radius: Length::new::<millimeter>(55.0),
        yoke_radius: Length::new::<millimeter>(85.0),
        axial_length: Length::new::<millimeter>(165.0),
        axial_coil_overhang: Length::new::<millimeter>(0.0),
        iron_fill_factor: 0.95,
        material: Arc::new(Material::default()),
        pole_pairs: 2,
        skew_angle: 0.0,
        air_gap: Box::new(AirGapSlotted {
            slots: 36,
            starts_in_slot_middle: true,
            carter_factor_model: CarterFactorModel::Bin12,
            slot: Box::new(slot),
        }),
        flux_barrier: None,
    }
    .try_into()
    .expect("valid magnetic core");
}

fn create_skewed_core() -> CoreRot {
    let slot = SlotTrapezoidSemi::new(
        Length::new::<millimeter>(9.21),
        Length::new::<millimeter>(6.2353854401185835),
        Length::new::<millimeter>(2.0),
        Length::new::<millimeter>(17.75),
        Length::new::<millimeter>(17.0),
        Length::new::<millimeter>(0.75),
        0.17453292519943295,
        1.4835298641951802,
        1.658062789394613,
        Length::new::<millimeter>(0.5),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(0.0),
        true,
    )
    .unwrap();

    return CoreRotBuilder {
        air_gap_radius: Length::new::<millimeter>(55.0),
        yoke_radius: Length::new::<millimeter>(85.0),
        axial_length: Length::new::<millimeter>(165.0),
        axial_coil_overhang: Length::new::<millimeter>(0.0),
        iron_fill_factor: 0.95,
        material: Arc::new(Material::default()),
        pole_pairs: 2,
        skew_angle: 10.0 / 180.0 * PI,
        air_gap: Box::new(AirGapSlotted {
            slots: 36,
            starts_in_slot_middle: true,
            carter_factor_model: CarterFactorModel::Bin12,
            slot: Box::new(slot),
        }),
        flux_barrier: None,
    }
    .try_into()
    .expect("valid magnetic core");
}

#[test]
fn test_distributed_winding_364_sl() {
    let core = create_core_trap();

    let copper: Material = create_dbm().read("Copper").unwrap();
    let copper_arc = Arc::new(copper);
    let wire_1 = RoundWire::new(
        copper_arc.clone(),
        Length::new::<millimeter>(0.67),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(0.0),
    )
    .unwrap();
    let wire_2 = RoundWire::new(
        copper_arc.clone(),
        Length::new::<millimeter>(0.71),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(0.0),
    )
    .unwrap();

    let strand_list = vec![
        WireGroup::new(Box::new(wire_1), 2),
        WireGroup::new(Box::new(wire_2), 3),
    ];
    let wire = StrandedWire::new(strand_list).unwrap();
    let winding = WindingDistributed::new(
        36,
        2,
        3,
        1,
        0,
        0,
        31,
        1,
        Connection::Star,
        0.25,
        Box::new(wire.clone()),
        WindingTableMethod::Tingley,
        false,
    )
    .unwrap();

    // Check the coil turn length in the core
    approxim::assert_abs_diff_eq!(
        core.axial_coil_length().get::<meter>(),
        0.165, // Expected value in m
        epsilon = 0.0001
    );

    // Check the mean end winding length
    approxim::assert_abs_diff_eq!(
        winding
            .end_winding_half_turn_length(
                core.as_lin_or_rot(),
                Zone::new(0, 0),
                &Default::default(),
            )
            .unwrap()
            .get::<meter>(),
        0.15757, // Expected value in m
        epsilon = 0.0001
    );

    // Check the wire volume
    approxim::assert_abs_diff_eq!(
        winding
            .coil_properties(core.as_lin_or_rot(), &Default::default())
            .map(|cp| cp.volume())
            .sum::<Volume>()
            .get::<cubic_millimeter>(),
        681491.68774, // Expected value in m
        epsilon = 0.0001
    );

    // Check the phase resistance
    approxim::assert_abs_diff_eq!(
        winding
            .resistance(1, core.as_lin_or_rot(), &[], &Default::default(),)
            .get::<ohm>(),
        1.13214, // Expected value in Ohm
        epsilon = 0.0001
    );

    // Slot leakage inductance
    approxim::assert_abs_diff_eq!(
        winding
            .slot_leakage_inductance(
                1,
                core.as_lin_or_rot(),
                Length::new::<millimeter>(1.0),
                &[],
                &Default::default(),
            )
            .get::<henry>(),
        0.0034017, // Expected value in H
        epsilon = 1e-6
    );

    approxim::assert_abs_diff_eq!(
        winding
            .end_winding_leakage_inductance(1, core.as_lin_or_rot(), &Default::default())
            .get::<henry>(),
        0.0017129, // Expected value in H
        epsilon = 1e-6
    );

    let winding_identical_to_dl = WindingDistributed::new(
        36,
        2,
        3,
        1,
        0,
        0,
        62,
        1,
        Connection::Star,
        0.25,
        Box::new(wire.clone()),
        WindingTableMethod::Tingley,
        false,
    )
    .unwrap();

    // Slot leakage inductance
    approxim::assert_abs_diff_eq!(
        winding_identical_to_dl
            .slot_leakage_inductance(
                1,
                core.as_lin_or_rot(),
                Length::new::<millimeter>(1.0),
                &[],
                &Default::default(),
            )
            .get::<henry>(),
        0.0136079, // Expected value in H
        epsilon = 1e-6
    );
}

#[test]
fn test_slot_leakage_inductance() {
    fn create_winding(coil_span_reduction: i32) -> WindingDistributed {
        let copper: Material = create_dbm().read("Copper").unwrap();
        let wire = RoundWire::new(
            Arc::new(copper),
            Length::new::<millimeter>(0.67),
            Length::new::<millimeter>(0.0),
            Length::new::<millimeter>(0.0),
        )
        .unwrap();
        return WindingDistributed::new(
            36,
            2,
            3,
            2,
            coil_span_reduction,
            0,
            10,
            1,
            Connection::Star,
            0.25,
            Box::new(wire),
            WindingTableMethod::Tingley,
            false,
        )
        .unwrap();
    }

    fn resulting_leakage_coeff(
        lambda_l: f64,
        lambda_res: f64,
        winding: &WindingDistributed,
    ) -> f64 {
        let span = winding.pole_pitch() - winding.coil_span_reduction() as f64;
        return (1.0 - 9.0 / 16.0 * (1.0 - span as f64 / winding.pole_pitch())) * lambda_l
            + (1.0 - 3.0 / 4.0 * (1.0 - span as f64 / winding.pole_pitch())) * lambda_res;
    }

    let core = create_core_rect();

    // Compare with the analytical calculation from from [MVP08]
    let air_gap = Length::new::<millimeter>(1.0);
    let width = core.slot().unwrap().bottom_width();
    let width_opening = core.slot().unwrap().opening_width();
    let height = core.slot().unwrap().height();
    let height_opening = core.slot().unwrap().opening_height();
    let height_layer = height - height_opening;

    let lambda_l = f64::from(height_layer / (3.0 * width));
    let lambda_res = f64::from(height_opening / width_opening)
        + core.slot().unwrap().leakage_coefficient_tooth_tip(air_gap); // eq. (3.7.2a)

    // ===============================
    // No short pitching

    let winding = create_winding(0);

    // Slot leakage inductance from [MVP08], eq. (3.7.15)
    let leakage_inductance = 2.0
        * *material::VACUUM_PERMEABILITY
        * core.axial_coil_length()
        * winding.turns_per_phase(1).to_integer().pow(2) as f64
        / (winding.pole_pairs() as f64 * winding.hole_number_float())
        * resulting_leakage_coeff(lambda_l, lambda_res, &winding);

    approxim::assert_abs_diff_eq!(
        leakage_inductance.get::<henry>(),
        0.0026540, // Expected value in H
        epsilon = 1e-7
    );
    approxim::assert_abs_diff_eq!(
        leakage_inductance.get::<henry>(),
        winding
            .slot_leakage_inductance(1, core.as_lin_or_rot(), air_gap, &[], &Default::default(),)
            .get::<henry>(),
        epsilon = 1e-7
    );

    // ===============================
    // Short pitching by one slot

    let winding = create_winding(1);

    // Slot leakage inductance from [MVP08], eq. (3.7.15)
    let leakage_inductance = 2.0
        * *material::VACUUM_PERMEABILITY
        * core.axial_coil_length()
        * winding.turns_per_phase(1).to_integer().pow(2) as f64
        / (winding.pole_pairs() as f64 * winding.hole_number_float())
        * resulting_leakage_coeff(lambda_l, lambda_res, &winding);

    approxim::assert_abs_diff_eq!(
        leakage_inductance.get::<henry>(),
        0.0024743, // Expected value in H
        epsilon = 1e-7
    );
    approxim::assert_abs_diff_eq!(
        leakage_inductance.get::<henry>(),
        winding
            .slot_leakage_inductance(1, core.as_lin_or_rot(), air_gap, &[], &Default::default(),)
            .get::<henry>(),
        epsilon = 1e-7
    );

    // ===============================
    // Short pitching by two slots

    let winding = create_winding(2);

    // Slot leakage inductance from [MVP08], eq. (3.7.15)
    let leakage_inductance = 2.0
        * *material::VACUUM_PERMEABILITY
        * core.axial_coil_length()
        * winding.turns_per_phase(1).to_integer().pow(2) as f64
        / (winding.pole_pairs() as f64 * winding.hole_number_float())
        * resulting_leakage_coeff(lambda_l, lambda_res, &winding);

    approxim::assert_abs_diff_eq!(
        leakage_inductance.get::<henry>(),
        0.0022946, // Expected value in H
        epsilon = 1e-7
    );
    approxim::assert_abs_diff_eq!(
        leakage_inductance.get::<henry>(),
        winding
            .slot_leakage_inductance(1, core.as_lin_or_rot(), air_gap, &[], &Default::default(),)
            .get::<henry>(),
        epsilon = 1e-7
    );

    // ===============================
    // Short pitching by three slots

    let winding = create_winding(3);

    // Slot leakage inductance from [MVP08], eq. (3.7.15)
    let leakage_inductance = 2.0
        * *material::VACUUM_PERMEABILITY
        * core.axial_coil_length()
        * winding.turns_per_phase(1).to_integer().pow(2) as f64
        / (winding.pole_pairs() as f64 * winding.hole_number_float())
        * resulting_leakage_coeff(lambda_l, lambda_res, &winding);

    approxim::assert_abs_diff_eq!(
        leakage_inductance.get::<henry>(),
        0.0021149, // Expected value in H
        epsilon = 1e-7
    );
    approxim::assert_abs_diff_eq!(
        leakage_inductance.get::<henry>(),
        winding
            .slot_leakage_inductance(1, core.as_lin_or_rot(), air_gap, &[], &Default::default(),)
            .get::<henry>(),
        epsilon = 1e-7
    );
}

#[test]
fn test_distributed_winding_364_dl() {
    let core = create_core_trap();

    let copper: Material = create_dbm().read("Copper").unwrap();
    let copper_arc = Arc::new(copper);
    let wire_1 = RoundWire::new(
        copper_arc.clone(),
        Length::new::<millimeter>(0.67),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(0.0),
    )
    .unwrap();
    let wire_2 = RoundWire::new(
        copper_arc.clone(),
        Length::new::<millimeter>(0.71),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(0.0),
    )
    .unwrap();

    let strand_list = vec![
        WireGroup::new(Box::new(wire_1), 2),
        WireGroup::new(Box::new(wire_2), 3),
    ];
    let wire = StrandedWire::new(strand_list).unwrap();
    let winding = WindingDistributed::new(
        36,
        2,
        3,
        2,
        0,
        0,
        31,
        1,
        Connection::Star,
        0.25,
        Box::new(wire),
        WindingTableMethod::Tingley,
        false,
    )
    .unwrap();

    // Check the coil turn length in the core
    approxim::assert_abs_diff_eq!(
        core.axial_coil_length().get::<meter>(),
        0.165, // Expected value in m
        epsilon = 0.0001
    );

    // Check the mean end winding length
    approxim::assert_abs_diff_eq!(
        winding
            .end_winding_half_turn_length(
                core.as_lin_or_rot(),
                Zone::new(0, 0),
                &Default::default(),
            )
            .unwrap()
            .get::<meter>(),
        0.15757, // Expected value in m
        epsilon = 0.0001
    );

    // Check the phase resistance
    approxim::assert_abs_diff_eq!(
        winding
            .resistance(1, core.as_lin_or_rot(), &[], &Default::default(),)
            .get::<ohm>(),
        2.26429, // Expected value in Ohm
        epsilon = 0.0001
    );

    // Slot leakage inductance
    approxim::assert_abs_diff_eq!(
        winding
            .slot_leakage_inductance(
                1,
                core.as_lin_or_rot(),
                Length::new::<millimeter>(1.0),
                &[],
                &Default::default(),
            )
            .get::<henry>(),
        0.0134337, // Expected value in H
        epsilon = 1e-6
    );
}

#[test]
fn test_distributed_winding_364_sl_skewed() {
    let core = create_skewed_core();

    let copper: Material = create_dbm().read("Copper").unwrap();
    let copper_arc = Arc::new(copper);
    let wire_1 = RoundWire::new(
        copper_arc.clone(),
        Length::new::<millimeter>(0.67),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(0.0),
    )
    .unwrap();
    let wire_2 = RoundWire::new(
        copper_arc.clone(),
        Length::new::<millimeter>(0.71),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(0.0),
    )
    .unwrap();

    let strand_list = vec![
        WireGroup::new(Box::new(wire_1), 2),
        WireGroup::new(Box::new(wire_2), 3),
    ];
    let wire = StrandedWire::new(strand_list).unwrap();
    let winding = WindingDistributed::new(
        36,
        2,
        3,
        2,
        0,
        0,
        31,
        1,
        Connection::Star,
        0.25,
        Box::new(wire),
        WindingTableMethod::Tingley,
        false,
    )
    .unwrap();

    // Check the coil turn length in the core
    approxim::assert_abs_diff_eq!(
        core.axial_coil_length().get::<meter>(),
        0.165 / (10.0 / 180.0 * PI).cos(), // Expected value in m
        epsilon = 0.0001
    );

    // Check the mean end winding length
    approxim::assert_abs_diff_eq!(
        winding
            .end_winding_half_turn_length(
                core.as_lin_or_rot(),
                Zone::new(0, 0),
                &Default::default(),
            )
            .unwrap()
            .get::<meter>(),
        0.15757, // Expected value in m
        epsilon = 0.0001
    );

    // Check the phase resistance
    approxim::assert_abs_diff_eq!(
        winding
            .resistance(1, core.as_lin_or_rot(), &[], &Default::default(),)
            .get::<ohm>(),
        2.28215, // Expected value in Ohm
        epsilon = 0.0001
    );
}

#[test]
fn test_distributed_winding_364_dl_short_pitched() {
    let core = create_core_trap();

    let copper: Material = create_dbm().read("Copper").unwrap();
    let copper_arc = Arc::new(copper);
    let wire_1 = RoundWire::new(
        copper_arc.clone(),
        Length::new::<millimeter>(0.67),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(0.0),
    )
    .unwrap();
    let wire_2 = RoundWire::new(
        copper_arc.clone(),
        Length::new::<millimeter>(0.71),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(0.0),
    )
    .unwrap();

    let strand_list = vec![
        WireGroup::new(Box::new(wire_1), 2),
        WireGroup::new(Box::new(wire_2), 3),
    ];
    let wire = StrandedWire::new(strand_list).unwrap();
    let winding = WindingDistributed::new(
        36,
        2,
        3,
        2,
        1,
        0,
        31,
        1,
        Connection::Star,
        0.25,
        Box::new(wire),
        WindingTableMethod::Tingley,
        false,
    )
    .unwrap();

    // Check the mean end winding length
    approxim::assert_abs_diff_eq!(
        winding
            .end_winding_half_turn_length(
                core.as_lin_or_rot(),
                Zone::new(0, 0),
                &Default::default(),
            )
            .unwrap()
            .get::<meter>(),
        0.14009, // Expected value in m
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        winding
            .slot_leakage_inductance(
                1,
                core.as_lin_or_rot(),
                Length::new::<millimeter>(1.0),
                &[],
                &Default::default(),
            )
            .get::<henry>(),
        0.0124574, // Expected value in H
        epsilon = 1e-6
    );
}

#[test]
fn test_harmonic_ordinal_and_amplitude() {
    {
        let winding = WindingDistributed::new(
            18,
            1,
            3,
            2,
            2,
            0,
            1,
            1,
            Connection::Star,
            0.25,
            Box::new(RoundWire::default()),
            WindingTableMethod::Tingley,
            false,
        )
        .unwrap();

        let mut iter = winding.harmonic_inductions();

        {
            let (ratio, amp) = iter.next().unwrap();
            assert_eq!(ratio, num::rational::Ratio::new(1, 1));
            approxim::assert_abs_diff_eq!(amp, 0.901912, epsilon = 1e-6);
        }

        {
            let (ratio, amp) = iter.next().unwrap();
            assert_eq!(ratio, num::rational::Ratio::new(-5, 1));
            approxim::assert_abs_diff_eq!(amp, 0.007556, epsilon = 1e-6);
        }

        {
            let (ratio, amp) = iter.next().unwrap();
            assert_eq!(ratio, num::rational::Ratio::new(7, 1));
            approxim::assert_abs_diff_eq!(amp, 0.019409, epsilon = 1e-6);
        }

        {
            let (ratio, amp) = iter.next().unwrap();
            assert_eq!(ratio, num::rational::Ratio::new(-11, 1));
            approxim::assert_abs_diff_eq!(amp, 0.0123516, epsilon = 1e-6);
        }

        {
            let (ratio, amp) = iter.next().unwrap();
            assert_eq!(ratio, num::rational::Ratio::new(13, 1));
            approxim::assert_abs_diff_eq!(amp, 0.002906, epsilon = 1e-6);
        }

        {
            let (ratio, amp) = iter.next().unwrap();
            assert_eq!(ratio, num::rational::Ratio::new(-17, 1));
            approxim::assert_abs_diff_eq!(amp, 0.0530536, epsilon = 1e-6);
        }

        {
            let (ratio, amp) = iter.next().unwrap();
            assert_eq!(ratio, num::rational::Ratio::new(19, 1));
            approxim::assert_abs_diff_eq!(amp, 0.0474696, epsilon = 1e-6);
        }
    }
}
