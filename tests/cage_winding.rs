use std::num::NonZeroU16;

use approxim;
use stem_winding::prelude::*;

const ONE: NonZeroU16 = NonZeroU16::MIN;

#[test]
fn test_cage_winding_properties() {
    let cage_winding = SquirrelCageWinding::from(SquirrelCageMinimalBuilder {
        slots: 18.try_into().expect("not zero"),
        pole_pairs: ONE,
    });
    assert_eq!(cage_winding.base_winding_count().get(), 1); // Holds true for all cage windings
    assert_eq!(cage_winding.slots().get(), 18); // Holds true for all cage windings
    assert_eq!(cage_winding.phases().get(), 18); // Holds true for all cage windings
    assert_eq!(
        cage_winding.turns_per_phase(ONE),
        num::rational::Ratio::new_raw(1, 2)
    ); // Holds true for all cage windings
    assert_eq!(cage_winding.turns_at(Zone::new(2, 0)), 1); // Holds true for all cage windings

    assert_eq!(cage_winding.phase_at(Zone::new(2, 0)), 3); // Holds true for all cage windings
    assert_eq!(cage_winding.phase_at(Zone::new(3, 0)), 4); // Holds true for all cage windings
}

#[test]
fn test_air_gap_leakage_factor() {
    let winding = SquirrelCageWinding::new(SquirrelCageMinimalBuilder {
        slots: 14.try_into().expect("not zero"),
        pole_pairs: ONE,
    })
    .expect("infallible");
    assert_eq!(winding.phases().get(), 14);
    assert_eq!(winding.base_winding_count().get(), 1);
    assert_eq!(
        winding.turns_per_phase(ONE),
        num::rational::Ratio::new(1, 2)
    ); // Holds true for all cage windings
    approxim::assert_abs_diff_eq!(0.01696, winding.air_gap_leakage_factor(), epsilon = 0.0001);

    let winding = SquirrelCageWinding::from(SquirrelCageMinimalBuilder {
        slots: 28.try_into().expect("not zero"),
        pole_pairs: 2.try_into().expect("not zero"),
    });
    assert_eq!(winding.phases().get(), 28);
    assert_eq!(winding.base_winding_count().get(), 2);
    assert_eq!(
        winding.turns_per_phase(ONE),
        num::rational::Ratio::new(1, 2)
    ); // Holds true for all cage windings
    approxim::assert_abs_diff_eq!(0.01696, winding.air_gap_leakage_factor(), epsilon = 0.0001);

    let winding = SquirrelCageWinding::from(SquirrelCageMinimalBuilder {
        slots: 56.try_into().expect("not zero"),
        pole_pairs: 4.try_into().expect("not zero"),
    });
    assert_eq!(winding.phases().get(), 56);
    assert_eq!(winding.base_winding_count().get(), 4);
    assert_eq!(
        winding.turns_per_phase(ONE),
        num::rational::Ratio::new(1, 2)
    ); // Holds true for all cage windings
    approxim::assert_abs_diff_eq!(0.01696, winding.air_gap_leakage_factor(), epsilon = 0.0001);
}

#[cfg(feature = "serde")]
mod serde_tests {

    use super::*;
    use indoc::indoc;

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

