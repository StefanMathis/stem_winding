#[cfg(feature = "cairo")]
mod cairo_tests {

    use cairo_viewport::{SideLength, Viewport, compare_or_create};
    use stem_winding::prelude::*;

    #[test]
    fn test_from_winding() {
        let winding = DistributedWinding::default();
        let core = RotCore::from_winding(&winding);

        // Compare the core area
        let drawable = core.drawable();
        let view = Viewport::from_bounded_entity(&drawable, SideLength::Long(500));
        let path = std::path::Path::new("tests/img/rot_slotted_from_winding.png");
        let callback = move |path: &std::path::Path| {
            return view.write_to_file(path, |cr| {
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.paint()?;
                drawable.draw(cr)
            });
        };
        assert!(compare_or_create(path, &callback, 0.99).is_ok());
    }

    #[test]
    fn test_plot_distributed_sl_arrow() {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: 6.try_into().expect("not zero"),
            pole_pairs: 1.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_span_reduction: 0,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();
        let core = RotCore::from_winding(&winding);

        let zone_config = ZoneConfig::new(
            ZoneBackgroundColor::Phase,
            Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                false, 0.8, None,
            ))),
            true,
        );

        let view = Viewport::from_bounded_entity(&core.drawable(), SideLength::Long(500));
        let path = std::path::Path::new("tests/img/rot_sl_arrow.png");

        let callback = |path: &std::path::Path| {
            return view.write_to_file(path, |cr| {
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.paint()?;

                core.drawable().draw(cr)?;

                for (_, drawable) in winding.zone_drawables(core.as_core_ref(), &zone_config) {
                    drawable.draw(cr)?;
                }
                return Ok(());
            });
        };
        assert!(compare_or_create(path, &callback, 0.99).is_ok());
    }

    #[test]
    fn test_plot_distributed_sl_ampere_turns() {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: 6.try_into().expect("not zero"),
            pole_pairs: 1.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_span_reduction: 0,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();
        let core = RotCore::from_winding(&winding);

        let zone_config = ZoneConfig::new(
            ZoneBackgroundColor::Phase,
            Some(ZoneCenterConfig::AmpereTurns(15.0)),
            true,
        );

        let view = Viewport::from_bounded_entity(&core.drawable(), SideLength::Long(500));
        let path = std::path::Path::new("tests/img/rot_sl_ampere_turns.png");

        let callback = |path: &std::path::Path| {
            return view.write_to_file(path, |cr| {
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.paint()?;

                core.drawable().draw(cr)?;

                for (_, drawable) in winding.zone_drawables(core.as_core_ref(), &zone_config) {
                    drawable.draw(cr)?;
                }
                return Ok(());
            });
        };
        assert!(compare_or_create(path, &callback, 0.99).is_ok());
    }

    #[test]
    fn test_plot_quadruple_layer_arrows() {
        {
            // Single shift
            let winding: QuadrupleLayerToothCoilWinding = QuadrupleLayerToothCoilMinimalBuilder {
                slots: 9.try_into().expect("not zero"),
                pole_pairs: 4.try_into().expect("not zero"),
                phases: 3.try_into().expect("not zero"),
                winding_table_method: WindingTableMethod::Tingley,
                turns_per_slot_side: 3.try_into().expect("not zero"),
                turns_upper_layer_coils: vec![1.try_into().expect("not zero")],
            }
            .try_into()
            .unwrap();
            let core = RotCore::from_winding(&winding);

            let zone_config = ZoneConfig::new(
                ZoneBackgroundColor::Phase,
                Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                    false, 0.8, None,
                ))),
                true,
            );

            let view = Viewport::from_bounded_entity(&core.drawable(), SideLength::Long(500));
            let path = std::path::Path::new("tests/img/rot_quadruple_layer_single_shift.png");

            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, |cr| {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.paint()?;

                    core.drawable().draw(cr)?;

                    for (_, drawable) in winding.zone_drawables(core.as_core_ref(), &zone_config) {
                        drawable.draw(cr)?;
                    }
                    return Ok(());
                });
            };
            assert!(compare_or_create(path, &callback, 0.99).is_ok());
        }
        {
            // Double shift
            let winding: QuadrupleLayerToothCoilWinding = QuadrupleLayerToothCoilMinimalBuilder {
                slots: 9.try_into().expect("not zero"),
                pole_pairs: 4.try_into().expect("not zero"),
                phases: 3.try_into().expect("not zero"),
                winding_table_method: WindingTableMethod::Tingley,
                turns_per_slot_side: 3.try_into().expect("not zero"),
                turns_upper_layer_coils: vec![
                    1.try_into().expect("not zero"),
                    1.try_into().expect("not zero"),
                ],
            }
            .try_into()
            .unwrap();
            let core = RotCore::from_winding(&winding);

            let zone_config = ZoneConfig::new(
                ZoneBackgroundColor::Phase,
                Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                    false, 0.8, None,
                ))),
                true,
            );

            let view = Viewport::from_bounded_entity(&core.drawable(), SideLength::Long(500));
            let path = std::path::Path::new("tests/img/rot_quadruple_layer_double_shift.png");

            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, |cr| {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.paint()?;

                    core.drawable().draw(cr)?;

                    for (_, drawable) in winding.zone_drawables(core.as_core_ref(), &zone_config) {
                        drawable.draw(cr)?;
                    }
                    return Ok(());
                });
            };
            assert!(compare_or_create(path, &callback, 0.99).is_ok());
        }
    }

    #[test]
    fn test_plot_quadruple_layer_ampere_turns() {
        let winding: QuadrupleLayerToothCoilWinding = QuadrupleLayerToothCoilMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 5.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            winding_table_method: WindingTableMethod::Tingley,
            turns_per_slot_side: 4.try_into().expect("not zero"),
            turns_upper_layer_coils: vec![3.try_into().expect("not zero")],
        }
        .try_into()
        .unwrap();
        let core = RotCore::from_winding(&winding);

        {
            // Test with ampere turns
            let zone_config = ZoneConfig::new(
                ZoneBackgroundColor::Phase,
                Some(ZoneCenterConfig::AmpereTurns(15.0)),
                true,
            );

            let view = Viewport::from_bounded_entity(&core.drawable(), SideLength::Long(500));
            let path = std::path::Path::new("tests/img/rot_quadruple_layer_ampere_turns.png");

            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, |cr| {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.paint()?;

                    core.drawable().draw(cr)?;

                    for (_, drawable) in winding.zone_drawables(core.as_core_ref(), &zone_config) {
                        drawable.draw(cr)?;
                    }
                    return Ok(());
                });
            };
            assert!(compare_or_create(path, &callback, 0.99).is_ok());
        }
    }

    #[test]
    fn test_plot_quadruple_layer_current_vector_arrow() {
        let winding: QuadrupleLayerToothCoilWinding = QuadrupleLayerToothCoilMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 5.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            winding_table_method: WindingTableMethod::Tingley,
            turns_per_slot_side: 4.try_into().expect("not zero"),
            turns_upper_layer_coils: vec![3.try_into().expect("not zero")],
        }
        .try_into()
        .unwrap();
        let core = RotCore::from_winding(&winding);

        {
            let zone_config = ZoneConfig::new(
                ZoneBackgroundColor::Phase,
                Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                    false,
                    0.8,
                    Some(vec![0.0, -0.5, 0.5]),
                ))),
                true,
            );

            let view = Viewport::from_bounded_entity(&core.drawable(), SideLength::Long(500));
            let path = std::path::Path::new("tests/img/rot_quadruple_layer_currents.png");

            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, |cr| {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.paint()?;

                    core.drawable().draw(cr)?;

                    for (_, drawable) in winding.zone_drawables(core.as_core_ref(), &zone_config) {
                        drawable.draw(cr)?;
                    }
                    return Ok(());
                });
            };
            assert!(compare_or_create(path, &callback, 0.99).is_ok());
        }
        {
            let zone_config = ZoneConfig::new(
                ZoneBackgroundColor::None,
                Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                    true,
                    1.0,
                    Some(vec![0.0, -0.5, 0.5]),
                ))),
                true,
            );

            let view = Viewport::from_bounded_entity(&core.drawable(), SideLength::Long(500));
            let path = std::path::Path::new("tests/img/rot_quadruple_layer_currents_no_bg.png");

            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, |cr| {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.paint()?;

                    core.drawable().draw(cr)?;

                    for (_, drawable) in winding.zone_drawables(core.as_core_ref(), &zone_config) {
                        drawable.draw(cr)?;
                    }
                    return Ok(());
                });
            };
            assert!(compare_or_create(path, &callback, 0.99).is_ok());
        }
    }
}
