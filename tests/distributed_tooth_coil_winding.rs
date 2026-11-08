use std::num::{NonZeroU16, NonZeroUsize};

use approxim;
use stem_winding::{prelude::*, winding::WindingHarmonic};

const ONE: NonZeroU16 = NonZeroU16::MIN;

#[test]
fn test_number_of_turns() {
    // Calculate with 1 pole pair as working harmonic (one basic winding)
    assert!(
        DistributedToothCoilWinding::try_from(DistributedToothCoilMinimalBuilder {
            base_winding_count: 1.try_into().expect("not zero"),
            pole_pairs: 2.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_group_turns: Vec::new(),
            double_zone_span: true,
        })
        .is_err()
    );
    assert!(
        DistributedToothCoilWinding::try_from(DistributedToothCoilMinimalBuilder {
            base_winding_count: 1.try_into().expect("not zero"),
            pole_pairs: 2.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_group_turns: vec![NonZeroUsize::new(1).expect("not zero")],
            double_zone_span: true,
        })
        .is_ok()
    );
    assert!(
        DistributedToothCoilWinding::try_from(DistributedToothCoilMinimalBuilder {
            base_winding_count: 1.try_into().expect("not zero"),
            pole_pairs: 2.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_group_turns: vec![
                NonZeroUsize::new(1).expect("not zero"),
                NonZeroUsize::new(1).expect("not zero")
            ],
            double_zone_span: true,
        })
        .is_ok()
    );
}

#[test]
fn test_coil_analysis() {
    {
        let winding = DistributedToothCoilWinding::try_from(DistributedToothCoilMinimalBuilder {
            base_winding_count: 1.try_into().expect("not zero"),
            pole_pairs: 2.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_group_turns: vec![
                NonZeroUsize::new(1).expect("not zero"),
                NonZeroUsize::new(1).expect("not zero"),
            ],
            double_zone_span: true,
        })
        .unwrap();
        for coil in winding.coils_iter() {
            if let Coil::Full(coil) = coil {
                let throw = coil.throw(Some(winding.slots()));
                assert!(throw == 1 || throw == 3); // Coil throw is either 1 or 3

                // All coils of this winding are clockwise
                assert!(coil.positive_slot_direction());
            }
        }
    }
}

