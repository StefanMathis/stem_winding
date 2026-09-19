use std::f64::consts::PI;
use std::num::NonZeroU16;
use std::sync::Arc;

use approxim::assert_abs_diff_eq;

use stem_winding::core_support::*;
use stem_winding::prelude::*;

#[test]
fn test_end_winding_half_turn_length_semicircle() {
    {
        let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: NonZeroU16::new(12).expect("not zero"),
            pole_pairs: NonZeroU16::new(5).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(1).expect("not zero"),
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let core = RotCore::from_winding(&winding);

        for coil in winding.coils() {
            for zone in coil.zones() {
                let len =
                    end_winding_half_turn_length_semicircle(&winding, CoreRef::from(&core), zone)
                        .get::<meter>();
                assert_abs_diff_eq!(len, 0.013964, epsilon = 1e-6);
            }
        }
    }
    {
        let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: NonZeroU16::new(12).expect("not zero"),
            pole_pairs: NonZeroU16::new(5).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let core = RotCore::from_winding(&winding);

        for coil in winding.coils() {
            for zone in coil.zones() {
                let len =
                    end_winding_half_turn_length_semicircle(&winding, CoreRef::from(&core), zone)
                        .get::<meter>();
                assert_abs_diff_eq!(len, 0.009037, epsilon = 1e-6);
            }
        }
    }
}

#[test]
fn test_end_winding_half_turn_length_circular_arc() {
    {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: NonZeroU16::new(12).expect("not zero"),
            pole_pairs: NonZeroU16::new(1).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(1).expect("not zero"),
            coil_span_reduction: 0,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let core = RotCore::from_winding(&winding);
        for coil in winding.coils() {
            for zone in coil.zones() {
                let len =
                    end_winding_half_turn_length_circular_arc(&winding, &core, zone).get::<meter>();
                assert_abs_diff_eq!(len, 0.072185, epsilon = 1e-6);
            }
        }
    }
    {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: NonZeroU16::new(12).expect("not zero"),
            pole_pairs: NonZeroU16::new(1).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            coil_span_reduction: 0,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let core = RotCore::from_winding(&winding);
        for coil in winding.coils() {
            for zone in coil.zones() {
                let len =
                    end_winding_half_turn_length_circular_arc(&winding, &core, zone).get::<meter>();
                assert_abs_diff_eq!(len, 0.068186, epsilon = 1e-6);
            }
        }
    }
    {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: NonZeroU16::new(12).expect("not zero"),
            pole_pairs: NonZeroU16::new(1).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            coil_span_reduction: 1,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let core = RotCore::from_winding(&winding);
        for coil in winding.coils() {
            for zone in coil.zones() {
                let len =
                    end_winding_half_turn_length_circular_arc(&winding, &core, zone).get::<meter>();
                assert_abs_diff_eq!(len, 0.059218, epsilon = 1e-6);
            }
        }
    }
    {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: NonZeroU16::new(12).expect("not zero"),
            pole_pairs: NonZeroU16::new(1).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            coil_span_reduction: 2,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let core = RotCore::from_winding(&winding);
        for coil in winding.coils() {
            for zone in coil.zones() {
                let len =
                    end_winding_half_turn_length_circular_arc(&winding, &core, zone).get::<meter>();
                assert_abs_diff_eq!(len, 0.050251, epsilon = 1e-6);
            }
        }
    }
}

#[test]
fn test_end_winding_half_turn_phd_motor() {
    /*
    Motor from:
    Mathis, S.: Permanentmagneterregte Line-Start-Antriebe in Ferrittechnik,
    PhD thesis, Shaker, 2019, URL:
    <https://kluedo.ub.rptu.de/frontdoor/index/index/docId/8192>
    */

    let winding: DistributedWinding = DistributedMinimalBuilder {
        slots: NonZeroU16::new(36).expect("not zero"),
        pole_pairs: NonZeroU16::new(2).expect("not zero"),
        phases: NonZeroU16::new(3).expect("not zero"),
        layers: NonZeroU16::new(1).expect("not zero"),
        coil_span_reduction: 0,
        zone_span_variation: 0,
        winding_table_method: WindingTableMethod::Tingley,
    }
    .try_into()
    .unwrap();

    let slot_angle = PI / 18.0;
    let bottom_width = Length::new::<millimeter>(9.2);
    let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
        bottom_width,
        opening_width: Length::new::<millimeter>(2.0),
        height: Length::new::<millimeter>(17.75),
        opening_height: Length::new::<millimeter>(2.0),
        slot_angle,
        bottom_radius: Length::new::<millimeter>(2.0),
        top_radius: Length::new::<millimeter>(2.0),
        opening_radius: Length::new::<millimeter>(0.5),
        consider_tooth_tip_leakage: false,
    }
    .try_into()
    .unwrap();

    let core: RotCore = RotCoreBuilder {
        air_gap_radius: Length::new::<millimeter>(55.0),
        yoke_radius: Length::new::<millimeter>(85.0),
        axial_length: Length::new::<millimeter>(165.0),
        axial_coil_overhang: Length::new::<millimeter>(0.0),
        iron_fill_factor: 1.0,
        material: Arc::new(Material::default()),
        pole_pairs: 2.try_into().expect("not zero"),
        skew_angle: 0.0,
        air_gap: Box::new(SlottedAirGap::new(
            36.try_into().expect("not zero"),
            false,
            CarterFactorModel::Bin12,
            Box::new(slot.clone()),
        )),
        flux_barrier: None,
    }
    .try_into()
    .expect("valid magnetic core");

    for coil in winding.coils() {
        for zone in coil.zones() {
            let len_func =
                end_winding_half_turn_length_circular_arc(&winding, &core, zone).get::<meter>();
            let len_method = winding
                .end_winding_half_turn_length(CoreRef::Rot(&core), zone)
                .get::<meter>();
            assert_abs_diff_eq!(len_func, 0.133768, epsilon = 1e-6);
            assert_abs_diff_eq!(len_func, len_method, epsilon = 1e-6);
        }
    }
}

