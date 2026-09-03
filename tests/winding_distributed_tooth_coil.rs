use approxim;
use indoc::indoc;
use nalgebra::DMatrix;
use test_database::create_dbm;
use winding::*;
use wire::{IsWire, RoundWire};

#[test]
fn test_iterator() {
    let mut iter = CoilsPerCoilGroupIterator::new(24, 3, 1, false);
    assert_eq!(iter.next().unwrap(), 1);
    assert_eq!(iter.next().unwrap(), 2);
    assert!(iter.next().is_none());
    assert!(iter.next().is_none());

    let mut iter = CoilsPerCoilGroupIterator::new(48, 3, 1, false);
    assert_eq!(iter.next().unwrap(), 1);
    assert_eq!(iter.next().unwrap(), 2);
    assert_eq!(iter.next().unwrap(), 4);
    assert!(iter.next().is_none());
    assert!(iter.next().is_none());

    let mut iter = CoilsPerCoilGroupIterator::new(48, 3, 2, false);
    assert_eq!(iter.next().unwrap(), 1);
    assert_eq!(iter.next().unwrap(), 2);
    assert_eq!(iter.next().unwrap(), 4);
    assert_eq!(iter.next().unwrap(), 8);
    assert!(iter.next().is_none());
    assert!(iter.next().is_none());

    let mut iter = CoilsPerCoilGroupIterator::new(48, 3, 1, true);
    assert_eq!(iter.next().unwrap(), 1);
    assert_eq!(iter.next().unwrap(), 2);
    assert_eq!(iter.next().unwrap(), 4);
    assert_eq!(iter.next().unwrap(), 8);
    assert!(iter.next().is_none());
    assert!(iter.next().is_none());

    let mut iter = CoilsPerCoilGroupIterator::new(48, 3, 2, true);
    assert_eq!(iter.next().unwrap(), 1);
    assert_eq!(iter.next().unwrap(), 2);
    assert_eq!(iter.next().unwrap(), 4);
    assert_eq!(iter.next().unwrap(), 8);
    assert_eq!(iter.next().unwrap(), 16);
    assert!(iter.next().is_none());
    assert!(iter.next().is_none());
}