#[test]
fn test_three_zones_single_layer() {
    let winding_table_expected = WindingTable::from_layer_major(
        [1, 1, -1, -1, 2, 2, -2, -2, 3, 3, -3, -3].into_iter(),
        NonZeroU16::new(12).expect("not zero"),
        NonZeroU16::new(1).expect("not zero"),
    );

    {
        // Calculate with 1 pole pair as working harmonic (one basic winding)
        let winding = DistributedToothCoilWinding::new(DistributedToothCoilMinimalBuilder {
            base_winding_count: 1.try_into().expect("not zero"),
            pole_pairs: 2.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_group_turns: vec![
                NonZeroUsize::new(1).expect("not zero"),
                NonZeroUsize::new(1).expect("not zero"),
            ],
            double_zone_span: true,
        })
        .unwrap();

        assert_eq!(winding.base_winding_count().get(), 1);

        // Compare the zone plan
        let winding_table = winding.winding_table(false);
        assert_eq!(winding_table, winding_table_expected);

        // Check the winding factor
        approxim::assert_abs_diff_eq!(
            0.4830,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(1)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.75,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(2)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.4330,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(4)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.1294,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(5)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.1294,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(7)),
            epsilon = 0.0001
        );
    }

    {
        // Calculate with 2 pole pairs as working harmonic (two basic windings)
        let winding = DistributedToothCoilWinding::new(DistributedToothCoilMinimalBuilder {
            base_winding_count: 2.try_into().expect("not zero"),
            pole_pairs: 4.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_group_turns: vec![
                NonZeroUsize::new(1).expect("not zero"),
                NonZeroUsize::new(1).expect("not zero"),
            ],
            double_zone_span: true,
        })
        .unwrap();

        assert_eq!(winding.base_winding_count().get(), 2);

        // Compare the zone plan
        let winding_table = winding.winding_table(false);
        assert_eq!(winding_table, winding_table_expected);

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
                spatial_order: SpatialOrder::Mechanical(4),
                is_positive: false
            }
        );
        assert_eq!(
            harmonics[2],
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(8),
                is_positive: true
            }
        );
        assert_eq!(
            harmonics[3],
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(10),
                is_positive: false
            }
        );
        assert_eq!(
            harmonics[4],
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(14),
                is_positive: true
            }
        );

        // Check the winding factor
        approxim::assert_abs_diff_eq!(
            0.4830,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(2)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.75,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(4)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.4330,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(8)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.1294,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(10)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.1294,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(14)),
            epsilon = 0.0001
        );
    }

    {
        // Calculate with 4 pole pairs as working harmonic (two basic windings)
        let winding = DistributedToothCoilWinding::new(DistributedToothCoilMinimalBuilder {
            base_winding_count: 2.try_into().expect("not zero"),
            pole_pairs: 8.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_group_turns: vec![
                NonZeroUsize::new(1).expect("not zero"),
                NonZeroUsize::new(1).expect("not zero"),
            ],
            double_zone_span: true,
        })
        .unwrap();

        assert_eq!(winding.base_winding_count().get(), 2);

        // Compare the zone plan
        let winding_table = winding.winding_table(false);
        assert_eq!(winding_table, winding_table_expected);

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
                spatial_order: SpatialOrder::Mechanical(4),
                is_positive: false
            }
        );
        assert_eq!(
            harmonics[2],
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(8),
                is_positive: true
            }
        );
        assert_eq!(
            harmonics[3],
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(10),
                is_positive: false
            }
        );
        assert_eq!(
            harmonics[4],
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(14),
                is_positive: true
            }
        );

        // Check the winding factor
        approxim::assert_abs_diff_eq!(
            0.4830,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(2)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.75,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(4)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.4330,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(8)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.1294,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(10)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.1294,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(14)),
            epsilon = 0.0001
        );
    }
}

#[test]
fn test_three_zones_single_layer_differing_number_of_coils() {
    let mut coil_group_turns: Vec<NonZeroUsize> = Vec::with_capacity(2);
    for coil_turns in 2..4 {
        coil_group_turns.push(coil_turns.try_into().unwrap());
    }

    let winding = DistributedToothCoilWinding::new(DistributedToothCoilMinimalBuilder {
        base_winding_count: 2.try_into().expect("not zero"),
        pole_pairs: 4.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 1.try_into().expect("not zero"),
        coil_group_turns,
        double_zone_span: true,
    })
    .unwrap();

    // Check the winding factor
    approxim::assert_abs_diff_eq!(
        0.5278,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(2)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.8,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(4)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.3464,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(8)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.0379,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(10)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.0379,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(14)),
        epsilon = 0.0001
    );
}

