#[cfg(feature = "cairo")]
mod cairo_tests {

    use std::{f64::consts::PI, num::NonZeroU16};

    use cairo_viewport::*;
    use stem_core::planar_geo::draw::{Drawable, Style};
    use stem_winding::prelude::*;

    #[test]
    fn test_coil() {
        let params = CoilDrawablesParameters {
            arrowhead_length: 0.5,
            line_width: 1.0,
            slot_width: 1.0,
            tooth_width: 1.0,
            axial_length: 4.0,
            axial_coil_overhang: 0.8,
            arrow_head_height: 1.0,
            start_height: 0.0,
            delta_empty_zone: 0.1,
            font_size: 14,
            end_winding_style: EndWindingStyle::Pointed {
                end_winding_coil_angle: 30.0 / 180.0 * PI,
            },
            draw_both_sides: true,
            cyclic: true,
        };

        {
            let x_outward = 2.5;
            let x_return = 3.5;

            let style = Style::default();
            let drawables = params.coil_drawable_from_coordinates(
                NonZeroU16::new(6).unwrap(),
                NonZeroU16::new(1).unwrap(),
                x_outward,
                x_return,
                style.clone(),
                true,
                0,
                None,
            );

            let view = Viewport::from_bounded_entities(
                drawables.iter().filter_map(|d| d.as_ref()),
                SideLength::Long(500),
            )
            .unwrap();
            let path = std::path::Path::new("tests/img/end_winding_coil_1.png");
            let callback = move |path: &std::path::Path| {
                return view.write_to_file(path, |cr| {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.paint()?;

                    for drawable in drawables.iter().filter_map(|d| d.as_ref()) {
                        drawable.draw(cr)?;
                    }
                    return Ok(());
                });
            };
            assert!(compare_or_create(path, &callback, 0.99).is_ok());
        }
        {
            let x_outward = 2.5;
            let x_return = 1.0;
            let drawables = params.coil_drawable_from_coordinates(
                NonZeroU16::new(6).unwrap(),
                NonZeroU16::new(1).unwrap(),
                x_outward,
                x_return,
                Style::default(),
                true,
                0,
                None,
            );

            let view = Viewport::from_bounded_entities(
                drawables.iter().filter_map(|d| d.as_ref()),
                SideLength::Long(500),
            )
            .unwrap();
            let path = std::path::Path::new("tests/img/end_winding_coil_2.png");
            let callback = move |path: &std::path::Path| {
                return view.write_to_file(path, |cr| {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.paint()?;

                    for drawable in drawables.iter().filter_map(|d| d.as_ref()) {
                        drawable.draw(cr)?;
                    }
                    return Ok(());
                });
            };
            assert!(compare_or_create(path, &callback, 0.99).is_ok());
        }
        {
            let x_outward = 2.5;
            let x_return = -0.5;
            let drawables = params.coil_drawable_from_coordinates(
                NonZeroU16::new(6).unwrap(),
                NonZeroU16::new(1).unwrap(),
                x_outward,
                x_return,
                Style::default(),
                true,
                0,
                None,
            );

            let view = Viewport::from_bounded_entities(
                drawables.iter().filter_map(|d| d.as_ref()),
                SideLength::Long(500),
            )
            .unwrap();
            let path = std::path::Path::new("tests/img/end_winding_coil_3.png");
            let callback = move |path: &std::path::Path| {
                return view.write_to_file(path, |cr| {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.paint()?;

                    for drawable in drawables.iter().filter_map(|d| d.as_ref()) {
                        drawable.draw(cr)?;
                    }
                    return Ok(());
                });
            };
            assert!(compare_or_create(path, &callback, 0.99).is_ok());
        }
        {
            let x_outward = 2.5;
            let x_return = 0.0;
            let drawables = params.coil_drawable_from_coordinates(
                NonZeroU16::new(6).unwrap(),
                NonZeroU16::new(1).unwrap(),
                x_outward,
                x_return,
                Style::default(),
                true,
                0,
                None,
            );

            let view = Viewport::from_bounded_entities(
                drawables.iter().filter_map(|d| d.as_ref()),
                SideLength::Long(500),
            )
            .unwrap();
            let path = std::path::Path::new("tests/img/end_winding_coil_4.png");
            let callback = move |path: &std::path::Path| {
                return view.write_to_file(path, |cr| {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.paint()?;

                    for drawable in drawables.iter().filter_map(|d| d.as_ref()) {
                        drawable.draw(cr)?;
                    }
                    return Ok(());
                });
            };
            assert!(compare_or_create(path, &callback, 0.99).is_ok());
        }
    }

