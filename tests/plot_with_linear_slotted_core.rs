#[cfg(feature = "cairo")]
mod cairo_tests {

    use std::{num::NonZeroU16, sync::Arc};

    use cairo_viewport::{SideLength, Viewport, compare_or_create};
    use stem_winding::prelude::*;

    fn create_core() -> LinCore {
        let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
            bottom_width: Length::new::<millimeter>(8.0),
            opening_width: Length::new::<millimeter>(2.0),
            height: Length::new::<millimeter>(17.75),
            opening_height: Length::new::<millimeter>(0.75),
            slot_angle: 0.0,
            bottom_radius: Length::new::<millimeter>(3.0),
            top_radius: Length::new::<millimeter>(2.0),
            opening_radius: Length::new::<millimeter>(0.0),
            consider_tooth_tip_leakage: false,
        }
        .try_into()
        .unwrap();

        return LinCoreBuilder {
            height: Length::new::<millimeter>(25.0),
            width: Length::new::<millimeter>(150.0),
            axial_length: Length::new::<millimeter>(100.0),
            axial_coil_overhang: Length::new::<millimeter>(0.0),
            skew_angle: 0.0,
            iron_fill_factor: 1.0,
            material: Arc::new(Material::default()),
            pole_pairs: NonZeroU16::new(5).unwrap(),
            air_gap: Box::new(SlottedAirGap {
                slots: NonZeroU16::new(12).unwrap(),
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
    fn test_plot_tooth_coil_dl() {
        let core = create_core();
        let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 5.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let zone_config = ZoneConfig::new(
            ZoneBackgroundColor::Phase,
            Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                false, 0.8, None,
            ))),
            true,
        );

        let view = Viewport::from_bounded_entity(&core.drawable(), SideLength::Long(500));
        let path = std::path::Path::new("tests/img/lin_tooth_coil_dl.png");

        let callback = |path: &std::path::Path| {
            return view.write_to_file(path, |cr| {
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.paint()?;

                core.drawable().draw(cr)?;

                for (_, drawable) in winding.drawables(core.as_core_ref(), &zone_config) {
                    drawable.draw(cr)?;
                }
                return Ok(());
            });
        };
        assert!(compare_or_create(path, &callback, 0.99).is_ok());
    }