#[test]
fn test_six_zones_single_layer() {
    let winding_table_expected = WindingTable::from_layer_major(
        [
            1, 1, -1, -1, -3, -3, 3, 3, 2, 2, -2, -2, -1, -1, 1, 1, 3, 3, -3, -3, -2, -2, 2, 2,
        ]
        .into_iter(),
        NonZeroU16::new(24).expect("not zero"),
        NonZeroU16::new(1).expect("not zero"),
    );

    {
        // Calculate with 2 pole pairs as working harmonic (two basic windings)
        let winding = DistributedToothCoilWinding::try_from(DistributedToothCoilMinimalBuilder {
            base_winding_count: 2.try_into().expect("not zero"),
            pole_pairs: 10.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_group_turns: vec![
                NonZeroUsize::new(1).expect("not zero"),
                NonZeroUsize::new(1).expect("not zero"),
            ],
            double_zone_span: false,
        })
        .unwrap();

        assert_eq!(winding.base_winding_count().get(), 2);

        // Compare the zone plan
        let winding_table = winding.winding_table(false);
        assert_eq!(winding_table, winding_table_expected);

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
            0.2566,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(2)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.7663,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(10)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.5880,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(14)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.0338,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(22)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.0338,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(26)),
            epsilon = 0.0001
        );
    }

    {
        // Calculate with 7 pole pairs as working harmonic (two basic windings)
        let winding = DistributedToothCoilWinding::new(DistributedToothCoilMinimalBuilder {
            base_winding_count: 2.try_into().expect("not zero"),
            pole_pairs: 14.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_group_turns: vec![
                NonZeroUsize::new(1).expect("not zero"),
                NonZeroUsize::new(1).expect("not zero"),
            ],
            double_zone_span: false,
        })
        .unwrap();

        assert_eq!(winding.base_winding_count().get(), 2);

        // Compare the zone plan
        let winding_table = winding.winding_table(false);
        assert_eq!(winding_table, winding_table_expected);

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
            0.2566,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(2)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.7663,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(10)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.5880,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(14)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.0338,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(22)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.0338,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(26)),
            epsilon = 0.0001
        );
    }
}

#[test]
fn test_single_layer_24_1() {
    {
        let coil_group_turns = DistributedToothCoilWinding::even_turn_distribution(
            2.try_into().unwrap(),
            2.try_into().expect("not zero"),
            0,
        )
        .unwrap();
        let winding = DistributedToothCoilWinding::try_from(DistributedToothCoilMinimalBuilder {
            base_winding_count: 1.try_into().expect("not zero"),
            pole_pairs: 1.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_group_turns,
            double_zone_span: false,
        })
        .unwrap();

        assert_eq!(winding.layers().get(), 1);
        assert_eq!(winding.base_winding_count().get(), 1);
    }
    {
        // This fails, since the coil_group_turns vector has not the correct length
        assert!(
            DistributedToothCoilWinding::try_from(DistributedToothCoilMinimalBuilder {
                base_winding_count: 2.try_into().expect("not zero"),
                pole_pairs: 1.try_into().expect("not zero"),
                phases: 3.try_into().expect("not zero"),
                layers: 1.try_into().expect("not zero"),
                coil_group_turns: vec![NonZeroUsize::new(1).expect("not zero")],
                double_zone_span: true,
            })
            .is_err()
        );
    }
}

#[test]
fn test_double_layer_24_2() {
    // Calculate with 1 pole pairs as working harmonic (two basic windings)
    let coil_group_turns = DistributedToothCoilWinding::even_turn_distribution(
        2.try_into().unwrap(),
        2.try_into().expect("not zero"),
        0,
    )
    .unwrap();

    let winding = DistributedToothCoilWinding::try_from(DistributedToothCoilMinimalBuilder {
        base_winding_count: 2.try_into().expect("not zero"),
        pole_pairs: 2.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 2.try_into().expect("not zero"),
        coil_group_turns,
        double_zone_span: false,
    })
    .unwrap();

    assert_eq!(winding.layers().get(), 2);
    assert_eq!(winding.base_winding_count().get(), 2);

    // Compare the zone plan
    let winding_table = winding.winding_table(false);

    let expected_result = WindingTable::from_layer_major(
        [
            1, 1, -1, -1, 2, 2, -2, -2, 3, 3, -3, -3, 2, 2, -3, -3, 3, 3, -1, -1, 1, 1, -2, -2,
        ]
        .into_iter(),
        NonZeroU16::new(12).expect("not zero"),
        NonZeroU16::new(2).expect("not zero"),
    );
    assert_eq!(winding_table, expected_result);

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
        0.4830,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(2)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.1294,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(10)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.1294,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(14)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.4830,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(22)),
        epsilon = 0.0001
    );
    approxim::assert_abs_diff_eq!(
        0.4830,
        winding.winding_factor(ONE, SpatialOrder::Mechanical(26)),
        epsilon = 0.0001
    );
}