    fn check<W: Winding>(winding: &W, params: &CoilDrawablesParameters, name: &str) {
        let mut drawables: Vec<Drawable> = (0..(winding.slots().get() + 1))
            .map(|slot| params.tooth_drawable(slot))
            .collect();
        for d in winding.coil_drawables(&params).map(|t| t.1) {
            drawables.push(d);
        }

        let bb1 = BoundingBox::from_bounded_entities(drawables.iter()).unwrap();
        let bb2 = params.bounding_box(winding, true);
        assert!(bb1.approx_eq(&bb2, 1e-10));

        let view = Viewport::from_bounding_box(&bb1, SideLength::Long(2000));
        let path = std::path::Path::new(&name);
        let callback = |path: &std::path::Path| {
            return view.write_to_file(path, |cr| {
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.paint()?;

                for drawable in drawables.iter() {
                    drawable.draw(cr)?;
                }
                return Ok(());
            });
        };
        assert!(compare_or_create(path, &callback, 0.99).is_ok());

        // Test increasing zone index
        let mut current_zone = Zone { slot: 0, layer: 0 };
        for (zone, _) in winding.coil_drawables(&params) {
            assert!(zone >= current_zone);
            current_zone = zone;
        }
    }

    #[test]
    fn test_distributed_winding_pointed() {
        distributed_winding_impl(EndWindingStyle::Pointed {
            end_winding_coil_angle: 30.0 / 180.0 * PI,
        });
    }

    #[test]
    fn test_distributed_winding_layered() {
        distributed_winding_impl(EndWindingStyle::Layered);
    }

