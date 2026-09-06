use std::num::NonZeroU16;

use stem_winding::iterators::ParallelPathIterator;
use stem_winding::prelude::*;
use stem_winding::variants::distributed::DistributedMinimalBuilder;
use stem_winding::variants::tooth_coil::ToothCoilMinimalBuilder;

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
fn test_harmonic_ordinals() {
    {
        let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: NonZeroU16::new(24).expect("not zero"),
            pole_pairs: NonZeroU16::new(10).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let ordinals: Vec<num::rational::Ratio<i32>> =
            winding.harmonic_ordinals().take(5).collect();
        assert_eq!(ordinals[0], num::rational::Ratio::new(-1, 5));
        assert_eq!(ordinals[1], num::rational::Ratio::new(5, 5));
        assert_eq!(ordinals[2], num::rational::Ratio::new(-7, 5));
        assert_eq!(ordinals[3], num::rational::Ratio::new(11, 5));
        assert_eq!(ordinals[4], num::rational::Ratio::new(-13, 5));
        assert_eq!(winding.harmonic_ordinals().coupling(), -1);
    }

    {
        let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: NonZeroU16::new(24).expect("not zero"),
            pole_pairs: NonZeroU16::new(16).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let ordinals: Vec<num::rational::Ratio<i32>> =
            winding.harmonic_ordinals().take(5).collect();
        assert_eq!(ordinals[0], num::rational::Ratio::new(-1, 2));
        assert_eq!(ordinals[1], num::rational::Ratio::new(2, 2));
        assert_eq!(ordinals[2], num::rational::Ratio::new(-4, 2));
        assert_eq!(ordinals[3], num::rational::Ratio::new(5, 2));
        assert_eq!(ordinals[4], num::rational::Ratio::new(-7, 2));
        assert_eq!(winding.harmonic_ordinals().coupling(), -1);
    }

    {
        let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: NonZeroU16::new(9).expect("not zero"),
            pole_pairs: NonZeroU16::new(4).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let ordinals: Vec<num::rational::Ratio<i32>> =
            winding.harmonic_ordinals().take(5).collect();
        assert_eq!(ordinals[0], num::rational::Ratio::new(1, 4));
        assert_eq!(ordinals[1], num::rational::Ratio::new(-2, 4));
        assert_eq!(ordinals[2], num::rational::Ratio::new(4, 4));
        assert_eq!(ordinals[3], num::rational::Ratio::new(-5, 4));
        assert_eq!(ordinals[4], num::rational::Ratio::new(7, 4));
        assert_eq!(winding.harmonic_ordinals().coupling(), 1);
    }

    // From trait object
    {
        let winding_org: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: NonZeroU16::new(9).expect("not zero"),
            pole_pairs: NonZeroU16::new(4).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let winding: &dyn Winding = &winding_org;
        let ordinals: Vec<num::rational::Ratio<i32>> =
            winding.harmonic_ordinals().take(5).collect();
        assert_eq!(ordinals[0], num::rational::Ratio::new(1, 4));
        assert_eq!(ordinals[1], num::rational::Ratio::new(-2, 4));
        assert_eq!(ordinals[2], num::rational::Ratio::new(4, 4));
        assert_eq!(ordinals[3], num::rational::Ratio::new(-5, 4));
        assert_eq!(ordinals[4], num::rational::Ratio::new(7, 4));
        assert_eq!(winding.harmonic_ordinals().coupling(), 1);
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
            winding_table_method: WindingTableMethod::Tingley,
            coil_span_reduction: 0,
            zone_span_variation: 0,
        }
        .try_into()
        .unwrap();

        assert_eq!(wdg.periodicity(), NonZeroU16::new(1).expect("not zero"));
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
            winding_table_method: WindingTableMethod::Tingley,
            coil_span_reduction: 0,
            zone_span_variation: 0,
        }
        .try_into()
        .unwrap();

        assert_eq!(wdg.periodicity(), NonZeroU16::new(2).expect("not zero"));
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
            winding_table_method: WindingTableMethod::CoilSide,
            coil_span_reduction: 0,
            zone_span_variation: 0,
        }
        .try_into()
        .unwrap();

        assert_eq!(wdg.periodicity(), NonZeroU16::new(1).expect("not zero"));
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
            winding_table_method: WindingTableMethod::Tingley,
            coil_span_reduction: 0,
            zone_span_variation: 0,
        }
        .try_into()
        .unwrap();

        assert_eq!(wdg.periodicity(), NonZeroU16::new(1).expect("not zero"));
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
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        assert_eq!(wdg.periodicity(), NonZeroU16::new(1).expect("not zero"));
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
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        assert_eq!(wdg.periodicity(), NonZeroU16::new(1).expect("not zero"));
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
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        assert_eq!(wdg.periodicity(), NonZeroU16::new(4).expect("not zero"));
        assert_eq!(
            wdg.coil_groups_per_phase(),
            NonZeroU16::new(4).expect("not zero")
        );
    }
}
