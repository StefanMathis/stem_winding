use std::{
    num::{NonZeroU16, NonZeroUsize},
    sync::Arc,
};

use stem_winding::prelude::*;

#[test]
fn test_coil_zones_iterator() {
    {
        // Half coil
        let coil: Coil = HalfCoil::new(
            Zone::new(0, 1),
            true,
            NonZeroUsize::MIN,
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .into();
        let mut iter = coil.zones_and_polarities();
        assert_eq!(
            iter.next(),
            Some(ZoneAndPolarity {
                zone: Zone::new(0, 1),
                is_positive: true
            })
        );
        assert_eq!(iter.next(), None);
    }
    {
        // Half coil
        let coil: Coil = HalfCoil::new(
            Zone::new(2, 1),
            false,
            NonZeroUsize::MIN,
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .into();
        let mut iter = coil.zones_and_polarities();
        assert_eq!(
            iter.next(),
            Some(ZoneAndPolarity {
                zone: Zone::new(2, 1),
                is_positive: false
            })
        );
        assert_eq!(iter.next(), None);
    }
    {
        // Full coil
        let coil: Coil = FullCoil::new(
            Zone::new(2, 1),
            Zone::new(1, 1),
            false,
            NonZeroUsize::MIN,
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .unwrap()
        .into();
        let mut iter = coil.zones_and_polarities();
        assert_eq!(
            iter.next(),
            Some(ZoneAndPolarity {
                zone: Zone::new(2, 1),
                is_positive: true
            })
        );
        assert_eq!(
            iter.next(),
            Some(ZoneAndPolarity {
                zone: Zone::new(1, 1),
                is_positive: false
            })
        );
        assert_eq!(iter.next(), None);
    }
    {
        // Full coil
        let coil: Coil = FullCoil::new(
            Zone::new(3, 2),
            Zone::new(4, 1),
            false,
            NonZeroUsize::MIN,
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .unwrap()
        .into();
        let mut iter = coil.zones_and_polarities();
        assert_eq!(
            iter.next(),
            Some(ZoneAndPolarity {
                zone: Zone::new(3, 2),
                is_positive: true
            })
        );
        assert_eq!(
            iter.next(),
            Some(ZoneAndPolarity {
                zone: Zone::new(4, 1),
                is_positive: false
            })
        );
        assert_eq!(iter.next(), None);
    }
    {
        // Full coil
        let coil: Coil = FullCoil::new(
            Zone::new(3, 2),
            Zone::new(3, 1),
            false,
            NonZeroUsize::MIN,
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .unwrap()
        .into();
        let mut iter = coil.zones_and_polarities();
        assert_eq!(
            iter.next(),
            Some(ZoneAndPolarity {
                zone: Zone::new(3, 2),
                is_positive: true
            })
        );
        assert_eq!(
            iter.next(),
            Some(ZoneAndPolarity {
                zone: Zone::new(3, 1),
                is_positive: false
            })
        );
        assert_eq!(iter.next(), None);
    }
}

#[test]
fn full_coil_same_zones() {
    // The two sides of a full coil cannot occupy the same zones_and_polarities!
    assert!(
        FullCoil::new(
            Zone::new(0, 0),
            Zone::new(0, 0),
            false,
            NonZeroUsize::MIN,
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .is_err()
    );
}

#[test]
fn test_coil_resistance() {
    let mut material: Material = Default::default();
    material.set_electrical_resistivity(ElectricalResistivity::new::<ohm_meter>(1.0).into());
    let wire = RoundWire::new(
        Arc::new(material),
        Length::new::<millimeter>(1.0),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(0.1),
    )
    .unwrap();

    let zone_area = Area::new::<square_meter>(1.0);
    let length = Length::new::<meter>(1.0);
    let conditions = &[];

    let resistance = 1273239.5;
    approxim::assert_abs_diff_eq!(
        wire.resistance(length, zone_area, NonZeroUsize::MIN, conditions)
            .get::<ohm>(),
        resistance,
        epsilon = 1.0
    );

    {
        let coil = FullCoil::new(
            Zone::new(0, 0),
            Zone::new(1, 0),
            true,
            NonZeroUsize::new(2).unwrap(),
            NonZeroU16::MIN,
            Box::new(wire.clone()),
        )
        .unwrap();
        approxim::assert_abs_diff_eq!(
            coil.resistance(zone_area, length, conditions).get::<ohm>(),
            usize::from(coil.turns()) as f64 * resistance,
            epsilon = 1.0
        );
    }
    {
        let coil = FullCoil::new(
            Zone::new(0, 0),
            Zone::new(1, 0),
            true,
            NonZeroUsize::new(10).unwrap(),
            NonZeroU16::MIN,
            Box::new(wire.clone()),
        )
        .unwrap();
        approxim::assert_abs_diff_eq!(
            coil.resistance(zone_area, length, conditions).get::<ohm>(),
            usize::from(coil.turns()) as f64 * resistance,
            epsilon = 1.0
        );

        // Slot filling factor
        let slot_filling_factor_el = coil
            .wire()
            .slot_fill_factor_conductor(zone_area, NonZeroUsize::new(10).unwrap());
        let slot_filling_factor_mech = coil
            .wire()
            .slot_fill_factor_overall(zone_area, NonZeroUsize::new(10).unwrap());

        let wire_sff = SffWire::new(
            wire.material_arc().clone(),
            slot_filling_factor_el,
            slot_filling_factor_mech,
        )
        .unwrap();

        let coil_sff = FullCoil::new(
            Zone::new(0, 0),
            Zone::new(1, 0),
            true,
            NonZeroUsize::new(10).unwrap(),
            NonZeroU16::MIN,
            Box::new(wire_sff),
        )
        .unwrap();
        approxim::assert_abs_diff_eq!(
            coil_sff
                .resistance(zone_area, length, conditions)
                .get::<ohm>(),
            usize::from(coil_sff.turns()) as f64 * resistance,
            epsilon = 1.0
        );
    }
}

#[test]
fn test_covered_slots() {
    {
        let coil = FullCoil::new(
            Zone::new(0, 0),
            Zone::new(1, 0),
            true,
            NonZeroUsize::new(2).unwrap(),
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .unwrap();
        let mut covered = coil.covered_slots(Some(NonZeroU16::new(6).unwrap()));
        assert_eq!(covered.next(), Some(0));
        assert_eq!(covered.next(), Some(1));
        assert_eq!(covered.next(), None);
    }
    {
        let coil = FullCoil::new(
            Zone::new(0, 0),
            Zone::new(1, 0),
            false,
            NonZeroUsize::new(2).unwrap(),
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .unwrap();
        let mut covered = coil.covered_slots(Some(NonZeroU16::new(6).unwrap()));
        assert_eq!(covered.next(), Some(0));
        assert_eq!(covered.next(), Some(5));
        assert_eq!(covered.next(), Some(4));
        assert_eq!(covered.next(), Some(3));
        assert_eq!(covered.next(), Some(2));
        assert_eq!(covered.next(), Some(1));
        assert_eq!(covered.next(), None);
    }
    {
        let coil = FullCoil::new(
            Zone::new(0, 0),
            Zone::new(1, 0),
            true,
            NonZeroUsize::new(2).unwrap(),
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .unwrap();
        let mut covered = coil.covered_slots(Some(NonZeroU16::new(6).unwrap()));
        assert_eq!(covered.next(), Some(0));
        assert_eq!(covered.next(), Some(1));
        assert_eq!(covered.next(), None);
    }
    {
        let coil = FullCoil::new(
            Zone::new(0, 0),
            Zone::new(1, 0),
            false,
            NonZeroUsize::new(2).unwrap(),
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .unwrap();
        let mut covered = coil.covered_slots(Some(NonZeroU16::new(6).unwrap()));
        assert_eq!(covered.next(), Some(0));
        assert_eq!(covered.next(), Some(5));
        assert_eq!(covered.next(), Some(4));
        assert_eq!(covered.next(), Some(3));
        assert_eq!(covered.next(), Some(2));
        assert_eq!(covered.next(), Some(1));
        assert_eq!(covered.next(), None);
    }
    {
        let coil = FullCoil::new(
            Zone::new(0, 0),
            Zone::new(1, 0),
            true,
            NonZeroUsize::new(2).unwrap(),
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .unwrap();
        let mut covered = coil.covered_slots(None);
        assert_eq!(covered.next(), Some(0));
        assert_eq!(covered.next(), Some(1));
        assert_eq!(covered.next(), None);
    }
    {
        let coil = FullCoil::new(
            Zone::new(0, 0),
            Zone::new(1, 0),
            false,
            NonZeroUsize::new(2).unwrap(),
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .unwrap();
        let mut covered = coil.covered_slots(None);
        assert_eq!(covered.next(), Some(0));
        assert_eq!(covered.next(), Some(1));
        assert_eq!(covered.next(), None);
    }
    {
        let coil = FullCoil::new(
            Zone::new(1, 0),
            Zone::new(0, 0),
            true,
            NonZeroUsize::new(2).unwrap(),
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .unwrap();
        let mut covered = coil.covered_slots(None);
        assert_eq!(covered.next(), Some(1));
        assert_eq!(covered.next(), Some(0));
        assert_eq!(covered.next(), None);
    }
    {
        let coil = FullCoil::new(
            Zone::new(1, 0),
            Zone::new(0, 0),
            false,
            NonZeroUsize::new(2).unwrap(),
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .unwrap();
        let mut covered = coil.covered_slots(None);
        assert_eq!(covered.next(), Some(1));
        assert_eq!(covered.next(), Some(0));
        assert_eq!(covered.next(), None);
    }
}

#[test]
fn test_coil_orientation_cyclic() {
    // Tooth coil
    let coil = FullCoil::new(
        Zone::new(0, 0),
        Zone::new(1, 0),
        true,
        NonZeroUsize::MIN,
        NonZeroU16::MIN,
        Box::new(RoundWire::default()),
    )
    .unwrap();
    assert_eq!(coil.throw(Some(NonZeroU16::new(12).expect("not zero"))), 1);
    let coil = FullCoil::new(
        Zone::new(1, 0),
        Zone::new(0, 0),
        false,
        NonZeroUsize::MIN,
        NonZeroU16::MIN,
        Box::new(RoundWire::default()),
    )
    .unwrap();
    assert_eq!(coil.throw(Some(NonZeroU16::new(12).expect("not zero"))), 1);

    // Another tooth coil which wraps around (11 -> 0)
    let coil = FullCoil::new(
        Zone::new(11, 0),
        Zone::new(0, 0),
        true,
        NonZeroUsize::MIN,
        NonZeroU16::MIN,
        Box::new(RoundWire::default()),
    )
    .unwrap();
    assert_eq!(coil.throw(Some(NonZeroU16::new(12).expect("not zero"))), 1);
    let coil = FullCoil::new(
        Zone::new(11, 0),
        Zone::new(0, 0),
        false,
        NonZeroUsize::MIN,
        NonZeroU16::MIN,
        Box::new(RoundWire::default()),
    )
    .unwrap();
    assert_eq!(coil.throw(Some(NonZeroU16::new(12).expect("not zero"))), 11);
    let coil = FullCoil::new(
        Zone::new(0, 0),
        Zone::new(11, 0),
        true,
        NonZeroUsize::MIN,
        NonZeroU16::MIN,
        Box::new(RoundWire::default()),
    )
    .unwrap();
    assert_eq!(coil.throw(Some(NonZeroU16::new(12).expect("not zero"))), 11);
    let coil = FullCoil::new(
        Zone::new(0, 0),
        Zone::new(11, 0),
        false,
        NonZeroUsize::MIN,
        NonZeroU16::MIN,
        Box::new(RoundWire::default()),
    )
    .unwrap();
    assert_eq!(coil.throw(Some(NonZeroU16::new(12).expect("not zero"))), 1);

    // Both coil zones occupy the same slot
    let coil = FullCoil::new(
        Zone::new(0, 0),
        Zone::new(0, 1),
        true,
        NonZeroUsize::MIN,
        NonZeroU16::MIN,
        Box::new(RoundWire::default()),
    )
    .unwrap();
    assert_eq!(coil.throw(Some(NonZeroU16::new(12).expect("not zero"))), 0);

    // Both coil zones occupy the same slot
    let coil = FullCoil::new(
        Zone::new(0, 0),
        Zone::new(0, 1),
        false,
        NonZeroUsize::MIN,
        NonZeroU16::MIN,
        Box::new(RoundWire::default()),
    )
    .unwrap();
    assert_eq!(coil.throw(Some(NonZeroU16::new(12).expect("not zero"))), 12);
}

#[test]
fn test_coil_orientation_linear() {
    // Tooth coil
    let coil = FullCoil::new(
        Zone::new(0, 0),
        Zone::new(1, 0),
        true,
        NonZeroUsize::MIN,
        NonZeroU16::MIN,
        Box::new(RoundWire::default()),
    )
    .unwrap();
    assert_eq!(coil.throw(None), 1);
    let coil = FullCoil::new(
        Zone::new(1, 0),
        Zone::new(0, 0),
        false,
        NonZeroUsize::MIN,
        NonZeroU16::MIN,
        Box::new(RoundWire::default()),
    )
    .unwrap();
    assert_eq!(coil.throw(None), 1);

    // Another tooth coil which wraps around (11 -> 0)
    let coil = FullCoil::new(
        Zone::new(11, 0),
        Zone::new(0, 0),
        true,
        NonZeroUsize::MIN,
        NonZeroU16::MIN,
        Box::new(RoundWire::default()),
    )
    .unwrap();
    assert_eq!(coil.throw(None), 11);
    let coil = FullCoil::new(
        Zone::new(11, 0),
        Zone::new(0, 0),
        false,
        NonZeroUsize::MIN,
        NonZeroU16::MIN,
        Box::new(RoundWire::default()),
    )
    .unwrap();
    assert_eq!(coil.throw(None), 11);
    let coil = FullCoil::new(
        Zone::new(0, 0),
        Zone::new(11, 0),
        true,
        NonZeroUsize::MIN,
        NonZeroU16::MIN,
        Box::new(RoundWire::default()),
    )
    .unwrap();
    assert_eq!(coil.throw(None), 11);
    let coil = FullCoil::new(
        Zone::new(0, 0),
        Zone::new(11, 0),
        false,
        NonZeroUsize::MIN,
        NonZeroU16::MIN,
        Box::new(RoundWire::default()),
    )
    .unwrap();
    assert_eq!(coil.throw(None), 11);

    // Both coil zones occupy the same slot
    let coil = FullCoil::new(
        Zone::new(0, 0),
        Zone::new(0, 1),
        true,
        NonZeroUsize::MIN,
        NonZeroU16::MIN,
        Box::new(RoundWire::default()),
    )
    .unwrap();
    assert_eq!(coil.throw(None), 0);

    // Both coil zones occupy the same slot
    let coil = FullCoil::new(
        Zone::new(0, 0),
        Zone::new(0, 1),
        false,
        NonZeroUsize::MIN,
        NonZeroU16::MIN,
        Box::new(RoundWire::default()),
    )
    .unwrap();
    assert_eq!(coil.throw(None), 0);
}
