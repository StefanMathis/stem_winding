use std::num::NonZeroU16;

use stem_winding::winding_table::*;

fn create_table() -> WindingTable {
    let mut table = WindingTable::new(
        NonZeroU16::new(6).expect("not zero"),
        NonZeroU16::new(2).expect("not zero"),
    );
    table[Zone { slot: 0, layer: 0 }] = 1;
    table[Zone { slot: 0, layer: 1 }] = -2;
    table[Zone { slot: 1, layer: 0 }] = -3;
    table[Zone { slot: 1, layer: 1 }] = 1;
    table[Zone { slot: 2, layer: 0 }] = 2;
    table[Zone { slot: 2, layer: 1 }] = -3;
    table[Zone { slot: 3, layer: 0 }] = -1;
    table[Zone { slot: 3, layer: 1 }] = 2;
    table[Zone { slot: 4, layer: 0 }] = 3;
    table[Zone { slot: 4, layer: 1 }] = -1;
    table[Zone { slot: 5, layer: 0 }] = -2;
    table[Zone { slot: 5, layer: 1 }] = 3;
    return table;
}

#[test]
fn test_indexing() {
    let mut table = create_table();
    assert_eq!(table[Zone { slot: 2, layer: 1 }], -3);
    assert_eq!(table[Zone { slot: 3, layer: 1 }], 2);

    assert_eq!(table.get(Zone { slot: 2, layer: 0 }), Some(&2));
    assert_eq!(table.get(Zone { slot: 3, layer: 1 }), Some(&2));
    assert!(table.get(Zone { slot: 2, layer: 2 }).is_none());

    assert_eq!(table.get_mut(Zone { slot: 2, layer: 0 }), Some(&mut 2));
    assert_eq!(
        table.get_cyclic(Zone { slot: 6, layer: 2 }),
        table.get(Zone { slot: 0, layer: 0 }).expect("exists")
    );
    assert_eq!(table.get_cyclic_mut(Zone { slot: 2, layer: 2 }), &mut 2i32);
}

#[test]
fn test_iterate() {
    let mut table = create_table();

    // Count the number of elements from the iterators
    assert_eq!(table.iter_slots().count(), 12);
    assert_eq!(table.iter_slots_mut().count(), 12);
    assert_eq!(table.iter_layers().count(), 12);
    assert_eq!(table.iter_layers_mut().count(), 12);

    // Check the order of returned elements
    {
        let mut iterator = table.iter_slots();
        assert_eq!(iterator.next(), Some((Zone { slot: 0, layer: 0 }, &1)));
        assert_eq!(iterator.next(), Some((Zone { slot: 0, layer: 1 }, &-2)));
        assert_eq!(iterator.next(), Some((Zone { slot: 1, layer: 0 }, &-3)));
        assert_eq!(iterator.next(), Some((Zone { slot: 1, layer: 1 }, &1)));
    }
    {
        let mut iterator = table.iter_slots_mut();
        assert_eq!(iterator.next(), Some((Zone { slot: 0, layer: 0 }, &mut 1)));
        assert_eq!(iterator.next(), Some((Zone { slot: 0, layer: 1 }, &mut -2)));
        assert_eq!(iterator.next(), Some((Zone { slot: 1, layer: 0 }, &mut -3)));
        assert_eq!(iterator.next(), Some((Zone { slot: 1, layer: 1 }, &mut 1)));
    }
    {
        let mut iterator = table.iter_layers();
        assert_eq!(iterator.next(), Some((Zone { slot: 0, layer: 0 }, &1)));
        assert_eq!(iterator.next(), Some((Zone { slot: 1, layer: 0 }, &-3)));
        assert_eq!(iterator.next(), Some((Zone { slot: 2, layer: 0 }, &2)));
        assert_eq!(iterator.next(), Some((Zone { slot: 3, layer: 0 }, &-1)));
        assert_eq!(iterator.next(), Some((Zone { slot: 4, layer: 0 }, &3)));
        assert_eq!(iterator.next(), Some((Zone { slot: 5, layer: 0 }, &-2)));
        assert_eq!(iterator.next(), Some((Zone { slot: 0, layer: 1 }, &-2)));
    }
    {
        let mut iterator = table.iter_layers_mut();
        assert_eq!(iterator.next(), Some((Zone { slot: 0, layer: 0 }, &mut 1)));
        assert_eq!(iterator.next(), Some((Zone { slot: 1, layer: 0 }, &mut -3)));
        assert_eq!(iterator.next(), Some((Zone { slot: 2, layer: 0 }, &mut 2)));
        assert_eq!(iterator.next(), Some((Zone { slot: 3, layer: 0 }, &mut -1)));
        assert_eq!(iterator.next(), Some((Zone { slot: 4, layer: 0 }, &mut 3)));
        assert_eq!(iterator.next(), Some((Zone { slot: 5, layer: 0 }, &mut -2)));
        assert_eq!(iterator.next(), Some((Zone { slot: 0, layer: 1 }, &mut -2)));
    }
}