#[test]
fn test_double_layer_differing_number_of_coils_24_2_dl() {
    {
        // Calculate with 2 pole pairs as working harmonic (one basic winding)
        let coil_group_turns = DistributedToothCoilWinding::even_turn_distribution(
            2.try_into().unwrap(),
            200.try_into().expect("not zero"),
            15,
        )
        .unwrap();

        let winding = DistributedToothCoilWinding::try_from(DistributedToothCoilMinimalBuilder {
            base_winding_count: 1.try_into().expect("not zero"),
            pole_pairs: 1.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            coil_group_turns,
            double_zone_span: false,
        })
        .unwrap();

        assert_eq!(winding.base_winding_count().get(), 1);

        // Check the winding factor
        approxim::assert_abs_diff_eq!(
            0.5166,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(1)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.0039,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(5)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.0039,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(7)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.5166,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(11)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.5166,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(13)),
            epsilon = 0.0001
        );
    }

    {
        // Calculate with 2 pole pairs as working harmonic (two basic windings)
        let coil_group_turns = DistributedToothCoilWinding::even_turn_distribution(
            2.try_into().unwrap(),
            200.try_into().expect("not zero"),
            15,
        )
        .unwrap();

        assert_eq!(
            coil_group_turns,
            vec![
                NonZeroUsize::new(85).expect("not zero"),
                NonZeroUsize::new(115).expect("not zero")
            ]
        );

        let winding = DistributedToothCoilWinding::try_from(DistributedToothCoilMinimalBuilder {
            base_winding_count: 2.try_into().expect("not zero"),
            pole_pairs: 2.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            coil_group_turns,
            double_zone_span: false,
        })
        .unwrap();

        assert_eq!(winding.base_winding_count().get(), 2);

        assert_eq!(winding.turns_at(Zone::new(0, 0)), 115);
        assert_eq!(winding.turns_at(Zone::new(1, 0)), 85);
        assert_eq!(winding.turns_at(Zone::new(2, 0)), 85);
        assert_eq!(winding.turns_at(Zone::new(3, 0)), 115);

        assert_eq!(winding.turns_at(Zone::new(12, 0)), 115);
        assert_eq!(winding.turns_at(Zone::new(13, 0)), 85);
        assert_eq!(winding.turns_at(Zone::new(14, 0)), 85);
        assert_eq!(winding.turns_at(Zone::new(15, 0)), 115);

        assert_eq!(winding.turns_at(Zone::new(0, 1)), 85);
        assert_eq!(winding.turns_at(Zone::new(1, 1)), 115);
        assert_eq!(winding.turns_at(Zone::new(2, 1)), 115);
        assert_eq!(winding.turns_at(Zone::new(3, 1)), 85);

        assert_eq!(winding.phase_at(Zone::new(0, 0)), 1);
        assert_eq!(winding.phase_at(Zone::new(1, 0)), 1);
        assert_eq!(winding.phase_at(Zone::new(2, 0)), -1);
        assert_eq!(winding.phase_at(Zone::new(3, 0)), -1);

        assert_eq!(winding.phase_at(Zone::new(12, 0)), 1);
        assert_eq!(winding.phase_at(Zone::new(13, 0)), 1);
        assert_eq!(winding.phase_at(Zone::new(14, 0)), -1);
        assert_eq!(winding.phase_at(Zone::new(15, 0)), -1);

        assert_eq!(winding.phase_at(Zone::new(0, 1)), 2);
        assert_eq!(winding.phase_at(Zone::new(1, 1)), 2);
        assert_eq!(winding.phase_at(Zone::new(2, 1)), -3);
        assert_eq!(winding.phase_at(Zone::new(3, 1)), -3);

        // Check the winding factor
        approxim::assert_abs_diff_eq!(
            0.5166,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(2)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.0039,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(10)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.0039,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(14)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.5166,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(22)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.5166,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(26)),
            epsilon = 0.0001
        );
    }

    {
        // Calculate with 1 pole pairs as working harmonic (two basic windings)
        let coil_group_turns = DistributedToothCoilWinding::even_turn_distribution(
            2.try_into().unwrap(),
            200.try_into().expect("not zero"),
            30,
        )
        .unwrap();
        let winding = DistributedToothCoilWinding::try_from(DistributedToothCoilMinimalBuilder {
            base_winding_count: 2.try_into().expect("not zero"),
            pole_pairs: 2.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            coil_group_turns,
            double_zone_span: false,
        })
        .unwrap();

        // Check the winding factor
        approxim::assert_abs_diff_eq!(
            0.5502,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(2)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.1215,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(10)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.1215,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(14)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.5502,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(22)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.5502,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(26)),
            epsilon = 0.0001
        );
    }
}

