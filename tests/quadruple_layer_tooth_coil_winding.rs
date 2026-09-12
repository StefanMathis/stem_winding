use std::num::{NonZeroU16, NonZeroUsize};

use approxim;
use stem_winding::prelude::*;

const ONE: NonZeroU16 = NonZeroU16::MIN;

// Shorter aliases
const LL: u16 = stem_coil_layout::QUADRUPLE_LAYER_BOTTOM_LEFT;
const UL: u16 = stem_coil_layout::QUADRUPLE_LAYER_TOP_LEFT;
const UR: u16 = stem_coil_layout::QUADRUPLE_LAYER_TOP_RIGHT;
const LR: u16 = stem_coil_layout::QUADRUPLE_LAYER_BOTTOM_RIGHT;

#[test]
fn test_coil_direction() {
    {
        // 9/8 QL
        let winding: QuadrupleLayerToothCoilWinding = QuadrupleLayerToothCoilMinimalBuilder {
            slots: 9.try_into().expect("not zero"),
            pole_pairs: 4.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            winding_table_method: WindingTableMethod::Tingley,
            turns_per_slot_side: 2.try_into().expect("not zero"),
            turns_upper_layer_coils: Vec::new(),
        }
        .try_into()
        .unwrap();

        for coil in winding.coils() {
            if let Coil::Full(coil) = coil {
                assert_eq!(coil.span(winding.slots()), 1); // A tooth coil winding always has a throw of 1

                // Exclude corner cases
                if coil.positive_zone().slot > 0 && coil.positive_zone().slot < 11 {
                    if coil.positive_zone() < coil.negative_zone() {
                        assert!(coil.clockwise());
                    } else {
                        assert!(!coil.clockwise());
                    }
                }
            }
        }
    }
}

/**
These windings have lead to crashes in the winding explorer UI
 */
#[test]
fn test_turn_creator_regression_test() {
    assert!(
        QuadrupleLayerToothCoilWinding::new(QuadrupleLayerToothCoilMinimalBuilder {
            slots: 9.try_into().expect("not zero"),
            pole_pairs: 1.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            turns_per_slot_side: 2.try_into().expect("not zero"),
            turns_upper_layer_coils: Vec::new(),
            winding_table_method: WindingTableMethod::Tingley,
        })
        .is_err()
    );
    assert!(
        QuadrupleLayerToothCoilWinding::new(QuadrupleLayerToothCoilMinimalBuilder {
            slots: 9.try_into().expect("not zero"),
            pole_pairs: 1.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            turns_per_slot_side: 2.try_into().expect("not zero"),
            turns_upper_layer_coils: vec![NonZeroUsize::new(1).expect("not zero")],
            winding_table_method: WindingTableMethod::Tingley,
        })
        .is_err()
    );
}

#[test]
fn test_err_when_turns_per_slot_side_smaller_than_two() {
    // 1 Turn per slot side
    assert!(
        QuadrupleLayerToothCoilWinding::new(QuadrupleLayerToothCoilMinimalBuilder {
            slots: 9.try_into().expect("not zero"),
            pole_pairs: 4.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            turns_per_slot_side: 1.try_into().expect("not zero"),
            turns_upper_layer_coils: vec![NonZeroUsize::new(1).expect("not zero")],
            winding_table_method: WindingTableMethod::Tingley,
        })
        .is_err()
    );

    // 2 Turns per slot side
    assert!(
        QuadrupleLayerToothCoilWinding::new(QuadrupleLayerToothCoilMinimalBuilder {
            slots: 9.try_into().expect("not zero"),
            pole_pairs: 4.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            turns_per_slot_side: 2.try_into().expect("not zero"),
            turns_upper_layer_coils: vec![NonZeroUsize::new(1).expect("not zero")],
            winding_table_method: WindingTableMethod::Tingley,
        })
        .is_ok()
    );

    // 3 Turns per slot side
    assert!(
        QuadrupleLayerToothCoilWinding::new(QuadrupleLayerToothCoilMinimalBuilder {
            slots: 9.try_into().expect("not zero"),
            pole_pairs: 4.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            turns_per_slot_side: 3.try_into().expect("not zero"),
            turns_upper_layer_coils: vec![NonZeroUsize::new(1).expect("not zero")],
            winding_table_method: WindingTableMethod::Tingley,
        })
        .is_ok()
    );
}

