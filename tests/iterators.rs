use std::num::NonZeroU16;

use stem_winding::iterators::*;
use stem_winding::prelude::*;

#[test]
fn test_parallel_paths() {
    let pp_iter: Vec<NonZeroU16> =
        ParallelPathIterator::new(NonZeroU16::new(2).expect("not zero")).collect();
    assert_eq!(
        pp_iter,
        vec![
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(2).expect("not zero")
        ]
    );

    let pp_iter: Vec<NonZeroU16> =
        ParallelPathIterator::new(NonZeroU16::new(3).expect("not zero")).collect();
    assert_eq!(
        pp_iter,
        vec![
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(3).expect("not zero")
        ]
    );

    let pp_iter: Vec<NonZeroU16> =
        ParallelPathIterator::new(NonZeroU16::new(4).expect("not zero")).collect();
    assert_eq!(
        pp_iter,
        vec![
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(4).expect("not zero")
        ]
    );

    let pp_iter: Vec<NonZeroU16> =
        ParallelPathIterator::new(NonZeroU16::new(8).expect("not zero")).collect();
    assert_eq!(
        pp_iter,
        vec![
            NonZeroU16::new(1).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(4).expect("not zero"),
            NonZeroU16::new(8).expect("not zero")
        ]
    );
}

#[test]
fn test_harmonic_orders() {
    {
        let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: NonZeroU16::new(24).expect("not zero"),
            pole_pairs: NonZeroU16::new(10).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            winding_table_constructor: WindingTableConstructor::Tingley,
        }
        .try_into()
        .unwrap();

        let orders: Vec<num::rational::Ratio<i32>> = winding.harmonic_orders().take(5).collect();
        assert_eq!(orders[0], num::rational::Ratio::new(-1, 5));
        assert_eq!(orders[1], num::rational::Ratio::new(5, 5));
        assert_eq!(orders[2], num::rational::Ratio::new(-7, 5));
        assert_eq!(orders[3], num::rational::Ratio::new(11, 5));
        assert_eq!(orders[4], num::rational::Ratio::new(-13, 5));
        assert_eq!(winding.harmonic_orders().coupling(), -1);
    }

    {
        let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: NonZeroU16::new(24).expect("not zero"),
            pole_pairs: NonZeroU16::new(16).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            winding_table_constructor: WindingTableConstructor::Tingley,
        }
        .try_into()
        .unwrap();

        let orders: Vec<num::rational::Ratio<i32>> = winding.harmonic_orders().take(5).collect();
        assert_eq!(orders[0], num::rational::Ratio::new(-1, 2));
        assert_eq!(orders[1], num::rational::Ratio::new(2, 2));
        assert_eq!(orders[2], num::rational::Ratio::new(-4, 2));
        assert_eq!(orders[3], num::rational::Ratio::new(5, 2));
        assert_eq!(orders[4], num::rational::Ratio::new(-7, 2));
        assert_eq!(winding.harmonic_orders().coupling(), -1);
    }

    {
        let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: NonZeroU16::new(9).expect("not zero"),
            pole_pairs: NonZeroU16::new(4).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            winding_table_constructor: WindingTableConstructor::Tingley,
        }
        .try_into()
        .unwrap();

        let orders: Vec<num::rational::Ratio<i32>> = winding.harmonic_orders().take(5).collect();
        assert_eq!(orders[0], num::rational::Ratio::new(1, 4));
        assert_eq!(orders[1], num::rational::Ratio::new(-2, 4));
        assert_eq!(orders[2], num::rational::Ratio::new(4, 4));
        assert_eq!(orders[3], num::rational::Ratio::new(-5, 4));
        assert_eq!(orders[4], num::rational::Ratio::new(7, 4));
        assert_eq!(winding.harmonic_orders().coupling(), 1);
    }

    // From trait object
    {
        let winding_org: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: NonZeroU16::new(9).expect("not zero"),
            pole_pairs: NonZeroU16::new(4).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            winding_table_constructor: WindingTableConstructor::Tingley,
        }
        .try_into()
        .unwrap();

        let winding: &dyn Winding = &winding_org;
        let orders: Vec<num::rational::Ratio<i32>> = winding.harmonic_orders().take(5).collect();
        assert_eq!(orders[0], num::rational::Ratio::new(1, 4));
        assert_eq!(orders[1], num::rational::Ratio::new(-2, 4));
        assert_eq!(orders[2], num::rational::Ratio::new(4, 4));
        assert_eq!(orders[3], num::rational::Ratio::new(-5, 4));
        assert_eq!(orders[4], num::rational::Ratio::new(7, 4));
        assert_eq!(winding.harmonic_orders().coupling(), 1);
    }
}