/**
Tests for an overflow which could occur in the harmonic orders iterator
 */
#[test]
fn test_overflow_iterator_bug() {
    let coil_group_turns = DistributedToothCoilWinding::even_turn_distribution(
        2.try_into().unwrap(),
        100.try_into().expect("not zero"),
        15,
    )
    .unwrap();

    // Pole pair number of 1 is ok
    assert!(
        DistributedToothCoilWinding::try_from(DistributedToothCoilMinimalBuilder {
            base_winding_count: 1.try_into().expect("not zero"),
            pole_pairs: 1.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            coil_group_turns: coil_group_turns.clone(),
            double_zone_span: false
        })
        .is_ok()
    );

    // Pole pair number of 2 could lead to overflow due to the bug
    assert!(
        DistributedToothCoilWinding::try_from(DistributedToothCoilMinimalBuilder {
            base_winding_count: 1.try_into().expect("not zero"),
            pole_pairs: 2.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            coil_group_turns,
            double_zone_span: false
        })
        .is_err()
    );
}

#[cfg(feature = "serde")]
mod serde_tests {

    use super::*;
    use indoc::indoc;

    #[test]
    fn test_serialize_and_deserialize() {
        let coil_group_turns = DistributedToothCoilWinding::even_turn_distribution(
            2.try_into().unwrap(),
            200.try_into().expect("not zero"),
            15,
        )
        .unwrap();

        let winding = DistributedToothCoilWinding::try_from(DistributedToothCoilMinimalBuilder {
            base_winding_count: 2.try_into().expect("not zero"),
            pole_pairs: 2.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            coil_group_turns,
            double_zone_span: false,
        })
        .unwrap();
        let string = serde_yaml::to_string(&winding).expect("can be serialized");
        let winding_de: DistributedToothCoilWinding =
            serde_yaml::from_str(&string).expect("can be deserialized");

        assert_eq!(winding.slots(), winding_de.slots());
        assert_eq!(winding.layers(), winding_de.layers());
        assert_eq!(winding.pole_pairs(), winding_de.pole_pairs());
        assert_eq!(winding.winding_table(true), winding_de.winding_table(true));
    }

    #[test]
    fn test_deserialize_min_single_layer() {
        // Minimal winding
        let yaml = indoc! {"
            ---
            base_winding_count: 1
            pole_pairs: 2
            phases: 3
            layers: 1
            coil_group_turns: [1, 1]
            double_zone_span: true
            "};

        let winding: DistributedToothCoilWinding = serde_yaml::from_str(yaml).unwrap();

        // Check the winding factor
        approxim::assert_abs_diff_eq!(
            0.4830,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(1)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.75,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(2)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.4330,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(4)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.1294,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(5)),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.1294,
            winding.winding_factor(ONE, SpatialOrder::Mechanical(7)),
            epsilon = 0.0001
        );
    }
}