#[test]
fn test_from_iter() {
    {
        let winding_table = create_table();
        let iterator = winding_table.iter_slots().map(|(_, value)| *value);
        let winding_table_from_iter = WindingTable::from_slot_major(
            iterator,
            NonZeroU16::new(winding_table.slots()).expect("not zero"),
            NonZeroU16::new(winding_table.layers()).expect("not zero"),
        );
        assert_eq!(winding_table, winding_table_from_iter);
    }
    {
        let winding_table = create_table();
        let iterator = winding_table.iter_layers().map(|(_, value)| *value);
        let winding_table_from_iter = WindingTable::from_layer_major(
            iterator,
            NonZeroU16::new(winding_table.slots()).expect("not zero"),
            NonZeroU16::new(winding_table.layers()).expect("not zero"),
        );
        assert_eq!(winding_table, winding_table_from_iter);
    }
}

/**
These windings have lead to crashes in the winding explorer UI
 */
#[test]
fn test_tingley_single_layer_tooth_coil_failure() {
    assert!(
        WindingTable::with_method(
            WindingTableMethod::Tingley,
            NonZeroU16::new(7).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1,
        )
        .is_err()
    );
}

#[test]
fn test_tingley_success() {
    {
        // 9/10 double-layer winding with 3 phases
        let winding_table = WindingTable::with_method(
            WindingTableMethod::Tingley,
            NonZeroU16::new(9).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1,
        )
        .unwrap();
        let expected_result = WindingTable::from_slot_major(
            [
                -2, 1, -1, -1, 1, 1, -1, 3, -3, -3, 3, 3, -3, 2, -2, -2, 2, 2,
            ]
            .into_iter(),
            NonZeroU16::new(9).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }
    {
        // Test case from [Hut06], fig. 2: 9/8 double-layer winding with 3 phases
        let winding_table = WindingTable::with_method(
            WindingTableMethod::Tingley,
            NonZeroU16::new(9).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(4).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1i32, -1, -2, 2, -2, -3, 3, -3, -1, 1, 2, -2, 2, 3, -3, 3, 1, -1,
            ]
            .into_iter(),
            NonZeroU16::new(9).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }
    {
        // Test case: 12/10 double-layer winding with 3 phases. The coil span is 1
        // (tooth-coil winding)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::Tingley,
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1i32, -1, -2, 2, 3, -3, -1, 1, 2, -2, -3, 3, 1, 2, -2, -3, 3, 1, -1, -2, 2, 3, -3,
                -1,
            ]
            .into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }
    {
        // Test case: 12/2 double-layer winding with 3 phases. The coil span is 6
        // (integer slot winding)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::Tingley,
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            6,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1i32, 1, -3, -3, 2, 2, -1, -1, 3, 3, -2, -2, 1, 1, -3, -3, 2, 2, -1, -1, 3, 3, -2,
                -2,
            ]
            .into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }
    {
        // Test case: 24/10 double-layer winding with 3 phases. The coil span is 2
        // (short pitching)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::Tingley,
            NonZeroU16::new(24).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            2,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1, -3, -1, 3, -2, 1, 2, -1, 3, -2, -3, 2, -1, 3, 1, -3, 2, -1, -2, 1, -3, 2, 3, -2,
                1, -3, 2, -1, -2, 1, -3, 2, 3, -2, 1, -3, -1, 3, -2, 1, 2, -1, 3, -2, -3, 2, -1, 3,
            ]
            .into_iter(),
            NonZeroU16::new(24).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }
    {
        // Test case: 15/10 double-layer winding with 3 phases. The coil span is 1
        // (tooth-coil winding)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::Tingley,
            NonZeroU16::new(15).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                -3, -1, -2, -3, -1, -2, -3, -1, -2, -3, -1, -2, -3, -1, -2, 1, 2, 3, 1, 2, 3, 1, 2,
                3, 1, 2, 3, 1, 2, 3,
            ]
            .into_iter(),
            NonZeroU16::new(15).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case: 12/2 single-layer winding with 3 phases. The coil span is 6
        let winding_table = WindingTable::with_method(
            WindingTableMethod::Tingley,
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            6,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [1, 1, -3, -3, 2, 2, -1, -1, 3, 3, -2, -2].into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case: 12/2 double-layer winding with 3 phases. The coil span is 5
        // (integer slot winding with short pitching)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::Tingley,
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            5,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1, -3, -3, 2, 2, -1, -1, 3, 3, -2, -2, 1, 1, 1, -3, -3, 2, 2, -1, -1, 3, 3, -2, -2,
            ]
            .into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case: 18/4 single-layer winding with 3 phases (fractional-slot winding).
        let winding_table = WindingTable::with_method(
            WindingTableMethod::Tingley,
            NonZeroU16::new(18).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            5,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1i32, 1, -3, 2, -1, -1, 3, 3, -2, 1, -3, -3, 2, 2, -1, 3, -2, -2,
            ]
            .into_iter(),
            NonZeroU16::new(18).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case: 12/10 single-layer winding with 3 phases (fractional-slot
        // winding).
        let winding_table = WindingTable::with_method(
            WindingTableMethod::Tingley,
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [3, 1, -1, -2, 2, 3, -3, -1, 1, 2, -2, -3].into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case: 6/2 single-layer winding with 3 phases. The coil span is 4
        // (distributed winding). This test was introduced due to a found bug
        let winding_table = WindingTable::with_method(
            WindingTableMethod::Tingley,
            NonZeroU16::new(6).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            4,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [1, -3, 2, -1, 3, -2].into_iter(),
            NonZeroU16::new(6).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case: 12/4 single-layer winding with 3 phases. The coil span is 4
        // (distributed winding). This test was introduced due to a found bug
        let winding_table = WindingTable::with_method(
            WindingTableMethod::Tingley,
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            4,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [1, -3, 2, -1, 3, -2, 1, -3, 2, -1, 3, -2].into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }
    {
        // Test case: 12/10 single-layer winding with 3 phases. The coil span is 1
        // (tooth-coil winding)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::Tingley,
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [3, 1, -1, -2, 2, 3, -3, -1, 1, 2, -2, -3].into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case 6: 24/20 single-layer winding with 3 phases. The coil span is 1
        // (tooth-coil winding)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::Tingley,
            NonZeroU16::new(24).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(10).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                3, 1, -1, -2, 2, 3, -3, -1, 1, 2, -2, -3, 3, 1, -1, -2, 2, 3, -3, -1, 1, 2, -2, -3,
            ]
            .into_iter(),
            NonZeroU16::new(24).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }
}

#[test]
fn test_winding_table_tingley_error() {
    // Test case 1: 12/10 triple-layer winding with 3 phases. The coil span is 1
    // (tooth-coil winding)
    let winding_table = WindingTable::with_method(
        WindingTableMethod::Tingley,
        NonZeroU16::new(12).expect("not zero"),
        NonZeroU16::new(3).expect("not zero"),
        NonZeroU16::new(5).expect("not zero"),
        NonZeroU16::new(3).expect("not zero"),
        1,
    );
    assert!(winding_table.is_err());

    // Test case 2: 12/10 double-layer winding with 5 phases. The coil span is 1
    // (tooth-coil winding)
    let winding_table = WindingTable::with_method(
        WindingTableMethod::Tingley,
        NonZeroU16::new(12).expect("not zero"),
        NonZeroU16::new(2).expect("not zero"),
        NonZeroU16::new(5).expect("not zero"),
        NonZeroU16::new(5).expect("not zero"),
        1,
    );
    assert!(winding_table.is_err());

    // Test case 3: 13/10 double-layer winding with 3 phases. The coil span is 1
    // (tooth-coil winding)
    let winding_table = WindingTable::with_method(
        WindingTableMethod::Tingley,
        NonZeroU16::new(13).expect("not zero"),
        NonZeroU16::new(1).expect("not zero"),
        NonZeroU16::new(5).expect("not zero"),
        NonZeroU16::new(3).expect("not zero"),
        1,
    );
    assert!(winding_table.is_err());
}

#[test]
fn test_tingley_shift_zones() {
    {
        // Test case: 12/2 double-layer winding with 3 phases. The coil span is 6
        // (integer slot winding). A zone shift of two is performed, resulting in a
        // doubled zone span
        let mut winding_table = WindingTable::with_method(
            WindingTableMethod::Tingley,
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            6,
        )
        .unwrap();
        winding_table.shift_zones(2);
        let expected_result = WindingTable::from_layer_major(
            [
                1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, -2, -2, -3, -3, -3, -3, -1, -1, -1, -1, -2, -2,
            ]
            .into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case: 12/2 double-layer winding with 3 phases. The coil span is 5
        // (integer slot winding with short pitching). A zone shift of two is
        // performed, resulting in a doubled zone span
        let mut winding_table = WindingTable::with_method(
            WindingTableMethod::Tingley,
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            5,
        )
        .unwrap();
        winding_table.shift_zones(2);
        let expected_result = WindingTable::from_layer_major(
            [
                1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 1, -2, -2, -3, -3, -3, -3, -1, -1, -1, -1, -2, -2,
            ]
            .into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }
}

#[test]
fn test_zone_plan_coil_side_success() {
    {
        // 18/4 single-layer winding with 3 phases.
        let winding_table = WindingTable::with_method(
            WindingTableMethod::CoilSide,
            NonZeroU16::new(18).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1, // gets ignored
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1i32, 1, -3, 2, -1, -1, 3, 3, -2, 1, -3, -3, 2, 2, -1, 3, -2, -2,
            ]
            .into_iter(),
            NonZeroU16::new(18).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // 24/10 single-layer winding with 3 phases.
        let winding_table = WindingTable::with_method(
            WindingTableMethod::CoilSide,
            NonZeroU16::new(24).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1, // gets ignored
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1i32, 2, -1, 3, -2, 1, -3, -1, 3, 1, -3, 2, -1, 3, -2, -3, 2, 3, -2, 1, -3, 2, -1,
                -2,
            ]
            .into_iter(),
            NonZeroU16::new(24).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // 36/10 single-layer winding with 3 phases.
        let winding_table = WindingTable::with_method(
            WindingTableMethod::CoilSide,
            NonZeroU16::new(36).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1, // gets ignored
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1i32, 1, -3, 2, -1, -1, 3, -2, 1, -3, 2, -1, 3, 3, -2, 1, -3, -3, 2, -1, 3, -2, 1,
                -3, 2, 2, -1, 3, -2, -2, 1, -3, 2, -1, 3, -2,
            ]
            .into_iter(),
            NonZeroU16::new(36).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // 6/2 single-layer winding with 3 phases.
        let winding_table = WindingTable::with_method(
            WindingTableMethod::CoilSide,
            NonZeroU16::new(6).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1, // gets ignored
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [1i32, -3, 2, -1, 3, -2].into_iter(),
            NonZeroU16::new(6).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }
}

#[test]
fn test_algebraic_algorithm_success() {
    {
        // Test case 1: 24/10 double-layer winding with 3 phases.
        let winding_table = WindingTable::with_method(
            WindingTableMethod::AlgebraicAlgorithm,
            NonZeroU16::new(24).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            2,
        )
        .unwrap();
        let expected_result = WindingTable::from_slot_major(
            [
                1i32, 1, -3, -3, 2, -1, -1, 3, -2, -2, 1, 1, -3, 2, 2, -1, 3, 3, -2, -2, 1, -3, -3,
                2, -1, -1, 3, 3, -2, 1, 1, -3, 2, 2, -1, -1, 3, -2, -2, 1, -3, -3, 2, 2, -1, 3, 3,
                -2,
            ]
            .into_iter(),
            NonZeroU16::new(24).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case 2: 18/4 single-layer winding with 3 phases.
        let winding_table = WindingTable::with_method(
            WindingTableMethod::AlgebraicAlgorithm,
            NonZeroU16::new(18).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            3,
        )
        .unwrap();
        let expected_result = WindingTable::from_slot_major(
            [
                1i32, -3, -3, -1, 2, 3, 3, -2, -2, -3, 1, 2, 2, -1, -1, -2, 3, 1,
            ]
            .into_iter(),
            NonZeroU16::new(18).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case 3: 12/10 double-layer winding with 3 phases. The coil span is 1
        // (tooth-coil winding)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::AlgebraicAlgorithm,
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1, // gets ignored
        )
        .unwrap();
        let expected_result = WindingTable::from_slot_major(
            [
                1i32, 1, 2, -1, -2, -2, -3, 2, 3, 3, 1, -3, -1, -1, -2, 1, 2, 2, 3, -2, -3, -3, -1,
                3,
            ]
            .into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case 4: 12/10 single-layer winding with 3 phases. The coil span is 1
        // (tooth-coil winding)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::AlgebraicAlgorithm,
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [1, -1, -2, 2, 3, -3, -1, 1, 2, -2, -3, 3].into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case 5: 18/2 double-layer winding with 3 phases. The coil span is 9
        let winding_table = WindingTable::with_method(
            WindingTableMethod::AlgebraicAlgorithm,
            NonZeroU16::new(18).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            9,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1, 1, 1, -3, -3, -3, 2, 2, 2, -1, -1, -1, 3, 3, 3, -2, -2, -2, 1, 1, 1, -3, -3, -3,
                2, 2, 2, -1, -1, -1, 3, 3, 3, -2, -2, -2,
            ]
            .into_iter(),
            NonZeroU16::new(18).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }
}

#[test]
fn test_star_of_slots_success() {
    {
        // Test case 1: 12/10 double-layer winding with 3 phases. The coil span is 1
        // (tooth-coil winding)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::StarOfSlots,
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1, -1, -2, 2, 3, -3, -1, 1, 2, -2, -3, 3, -3, -1, 1, 2, -2, -3, 3, 1, -1, -2, 2, 3,
            ]
            .into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case 2: 12/2 double-layer winding with 3 phases. The coil span is 6
        // (integer slot winding)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::StarOfSlots,
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            6,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1, -3, -3, 2, 2, -1, -1, 3, 3, -2, -2, 1, 1, -3, -3, 2, 2, -1, -1, 3, 3, -2, -2, 1,
            ]
            .into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case 3: 24/10 double-layer winding with 3 phases. The coil span is 2
        // (short pitching)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::StarOfSlots,
            NonZeroU16::new(24).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            2,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1, -3, -1, 3, -2, 1, 2, -1, 3, -2, -3, 2, -1, 3, 1, -3, 2, -1, -2, 1, -3, 2, 3, -2,
                -3, 2, -1, 3, 1, -3, 2, -1, -2, 1, -3, 2, 3, -2, 1, -3, -1, 3, -2, 1, 2, -1, 3, -2,
            ]
            .into_iter(),
            NonZeroU16::new(24).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case 4: 15/10 double-layer winding with 3 phases. The coil span is 1
        // (tooth-coil winding)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::StarOfSlots,
            NonZeroU16::new(15).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1,
        )
        .unwrap();
        let expected_result = WindingTable::from_slot_major(
            [
                1i32, -3, 2, -1, 3, -2, 1i32, -3, 2, -1, 3, -2, 1i32, -3, 2, -1, 3, -2, 1i32, -3,
                2, -1, 3, -2, 1i32, -3, 2, -1, 3, -2,
            ]
            .into_iter(),
            NonZeroU16::new(15).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case 5: 12/10 single-layer winding with 3 phases. The coil span is 1
        // (tooth-coil winding)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::StarOfSlots,
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [1i32, -1, -2, 2, 3, -3, -1, 1, 2, -2, -3, 3].into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case 6: 18/4 single-layer winding with 3 phases.
        let winding_table = WindingTable::with_method(
            WindingTableMethod::StarOfSlots,
            NonZeroU16::new(18).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            4,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1, -3, -3, 2, -1, 3, 3, -2, -2, 1, -3, 2, 2, -1, -1, 3, -2, 1,
            ]
            .into_iter(),
            NonZeroU16::new(18).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case 7: 24/10 single-layer winding with 3 phases.
        let winding_table = WindingTable::with_method(
            WindingTableMethod::StarOfSlots,
            NonZeroU16::new(24).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            2,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1, 2, -1, 3, -2, -3, 2, -1, 3, 1, -3, 2, -1, -2, 1, -3, 2, 3, -2, 1, -3, -1, 3, -2,
            ]
            .into_iter(),
            NonZeroU16::new(24).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case 8: 6/2 single-layer winding with 3 phases.
        let winding_table = WindingTable::with_method(
            WindingTableMethod::StarOfSlots,
            NonZeroU16::new(6).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            3,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [1i32, -3, 2, -1, 3, -2].into_iter(),
            NonZeroU16::new(6).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case 9: 36/10 single-layer winding with 3 phases.
        let winding_table = WindingTable::with_method(
            WindingTableMethod::StarOfSlots,
            NonZeroU16::new(36).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            3,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1, -3, 2, -1, -1, -2, -2, 1, -3, 2, 2, 3, 3, -2, 1, -3, -3, -1, -1, 3, -2, 1, 1, 2,
                2, -1, 3, -2, -2, -3, -3, 2, -1, 3, 3, 1,
            ]
            .into_iter(),
            NonZeroU16::new(36).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }
}

#[test]
fn test_distribution_table() {
    {
        // Test case 1: 12/10 double-layer winding with 3 phases. The coil span is 1
        // (tooth-coil winding)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::DistributionTable,
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1, 2, -2, -3, 3, 1, -1, -2, 2, 3, -3, -1, 1, -1, -2, 2, 3, -3, -1, 1, 2, -2, -3, 3,
            ]
            .into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }
    {
        // Test case 2: 12/2 double-layer winding with 3 phases. The coil span is 6
        // (integer slot winding)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::DistributionTable,
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            6,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1, 1, -3, -3, 2, 2, -1, -1, 3, 3, -2, -2, 1, 1, -3, -3, 2, 2, -1, -1, 3, 3, -2, -2,
            ]
            .into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }
    {
        // Test case 3: 24/10 double-layer winding with 3 phases. The coil span is 2
        // (short pitching)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::DistributionTable,
            NonZeroU16::new(24).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            2,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [
                1, -3, 2, -1, -2, 1, -3, 2, 3, -2, 1, -3, -1, 3, -2, 1, 2, -1, 3, -2, -3, 2, -1, 3,
                1, -3, -1, 3, -2, 1, 2, -1, 3, -2, -3, 2, -1, 3, 1, -3, 2, -1, -2, 1, -3, 2, 3, -2,
            ]
            .into_iter(),
            NonZeroU16::new(24).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }
    {
        // Test case 4: 3/2 double-layer winding with 3 phases. The coil span is 1
        // (tooth-coil winding)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::DistributionTable,
            NonZeroU16::new(3).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [1, 2, 3, -3, -1, -2].into_iter(),
            NonZeroU16::new(3).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }
    {
        // Test case 5: 12/10 single-layer winding with 3 phases. The coil span is 1
        // (tooth-coil winding)
        let winding_table = WindingTable::with_method(
            WindingTableMethod::DistributionTable,
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            1,
        )
        .unwrap();
        let expected_result = WindingTable::from_layer_major(
            [1, 2, -2, -3, 3, 1, -1, -2, 2, 3, -3, -1].into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }
    {
        // Test case 6: 18/4 single-layer winding with 3 phases.
        let winding_table = WindingTable::with_method(
            WindingTableMethod::DistributionTable,
            NonZeroU16::new(18).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            3,
        )
        .unwrap();

        // This result is wrong, but expected. The problem is due to the underlaying
        // algorithm.
        let expected_result = WindingTable::from_layer_major(
            [
                1, 1, -3, 2, 2, -1, 3, 3, -2, 1, -3, -3, 2, -1, -1, 3, -2, -2,
            ]
            .into_iter(),
            NonZeroU16::new(18).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }

    {
        // Test case 7: 36/10 single-layer winding with 3 phases.
        let winding_table = WindingTable::with_method(
            WindingTableMethod::DistributionTable,
            NonZeroU16::new(36).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(5).expect("not zero"),
            NonZeroU16::new(3).expect("not zero"),
            3,
        )
        .unwrap();

        // This result is wrong, but expected. The problem is due to the underlaying
        // algorithm.
        let expected_result = WindingTable::from_layer_major(
            [
                1, 1, -3, 2, -1, 3, -2, -2, 1, -3, 2, -1, 3, 3, -2, 1, -3, 2, -1, -1, 3, -2, 1, -3,
                2, 2, -1, 3, -2, 1, -3, -3, 2, -1, 3, -2,
            ]
            .into_iter(),
            NonZeroU16::new(36).expect("not zero"),
            NonZeroU16::new(1).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
    }
}