#[test]
fn test_equal_turns_per_coil_9_8() {
    {
        // Check the 9/8 winding from [Alb11]

        // No zone shift -> Winding factor should be the same as that of a two-layer
        // winding
        let winding = QuadrupleLayerToothCoilWinding::new(QuadrupleLayerToothCoilMinimalBuilder {
            slots: 9.try_into().expect("not zero"),
            pole_pairs: 4.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            turns_per_slot_side: 2.try_into().expect("not zero"),
            turns_upper_layer_coils: Vec::new(),
            winding_table_method: WindingTableMethod::Tingley,
        })
        .unwrap();

        approxim::assert_abs_diff_eq!(0.061, winding.winding_factor(ONE, 0.25), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.139, winding.winding_factor(ONE, 0.5), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.945, winding.winding_factor(ONE, 1.0), epsilon = 0.001);

        // Compare the zone plan
        let winding_table = winding.winding_table(true);
        let expected_result = WindingTable::from_slot_major(
            [
                1, 1, 1, 1, -1, -1, 2, 2, -2, -2, -2, -2, 2, 2, 2, 2, -2, -2, 3, 3, -3, -3, -3, -3,
                3, 3, 3, 3, -3, -3, 1, 1, -1, -1, -1, -1,
            ]
            .into_iter(),
            winding.slots(),
            winding.layers(),
        );
        assert_eq!(winding_table, expected_result);

        // All layers in slot 1
        assert_eq!(winding.phase_at(Zone::new(0, LL)).unwrap(), 1);
        assert_eq!(winding.phase_at(Zone::new(0, UL)).unwrap(), 1);
        assert_eq!(winding.phase_at(Zone::new(0, UR)).unwrap(), 1);
        assert_eq!(winding.phase_at(Zone::new(0, LR)).unwrap(), 1);

        // All layers in slot 2
        assert_eq!(winding.phase_at(Zone::new(1, LL)).unwrap(), -1);
        assert_eq!(winding.phase_at(Zone::new(1, UL)).unwrap(), -1);
        assert_eq!(winding.phase_at(Zone::new(1, UR)).unwrap(), 2);
        assert_eq!(winding.phase_at(Zone::new(1, LR)).unwrap(), 2);

        // All layers in slot 3
        assert_eq!(winding.phase_at(Zone::new(2, LL)).unwrap(), -2);
        assert_eq!(winding.phase_at(Zone::new(2, UL)).unwrap(), -2);
        assert_eq!(winding.phase_at(Zone::new(2, UR)).unwrap(), -2);
        assert_eq!(winding.phase_at(Zone::new(2, LR)).unwrap(), -2);
    }

    {
        // Zone shift by 1 (Fig. 5 in [Alb11])
        let winding = QuadrupleLayerToothCoilWinding::new(QuadrupleLayerToothCoilMinimalBuilder {
            slots: 9.try_into().expect("not zero"),
            pole_pairs: 4.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            turns_per_slot_side: 2.try_into().expect("not zero"),
            turns_upper_layer_coils: vec![NonZeroUsize::new(1).expect("not zero")],
            winding_table_method: WindingTableMethod::Tingley,
        })
        .unwrap();

        // Compare the zone plan
        let winding_table = winding.winding_table(true);
        let expected_result = WindingTable::from_slot_major(
            [
                1, 1, 1, 1, -1, -1, -1, 2, -2, 1, -2, -2, 2, 2, 2, 2, -2, -2, -2, 3, -3, 2, -3, -3,
                3, 3, 3, 3, -3, -3, -3, 1, -1, 3, -1, -1,
            ]
            .into_iter(),
            winding.slots(),
            winding.layers(),
        );
        assert_eq!(winding_table, expected_result);

        // All layers in slot 1
        assert_eq!(winding.phase_at(Zone::new(0, LL)).unwrap(), 1);
        assert_eq!(winding.phase_at(Zone::new(0, UL)).unwrap(), 1);
        assert_eq!(winding.phase_at(Zone::new(0, UR)).unwrap(), 1);
        assert_eq!(winding.phase_at(Zone::new(0, LR)).unwrap(), 1);

        // All layers in slot 2
        assert_eq!(winding.phase_at(Zone::new(1, LL)).unwrap(), -1);
        assert_eq!(winding.phase_at(Zone::new(1, UL)).unwrap(), -1);
        assert_eq!(winding.phase_at(Zone::new(1, UR)).unwrap(), -1);
        assert_eq!(winding.phase_at(Zone::new(1, LR)).unwrap(), 2);

        // All layers in slot 3
        assert_eq!(winding.phase_at(Zone::new(2, LL)).unwrap(), -2);
        assert_eq!(winding.phase_at(Zone::new(2, UL)).unwrap(), 1);
        assert_eq!(winding.phase_at(Zone::new(2, UR)).unwrap(), -2);
        assert_eq!(winding.phase_at(Zone::new(2, LR)).unwrap(), -2);

        // Check turns per coil at different positions

        // All layers in slot 1
        assert_eq!(winding.turns_at(Zone::new(0, LL)), 1);
        assert_eq!(winding.turns_at(Zone::new(0, UL)), 1);
        assert_eq!(winding.turns_at(Zone::new(0, UR)), 1);
        assert_eq!(winding.turns_at(Zone::new(0, LR)), 1);

        approxim::assert_abs_diff_eq!(0.021, winding.winding_factor(ONE, 0.25), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.090, winding.winding_factor(ONE, 0.5), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.931, winding.winding_factor(ONE, 1.0), epsilon = 0.001);
    }

    {
        // Zone shift by 2 (Fig. 4 in [Alb11])
        let winding = QuadrupleLayerToothCoilWinding::new(QuadrupleLayerToothCoilMinimalBuilder {
            slots: 9.try_into().expect("not zero"),
            pole_pairs: 4.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            turns_per_slot_side: 2.try_into().expect("not zero"),
            turns_upper_layer_coils: vec![
                NonZeroUsize::new(1).expect("not zero"),
                NonZeroUsize::new(1).expect("not zero"),
            ],
            winding_table_method: WindingTableMethod::Tingley,
        })
        .unwrap();

        // Compare the zone plan
        let winding_table = winding.winding_table(true);
        let expected_result = WindingTable::from_slot_major(
            [
                1, -3, 1, 1, -1, -1, -1, 2, -2, 1, 1, -2, 2, -1, 2, 2, -2, -2, -2, 3, -3, 2, 2, -3,
                3, -2, 3, 3, -3, -3, -3, 1, -1, 3, 3, -1,
            ]
            .into_iter(),
            winding.slots(),
            winding.layers(),
        );
        assert_eq!(winding_table, expected_result);

        // All layers in slot 1
        assert_eq!(winding.phase_at(Zone::new(0, LL)).unwrap(), 1);
        assert_eq!(winding.phase_at(Zone::new(0, UL)).unwrap(), -3);
        assert_eq!(winding.phase_at(Zone::new(0, UR)).unwrap(), 1);
        assert_eq!(winding.phase_at(Zone::new(0, LR)).unwrap(), 1);

        // All layers in slot 2
        assert_eq!(winding.phase_at(Zone::new(1, LL)).unwrap(), -1);
        assert_eq!(winding.phase_at(Zone::new(1, UL)).unwrap(), -1);
        assert_eq!(winding.phase_at(Zone::new(1, UR)).unwrap(), -1);
        assert_eq!(winding.phase_at(Zone::new(1, LR)).unwrap(), 2);

        // All layers in slot 3
        assert_eq!(winding.phase_at(Zone::new(2, LL)).unwrap(), -2);
        assert_eq!(winding.phase_at(Zone::new(2, UL)).unwrap(), 1);
        assert_eq!(winding.phase_at(Zone::new(2, UR)).unwrap(), 1);
        assert_eq!(winding.phase_at(Zone::new(2, LR)).unwrap(), -2);

        // All layers in slot 4
        assert_eq!(winding.phase_at(Zone::new(3, LL)).unwrap(), 2);
        assert_eq!(winding.phase_at(Zone::new(3, UL)).unwrap(), -1);
        assert_eq!(winding.phase_at(Zone::new(3, UR)).unwrap(), 2);
        assert_eq!(winding.phase_at(Zone::new(3, LR)).unwrap(), 2);

        approxim::assert_abs_diff_eq!(0.046, winding.winding_factor(ONE, 0.25), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.024, winding.winding_factor(ONE, 0.5), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.888, winding.winding_factor(ONE, 1.0), epsilon = 0.001);
    }
}

