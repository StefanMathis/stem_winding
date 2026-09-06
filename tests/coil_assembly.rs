use magnetic_core::{AirGapSlotted, CoreRotBuilder, IsCoreRef};
use magnetic_core::{CarterFactorModel, CoreRot};
use material::Material;
use slot::{CoilLayout, SlotTrapezoidSemi};
use std::sync::Arc;
use test_database::create_dbm;
use uom::si::electrical_resistance::ohm;
use uom::si::f64::*;
use uom::si::inductance::henry;
use uom::si::length::{meter, millimeter};
use uom::si::volume::cubic_millimeter;
use winding::*;
use wire::{Wire, WireGroup, RoundWire, StrandedWire};

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
        iron_fill_factor: 1.0,
        material: Arc::new(Material::default()),
        pole_pairs: 2,
        skew_angle: 0.0,
        air_gap: Box::new(AirGapSlotted {
            slots: 36,
            starts_in_slot_middle: false,
            carter_factor_model: CarterFactorModel::Bin12,
            slot: Box::new(slot.clone()),
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
    let winding = DistributedWinding::new(
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
    let coil_assembly = CoilAssembly::from(&winding);

    // Check the mean end winding length
    approxim::assert_abs_diff_eq!(
        coil_assembly
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
        coil_assembly
            .resistance(1, core.as_lin_or_rot(), &[], &Default::default(),)
            .get::<ohm>(),
        1.13214, // Expected value in Ohm
        epsilon = 0.0001
    );

    // Slot leakage inductance
    approxim::assert_abs_diff_eq!(
        coil_assembly
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
        coil_assembly
            .end_winding_leakage_inductance(1, core.as_lin_or_rot(), &Default::default())
            .get::<henry>(),
        0.0017129, // Expected value in H
        epsilon = 1e-6
    );
}

#[test]
fn test_build_from_scratch() {
    let mut coil_assembly =
        CoilAssembly::new_minimal(12, 5, 3, 2, Coils::new(), CoilLayout::DoubleHorizontal).unwrap();

    // Add a coil, make sure it's there and then remove it
    let wire = RoundWire::default();
    coil_assembly
        .insert(CoilHalf::new(
            Zone::new(0, 0),
            true,
            1,
            2,
            Box::new(wire.clone()),
        ))
        .unwrap();
    assert!(coil_assembly.coil_at(Zone::new(0, 0)).is_some());
    assert!(coil_assembly.remove(Zone::new(0, 0)).is_some());

    // Add another coil, make sure it's there and then remove it
    let wire = RoundWire::default();
    coil_assembly
        .insert(
            CoilFull::with_positive_and_negative_zones(
                Zone::new(1, 0),
                Zone::new(2, 0),
                true,
                1,
                2,
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
        let mut wires: Vec<Box<dyn Wire>> = Vec::with_capacity(2);
        for _ in 0..2 {
            wires.push(Box::new(RoundWire::default()));
        }
        let winding = DistributedToothCoilWinding::new(
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
        let coil_assembly = CoilAssembly::from(&winding);

        assert_eq!(coil_assembly.phases(), winding.phases());
        assert_eq!(coil_assembly.slots(), winding.slots());
        assert_eq!(coil_assembly.layers(), winding.layers());

        let coils: Vec<Coil> = coil_assembly.coils().cloned().collect();
        assert_eq!(coils.len(), 12);

        // Compare the zone plans
        assert_eq!(winding.winding_table(true), coil_assembly.winding_table(true));

        // Compare the winding factor of phase 1
        approxim::assert_abs_diff_eq!(
            coil_assembly.winding_factor(1, 1.0),
            winding.winding_factor(1, 1.0),
            epsilon = 0.0001
        );
    }

    {
        let winding =
            DistributedToothCoilWinding::new_minimal(48, 10, 3, 1, vec![1, 1], 1, false).unwrap();
        let coil_assembly = CoilAssembly::from(&winding);

        assert_eq!(coil_assembly.phases(), winding.phases());
        assert_eq!(coil_assembly.slots(), winding.slots());
        assert_eq!(coil_assembly.layers(), winding.layers());

        let coils: Vec<Coil> = coil_assembly.coils().cloned().collect();
        assert_eq!(coils.len(), 24);

        // Compare the zone plans
        assert_eq!(winding.winding_table(true), coil_assembly.winding_table(true));

        // Compare the winding factor of phase 1
        approxim::assert_abs_diff_eq!(
            coil_assembly.winding_factor(1, 1.0),
            winding.winding_factor(1, 1.0),
            epsilon = 0.0001
        );
    }

    {
        let turns_per_coil =
            DistributedToothCoilWinding::double_layer_turn_distribution(2, 100, 15).unwrap();
        let winding =
            DistributedToothCoilWinding::new_minimal(12, 1, 3, 2, turns_per_coil, 1, false)
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
        assert_eq!(winding.winding_table(true), coil_assembly.winding_table(true));

        // Compare the winding factor of phase 1
        approxim::assert_abs_diff_eq!(
            coil_assembly.winding_factor(1, 1.0),
            winding.winding_factor(1, 1.0),
            epsilon = 0.0001
        );
    }
}

#[test]
fn test_harmonic_ordinal_and_amplitude() {
    {
        let coil_assembly = {
            let mut coil_assembly = CoilAssembly::new(
                18,
                1,
                3,
                2,
                Default::default(),
                CoilLayout::DoubleVertical,
                0.0,
                1,
                Connection::Star,
            )
            .unwrap();

            // Coil group
            coil_assembly
                .insert(
                    CoilFull::new(
                        Zone { slot: 0, layer: 0 },
                        Zone { slot: 7, layer: 1 },
                        true,
                        true,
                        1,
                        1,
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();
            coil_assembly
                .insert(
                    CoilFull::new(
                        Zone { slot: 1, layer: 0 },
                        Zone { slot: 8, layer: 1 },
                        true,
                        true,
                        1,
                        1,
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();
            coil_assembly
                .insert(
                    CoilFull::new(
                        Zone { slot: 2, layer: 0 },
                        Zone { slot: 9, layer: 1 },
                        true,
                        true,
                        1,
                        1,
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();

            // Coil group
            coil_assembly
                .insert(
                    CoilFull::new(
                        Zone { slot: 6, layer: 0 },
                        Zone { slot: 13, layer: 1 },
                        true,
                        true,
                        1,
                        2,
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();
            coil_assembly
                .insert(
                    CoilFull::new(
                        Zone { slot: 7, layer: 0 },
                        Zone { slot: 14, layer: 1 },
                        true,
                        true,
                        1,
                        2,
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();
            coil_assembly
                .insert(
                    CoilFull::new(
                        Zone { slot: 8, layer: 0 },
                        Zone { slot: 15, layer: 1 },
                        true,
                        true,
                        1,
                        2,
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();

            // Coil group
            coil_assembly
                .insert(
                    CoilFull::new(
                        Zone { slot: 12, layer: 0 },
                        Zone { slot: 1, layer: 1 },
                        true,
                        true,
                        1,
                        3,
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();
            coil_assembly
                .insert(
                    CoilFull::new(
                        Zone { slot: 13, layer: 0 },
                        Zone { slot: 2, layer: 1 },
                        true,
                        true,
                        1,
                        3,
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();
            coil_assembly
                .insert(
                    CoilFull::new(
                        Zone { slot: 14, layer: 0 },
                        Zone { slot: 3, layer: 1 },
                        true,
                        true,
                        1,
                        3,
                        Box::new(RoundWire::default()),
                    )
                    .unwrap(),
                )
                .unwrap();
            coil_assembly
        };

        let mut iter = coil_assembly.harmonic_inductions();

        {
            let (ratio, amp) = iter.next().unwrap();
            assert_eq!(ratio, num::rational::Ratio::new(1, 1));
            approxim::assert_abs_diff_eq!(amp, 0.901912, epsilon = 1e-6);
        }

        {
            let (ratio, amp) = iter.next().unwrap();
            assert_eq!(ratio, num::rational::Ratio::new(-2, 1));
            approxim::assert_abs_diff_eq!(amp, 0.271265, epsilon = 1e-6);
        }

        {
            let (ratio, amp) = iter.next().unwrap();
            assert_eq!(ratio, num::rational::Ratio::new(4, 1));
            approxim::assert_abs_diff_eq!(amp, 0.110568, epsilon = 1e-6);
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
            assert_eq!(ratio, num::rational::Ratio::new(-8, 1));
            approxim::assert_abs_diff_eq!(amp, 0.012531, epsilon = 1e-6);
        }

        {
            let (ratio, amp) = iter.next().unwrap();
            assert_eq!(ratio, num::rational::Ratio::new(10, 1));
            approxim::assert_abs_diff_eq!(amp, 0.010025, epsilon = 1e-6);
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
    }
}
