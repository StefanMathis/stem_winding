use approxim;
use cairo_viewport::*;
use magnetic_core::{AirGapSlotted, CarterFactorModel, CoreLin, CoreLinBuilder, IsCoreRef};
use material::Material;
use slot::CoilLayout;
use slot::{SlotTrapezoidSemi, is_slot::angle_bottom_no_slope, is_slot::angle_top_no_slope};
use std::sync::Arc;
use uom::si::f64::*;
use uom::si::length::{meter, millimeter};
use winding::*;
use wire::RoundWire;

fn create_core() -> CoreLin {
    let slot = SlotTrapezoidSemi::new(
        Length::new::<millimeter>(8.0),
        Length::new::<millimeter>(8.0),
        Length::new::<millimeter>(2.0),
        Length::new::<millimeter>(17.75),
        Length::new::<millimeter>(17.0),
        Length::new::<millimeter>(0.75),
        0.0,
        angle_bottom_no_slope(0.0),
        angle_top_no_slope(0.0),
        Length::new::<millimeter>(3.0),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(2.0),
        Length::new::<millimeter>(0.0),
        Length::new::<millimeter>(0.0),
        true,
    )
    .unwrap();

    return CoreLinBuilder {
        height: Length::new::<millimeter>(25.0),
        width: Length::new::<millimeter>(150.0),
        axial_length: Length::new::<millimeter>(100.0),
        axial_coil_overhang: Length::new::<millimeter>(0.0),
        skew_angle: 0.0,
        iron_fill_factor: 1.0,
        material: Arc::new(Material::default()),
        pole_pairs: 5,
        air_gap: Box::new(AirGapSlotted {
            slots: 12,
            starts_in_slot_middle: true,
            carter_factor_model: CarterFactorModel::Bin12,
            slot: Box::new(slot),
        }),
        flux_barrier: None,
    }
    .try_into()
    .unwrap();
}

#[test]
fn test_slot_positions() {
    let core = create_core();
    let mut positions = core.slot_positions();

    // Check all slot positions
    let pos = positions.next().unwrap();
    approxim::assert_abs_diff_eq!(pos.offset.x.get::<meter>(), 0.0, epsilon = 1e-6);
    approxim::assert_abs_diff_eq!(pos.offset.y.get::<meter>(), 0.0, epsilon = 1e-6);
    approxim::assert_abs_diff_eq!(pos.angle, 0.0, epsilon = 1e-6);

    let pos = positions.next().unwrap();
    approxim::assert_abs_diff_eq!(pos.offset.x.get::<meter>(), 0.0125, epsilon = 1e-6);
    approxim::assert_abs_diff_eq!(pos.offset.y.get::<meter>(), 0.0, epsilon = 1e-6);
    approxim::assert_abs_diff_eq!(pos.angle, 0.0, epsilon = 1e-6);

    let pos = positions.next().unwrap();
    approxim::assert_abs_diff_eq!(pos.offset.x.get::<meter>(), 0.0250, epsilon = 1e-6);
    approxim::assert_abs_diff_eq!(pos.offset.y.get::<meter>(), 0.0, epsilon = 1e-6);
    approxim::assert_abs_diff_eq!(pos.angle, 0.0, epsilon = 1e-6);

    let pos = positions.next().unwrap();
    approxim::assert_abs_diff_eq!(pos.offset.x.get::<meter>(), 0.0375, epsilon = 1e-6);
    approxim::assert_abs_diff_eq!(pos.offset.y.get::<meter>(), 0.0, epsilon = 1e-6);
    approxim::assert_abs_diff_eq!(pos.angle, 0.0, epsilon = 1e-6);

    // ... and so on. Exhaust the iterator now
    for _ in 5..13 {
        assert!(positions.next().is_some())
    }
    assert!(positions.next().is_none())
}

#[test]
fn test_plot_winding_shapes_dl_tooth_coil() {
    let core = create_core();
    let winding = ToothCoilWinding::new_minimal(12, 5, 3, 2, WindingTableMethod::Tingley).unwrap();

    let zone_config = ZoneConfig::new(
        ZoneBackgroundColor::Phase,
        Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
            false, 0.9, None,
        ))),
        true,
    );
    let mut drawables = winding.drawables(core.as_lin_or_rot(), &zone_config);
    drawables.push(core.drawable());

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/winding_shapes_dl_tooth_coil.png"); // Always compare to the same reference image
    let callback = move |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr)
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());
}

