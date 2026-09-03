use cairo_viewport::*;
use magnetic_core::{CoreRot, IsCoreRef};
use winding::*;
use winding::{WindingDistributed, WindingQuadrupleLayerToothCoil, WindingTableMethod};

#[test]
fn test_from_winding() {
    let winding = WindingDistributed::default();
    let core = CoreRot::from_winding(&winding);

    // Compare the core area
    let drawable = core.drawable();
    let view = visualization::Viewport::from_bounded_entity(&drawable, 500);
    let path = std::path::Path::new("img/core_rot_slotted_from_default_winding.png"); // Always compare to the same reference image
    let callback = move |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| drawable.draw(cr));
    };
    assert!(compare_or_create(path, &callback).is_ok());
}

#[test]
fn test_winding_shapes_single_layer_arrow() {
    let wdg = WindingDistributed::new_minimal(6, 1, 3, 1, 0, 0, WindingTableMethod::Tingley).unwrap();
    let core = CoreRot::from_winding(&wdg);

    let zone_config = ZoneConfig::new(
        ZoneBackgroundColor::Phase,
        Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
            false, 0.8, None,
        ))),
        true,
    );

    let drawables = wdg.drawables(core.as_lin_or_rot(), &zone_config);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/winding_shapes_single_layer_arrow.png"); // Always compare to the same reference image
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
fn test_winding_shapes_single_layer_ampere_turns() {
    let wdg = WindingDistributed::new_minimal(6, 1, 3, 1, 0, 0, WindingTableMethod::Tingley).unwrap();
    let core = CoreRot::from_winding(&wdg);

    let zone_config = ZoneConfig::new(
        ZoneBackgroundColor::Phase,
        Some(ZoneCenterConfig::AmpereTurns),
        true,
    );

    let drawables = wdg.drawables(core.as_lin_or_rot(), &zone_config);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/winding_shapes_single_layer_ampere_turns.png"); // Always compare to the same reference image
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
fn test_winding_shapes_quadruple_layer() {
    // Single shift
    let wdg =
        WindingQuadrupleLayerToothCoil::new_minimal(9, 4, 3, 3, vec![1], WindingTableMethod::Tingley)
            .unwrap();
    let core = CoreRot::from_winding(&wdg);

    let zone_config = ZoneConfig::new(
        ZoneBackgroundColor::Phase,
        Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
            false, 0.8, None,
        ))),
        true,
    );

    let drawables = wdg.drawables(core.as_lin_or_rot(), &zone_config);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/winding_shapes_quadruple_layer_single_shift.png"); // Always compare to the same reference image
    let callback = move |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr)
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());

    // Double shift
    let wdg = WindingQuadrupleLayerToothCoil::new_minimal(
        9,
        4,
        3,
        3,
        vec![1, 1],
        WindingTableMethod::Tingley,
    )
    .unwrap();
    let core = CoreRot::from_winding(&wdg);

    let drawables = wdg.drawables(core.as_lin_or_rot(), &zone_config);

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
    let path = std::path::Path::new("img/winding_shapes_quadruple_layer_double_shift.png"); // Always compare to the same reference image
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
fn test_plot_winding_shapes_rot_quadruple_layer_ampere_turns() {
    let winding =
        WindingQuadrupleLayerToothCoil::new_minimal(12, 5, 3, 4, vec![3], WindingTableMethod::Tingley)
            .unwrap();
    let core = CoreRot::from_winding(&winding);

    // Test with ampere turns
    let zone_config = ZoneConfig::new(
        ZoneBackgroundColor::Phase,
        Some(ZoneCenterConfig::AmpereTurns),
        true,
    );
    let mut drawables = winding.drawables(core.as_lin_or_rot(), &zone_config);
    drawables.push(core.drawable());

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 800).unwrap();
    let path = std::path::Path::new("img/winding_rot_quad_ampere_turns.png");
    let callback = move |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr)
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());

    // Test with current vector
    let zone_config = ZoneConfig::new(
        ZoneBackgroundColor::Phase,
        Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
            false,
            0.8,
            Some(vec![0.0, -0.5, 0.5]),
        ))),
        true,
    );
    let mut drawables = winding.drawables(core.as_lin_or_rot(), &zone_config);
    drawables.push(core.drawable());

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 800).unwrap();
    let path = std::path::Path::new("img/winding_rot_quad_currents.png"); // Always compare to the same reference image
    let callback = move |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr)
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());

    // Test with current vector
    let zone_config = ZoneConfig::new(
        ZoneBackgroundColor::None,
        Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
            true,
            1.0,
            Some(vec![0.0, -0.5, 0.5]),
        ))),
        true,
    );
    let mut drawables = winding.drawables(core.as_lin_or_rot(), &zone_config);
    drawables.push(core.drawable());

    let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 800).unwrap();
    let path = std::path::Path::new("img/winding_rot_quad_only_arrows.png"); // Always compare to the same reference image
    let callback = move |path: &std::path::Path| {
        return view.write_to_file(path, &|cr| {
            for drawable in drawables.iter() {
                drawable.draw(cr)
            }
        });
    };
    assert!(compare_or_create(path, &callback).is_ok());
}