#[test]
fn test_end_winding_half_turn_length_straight() {
    {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: NonZeroU16::new(12).expect("not zero"),
            pole_pairs: NonZeroU16::new(1).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            coil_span_reduction: 0,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::AlgebraicAlgorithm,
        }
        .try_into()
        .unwrap();

        let core = LinCore::from_winding(&winding);
        for coil in winding.coils() {
            for zone in coil.zones() {
                let len =
                    end_winding_half_turn_length_straight(&winding, &core, zone).get::<meter>();
                assert_abs_diff_eq!(len, 0.118980, epsilon = 1e-6);
            }
        }
    }
    {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: NonZeroU16::new(12).expect("not zero"),
            pole_pairs: NonZeroU16::new(1).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            coil_span_reduction: 1,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let core = LinCore::from_winding(&winding);
        for coil in winding.coils() {
            for zone in coil.zones() {
                let len =
                    end_winding_half_turn_length_straight(&winding, &core, zone).get::<meter>();
                if coil.throw(None) == 5 {
                    assert_abs_diff_eq!(len, 0.104079, epsilon = 1e-6);
                } else if coil.throw(None) == 7 {
                    assert_abs_diff_eq!(len, 0.133909, epsilon = 1e-6);
                } else {
                    panic!("Invalid coil throw")
                }
            }
        }
    }
    {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: NonZeroU16::new(12).expect("not zero"),
            pole_pairs: NonZeroU16::new(1).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            coil_span_reduction: 2,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let core = LinCore::from_winding(&winding);
        for coil in winding.coils() {
            for zone in coil.zones() {
                let len =
                    end_winding_half_turn_length_straight(&winding, &core, zone).get::<meter>();
                if coil.throw(None) == 4 {
                    assert_abs_diff_eq!(len, 0.089227, epsilon = 1e-6);
                } else if coil.throw(None) == 8 {
                    assert_abs_diff_eq!(len, 0.148855, epsilon = 1e-6);
                } else {
                    panic!("Invalid coil throw")
                }
            }
        }
    }
    {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: NonZeroU16::new(12).expect("not zero"),
            pole_pairs: NonZeroU16::new(1).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(2).expect("not zero"),
            coil_span_reduction: 6,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let core = LinCore::from_winding(&winding);
        for coil in winding.coils() {
            for zone in coil.zones() {
                let len =
                    end_winding_half_turn_length_straight(&winding, &core, zone).get::<meter>();
                assert_abs_diff_eq!(len, 0.034840, epsilon = 1e-6);
            }
        }
    }
    {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: NonZeroU16::new(12).expect("not zero"),
            pole_pairs: NonZeroU16::new(1).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(1).expect("not zero"),
            coil_span_reduction: 0,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::AlgebraicAlgorithm,
        }
        .try_into()
        .unwrap();

        let core = LinCore::from_winding(&winding);
        for coil in winding.coils() {
            for zone in coil.zones() {
                let len =
                    end_winding_half_turn_length_straight(&winding, &core, zone).get::<meter>();
                assert_abs_diff_eq!(len, 0.126730, epsilon = 1e-6);
            }
        }
    }
    {
        let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: NonZeroU16::new(12).expect("not zero"),
            pole_pairs: NonZeroU16::new(5).expect("not zero"),
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::new(1).expect("not zero"),
            winding_table_method: WindingTableMethod::AlgebraicAlgorithm,
        }
        .try_into()
        .unwrap();

        let core = LinCore::from_winding(&winding);
        for coil in winding.coils() {
            for zone in coil.zones() {
                let len =
                    end_winding_half_turn_length_straight(&winding, &core, zone).get::<meter>();
                assert_abs_diff_eq!(len, 0.051730, epsilon = 1e-6);
            }
        }
    }
}