#[test]
fn test_plot_winding_shapes_sl_tooth_coil() {
    let core = create_core();
    let winding = ToothCoilWinding::new_minimal(12, 5, 3, 1, WindingTableMethod::Tingley).unwrap();

    let zone_config = ZoneConfig::new(
        ZoneBackgroundColor::Phase,
        Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
            false, 0.9, None,
        ))),
        true,
    );
    let mut drawables = winding.drawables(core.as_lin_or_rot(), &zone_config);
    drawables.push(core.drawable());

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/winding_shapes_sl_tooth_coil.png"); // Always compare to the same reference image
    let callback = move |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr)
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());
}

#[test]
fn test_plot_winding_shapes_quadruple_layer() {
    let core = create_core();
    let winding =
        QuadrupleLayerToothCoilWinding::new_minimal(12, 5, 3, 2, vec![], WindingTableMethod::Tingley)
            .unwrap();

    let zone_config = ZoneConfig::new(
        ZoneBackgroundColor::Phase,
        Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
            false, 0.9, None,
        ))),
        true,
    );
    let mut drawables = winding.drawables(core.as_lin_or_rot(), &zone_config);
    drawables.push(core.drawable());

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/winding_shapes_quadruple_layer.png"); // Always compare to the same reference image
    let callback = move |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr)
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());
}

#[test]
fn test_plot_winding_shapes_quadruple_layer_ampere_turns() {
    let core = create_core();
    let winding =
        QuadrupleLayerToothCoilWinding::new_minimal(12, 5, 3, 4, vec![3], WindingTableMethod::Tingley)
            .unwrap();

    let zone_config = ZoneConfig::new(
        ZoneBackgroundColor::Phase,
        Some(ZoneCenterConfig::AmpereTurns),
        true,
    );
    let mut drawables = winding.drawables(core.as_lin_or_rot(), &zone_config);
    drawables.push(core.drawable());

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 800).unwrap();
    let path = std::path::Path::new("img/winding_shapes_quadruple_layer_ampere_turns.png"); // Always compare to the same reference image
    let callback = move |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr)
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());
}

#[test]
fn test_plot_winding_shapes_distributed() {
    let core = create_core();
    let winding =
        DistributedWinding::new_minimal(12, 1, 3, 2, 1, 0, WindingTableMethod::Tingley).unwrap();

    let zone_config = ZoneConfig::new(
        ZoneBackgroundColor::Phase,
        Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
            false, 0.9, None,
        ))),
        true,
    );
    let mut drawables = winding.drawables(core.as_lin_or_rot(), &zone_config);
    drawables.push(core.drawable());

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/winding_shapes_distributed.png"); // Always compare to the same reference image
    let callback = move |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr)
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());
}

#[test]
fn test_from_winding() {
    let winding =
        DistributedWinding::new_minimal(6, 1, 3, 1, 0, 0, WindingTableMethod::CoilSide).unwrap();

    let core = CoreLin::from_winding(&winding);

    let drawable = core.drawable();

    let view = visualization::Viewport::from_bounded_entity(&drawable, 500);

    let path = std::path::Path::new("img/core_lin_slotted_from_winding.png"); // Always compare to the same reference image
    let callback = |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| drawable.draw(cr));
    };
    assert!(compare_or_create(path, &callback).is_ok());
}