#[test]
fn test_coils_per_coil_group() {
    {
        let wdg: DistributedWinding = DistributedMinimalBuilder {
            slots: NonZeroU16::new(6).expect("not zero"),
            pole_pairs: NonZeroU16::new(1).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(1).expect("not zero"),
            winding_table_constructor: WindingTableConstructor::Tingley,
            coil_span_reduction: 0,
            zone_span_variation: 0,
        }
        .try_into()
        .unwrap();

        assert_eq!(
            wdg.base_winding_count(),
            NonZeroU16::new(1).expect("not zero")
        );
        assert_eq!(
            wdg.coil_groups_per_phase(),
            NonZeroU16::new(1).expect("not zero")
        );
    }
    {
        let wdg: DistributedWinding = DistributedMinimalBuilder {
            slots: NonZeroU16::new(18).expect("not zero"),
            pole_pairs: NonZeroU16::new(2).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            winding_table_constructor: WindingTableConstructor::Tingley,
            coil_span_reduction: 0,
            zone_span_variation: 0,
        }
        .try_into()
        .unwrap();

        assert_eq!(
            wdg.base_winding_count(),
            NonZeroU16::new(2).expect("not zero")
        );
        assert_eq!(
            wdg.coil_groups_per_phase(),
            NonZeroU16::new(2).expect("not zero")
        );
        assert_eq!(wdg.coils_per_phase(), 6);
        assert_eq!(wdg.coils_per_coil_group(), 3);
    }
    {
        let wdg: DistributedWinding = DistributedMinimalBuilder {
            slots: NonZeroU16::new(18).expect("not zero"),
            pole_pairs: NonZeroU16::new(2).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(1).expect("not zero"),
            winding_table_constructor: WindingTableConstructor::CoilSide,
            coil_span_reduction: 0,
            zone_span_variation: 0,
        }
        .try_into()
        .unwrap();

        assert_eq!(
            wdg.base_winding_count(),
            NonZeroU16::new(1).expect("not zero")
        );
        assert_eq!(
            wdg.coil_groups_per_phase(),
            NonZeroU16::new(1).expect("not zero")
        );
        assert_eq!(wdg.coils_per_phase(), 3);
        assert_eq!(wdg.coils_per_coil_group(), 3);
    }
    {
        let wdg: DistributedWinding = DistributedMinimalBuilder {
            slots: NonZeroU16::new(12).expect("not zero"),
            pole_pairs: NonZeroU16::new(1).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(1).expect("not zero"),
            winding_table_constructor: WindingTableConstructor::Tingley,
            coil_span_reduction: 0,
            zone_span_variation: 0,
        }
        .try_into()
        .unwrap();

        assert_eq!(
            wdg.base_winding_count(),
            NonZeroU16::new(1).expect("not zero")
        );
        assert_eq!(
            wdg.coil_groups_per_phase(),
            NonZeroU16::new(2).expect("not zero")
        );
        assert_eq!(wdg.coils_per_phase(), 2);
        assert_eq!(wdg.coils_per_coil_group(), 1);
    }
    {
        let wdg: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: NonZeroU16::new(12).expect("not zero"),
            pole_pairs: NonZeroU16::new(5).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            winding_table_constructor: WindingTableConstructor::Tingley,
        }
        .try_into()
        .unwrap();

        assert_eq!(
            wdg.base_winding_count(),
            NonZeroU16::new(1).expect("not zero")
        );
        assert_eq!(
            wdg.coil_groups_per_phase(),
            NonZeroU16::new(2).expect("not zero")
        );
    }
    {
        let wdg: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: NonZeroU16::new(12).expect("not zero"),
            pole_pairs: NonZeroU16::new(5).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(1).expect("not zero"),
            winding_table_constructor: WindingTableConstructor::Tingley,
        }
        .try_into()
        .unwrap();

        assert_eq!(
            wdg.base_winding_count(),
            NonZeroU16::new(1).expect("not zero")
        );
        assert_eq!(
            wdg.coil_groups_per_phase(),
            NonZeroU16::new(2).expect("not zero")
        );
    }
    {
        let wdg: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: NonZeroU16::new(12).expect("not zero"),
            pole_pairs: NonZeroU16::new(4).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            winding_table_constructor: WindingTableConstructor::Tingley,
        }
        .try_into()
        .unwrap();

        assert_eq!(
            wdg.base_winding_count(),
            NonZeroU16::new(4).expect("not zero")
        );
        assert_eq!(
            wdg.coil_groups_per_phase(),
            NonZeroU16::new(4).expect("not zero")
        );
    }
}

#[test]
fn test_phase_sequence() {
    {
        let sequence: Vec<_> = PhaseSequence::new(NonZeroU16::new(1).expect("not zero")).collect();
        assert_eq!(sequence, vec![1, -1]);
    }
    {
        let sequence: Vec<_> = PhaseSequence::new(NonZeroU16::new(2).expect("not zero")).collect();
        assert_eq!(sequence, vec![1, -2, 2, -1]);
    }
    {
        let sequence: Vec<_> = PhaseSequence::new(NonZeroU16::new(3).expect("not zero")).collect();
        assert_eq!(sequence, vec![1, -3, 2, -1, 3, -2]);
    }
    {
        let sequence: Vec<_> = PhaseSequence::new(NonZeroU16::new(4).expect("not zero")).collect();
        assert_eq!(sequence, vec![1, -3, 2, -4, 3, -1, 4, -2]);
    }
    {
        let sequence: Vec<_> = PhaseSequence::new(NonZeroU16::new(5).expect("not zero")).collect();
        assert_eq!(sequence, vec![1, -4, 2, -5, 3, -1, 4, -2, 5, -3]);
    }

    // Test the size hint
    let mut iter = PhaseSequence::new(NonZeroU16::new(5).expect("not zero"));
    assert_eq!(iter.size_hint().0, 10);
    iter.next();
    assert_eq!(iter.size_hint().0, 9);
}