    fn distributed_winding_impl(end_winding_style: EndWindingStyle) {
        let axial_coil_overhang = match end_winding_style {
            EndWindingStyle::Pointed {
                end_winding_coil_angle: _,
            } => 0.8,
            EndWindingStyle::Layered => 0.0,
        };

        let params = CoilDrawablesParameters {
            arrowhead_length: 0.5,
            line_width: 1.0,
            slot_width: 1.0,
            tooth_width: 1.0,
            axial_length: 4.0,
            arrow_head_height: 1.0,
            start_height: 0.0,
            delta_empty_zone: 0.1,
            font_size: 14,
            end_winding_style,
            draw_both_sides: true,
            axial_coil_overhang,
            cyclic: true,
        };

        let ew_style = match end_winding_style {
            EndWindingStyle::Pointed {
                end_winding_coil_angle: _,
            } => "pointed",
            EndWindingStyle::Layered => "layered",
        };

        {
            let winding: DistributedWinding = DistributedMinimalBuilder {
                slots: 18.try_into().expect("not zero"),
                pole_pairs: 4.try_into().expect("not zero"),
                phases: 3.try_into().expect("not zero"),
                layers: 1.try_into().expect("not zero"),
                coil_span_reduction: 0,
                zone_span_variation: 0,
                winding_table_method: WindingTableMethod::CoilSide,
            }
            .try_into()
            .unwrap();
            let name = format!("tests/img/end_winding_{ew_style}_18_4_SL.png");
            check(&winding, &params, &name);
        }

        {
            let winding: DistributedWinding = DistributedMinimalBuilder {
                slots: 36.try_into().expect("not zero"),
                pole_pairs: 2.try_into().expect("not zero"),
                phases: 3.try_into().expect("not zero"),
                layers: 2.try_into().expect("not zero"),
                coil_span_reduction: 0,
                zone_span_variation: 0,
                winding_table_method: WindingTableMethod::Tingley,
            }
            .try_into()
            .unwrap();
            let name = format!("tests/img/end_winding_{ew_style}_36_2_DL.png");
            check(&winding, &params, &name);
        }

        {
            let winding: DistributedWinding = DistributedMinimalBuilder {
                slots: 36.try_into().expect("not zero"),
                pole_pairs: 2.try_into().expect("not zero"),
                phases: 3.try_into().expect("not zero"),
                layers: 2.try_into().expect("not zero"),
                coil_span_reduction: 1,
                zone_span_variation: 0,
                winding_table_method: WindingTableMethod::Tingley,
            }
            .try_into()
            .unwrap();
            let name = format!("tests/img/end_winding_{ew_style}_36_2_DL_pitch_1.png");
            check(&winding, &params, &name);
        }
        {
            let mut winding: DistributedWinding = DistributedMinimalBuilder {
                slots: 12.try_into().expect("not zero"),
                pole_pairs: 1.try_into().expect("not zero"),
                phases: 3.try_into().expect("not zero"),
                layers: 1.try_into().expect("not zero"),
                coil_span_reduction: 0,
                zone_span_variation: 0,
                winding_table_method: WindingTableMethod::Tingley,
            }
            .try_into()
            .unwrap();
            winding.set_concentric_coils(true);
            let name = format!("tests/img/end_winding_{ew_style}_12_1_SL_concentric.png");
            check(&winding, &params, &name);
        }
        {
            let mut winding: DistributedWinding = DistributedMinimalBuilder {
                slots: 12.try_into().expect("not zero"),
                pole_pairs: 1.try_into().expect("not zero"),
                phases: 3.try_into().expect("not zero"),
                layers: 2.try_into().expect("not zero"),
                coil_span_reduction: 0,
                zone_span_variation: 0,
                winding_table_method: WindingTableMethod::Tingley,
            }
            .try_into()
            .unwrap();
            winding.set_concentric_coils(true);
            let name = format!("tests/img/end_winding_{ew_style}_12_1_DL_concentric.png");
            check(&winding, &params, &name);
        }
        {
            let winding: DistributedWinding = DistributedMinimalBuilder {
                slots: 12.try_into().expect("not zero"),
                pole_pairs: 1.try_into().expect("not zero"),
                phases: 3.try_into().expect("not zero"),
                layers: 2.try_into().expect("not zero"),
                coil_span_reduction: 0,
                zone_span_variation: 0,
                winding_table_method: WindingTableMethod::Tingley,
            }
            .try_into()
            .unwrap();
            let name = format!("tests/img/end_winding_{ew_style}_12_1_DL.png");
            check(&winding, &params, &name);
        }
        {
            let mut winding: DistributedWinding = DistributedMinimalBuilder {
                slots: 36.try_into().expect("not zero"),
                pole_pairs: 1.try_into().expect("not zero"),
                phases: 3.try_into().expect("not zero"),
                layers: 1.try_into().expect("not zero"),
                coil_span_reduction: 0,
                zone_span_variation: 0,
                winding_table_method: WindingTableMethod::Tingley,
            }
            .try_into()
            .unwrap();
            winding.set_concentric_coils(true);
            let name = format!("tests/img/end_winding_{ew_style}_36_1_SL_concentric.png");
            check(&winding, &params, &name);
        }
    }