    #[test]
    fn test_plot_tooth_coil_sl() {
        let core = create_core();
        let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 5.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let zone_config = ZoneConfig::new(
            ZoneBackgroundColor::Phase,
            Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                false, 0.8, None,
            ))),
            true,
        );

        let view = Viewport::from_bounded_entity(&core.drawable(), SideLength::Long(500));
        let path = std::path::Path::new("tests/img/lin_tooth_coil_sl.png");

        let callback = |path: &std::path::Path| {
            return view.write_to_file(path, |cr| {
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.paint()?;

                core.drawable().draw(cr)?;

                for (_, drawable) in winding.drawables(core.as_core_ref(), &zone_config) {
                    drawable.draw(cr)?;
                }
                return Ok(());
            });
        };
        assert!(compare_or_create(path, &callback, 0.99).is_ok());
    }

    #[test]
    fn test_plot_quadruple_layer_default() {
        let core = create_core();
        let winding: QuadrupleLayerToothCoilWinding = QuadrupleLayerToothCoilMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 5.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            winding_table_method: WindingTableMethod::Tingley,
            turns_per_slot_side: 2.try_into().expect("not zero"),
            turns_upper_layer_coils: Vec::new(),
        }
        .try_into()
        .unwrap();

        let zone_config = ZoneConfig::new(
            ZoneBackgroundColor::Phase,
            Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                false, 0.8, None,
            ))),
            true,
        );

        let view = Viewport::from_bounded_entity(&core.drawable(), SideLength::Long(500));
        let path = std::path::Path::new("tests/img/lin_quadruple_layer_default.png");

        let callback = |path: &std::path::Path| {
            return view.write_to_file(path, |cr| {
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.paint()?;

                core.drawable().draw(cr)?;

                for (_, drawable) in winding.drawables(core.as_core_ref(), &zone_config) {
                    drawable.draw(cr)?;
                }
                return Ok(());
            });
        };
        assert!(compare_or_create(path, &callback, 0.99).is_ok());
    }

    #[test]
    fn test_plot_quadruple_layer_ampere_turns() {
        let core = create_core();
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

        let zone_config = ZoneConfig::new(
            ZoneBackgroundColor::Phase,
            Some(ZoneCenterConfig::AmpereTurns(15.0)),
            true,
        );

        let view = Viewport::from_bounded_entity(&core.drawable(), SideLength::Long(800));
        let path = std::path::Path::new("tests/img/lin_quadruple_layer_ampere_turns.png");

        let callback = |path: &std::path::Path| {
            return view.write_to_file(path, |cr| {
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.paint()?;

                core.drawable().draw(cr)?;

                for (_, drawable) in winding.drawables(core.as_core_ref(), &zone_config) {
                    drawable.draw(cr)?;
                }
                return Ok(());
            });
        };
        assert!(compare_or_create(path, &callback, 0.99).is_ok());
    }

    #[test]
    fn test_plot_distributed_dl() {
        let core = create_core();
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 1.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            coil_span_reduction: 1,
            zone_span_variation: 0,
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .unwrap();

        let zone_config = ZoneConfig::new(
            ZoneBackgroundColor::Phase,
            Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                false, 0.8, None,
            ))),
            true,
        );

        let view = Viewport::from_bounded_entity(&core.drawable(), SideLength::Long(500));
        let path = std::path::Path::new("tests/img/lin_distributed_dl.png");

        let callback = |path: &std::path::Path| {
            return view.write_to_file(path, |cr| {
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.paint()?;

                core.drawable().draw(cr)?;

                for (_, drawable) in winding.drawables(core.as_core_ref(), &zone_config) {
                    drawable.draw(cr)?;
                }
                return Ok(());
            });
        };
        assert!(compare_or_create(path, &callback, 0.99).is_ok());
    }

    #[test]
    fn test_from_winding() {
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

        let core = LinCore::from_winding(&winding);

        let drawable = core.drawable();

        let view = Viewport::from_bounded_entity(&drawable, SideLength::Long(500));

        let path = std::path::Path::new("tests/img/lin_slotted_from_winding.png");
        let callback = |path: &std::path::Path| {
            return view.write_to_file(path, |cr| {
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.paint()?;
                drawable.draw(cr)
            });
        };
        assert!(compare_or_create(path, &callback, 0.99).is_ok());
    }

    #[test]
    fn test_double_layer_multi_vs_double_vertical() {
        // Create a coil assembly which is used to derive the shapes
        let mut coils = Coils::with_capacity(4);

        let wire = Box::new(RoundWire::default());

        // First slot
        coils.0.insert(
            Zone::new(0, 0),
            HalfCoil::new(
                Zone::new(0, 0),
                true,
                3.try_into().expect("not zero"),
                2.try_into().expect("not zero"),
                wire.clone(),
            )
            .into(),
        );
        coils.0.insert(
            Zone::new(0, 1),
            HalfCoil::new(
                Zone::new(0, 1),
                true,
                1.try_into().expect("not zero"),
                1.try_into().expect("not zero"),
                wire.clone(),
            )
            .into(),
        );

        {
            // CoilLayout::DoubleVertical
            let coil_assembly = CoilAssembly::new_minimal(
                1.try_into().expect("not zero"),
                2.try_into().expect("not zero"),
                3.try_into().expect("not zero"),
                CoilLayout::DoubleVertical,
                coils.clone(),
            )
            .unwrap();

            let core = LinCore::from_winding(&coil_assembly);

            let zone_config = ZoneConfig::new(
                ZoneBackgroundColor::Phase,
                Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                    false, 0.8, None,
                ))),
                true,
            );

            let view = Viewport::from_bounded_entity(&core.drawable(), SideLength::Long(500));
            let path = std::path::Path::new("tests/img/lin_double_layer_double_vertical.png");

            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, |cr| {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.paint()?;

                    core.drawable().draw(cr)?;

                    for (_, drawable) in coil_assembly.drawables(core.as_core_ref(), &zone_config) {
                        drawable.draw(cr)?;
                    }
                    return Ok(());
                });
            };
            assert!(compare_or_create(path, &callback, 0.99).is_ok());
        }

        {
            // CoilLayout::MultiVertical
            let coil_assembly = CoilAssembly::new_minimal(
                1.try_into().expect("not zero"),
                2.try_into().expect("not zero"),
                3.try_into().expect("not zero"),
                CoilLayout::MultiVertical(2.try_into().expect("not zero")),
                coils.clone(),
            )
            .unwrap();

            let core = LinCore::from_winding(&coil_assembly);

            let zone_config = ZoneConfig::new(
                ZoneBackgroundColor::Phase,
                Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                    false, 0.8, None,
                ))),
                true,
            );

            let view = Viewport::from_bounded_entity(&core.drawable(), SideLength::Long(500));
            let path = std::path::Path::new("tests/img/lin_double_layer_double_vertical.png");

            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, |cr| {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.paint()?;

                    core.drawable().draw(cr)?;

                    for (_, drawable) in coil_assembly.drawables(core.as_core_ref(), &zone_config) {
                        drawable.draw(cr)?;
                    }
                    return Ok(());
                });
            };
            assert!(compare_or_create(path, &callback, 0.99).is_ok());
        }
    }

    #[test]
    fn test_four_layer_multi_vertical() {
        // Create a coil assembly which is used to derive the shapes
        let mut coils = Coils::with_capacity(4);

        // Left-most coil
        let wire = Box::new(RoundWire::default());

        // First slot
        coils.0.insert(
            Zone::new(0, 0),
            HalfCoil::new(
                Zone::new(0, 0),
                true,
                1.try_into().expect("not zero"),
                1.try_into().expect("not zero"),
                wire.clone(),
            )
            .into(),
        );
        coils.0.insert(
            Zone::new(0, 1),
            HalfCoil::new(
                Zone::new(0, 1),
                true,
                1.try_into().expect("not zero"),
                2.try_into().expect("not zero"),
                wire.clone(),
            )
            .into(),
        );
        coils.0.insert(
            Zone::new(0, 3),
            HalfCoil::new(
                Zone::new(0, 3),
                true,
                1.try_into().expect("not zero"),
                3.try_into().expect("not zero"),
                wire.clone(),
            )
            .into(),
        );

        let coil_assembly = CoilAssembly::new_minimal(
            1.try_into().expect("not zero"),
            2.try_into().expect("not zero"),
            3.try_into().expect("not zero"),
            CoilLayout::MultiVertical(4.try_into().expect("not zero")),
            coils.clone(),
        )
        .unwrap();

        let core = LinCore::from_winding(&coil_assembly);

        {
            let zone_config = ZoneConfig::new(
                ZoneBackgroundColor::Phase,
                Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                    false, 0.9, None,
                ))),
                true,
            );

            let view = Viewport::from_bounded_entity(&core.drawable(), SideLength::Long(500));
            let path =
                std::path::Path::new("tests/img/lin_four_layer_multi_vertical_show_empty.png");

            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, |cr| {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.paint()?;

                    core.drawable().draw(cr)?;

                    for (_, drawable) in coil_assembly.drawables(core.as_core_ref(), &zone_config) {
                        drawable.draw(cr)?;
                    }
                    return Ok(());
                });
            };
            assert!(compare_or_create(path, &callback, 0.99).is_ok());
        }
        {
            let zone_config = ZoneConfig::new(
                ZoneBackgroundColor::Phase,
                Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                    false, 0.9, None,
                ))),
                false,
            );

            let view = Viewport::from_bounded_entity(&core.drawable(), SideLength::Long(500));
            let path =
                std::path::Path::new("tests/img/lin_four_layer_multi_vertical_hide_empty.png");

            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, |cr| {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.paint()?;

                    core.drawable().draw(cr)?;

                    for (_, drawable) in coil_assembly.drawables(core.as_core_ref(), &zone_config) {
                        drawable.draw(cr)?;
                    }
                    return Ok(());
                });
            };
            assert!(compare_or_create(path, &callback, 0.99).is_ok());
        }
    }
}
