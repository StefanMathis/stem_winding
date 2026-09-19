use std::num::{NonZeroU16, NonZeroUsize};

use approxim;
use stem_winding::prelude::*;

const ONE: NonZeroU16 = NonZeroU16::MIN;

#[test]
fn test_build_from_scratch() {
    let mut coil_assembly = CoilAssembly::new_minimal(
        12.try_into().expect("not zero"),
        5.try_into().expect("not zero"),
        3.try_into().expect("not zero"),
        CoilLayout::DoubleHorizontal,
        Coils::new(),
    )
    .unwrap();

    // Add a coil, make sure it's there and then remove it
    let wire = RoundWire::default();
    coil_assembly
        .insert(HalfCoil::new(
            Zone::new(0, 0),
            true,
            1.try_into().expect("not zero"),
            2.try_into().expect("not zero"),
            Box::new(wire.clone()),
        ))
        .unwrap();
    assert!(coil_assembly.coil_at(Zone::new(0, 0)).is_some());
    assert!(coil_assembly.remove(Zone::new(0, 0)).is_some());

    // Add another coil, make sure it's there and then remove it
    let wire = RoundWire::default();
    coil_assembly
        .insert(
            FullCoil::new(
                Zone::new(1, 0),
                Zone::new(2, 0),
                true,
                1.try_into().expect("not zero"),
                2.try_into().expect("not zero"),
                Box::new(wire.clone()),
            )
            .unwrap(),
        )
        .unwrap();
    assert!(coil_assembly.coil_at(Zone::new(1, 0)).is_some());
    assert!(coil_assembly.coil_at(Zone::new(2, 0)).is_some());
    assert!(coil_assembly.remove(Zone::new(2, 0)).is_some());
    assert!(coil_assembly.coil_at(Zone::new(2, 0)).is_none());
    assert!(coil_assembly.coil_at(Zone::new(1, 0)).is_none());
}

#[test]
fn test_derive_from_winding() {
    {
        let mut wires: Vec<(NonZeroUsize, Box<dyn Wire>)> = Vec::with_capacity(2);
        for turns in 2..4 {
            wires.push((
                NonZeroUsize::new(turns).unwrap(),
                Box::new(RoundWire::default()),
            ));
        }
        let winding: DistributedToothCoilWinding = DistributedToothCoilBuilder {
            slots: 24.try_into().expect("not zero"),
            pole_pairs: 4.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            wires,
            parallel_paths: 1.try_into().expect("not zero"),
            double_zone_span: true,
            connection: Connection::Star,
            end_winding_leakage_coefficient: 0.0,
        }
        .try_into()
        .unwrap();
        let coil_assembly = CoilAssembly::from(&winding);

        assert_eq!(coil_assembly.phases(), winding.phases());
        assert_eq!(coil_assembly.slots(), winding.slots());
        assert_eq!(coil_assembly.layers(), winding.layers());

        let coils: Vec<Coil> = coil_assembly.coils().cloned().collect();
        assert_eq!(coils.len(), 12);

        // Compare the zone plans
        assert_eq!(
            winding.winding_table(true),
            coil_assembly.winding_table(true)
        );

        // Compare the winding factor of phase 1
        approxim::assert_abs_diff_eq!(
            coil_assembly.winding_factor(ONE, 1.0),
            winding.winding_factor(ONE, 1.0),
            epsilon = 0.0001
        );
    }

    {
        let winding: DistributedToothCoilWinding = DistributedToothCoilMinimalBuilder {
            slots: 48.try_into().expect("not zero"),
            pole_pairs: 10.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_group_turns: vec![NonZeroUsize::MIN, NonZeroUsize::MIN],
            parallel_paths: 1.try_into().expect("not zero"),
            double_zone_span: false,
        }
        .try_into()
        .unwrap();
        let coil_assembly = CoilAssembly::from(&winding);

        assert_eq!(coil_assembly.phases(), winding.phases());
        assert_eq!(coil_assembly.slots(), winding.slots());
        assert_eq!(coil_assembly.layers(), winding.layers());

        let coils: Vec<Coil> = coil_assembly.coils().cloned().collect();
        assert_eq!(coils.len(), 24);

        // Compare the zone plans
        assert_eq!(
            winding.winding_table(true),
            coil_assembly.winding_table(true)
        );

        // Compare the winding factor of phase 1
        approxim::assert_abs_diff_eq!(
            coil_assembly.winding_factor(ONE, 1.0),
            winding.winding_factor(ONE, 1.0),
            epsilon = 0.0001
        );
    }

    {
        let coil_group_turns = DistributedToothCoilWinding::double_layer_turn_distribution(
            2,
            100.try_into().unwrap(),
            15,
        )
        .unwrap();

        let winding: DistributedToothCoilWinding = DistributedToothCoilMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 1.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            coil_group_turns,
            parallel_paths: 1.try_into().expect("not zero"),
            double_zone_span: false,
        }
        .try_into()
        .unwrap();
        let coil_assembly = CoilAssembly::from(&winding);

        assert_eq!(coil_assembly.phases(), winding.phases());
        assert_eq!(coil_assembly.slots(), winding.slots());
        assert_eq!(coil_assembly.layers(), winding.layers());
        assert_eq!(
            coil_assembly.turns_at(Zone::new(1, 1)),
            winding.turns_at(Zone::new(1, 1))
        );

        let coils: Vec<Coil> = coil_assembly.coils().cloned().collect();
        assert_eq!(coils.len(), 12);

        // Compare the zone plans
        assert_eq!(
            winding.winding_table(true),
            coil_assembly.winding_table(true)
        );

        // Compare the winding factor of phase 1
        approxim::assert_abs_diff_eq!(
            coil_assembly.winding_factor(ONE, 1.0),
            winding.winding_factor(ONE, 1.0),
            epsilon = 0.0001
        );
    }
}