#[test]
fn test_varying_turns_per_coil_9_8() {
    {
        // Zone shift by 1 (Fig. 5 in [Alb11])
        let winding = QuadrupleLayerToothCoilWinding::new(QuadrupleLayerToothCoilMinimalBuilder {
            slots: 9.try_into().expect("not zero"),
            pole_pairs: 4.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            turns_per_slot_side: 9.try_into().expect("not zero"),
            turns_upper_layer_coils: vec![NonZeroUsize::new(4).expect("not zero")],
            winding_table_method: WindingTableMethod::Tingley,
        })
        .unwrap();

        approxim::assert_abs_diff_eq!(0.022, winding.winding_factor(ONE, 0.25), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.091, winding.winding_factor(ONE, 0.5), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.931, winding.winding_factor(ONE, 1.0), epsilon = 0.001);
    }

    {
        // Zone shift by 1 (Fig. 5 in [Alb11]
        let winding = QuadrupleLayerToothCoilWinding::new(QuadrupleLayerToothCoilMinimalBuilder {
            slots: 9.try_into().expect("not zero"),
            pole_pairs: 4.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            turns_per_slot_side: 9.try_into().expect("not zero"),
            turns_upper_layer_coils: vec![NonZeroUsize::new(3).expect("not zero")],
            winding_table_method: WindingTableMethod::Tingley,
        })
        .unwrap();

        // Check turns per coil at different positions

        // All layers in slot 1
        assert_eq!(winding.turns_at(Zone::new(0, LL)), 5);
        assert_eq!(winding.turns_at(Zone::new(0, UL)), 4);
        assert_eq!(winding.turns_at(Zone::new(0, UR)), 4);
        assert_eq!(winding.turns_at(Zone::new(0, LR)), 5);
        assert_eq!(winding.turns_in_slot(0), 18);

        // All layers in slot 2
        assert_eq!(winding.turns_at(Zone::new(1, LL)), 5);
        assert_eq!(winding.turns_at(Zone::new(1, UL)), 4);
        assert_eq!(winding.turns_at(Zone::new(1, UR)), 3);
        assert_eq!(winding.turns_at(Zone::new(1, LR)), 6);
        assert_eq!(winding.turns_in_slot(1), 18);

        // All layers in slot 3
        assert_eq!(winding.turns_at(Zone::new(2, LL)), 6);
        assert_eq!(winding.turns_at(Zone::new(2, UL)), 3);
        assert_eq!(winding.turns_at(Zone::new(2, UR)), 4);
        assert_eq!(winding.turns_at(Zone::new(2, LR)), 5);
        assert_eq!(winding.turns_in_slot(2), 18);

        approxim::assert_abs_diff_eq!(0.028, winding.winding_factor(ONE, 0.25), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.097, winding.winding_factor(ONE, 0.5), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.932, winding.winding_factor(ONE, 1.0), epsilon = 0.001);
    }

    {
        // Zone shift by 1 (Fig. 5 in [Alb11])
        let winding = QuadrupleLayerToothCoilWinding::new(QuadrupleLayerToothCoilMinimalBuilder {
            slots: 9.try_into().expect("not zero"),
            pole_pairs: 4.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            turns_per_slot_side: 9.try_into().expect("not zero"),
            turns_upper_layer_coils: vec![NonZeroUsize::new(2).expect("not zero")],
            winding_table_method: WindingTableMethod::Tingley,
        })
        .unwrap();

        approxim::assert_abs_diff_eq!(0.038, winding.winding_factor(ONE, 0.25), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.108, winding.winding_factor(ONE, 0.5), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.935, winding.winding_factor(ONE, 1.0), epsilon = 0.001);
    }

    {
        // Zone shift by 2 (Fig. 5 in [Alb11])
        let winding = QuadrupleLayerToothCoilWinding::new(QuadrupleLayerToothCoilMinimalBuilder {
            slots: 9.try_into().expect("not zero"),
            pole_pairs: 4.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            turns_per_slot_side: 9.try_into().expect("not zero"),
            turns_upper_layer_coils: vec![
                NonZeroUsize::new(5).expect("not zero"),
                NonZeroUsize::new(4).expect("not zero"),
            ],
            winding_table_method: WindingTableMethod::Tingley,
        })
        .unwrap();

        // All layers in slot 1
        assert_eq!(winding.turns_at(Zone::new(0, LL)), 5);
        assert_eq!(winding.turns_at(Zone::new(0, UL)), 4);
        assert_eq!(winding.turns_at(Zone::new(0, UR)), 4);
        assert_eq!(winding.turns_at(Zone::new(0, LR)), 5);
        assert_eq!(winding.turns_in_slot(0), 18);

        // All layers in slot 2
        assert_eq!(winding.turns_at(Zone::new(1, LL)), 5);
        assert_eq!(winding.turns_at(Zone::new(1, UL)), 4);
        assert_eq!(winding.turns_at(Zone::new(1, UR)), 5);
        assert_eq!(winding.turns_at(Zone::new(1, LR)), 4);
        assert_eq!(winding.turns_in_slot(1), 18);

        // All layers in slot 3
        assert_eq!(winding.turns_at(Zone::new(2, LL)), 4);
        assert_eq!(winding.turns_at(Zone::new(2, UL)), 5);
        assert_eq!(winding.turns_at(Zone::new(2, UR)), 4);
        assert_eq!(winding.turns_at(Zone::new(2, LR)), 5);
        assert_eq!(winding.turns_in_slot(2), 18);

        // All layers in slot 4
        assert_eq!(winding.turns_at(Zone::new(3, LL)), 5);
        assert_eq!(winding.turns_at(Zone::new(3, UL)), 4);
        assert_eq!(winding.turns_at(Zone::new(3, UR)), 4);
        assert_eq!(winding.turns_at(Zone::new(3, LR)), 5);
        assert_eq!(winding.turns_in_slot(3), 18);

        approxim::assert_abs_diff_eq!(0.035, winding.winding_factor(ONE, 0.25), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.006, winding.winding_factor(ONE, 0.5), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.895, winding.winding_factor(ONE, 1.0), epsilon = 0.001);
    }

    {
        // Zone shift by 2 (Fig. 5 in [Alb11])
        let winding = QuadrupleLayerToothCoilWinding::new(QuadrupleLayerToothCoilMinimalBuilder {
            slots: 9.try_into().expect("not zero"),
            pole_pairs: 4.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            turns_per_slot_side: 9.try_into().expect("not zero"),
            turns_upper_layer_coils: vec![
                NonZeroUsize::new(4).expect("not zero"),
                NonZeroUsize::new(3).expect("not zero"),
            ],
            winding_table_method: WindingTableMethod::Tingley,
        })
        .unwrap();

        // All layers in slot 1
        assert_eq!(winding.turns_at(Zone::new(0, LL)), 6);
        assert_eq!(winding.turns_at(Zone::new(0, UL)), 3);
        assert_eq!(winding.turns_at(Zone::new(0, UR)), 4);
        assert_eq!(winding.turns_at(Zone::new(0, LR)), 5);
        assert_eq!(winding.turns_in_slot(0), 18);

        // All layers in slot 2
        assert_eq!(winding.turns_at(Zone::new(1, LL)), 5);
        assert_eq!(winding.turns_at(Zone::new(1, UL)), 4);
        assert_eq!(winding.turns_at(Zone::new(1, UR)), 4);
        assert_eq!(winding.turns_at(Zone::new(1, LR)), 5);
        assert_eq!(winding.turns_in_slot(1), 18);

        // All layers in slot 3
        assert_eq!(winding.turns_at(Zone::new(2, LL)), 5);
        assert_eq!(winding.turns_at(Zone::new(2, UL)), 4);
        assert_eq!(winding.turns_at(Zone::new(2, UR)), 3);
        assert_eq!(winding.turns_at(Zone::new(2, LR)), 6);
        assert_eq!(winding.turns_in_slot(2), 18);

        // All layers in slot 4
        assert_eq!(winding.turns_at(Zone::new(3, LL)), 6);
        assert_eq!(winding.turns_at(Zone::new(3, UL)), 3);
        assert_eq!(winding.turns_at(Zone::new(3, UR)), 4);
        assert_eq!(winding.turns_at(Zone::new(3, LR)), 5);
        assert_eq!(winding.turns_in_slot(3), 18);

        approxim::assert_abs_diff_eq!(0.036, winding.winding_factor(ONE, 0.25), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.031, winding.winding_factor(ONE, 0.5), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.897, winding.winding_factor(ONE, 1.0), epsilon = 0.001);
    }
}

