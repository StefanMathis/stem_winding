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
fn test_harmonics() {
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

        let harmonics: Vec<WindingHarmonic> = winding.harmonics().take(5).collect();
        assert_eq!(
            harmonics[0],
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(8),
                is_positive: true
            }
        );
        assert_eq!(
            harmonics[1],
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(16),
                is_positive: false
            }
        );
        assert_eq!(
            harmonics[2],
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(32),
                is_positive: true
            }
        );
        assert_eq!(
            harmonics[3],
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(40),
                is_positive: false
            }
        );
        assert_eq!(
            harmonics[4],
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(56),
                is_positive: true
            }
        );
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
                spatial_order: SpatialOrder::Mechanical(2),
                is_positive: false
            }
        );
        assert_eq!(
            harmonics[2],
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(4),
                is_positive: true
            }
        );
        assert_eq!(
            harmonics[3],
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(5),
                is_positive: false
            }
        );
        assert_eq!(
            harmonics[4],
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(7),
                is_positive: true
            }
        );
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