#[test]
fn test_base_winding_count() {
    {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: 18.try_into().expect("not zero"),
            pole_pairs: 3.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            coil_span_reduction: 1,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let coil_assembly = CoilAssembly::from(&winding);
        assert_eq!(
            winding.base_winding_count(),
            coil_assembly.base_winding_count()
        );
        assert_eq!(
            coil_assembly.base_winding_count(),
            NonZeroU16::new(3).expect("not zero")
        );

        // As trait object
        let trait_obj = &coil_assembly as &dyn Winding;
        assert_eq!(
            trait_obj.base_winding_count(),
            NonZeroU16::new(3).expect("not zero")
        );
    }
    {
        let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: 24.try_into().expect("not zero"),
            pole_pairs: 10.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let coil_assembly = CoilAssembly::from(&winding);
        assert_eq!(
            winding.base_winding_count(),
            coil_assembly.base_winding_count()
        );
        assert_eq!(
            coil_assembly.base_winding_count(),
            NonZeroU16::new(2).expect("not zero")
        );
    }
}

#[test]
fn test_harmonic_ordinal_and_amplitude() {
    {
        let coil_assembly = {
            let mut coil_assembly = CoilAssembly::new(
                18.try_into().expect("not zero"),
                1.try_into().expect("not zero"),
                3.try_into().expect("not zero"),
                CoilLayout::DoubleVertical,
                Default::default(),
                0.0,
                1.try_into().expect("not zero"),
                Connection::Star,
            )
            .unwrap();

            // Coil group
            coil_assembly
                .insert(
                    FullCoil::new(
                        Zone { slot: 0, layer: 0 },
                        Zone { slot: 7, layer: 1 },
                        true,
                        1.try_into().expect("not zero"),
                        1.try_into().expect("not zero"),
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();
            coil_assembly
                .insert(
                    FullCoil::new(
                        Zone { slot: 1, layer: 0 },
                        Zone { slot: 8, layer: 1 },
                        true,
                        1.try_into().expect("not zero"),
                        1.try_into().expect("not zero"),
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();
            coil_assembly
                .insert(
                    FullCoil::new(
                        Zone { slot: 2, layer: 0 },
                        Zone { slot: 9, layer: 1 },
                        true,
                        1.try_into().expect("not zero"),
                        1.try_into().expect("not zero"),
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();

            // Coil group
            coil_assembly
                .insert(
                    FullCoil::new(
                        Zone { slot: 6, layer: 0 },
                        Zone { slot: 13, layer: 1 },
                        true,
                        1.try_into().expect("not zero"),
                        2.try_into().expect("not zero"),
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();
            coil_assembly
                .insert(
                    FullCoil::new(
                        Zone { slot: 7, layer: 0 },
                        Zone { slot: 14, layer: 1 },
                        true,
                        1.try_into().expect("not zero"),
                        2.try_into().expect("not zero"),
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();
            coil_assembly
                .insert(
                    FullCoil::new(
                        Zone { slot: 8, layer: 0 },
                        Zone { slot: 15, layer: 1 },
                        true,
                        1.try_into().expect("not zero"),
                        2.try_into().expect("not zero"),
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();

            // Coil group
            coil_assembly
                .insert(
                    FullCoil::new(
                        Zone { slot: 12, layer: 0 },
                        Zone { slot: 1, layer: 1 },
                        true,
                        1.try_into().expect("not zero"),
                        3.try_into().expect("not zero"),
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();
            coil_assembly
                .insert(
                    FullCoil::new(
                        Zone { slot: 13, layer: 0 },
                        Zone { slot: 2, layer: 1 },
                        true,
                        1.try_into().expect("not zero"),
                        3.try_into().expect("not zero"),
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();
            coil_assembly
                .insert(
                    FullCoil::new(
                        Zone { slot: 14, layer: 0 },
                        Zone { slot: 3, layer: 1 },
                        true,
                        1.try_into().expect("not zero"),
                        3.try_into().expect("not zero"),
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();
            coil_assembly
        };

        let mut iter = coil_assembly.harmonic_inductions();

        {
            let (r, amp) = iter.next().unwrap();
            assert_eq!(r, num::rational::Ratio::new(1, 1));
            approxim::assert_abs_diff_eq!(amp, 0.901912, epsilon = 1e-6);
        }

        {
            let (r, amp) = iter.next().unwrap();
            assert_eq!(r, num::rational::Ratio::new(-2, 1));
            approxim::assert_abs_diff_eq!(amp, 0.271265, epsilon = 1e-6);
        }

        {
            let (r, amp) = iter.next().unwrap();
            assert_eq!(r, num::rational::Ratio::new(4, 1));
            approxim::assert_abs_diff_eq!(amp, 0.110568, epsilon = 1e-6);
        }

        {
            let (r, amp) = iter.next().unwrap();
            assert_eq!(r, num::rational::Ratio::new(-5, 1));
            approxim::assert_abs_diff_eq!(amp, 0.007556, epsilon = 1e-6);
        }

        {
            let (r, amp) = iter.next().unwrap();
            assert_eq!(r, num::rational::Ratio::new(7, 1));
            approxim::assert_abs_diff_eq!(amp, 0.019409, epsilon = 1e-6);
        }

        {
            let (r, amp) = iter.next().unwrap();
            assert_eq!(r, num::rational::Ratio::new(-8, 1));
            approxim::assert_abs_diff_eq!(amp, 0.012531, epsilon = 1e-6);
        }

        {
            let (r, amp) = iter.next().unwrap();
            assert_eq!(r, num::rational::Ratio::new(10, 1));
            approxim::assert_abs_diff_eq!(amp, 0.010025, epsilon = 1e-6);
        }

        {
            let (r, amp) = iter.next().unwrap();
            assert_eq!(r, num::rational::Ratio::new(-11, 1));
            approxim::assert_abs_diff_eq!(amp, 0.0123516, epsilon = 1e-6);
        }

        {
            let (r, amp) = iter.next().unwrap();
            assert_eq!(r, num::rational::Ratio::new(13, 1));
            approxim::assert_abs_diff_eq!(amp, 0.002906, epsilon = 1e-6);
        }
    }
}

#[cfg(feature = "serde")]
mod serde_tests {

    use super::*;

    #[test]
    fn test_serialize_and_deserialize() {
        let winding = CoilAssembly::from(&DistributedWinding::default());

        let string = yaml_serde::to_string(&winding).expect("can be serialized");
        let winding_de: CoilAssembly = yaml_serde::from_str(&string).expect("can be deserialized");

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

    fn create_dbm() -> DatabaseManager {
        return DatabaseManager::open("tests", SerdeYaml).expect("must exist");
    }

    fn create_core_trap() -> RotCore {
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

        return RotCoreBuilder {
            air_gap_radius: Length::new::<millimeter>(55.0),
            yoke_radius: Length::new::<millimeter>(85.0),
            axial_length: Length::new::<millimeter>(165.0),
            axial_coil_overhang: Length::new::<millimeter>(0.0),
            iron_fill_factor: 0.95,
            material: Arc::new(Material::default()),
            pole_pairs: 2.try_into().expect("not zero"),
            skew_angle: 0.0,
            air_gap: Box::new(SlottedAirGap::new(
                36.try_into().expect("not zero"),
                true,
                CarterFactorModel::Bin12,
                Box::new(slot),
            )),
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
            WireGroup::new(Box::new(wire_1), 2.try_into().unwrap()),
            WireGroup::new(Box::new(wire_2), 3.try_into().unwrap()),
        ];
        let wire = StrandedWire::new(strand_list).unwrap();
        let winding: DistributedWinding = DistributedBuilder {
            slots: 36.try_into().expect("not zero"),
            pole_pairs: 2.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_span_reduction: 0,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::Tingley,
            turns_per_coil: 31.try_into().expect("not zero"),
            parallel_paths: 1.try_into().expect("not zero"),
            connection: Connection::Star,
            end_winding_leakage_coefficient: 0.25,
            wire: Box::new(wire.clone()),
            concentric_coils: false,
        }
        .try_into()
        .unwrap();
        let coil_assembly = CoilAssembly::from(&winding);

        // Check the mean end winding length
        approxim::assert_abs_diff_eq!(
            coil_assembly
                .end_winding_half_turn_length(CoreRef::Rot(&core), Zone::new(0, 0),)
                .get::<meter>(),
            0.13376,
            epsilon = 0.0001
        );

        // Check the wire volume
        approxim::assert_abs_diff_eq!(
            winding
                .coil_properties(CoreRef::Rot(&core), &Default::default())
                .map(|cp| cp.volume())
                .sum::<Volume>()
                .get::<cubic_millimeter>(),
            20359.228154,
            epsilon = 0.0001
        );

        // Check the phase resistance
        approxim::assert_abs_diff_eq!(
            coil_assembly
                .resistance(
                    CoreRef::Rot(&core),
                    NonZeroU16::MIN,
                    &[],
                    &Default::default(),
                )
                .get::<ohm>(),
            1.04848,
            epsilon = 0.0001
        );

        // Slot leakage inductance
        approxim::assert_abs_diff_eq!(
            coil_assembly
                .slot_leakage_inductance(
                    CoreRef::Rot(&core),
                    NonZeroU16::MIN,
                    Length::new::<millimeter>(1.0),
                    &[],
                    &Default::default(),
                )
                .get::<henry>(),
            0.0044122,
            epsilon = 1e-6
        );

        approxim::assert_abs_diff_eq!(
            coil_assembly
                .end_winding_leakage_inductance(CoreRef::Rot(&core), NonZeroU16::MIN, None,)
                .get::<henry>(),
            0.0,
            epsilon = 1e-6
        );
    }
}