#[test]
fn test_winding_table_creation_12_10_equal_turns_per_coil() {
    {
        // Check the 12/10 winding from [Wan15]

        // No zone shift -> Winding factor should be the same as that of a two-layer
        // winding
        let winding = QuadrupleLayerToothCoilWinding::new(QuadrupleLayerToothCoilMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 5.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            turns_per_slot_side: 2.try_into().expect("not zero"),
            turns_upper_layer_coils: Vec::new(),
            winding_table_method: WindingTableMethod::Tingley,
        })
        .unwrap();

        approxim::assert_abs_diff_eq!(0.0670, winding.winding_factor(ONE, 0.2), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.9330, winding.winding_factor(ONE, 1.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.9330, winding.winding_factor(ONE, 1.4), epsilon = 0.0001);
    }

    {
        // Zone shift by 1 (Fig. 1 in [Wan15])
        let winding = QuadrupleLayerToothCoilWinding::new(QuadrupleLayerToothCoilMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 5.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            turns_per_slot_side: 2.try_into().expect("not zero"),
            turns_upper_layer_coils: vec![NonZeroUsize::new(1).expect("not zero")],
            winding_table_method: WindingTableMethod::Tingley,
        })
        .unwrap();

        approxim::assert_abs_diff_eq!(0.0173, winding.winding_factor(ONE, 0.2), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.9012, winding.winding_factor(ONE, 1.0), epsilon = 0.0001);
        approxim::assert_abs_diff_eq!(0.9012, winding.winding_factor(ONE, 1.4), epsilon = 0.0001);

        // Compare the zone plan
        let winding_table = winding.winding_table(true);
        let expected_result = WindingTable::from_slot_major(
            [
                1, -3, 1, 1, -1, -1, -1, 2, -2, 1, -2, -2, 2, 2, 2, -3, 3, -2, 3, 3, -3, -3, -3, 1,
                -1, 3, -1, -1, 1, 1, 1, -2, 2, -1, 2, 2, -2, -2, -2, 3, -3, 2, -3, -3, 3, 3, 3, -1,
            ]
            .into_iter(),
            winding.slots(),
            winding.layers(),
        );
        assert_eq!(winding_table, expected_result);
    }
}