#[test]
fn test_deserialize_min_single_layer() {
    // Minimal winding
    let yaml = indoc! {"
                ---
                slots: 12
                pole_pairs: 2
                phases: 3
                layers: 1
                coil_group_turns: [1, 1]
                parallel_paths: 1
                double_zone_span: true
                "};

    let winding: WindingDistributedToothCoil = create_dbm().from_str(yaml).unwrap();

    // Check the winding factor
    approxim::assert_abs_diff_eq!(0.4830, winding.winding_factor(1, -0.5), epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(0.75, winding.winding_factor(1, 1.0), epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(0.4330, winding.winding_factor(1, -2.0), epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(0.1294, winding.winding_factor(1, 2.5), epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(0.1294, winding.winding_factor(1, -3.5), epsilon = 0.0001);
}

#[test]
fn test_deserialize_min_double_layer() {
    // Minimal winding
    let yaml = indoc! {"
                ---
                slots: 24
                pole_pairs: 2
                phases: 3
                mean_coil_group_turns: 2
                turn_difference: 0
                coils_per_coil_group: 2
                parallel_paths: 1
                double_zone_span: false
                "};

    let winding: WindingDistributedToothCoil = create_dbm().from_str(yaml).unwrap();

    // Check the winding factor
    approxim::assert_abs_diff_eq!(0.4830, winding.winding_factor(1, 1.0), epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(0.1294, winding.winding_factor(1, -5.0), epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(0.1294, winding.winding_factor(1, 7.0), epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(0.4830, winding.winding_factor(1, -11.0), epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(0.4830, winding.winding_factor(1, 13.0), epsilon = 0.0001);
}

#[test]
fn test_wrong_number_of_turns() {
    // Calculate with 1 pole pair as working harmonic (one basic winding)
    assert!(WindingDistributedToothCoil::new_minimal(12, 2, 3, 1, vec![], 1, true).is_err());
    assert!(WindingDistributedToothCoil::new_minimal(12, 2, 3, 1, vec![1], 1, true).is_ok());
    assert!(WindingDistributedToothCoil::new_minimal(12, 2, 3, 1, vec![1, 1], 1, true).is_ok());
}

#[test]
fn test_coil_analysis() {
    {
        let winding =
            WindingDistributedToothCoil::new_minimal(12, 2, 3, 1, vec![1, 1], 1, true).unwrap();
        for coil in winding.coils() {
            if let Coil::Full(coil) = coil {
                let throw = coil.span(winding.slots());
                assert!(throw == 1 || throw == 3); // Coil throw is either 1 or 3

                // All coils of this winding are clockwise
                assert!(coil.clockwise());
            }
        }
    }
}

#[test]
fn test_three_zones_single_layer() {
    let winding_table_expected = WindingTable(DMatrix::from_column_slice(
        1,
        12,
        &[1, 1, -1, -1, 2, 2, -2, -2, 3, 3, -3, -3],
    ));

    {
        // Calculate with 1 pole pair as working harmonic (one basic winding)
        let winding =
            WindingDistributedToothCoil::new_minimal(12, 2, 3, 1, vec![1, 1], 1, true).unwrap();

        assert_eq!(winding.periodicity(), 1);

        // Compare the zone plan
        let winding_table = winding.winding_table(false);
        assert_eq!(winding_table, winding_table_expected);

        // Check the winding factor
        approxim::assert_abs_diff_eq!(0.4830, winding.winding_factor(1, -0.5), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.75, winding.winding_factor(1, 1.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.4330, winding.winding_factor(1, -2.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.1294, winding.winding_factor(1, 2.5), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.1294, winding.winding_factor(1, -3.5), epsilon = 0.0001);
    }

    {
        // Calculate with 2 pole pairs as working harmonic (two basic windings)
        let winding =
            WindingDistributedToothCoil::new_minimal(24, 4, 3, 1, vec![1, 1], 1, true).unwrap();

        assert_eq!(winding.periodicity(), 2);

        // Compare the zone plan
        let winding_table = winding.winding_table(false);
        assert_eq!(winding_table, winding_table_expected);

        // Check the winding ordinals
        let ordinals: Vec<num::rational::Ratio<i32>> =
            winding.harmonic_ordinals().take(5).collect();
        assert_eq!(ordinals[0], num::rational::Ratio::new(-1, 2));
        assert_eq!(ordinals[1], num::rational::Ratio::new(2, 2));
        assert_eq!(ordinals[2], num::rational::Ratio::new(-4, 2));
        assert_eq!(ordinals[3], num::rational::Ratio::new(5, 2));
        assert_eq!(ordinals[4], num::rational::Ratio::new(-7, 2));

        // Check the winding factor
        approxim::assert_abs_diff_eq!(0.4830, winding.winding_factor(1, -0.5), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.75, winding.winding_factor(1, 1.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.4330, winding.winding_factor(1, -2.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.1294, winding.winding_factor(1, 2.5), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.1294, winding.winding_factor(1, -3.5), epsilon = 0.0001);
    }

    {
        // Calculate with 4 pole pairs as working harmonic (two basic windings)
        let winding =
            WindingDistributedToothCoil::new_minimal(24, 8, 3, 1, vec![1, 1], 1, true).unwrap();

        assert_eq!(winding.periodicity(), 2);

        // Compare the zone plan
        let winding_table = winding.winding_table(false);
        assert_eq!(winding_table, winding_table_expected);

        // Check the winding ordinals
        let ordinals: Vec<num::rational::Ratio<i32>> =
            winding.harmonic_ordinals().take(5).collect();
        assert_eq!(ordinals[0], num::rational::Ratio::new(1, 4));
        assert_eq!(ordinals[1], num::rational::Ratio::new(-2, 4));
        assert_eq!(ordinals[2], num::rational::Ratio::new(4, 4));
        assert_eq!(ordinals[3], num::rational::Ratio::new(-5, 4));
        assert_eq!(ordinals[4], num::rational::Ratio::new(7, 4));

        // Check the winding factor
        approxim::assert_abs_diff_eq!(0.4830, winding.winding_factor(1, 0.25), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.75, winding.winding_factor(1, -0.5), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.4330, winding.winding_factor(1, 1.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.1294, winding.winding_factor(1, -1.25), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.1294, winding.winding_factor(1, 1.75), epsilon = 0.0001);
    }
}

#[test]
fn test_three_zones_single_layer_differing_number_of_coils() {
    let mut wires: Vec<Box<dyn IsWire>> = Vec::with_capacity(2);
    for _ in 0..2 {
        wires.push(Box::new(RoundWire::default()));
    }
    let winding = WindingDistributedToothCoil::new(
        24,
        4,
        3,
        1,
        vec![2, 3],
        1,
        Connection::Star,
        0.0,
        wires,
        true,
    )
    .unwrap();

    // Check the winding factor
    approxim::assert_abs_diff_eq!(0.5278, winding.winding_factor(1, -0.5), epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(0.8, winding.winding_factor(1, 1.0), epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(0.3464, winding.winding_factor(1, -2.0), epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(0.0379, winding.winding_factor(1, 2.5), epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(0.0379, winding.winding_factor(1, -3.5), epsilon = 0.0001);
}

#[test]
fn test_six_zones_single_layer() {
    let winding_table_expected = WindingTable(DMatrix::from_column_slice(
        1,
        24,
        &[
            1, 1, -1, -1, -3, -3, 3, 3, 2, 2, -2, -2, -1, -1, 1, 1, 3, 3, -3, -3, -2, -2, 2, 2,
        ],
    ));

    {
        // Calculate with 2 pole pairs as working harmonic (two basic windings)
        let winding =
            WindingDistributedToothCoil::new_minimal(48, 10, 3, 1, vec![1, 1], 1, false).unwrap();

        assert_eq!(winding.periodicity(), 2);

        // Compare the zone plan
        let winding_table = winding.winding_table(false);
        assert_eq!(winding_table, winding_table_expected);

        // Check the winding ordinals
        let ordinals: Vec<num::rational::Ratio<i32>> =
            winding.harmonic_ordinals().take(5).collect();
        assert_eq!(ordinals[0], num::rational::Ratio::new(-1, 5));
        assert_eq!(ordinals[1], num::rational::Ratio::new(5, 5));
        assert_eq!(ordinals[2], num::rational::Ratio::new(-7, 5));
        assert_eq!(ordinals[3], num::rational::Ratio::new(11, 5));
        assert_eq!(ordinals[4], num::rational::Ratio::new(-13, 5));

        // Check the winding factor
        approxim::assert_abs_diff_eq!(0.2566, winding.winding_factor(1, -0.2), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.7663, winding.winding_factor(1, 1.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.5880, winding.winding_factor(1, -1.4), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.0338, winding.winding_factor(1, 2.2), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.0338, winding.winding_factor(1, -2.6), epsilon = 0.0001);
    }

    {
        // Calculate with 7 pole pairs as working harmonic (two basic windings)

        let winding =
            WindingDistributedToothCoil::new_minimal(48, 14, 3, 1, vec![1, 1], 1, false).unwrap();

        assert_eq!(winding.periodicity(), 2);

        // Compare the zone plan
        let winding_table = winding.winding_table(false);
        assert_eq!(winding_table, winding_table_expected);

        // Check the winding ordinals
        let ordinals: Vec<num::rational::Ratio<i32>> =
            winding.harmonic_ordinals().take(5).collect();
        assert_eq!(ordinals[0], num::rational::Ratio::new(1, 7));
        assert_eq!(ordinals[1], num::rational::Ratio::new(-5, 7));
        assert_eq!(ordinals[2], num::rational::Ratio::new(7, 7));
        assert_eq!(ordinals[3], num::rational::Ratio::new(-11, 7));
        assert_eq!(ordinals[4], num::rational::Ratio::new(13, 7));

        // Check the winding factor
        approxim::assert_abs_diff_eq!(
            0.2566,
            winding.winding_factor(1, 1.0 / 7.0),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.7663,
            winding.winding_factor(1, -5.0 / 7.0),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(0.5880, winding.winding_factor(1, 1.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(
            0.0338,
            winding.winding_factor(1, -11.0 / 7.0),
            epsilon = 0.0001
        );
        approxim::assert_abs_diff_eq!(
            0.0338,
            winding.winding_factor(1, 13.0 / 7.0),
            epsilon = 0.0001
        );
    }
}

#[test]
fn test_single_layer_24_1() {
    {
        // Calculate with 1 pole pairs as working harmonic (two basic windings)
        let coil_group_turns =
            WindingDistributedToothCoil::double_layer_turn_distribution(2, 2, 0).unwrap();
        let winding =
            WindingDistributedToothCoil::new_minimal(24, 1, 3, 1, coil_group_turns, 1, false)
                .unwrap();

        assert_eq!(winding.layers(), 1);
        assert_eq!(winding.periodicity(), 1);
    }
    {
        // This fails, since the coil_group_turns vector has not the correct length
        assert!(WindingDistributedToothCoil::new_minimal(24, 1, 3, 1, vec![1], 1, false).is_err());
    }
}

#[test]
fn test_double_layer_24_2() {
    // Calculate with 1 pole pairs as working harmonic (two basic windings)
    let coil_group_turns =
        WindingDistributedToothCoil::double_layer_turn_distribution(2, 2, 0).unwrap();
    let winding =
        WindingDistributedToothCoil::new_minimal(24, 2, 3, 2, coil_group_turns, 1, false).unwrap();

    assert_eq!(winding.layers(), 2);
    assert_eq!(winding.periodicity(), 2);

    // Compare the zone plan
    let winding_table = winding.winding_table(false);

    let expected_result = WindingTable(DMatrix::from_row_slice(
        2,
        12,
        &[
            1, 1, -1, -1, 2, 2, -2, -2, 3, 3, -3, -3, 2, 2, -3, -3, 3, 3, -1, -1, 1, 1, -2, -2,
        ],
    ));
    assert_eq!(winding_table, expected_result);

    // Check the winding ordinals
    let ordinals: Vec<num::rational::Ratio<i32>> = winding.harmonic_ordinals().take(5).collect();
    assert_eq!(ordinals[0], num::rational::Ratio::new(1, 1));
    assert_eq!(ordinals[1], num::rational::Ratio::new(-5, 1));
    assert_eq!(ordinals[2], num::rational::Ratio::new(7, 1));
    assert_eq!(ordinals[3], num::rational::Ratio::new(-11, 1));
    assert_eq!(ordinals[4], num::rational::Ratio::new(13, 1));

    // Check the winding factor
    approxim::assert_abs_diff_eq!(0.4830, winding.winding_factor(1, 1.0), epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(0.1294, winding.winding_factor(1, -5.0), epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(0.1294, winding.winding_factor(1, 7.0), epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(0.4830, winding.winding_factor(1, -11.0), epsilon = 0.0001);
    approxim::assert_abs_diff_eq!(0.4830, winding.winding_factor(1, 13.0), epsilon = 0.0001);
}

#[test]
fn test_double_layer_differing_number_of_coils_24_2_dl() {
    {
        // Calculate with 2 pole pairs as working harmonic (one basic winding)
        let coil_group_turns =
            WindingDistributedToothCoil::double_layer_turn_distribution(2, 200, 15).unwrap();
        let winding =
            WindingDistributedToothCoil::new_minimal(12, 1, 3, 2, coil_group_turns, 1, false)
                .unwrap();

        assert_eq!(winding.periodicity(), 1);

        // Check the winding factor
        approxim::assert_abs_diff_eq!(0.5166, winding.winding_factor(1, 1.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.0039, winding.winding_factor(1, -5.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.0039, winding.winding_factor(1, 7.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.5166, winding.winding_factor(1, -11.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.5166, winding.winding_factor(1, 13.0), epsilon = 0.0001);
    }

    {
        // Calculate with 2 pole pairs as working harmonic (two basic windings)
        let coil_group_turns =
            WindingDistributedToothCoil::double_layer_turn_distribution(2, 200, 15).unwrap();

        assert_eq!(coil_group_turns, vec![85, 115]);

        let winding =
            WindingDistributedToothCoil::new_minimal(24, 2, 3, 2, coil_group_turns, 1, false)
                .unwrap();

        assert_eq!(winding.periodicity(), 2);

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

        assert_eq!(winding.phase_at(Zone::new(0, 0)).unwrap(), 1);
        assert_eq!(winding.phase_at(Zone::new(1, 0)).unwrap(), 1);
        assert_eq!(winding.phase_at(Zone::new(2, 0)).unwrap(), -1);
        assert_eq!(winding.phase_at(Zone::new(3, 0)).unwrap(), -1);

        assert_eq!(winding.phase_at(Zone::new(12, 0)).unwrap(), 1);
        assert_eq!(winding.phase_at(Zone::new(13, 0)).unwrap(), 1);
        assert_eq!(winding.phase_at(Zone::new(14, 0)).unwrap(), -1);
        assert_eq!(winding.phase_at(Zone::new(15, 0)).unwrap(), -1);

        assert_eq!(winding.phase_at(Zone::new(0, 1)).unwrap(), 2);
        assert_eq!(winding.phase_at(Zone::new(1, 1)).unwrap(), 2);
        assert_eq!(winding.phase_at(Zone::new(2, 1)).unwrap(), -3);
        assert_eq!(winding.phase_at(Zone::new(3, 1)).unwrap(), -3);

        // Check the winding factor
        approxim::assert_abs_diff_eq!(0.5166, winding.winding_factor(1, 1.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.0039, winding.winding_factor(1, -5.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.0039, winding.winding_factor(1, 7.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.5166, winding.winding_factor(1, -11.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.5166, winding.winding_factor(1, 13.0), epsilon = 0.0001);
    }

    {
        // Calculate with 1 pole pairs as working harmonic (two basic windings)
        let coil_group_turns =
            WindingDistributedToothCoil::double_layer_turn_distribution(2, 200, 30).unwrap();
        let winding =
            WindingDistributedToothCoil::new_minimal(24, 2, 3, 2, coil_group_turns, 1, false)
                .unwrap();

        // Check the winding factor
        approxim::assert_abs_diff_eq!(0.5502, winding.winding_factor(1, 1.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.1215, winding.winding_factor(1, -5.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.1215, winding.winding_factor(1, 7.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.5502, winding.winding_factor(1, -11.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.5502, winding.winding_factor(1, 13.0), epsilon = 0.0001);
    }
}

/**
Tests for an overflow which could occur in the harmonic ordinals iterator
 */
#[test]
fn test_overflow_iterator_bug() {
    let coil_group_turns =
        WindingDistributedToothCoil::double_layer_turn_distribution(2, 100, 15).unwrap();

    // Pole pair number of 1 is ok
    assert!(
        WindingDistributedToothCoil::new_minimal(12, 1, 3, 2, coil_group_turns.clone(), 1, false,)
            .is_ok()
    );

    // Pole pair number of 2 could lead to overflow due to the bug
    assert!(
        WindingDistributedToothCoil::new_minimal(12, 2, 3, 2, coil_group_turns, 1, false).is_err()
    );
}