    #[test]
    fn test_tooth_coil_winding() {
        let params = CoilDrawablesParameters {
            arrowhead_length: 0.5,
            line_width: 1.0,
            slot_width: 1.0,
            tooth_width: 1.0,
            axial_length: 4.0,
            axial_coil_overhang: 0.8,
            arrow_head_height: 1.0,
            start_height: 0.0,
            delta_empty_zone: 0.1,
            font_size: 14,
            end_winding_style: EndWindingStyle::Pointed {
                end_winding_coil_angle: 30.0 / 180.0 * PI,
            },
            draw_both_sides: true,
            cyclic: true,
        };

        {
            let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
                slots: 12.try_into().expect("not zero"),
                pole_pairs: 5.try_into().expect("not zero"),
                phases: 3.try_into().expect("not zero"),
                layers: 2.try_into().expect("not zero"),
                winding_table_method: WindingTableMethod::Tingley,
            }
            .try_into()
            .unwrap();
            check(&winding, &params, "tests/img/end_winding_12_5_DL.png");
        }
    }

    #[test]
    fn test_tooth_coil_winding_half() {
        let params = CoilDrawablesParameters {
            arrowhead_length: 0.5,
            line_width: 1.0,
            slot_width: 1.0,
            tooth_width: 1.0,
            axial_length: 4.0,
            axial_coil_overhang: 0.8,
            arrow_head_height: 1.0,
            start_height: 0.0,
            delta_empty_zone: 0.1,
            font_size: 14,
            end_winding_style: EndWindingStyle::Pointed {
                end_winding_coil_angle: 30.0 / 180.0 * PI,
            },
            draw_both_sides: false,
            cyclic: true,
        };

        {
            let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
                slots: 12.try_into().expect("not zero"),
                pole_pairs: 5.try_into().expect("not zero"),
                phases: 3.try_into().expect("not zero"),
                layers: 2.try_into().expect("not zero"),
                winding_table_method: WindingTableMethod::Tingley,
            }
            .try_into()
            .unwrap();
            check(
                &winding,
                &params,
                "tests/img/end_winding_12_5_DL_halfed.png",
            );
        }
    }

    #[test]
    fn test_coil_assembly() {
        let params = CoilDrawablesParameters {
            arrowhead_length: 0.5,
            line_width: 1.0,
            slot_width: 1.0,
            tooth_width: 1.0,
            axial_length: 4.0,
            axial_coil_overhang: 0.8,
            arrow_head_height: 1.0,
            start_height: 0.0,
            delta_empty_zone: 0.1,
            font_size: 14,
            end_winding_style: EndWindingStyle::Pointed {
                end_winding_coil_angle: 30.0 / 180.0 * PI,
            },
            draw_both_sides: true,
            cyclic: true,
        };

        {
            let mut coils = Coils::new();
            let coil = FullCoil::new(
                Zone::new(0, 0),
                Zone::new(11, 0),
                false,
                1.try_into().unwrap(),
                1.try_into().unwrap(),
                Box::new(RoundWire::default()),
            )
            .unwrap();

            coils.insert_coil(coil.into()).unwrap();
            let coil = FullCoil::new(
                Zone::new(1, 1),
                Zone::new(10, 1),
                false,
                1.try_into().unwrap(),
                1.try_into().unwrap(),
                Box::new(RoundWire::default()),
            )
            .unwrap();
            coils.insert_coil(coil.into()).unwrap();

            let coil = FullCoil::new(
                Zone::new(2, 0),
                Zone::new(9, 1),
                false,
                1.try_into().unwrap(),
                2.try_into().unwrap(),
                Box::new(RoundWire::default()),
            )
            .unwrap();
            coils.insert_coil(coil.into()).unwrap();

            let coil = FullCoil::new(
                Zone::new(2, 1),
                Zone::new(9, 0),
                false,
                1.try_into().unwrap(),
                2.try_into().unwrap(),
                Box::new(RoundWire::default()),
            )
            .unwrap();
            coils.insert_coil(coil.into()).unwrap();
            let winding = CoilAssembly::new_minimal(
                12.try_into().unwrap(),
                5.try_into().unwrap(),
                3.try_into().unwrap(),
                CoilLayout::DoubleVertical,
                coils,
            )
            .unwrap();
            check(&winding, &params, "tests/img/end_winding_ca_12_1_a.png");
        }
    }
}