#[cfg(feature = "serde")]
mod serde_tests {

    use super::*;
    use indoc::indoc;

    #[test]
    fn test_serialize_and_deserialize() {
        let winding = QuadrupleLayerToothCoilWinding::new(QuadrupleLayerToothCoilMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 5.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            turns_per_slot_side: 2.try_into().expect("not zero"),
            turns_upper_layer_coils: vec![NonZeroUsize::new(1).expect("not zero")],
            winding_table_method: WindingTableMethod::Tingley,
        })
        .unwrap();
        let string = yaml_serde::to_string(&winding).expect("can be serialized");
        let winding_de: QuadrupleLayerToothCoilWinding =
            yaml_serde::from_str(&string).expect("can be deserialized");

        assert_eq!(winding.slots(), winding_de.slots());
        assert_eq!(winding.layers(), winding_de.layers());
        assert_eq!(winding.pole_pairs(), winding_de.pole_pairs());
        assert_eq!(winding.winding_table(true), winding_de.winding_table(true));
    }

    #[test]
    fn test_deserialize_min() {
        // Minimal winding
        let yaml = indoc! {"
            ---
            slots: 9
            pole_pairs: 4
            phases: 3
            turns_per_slot_side: 2
            turns_upper_layer_coils: []
            winding_table_method: Tingley
            "};

        let winding: QuadrupleLayerToothCoilWinding = yaml_serde::from_str(yaml).unwrap();

        approxim::assert_abs_diff_eq!(0.061, winding.winding_factor(ONE, 0.25), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.139, winding.winding_factor(ONE, 0.5), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.945, winding.winding_factor(ONE, 1.0), epsilon = 0.001);

        assert_eq!(winding.layers().get(), 4);
        assert_eq!(winding.coils_per_phase(), 6);
        assert_eq!(winding.coil_groups_per_phase().get(), 1);
        assert_eq!(winding.coils_per_coil_group(), 6);

        // Minimal winding
        let yaml = indoc! {"
            ---
            slots: 18
            pole_pairs: 8
            phases: 3
            turns_per_slot_side: 2
            turns_upper_layer_coils: []
            winding_table_method: Tingley
            "};

        let winding: QuadrupleLayerToothCoilWinding = yaml_serde::from_str(yaml).unwrap();

        approxim::assert_abs_diff_eq!(0.061, winding.winding_factor(ONE, 0.25), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.139, winding.winding_factor(ONE, 0.5), epsilon = 0.001);
        approxim::assert_abs_diff_eq!(0.945, winding.winding_factor(ONE, 1.0), epsilon = 0.001);

        assert_eq!(winding.layers().get(), 4);
        assert_eq!(winding.coils_per_phase(), 12);
        assert_eq!(winding.coil_groups_per_phase().get(), 2);
        assert_eq!(winding.coils_per_coil_group(), 6);
    }
}