    #[test]
    fn test_deserialize_failed_to_create_winding_table() {
        // Fails no slots have been given
        let yaml = indoc! {"
            ---
            slots: 0
            pole_pairs: 1
            "};

        assert!(yaml_serde::from_str::<SquirrelCageWinding>(yaml).is_err());
    }

    #[test]
    fn test_serialize() {
        let winding = SquirrelCageWinding::from(SquirrelCageMinimalBuilder {
            slots: 14.try_into().expect("not zero"),
            pole_pairs: 1.try_into().expect("not zero"),
        });
        let _ = yaml_serde::to_string(&winding).unwrap();
    }

    #[test]
    fn test_deserialize_min() {
        // Minimal winding
        let yaml = indoc! {"
            ---
            slots: 28
            pole_pairs: 2
            "};

        let winding: SquirrelCageWinding = yaml_serde::from_str(yaml).unwrap();
        approxim::assert_abs_diff_eq!(1.0, winding.winding_factor(ONE, 1.0), epsilon = 1e-6);
    }

    #[test]
    fn test_deserialize() {
        {
            let yaml = indoc! {"
            ---
            slots: 28
            pole_pairs: 2
            end_winding_leakage_coefficient: 0.25
            wire:
                RoundWire:
                    outer_diameter: 1e-3
                    inner_diameter: 0.0
                    insulation_thickness: 0.0
                    conductor_material:
                        name: Copper
                        relative_permeability: 1.0
            end_ring_width: 10e-3
            end_ring_height: 10e-3
            consider_current_displacement: true
            "};

            let winding: SquirrelCageWinding = yaml_serde::from_str(yaml).unwrap();

            approxim::assert_abs_diff_eq!(1.0, winding.winding_factor(ONE, 1.0), epsilon = 1e-6);
            assert_eq!(0.25, winding.end_winding_leakage_coefficient());
        }

        {
            let yaml = indoc! {"
            ---
            slots: 28
            pole_pairs: 2
            end_winding_leakage_coefficient: 0.35
            wire:
                RectangularWire:
                    conductor_material:
                        name: Copper
                        relative_permeability: 1.0
                    height: 5.5e-3
                    width: 6.7e-3
                    insulation_thickness: 0.0
            end_ring_width: 11e-3
            end_ring_height: 11.2e-3
            consider_current_displacement: true
            "};

            let winding: SquirrelCageWinding = yaml_serde::from_str(yaml).unwrap();

            approxim::assert_abs_diff_eq!(1.0, winding.winding_factor(ONE, 1.0), epsilon = 1e-6);
            assert_eq!(0.35, winding.end_winding_leakage_coefficient());
        }
    }
}

#[cfg(feature = "stem_core")]
mod stem_core_tests {

    use std::sync::Arc;

    use super::*;

    #[test]
    fn test_resistance_and_properties() {
        let mut material = Material::default();
        material
            .set_electrical_resistivity(ElectricalResistivity::new::<ohm_meter>(1.7857e-8).into());

        let rotor_winding: SquirrelCageWinding = SquirrelCageBuilder {
            slots: 28.try_into().expect("not zero"),
            pole_pairs: 2.try_into().expect("not zero"),
            end_winding_leakage_coefficient: 0.0,
            wire: Box::new(SffWire::new(Arc::new(material), 1.0, 1.0).unwrap()),
            end_ring_width: Length::new::<millimeter>(11.0),
            end_ring_height: Length::new::<millimeter>(11.2),
            consider_current_displacement: true,
        }
        .try_into()
        .unwrap();

        let slot: SemiTrapezoidSlot = SemiTrapezoidWidthsAndHeightsBuilder {
            bottom_width: Length::new::<millimeter>(6.76),
            bottom_side_width: Length::new::<millimeter>(6.76),
            top_side_width: Length::new::<millimeter>(8.0),
            top_width: Length::new::<millimeter>(1.5),
            opening_width: Length::new::<millimeter>(1.5),
            bottom_height: Length::new::<millimeter>(0.0),
            side_height: Length::new::<millimeter>(6.79 - 0.75 - 0.5),
            top_height: Length::new::<millimeter>(0.5),
            opening_height: Length::new::<millimeter>(0.75),
            bottom_radius: Length::new::<millimeter>(0.0),
            bottom_side_radius: Length::new::<millimeter>(0.0),
            top_radius: Length::new::<millimeter>(0.0),
            top_side_radius: Length::new::<millimeter>(0.0),
            opening_radius: Length::new::<millimeter>(0.0),
            consider_tooth_tip_leakage: true,
        }
        .try_into()
        .unwrap();

        let core: RotCore = RotCoreBuilder {
            air_gap_radius: Length::new::<millimeter>(54.4),
            yoke_radius: Length::new::<millimeter>(19.0),
            axial_length: Length::new::<millimeter>(165.0),
            axial_coil_overhang: Length::new::<millimeter>(0.0),
            iron_fill_factor: 0.95,
            material: Arc::new(Material::default()),
            pole_pairs: 2.try_into().expect("not zero"),
            skew_angle: 0.0,
            air_gap: Box::new(SlottedAirGap {
                slots: 28.try_into().expect("not zero"),
                starts_in_slot_middle: true,
                carter_factor_model: CarterFactorModel::Bin12,
                slot: Box::new(slot),
            }),
            flux_barrier: None,
        }
        .try_into()
        .expect("valid magnetic core");

        approxim::assert_abs_diff_eq!(
            rotor_winding
                .resistance(
                    CoreRef::Rot(&core),
                    NonZeroU16::MIN,
                    &[
                        ThermodynamicTemperature::new::<degree_celsius>(20.0).into(),
                        Frequency::new::<hertz>(50.0).into(),
                    ],
                    &Default::default(),
                )
                .get::<ohm>(),
            8.6414323e-5,
            epsilon = 1e-10
        );

        approxim::assert_abs_diff_eq!(
            rotor_winding
                .slot_leakage_inductance(
                    CoreRef::Rot(&core),
                    NonZeroU16::MIN,
                    Length::new::<millimeter>(1.0),
                    &[],
                    &Default::default(),
                )
                .get::<henry>(),
            2.22086776e-7,
            epsilon = 1e-12
        );
    }
}
