use indoc::indoc;
use test_database::create_dbm;
use winding::*;

#[test]
fn test_cage_winding_properties() {
    let cage_winding = SquirrelCageWinding::new_minimal(18, 1).unwrap();
    assert_eq!(cage_winding.periodicity(), 1); // Holds true for all cage windings
    assert_eq!(cage_winding.slots(), 18); // Holds true for all cage windings
    assert_eq!(cage_winding.phases(), 18); // Holds true for all cage windings
    assert_eq!(
        cage_winding.turns_per_phase(1),
        num::rational::Ratio::new_raw(1, 2)
    ); // Holds true for all cage windings
    assert_eq!(cage_winding.turns_at(Zone::new(2, 0)), 1); // Holds true for all cage windings

    assert_eq!(cage_winding.phase_at(Zone::new(2, 0)).unwrap(), 3); // Holds true for all cage windings
    assert_eq!(cage_winding.phase_at(Zone::new(3, 0)).unwrap(), 4); // Holds true for all cage windings
}

#[test]
fn test_air_gap_leakage_factor() {
    let winding = SquirrelCageWinding::new_minimal(14, 1).unwrap();
    assert_eq!(winding.phases(), 14);
    assert_eq!(winding.periodicity(), 1);
    assert_eq!(winding.turns_per_phase(1), num::rational::Ratio::new(1, 2)); // Holds true for all cage windings
    approxim::assert_abs_diff_eq!(0.01696, winding.air_gap_leakage_factor(), epsilon = 0.0001);

    let winding = SquirrelCageWinding::new_minimal(28, 2).unwrap();
    assert_eq!(winding.phases(), 28);
    assert_eq!(winding.periodicity(), 2);
    assert_eq!(winding.turns_per_phase(1), num::rational::Ratio::new(1, 2)); // Holds true for all cage windings
    approxim::assert_abs_diff_eq!(0.01696, winding.air_gap_leakage_factor(), epsilon = 0.0001);

    let winding = SquirrelCageWinding::new_minimal(56, 4).unwrap();
    assert_eq!(winding.phases(), 56);
    assert_eq!(winding.periodicity(), 4);
    assert_eq!(winding.turns_per_phase(1), num::rational::Ratio::new(1, 2)); // Holds true for all cage windings
    approxim::assert_abs_diff_eq!(0.01696, winding.air_gap_leakage_factor(), epsilon = 0.0001);
}

#[test]
fn test_deserialize_failed_to_create_winding_table() {
    // Fails no slots have been given
    let yaml = indoc! {"
          ---
          slots: 0
          pole_pairs: 1
          "};

    let maybe_winding: std::io::Result<SquirrelCageWinding> = create_dbm().from_str(yaml);
    assert!(maybe_winding.is_err());
}

#[test]
fn test_serialize() {
    let winding = SquirrelCageWinding::new_minimal(14, 1).unwrap();
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

    let winding: SquirrelCageWinding = create_dbm().from_str(yaml).unwrap();

    approxim::assert_abs_diff_eq!(1.0, winding.winding_factor(1, 1.0), epsilon = 1e-6);
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
                material_conductor:
                    file_name: Copper
        end_ring_width: 10e-3
        end_ring_height: 10e-3
        consider_current_displacement: true
        "};

        let winding: SquirrelCageWinding = create_dbm().from_str(yaml).unwrap();

        approxim::assert_abs_diff_eq!(1.0, winding.winding_factor(1, 1.0), epsilon = 1e-6);
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
                material_conductor:
                    file_name: Copper
                height: 5.5e-3
                width: 6.7e-3
                insulation_thickness: 0.0
        end_ring_width: 11e-3
        end_ring_height: 11.2e-3
        consider_current_displacement: true
        "};

        let winding: SquirrelCageWinding = create_dbm().from_str(yaml).unwrap();

        approxim::assert_abs_diff_eq!(1.0, winding.winding_factor(1, 1.0), epsilon = 1e-6);
        assert_eq!(0.35, winding.end_winding_leakage_coefficient());
    }
}