#[test]
fn test_double_layer_multi_vs_double_vertical() {
    // Create a coil assembly which is used to derive the shapes
    let mut coils = Coils::with_capacity(4);

    // Left-most coil
    let wire = Box::new(RoundWire::default());

    // First slot
    coils.0.insert(
        Zone::new(0, 0),
        CoilHalf::new(Zone::new(0, 0), true, 3, 2, wire.clone()).into(),
    );
    coils.0.insert(
        Zone::new(0, 1),
        CoilHalf::new(Zone::new(0, 1), true, 1, 1, wire.clone()).into(),
    );

    {
        // CoilLayout::DoubleVertical
        let coil_assembly =
            CoilAssembly::new_minimal(1, 2, 3, 2, coils.clone(), CoilLayout::DoubleVertical)
                .unwrap();

        let core = CoreLin::from_winding(&coil_assembly);

        let zone_config = ZoneConfig::new(
            ZoneBackgroundColor::Phase,
            Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                false, 0.9, None,
            ))),
            true,
        );
        let mut drawables = coil_assembly.drawables(core.as_lin_or_rot(), &zone_config);
        drawables.push(core.drawable());

        let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
        let path = std::path::Path::new("img/double_layer_double_vertical.png"); // Always compare to the same reference image
        let callback = move |path: &std::path::Path| {
            return view.write_to_file(path, &|cr| {
                for drawable in drawables.iter() {
                    drawable.draw(cr)
                }
            });
        };
        assert!(compare_or_create(path, &callback).is_ok());
    }

    {
        // CoilLayout::MultiVertical
        let coil_assembly: CoilAssembly =
            CoilAssembly::new_minimal(1, 2, 3, 2, coils.clone(), CoilLayout::MultiVertical(2))
                .unwrap()
                .into();

        let core = CoreLin::from_winding(&coil_assembly);

        let zone_config = ZoneConfig::new(
            ZoneBackgroundColor::Phase,
            Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                false, 0.9, None,
            ))),
            true,
        );
        let mut drawables = coil_assembly.drawables(core.as_lin_or_rot(), &zone_config);
        drawables.push(core.drawable());

        let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
        let path = std::path::Path::new("img/double_layer_double_vertical.png"); // Always compare to the same reference image
        let callback = move |path: &std::path::Path| {
            return view.write_to_file(path, &|cr| {
                for drawable in drawables.iter() {
                    drawable.draw(cr)
                }
            });
        };
        assert!(compare_or_create(path, &callback).is_ok());
    }
}

#[test]
fn test_four_layer_multi_vertical() {
    // Create a coil assembly which is used to derive the shapes
    let mut coils = Coils::with_capacity(6);

    // Left-most coil
    let wire = Box::new(RoundWire::default());

    // First slot
    coils.0.insert(
        Zone::new(0, 0),
        CoilHalf::new(Zone::new(0, 0), true, 1, 1, wire.clone()).into(),
    );
    coils.0.insert(
        Zone::new(0, 1),
        CoilHalf::new(Zone::new(0, 1), true, 1, 2, wire.clone()).into(),
    );
    coils.0.insert(
        Zone::new(0, 3),
        CoilHalf::new(Zone::new(0, 3), true, 1, 3, wire.clone()).into(),
    );

    let coil_assembly =
        CoilAssembly::new_minimal(1, 2, 3, 4, coils.clone(), CoilLayout::MultiVertical(4)).unwrap();

    let core = CoreLin::from_winding(&coil_assembly);

    {
        let zone_config = ZoneConfig::new(
            ZoneBackgroundColor::Phase,
            Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                false, 0.9, None,
            ))),
            true,
        );
        let mut drawables = coil_assembly.drawables(core.as_lin_or_rot(), &zone_config);
        drawables.push(core.drawable());

        let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
        let path = std::path::Path::new("img/four_layer_multi_vertical_with_core.png"); // Always compare to the same reference image
        let callback = move |path: &std::path::Path| {
            return view.write_to_file(path, &|cr| {
                for drawable in drawables.iter() {
                    drawable.draw(cr)
                }
            });
        };
        assert!(compare_or_create(path, &callback).is_ok());
    }
    {
        let zone_config = ZoneConfig::new(
            ZoneBackgroundColor::Phase,
            Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                false, 0.9, None,
            ))),
            true,
        );
        let drawables = coil_assembly.drawables(core.as_lin_or_rot(), &zone_config);
        let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
        let path = std::path::Path::new("img/four_layer_multi_vertical_show_empty.png"); // Always compare to the same reference image
        let callback = move |path: &std::path::Path| {
            return view.write_to_file(path, &|cr| {
                for drawable in drawables.iter() {
                    drawable.draw(cr)
                }
            });
        };
        assert!(compare_or_create(path, &callback).is_ok());
    }
    {
        let zone_config = ZoneConfig::new(
            ZoneBackgroundColor::Phase,
            Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                false, 0.9, None,
            ))),
            false,
        );
        let drawables = coil_assembly.drawables(core.as_lin_or_rot(), &zone_config);
        let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
        let path = std::path::Path::new("img/four_layer_multi_vertical_hide_empty.png"); // Always compare to the same reference image
        let callback = move |path: &std::path::Path| {
            return view.write_to_file(path, &|cr| {
                for drawable in drawables.iter() {
                    drawable.draw(cr)
                }
            });
        };
        assert!(compare_or_create(path, &callback).is_ok());
    }
}
