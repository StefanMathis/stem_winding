#[cfg(feature = "cairo")]
mod cairo_tests {

    #[test]
    fn test_coil() {
        let ew_drawing = EndWindingAirGapView {
            arrowhead_length: 0.5,
            line_width: 1.0,
            slot_width: 1.0,
            tooth_width: 1.0,
            axial_length: 4.0,
            arrow_head_height: 1.0,
            start_height: 0.0,
            delta_empty_zone: 0.1,
            font_size: 14,
            end_winding_style: EndWindingStyle::Pointed {
                end_winding_coil_angle: 30.0 / 180.0 * PI,
            },
            draw_both_sides: true,
        };

        {
            let x_outward = 2.5;
            let x_return = 3.5;

            let style = Style::default();
            let (lines, arrows) = ew_drawing.coil_drawable_from_coordinates(
                6,
                1,
                x_outward,
                x_return,
                style.clone(),
                true,
                0,
                None,
            );

            let view = Viewport::from_bounded_entities(lines.iter(), 500).unwrap();
            let path = std::path::Path::new("img/end_winding_coil_1.png"); // Always compare to the same reference image
            let callback = move |path: &std::path::Path| {
                return view.write_to_file(path, &|cr| {
                    for drawable in lines.iter() {
                        drawable.draw(cr);
                    }
                    for drawable in arrows.iter() {
                        drawable.draw(cr);
                    }
                });
            };
            assert!(compare_or_create(path, &callback).is_ok());
        }
        {
            let x_outward = 2.5;
            let x_return = 1.0;
            let (lines, arrows) = ew_drawing.coil_drawable_from_coordinates(
                6,
                1,
                x_outward,
                x_return,
                Style::default(),
                true,
                0,
                None,
            );

            let view = Viewport::from_bounded_entities(lines.iter(), 500).unwrap();
            let path = std::path::Path::new("img/end_winding_coil_2.png"); // Always compare to the same reference image
            let callback = move |path: &std::path::Path| {
                return view.write_to_file(path, &|cr| {
                    for drawable in lines.iter() {
                        drawable.draw(cr)
                    }
                    for drawable in arrows.iter() {
                        drawable.draw(cr)
                    }
                });
            };
            assert!(compare_or_create(path, &callback).is_ok());
        }
        {
            let x_outward = 2.5;
            let x_return = -0.5;
            let (lines, arrows) = ew_drawing.coil_drawable_from_coordinates(
                6,
                1,
                x_outward,
                x_return,
                Style::default(),
                true,
                0,
                None,
            );

            let view = Viewport::from_bounded_entities(lines.iter(), 500).unwrap();
            let path = std::path::Path::new("img/end_winding_coil_3.png"); // Always compare to the same reference image
            let callback = move |path: &std::path::Path| {
                return view.write_to_file(path, &|cr| {
                    for drawable in lines.iter() {
                        drawable.draw(cr)
                    }
                    for drawable in arrows.iter() {
                        drawable.draw(cr)
                    }
                });
            };
            assert!(compare_or_create(path, &callback).is_ok());
        }
        {
            let x_outward = 2.5;
            let x_return = 0.0;
            let (lines, arrows) = ew_drawing.coil_drawable_from_coordinates(
                6,
                1,
                x_outward,
                x_return,
                Style::default(),
                true,
                0,
                None,
            );

            let view = Viewport::from_bounded_entities(lines.iter(), 500).unwrap();
            let path = std::path::Path::new("img/end_winding_coil_4.png"); // Always compare to the same reference image
            let callback = move |path: &std::path::Path| {
                return view.write_to_file(path, &|cr| {
                    for drawable in lines.iter() {
                        drawable.draw(cr)
                    }
                    for drawable in arrows.iter() {
                        drawable.draw(cr)
                    }
                });
            };
            assert!(compare_or_create(path, &callback).is_ok());
        }
    }

    #[test]
    fn test_distributed_winding_pointed() {
        distributed_winding_priv(EndWindingStyle::Pointed {
            end_winding_coil_angle: 30.0 / 180.0 * PI,
        });
    }

    #[test]
    fn test_distributed_winding_layered() {
        distributed_winding_priv(EndWindingStyle::Layered);
    }

    fn distributed_winding_priv(end_winding_style: EndWindingStyle) {
        let ew_drawing = EndWindingAirGapView {
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
        };

        let ew_style = match end_winding_style {
            EndWindingStyle::Pointed {
                end_winding_coil_angle: _,
            } => "pointed",
            EndWindingStyle::Layered => "layered",
        };

        {
            let winding =
                WindingDistributed::new_minimal(18, 4, 3, 1, 0, 0, ZonePlanMethod::CoilSide)
                    .unwrap();
            let drawables = ew_drawing.drawables(&winding);

            let view =
                visualization::Viewport::from_bounded_entities(drawables.iter(), 2000).unwrap();
            let name = format!("img/end_winding_{ew_style}_18_4_SL.png");
            let path = std::path::Path::new(&name); // Always compare to the same reference image
            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, &|cr| {
                    for drawable in drawables.iter() {
                        drawable.draw(cr);
                    }
                });
            };
            assert!(compare_or_create(path, &callback).is_ok());
        }

        {
            let winding =
                WindingDistributed::new_minimal(36, 2, 3, 2, 0, 0, ZonePlanMethod::Tingley)
                    .unwrap();
            let drawables = ew_drawing.drawables(&winding);

            let view =
                visualization::Viewport::from_bounded_entities(drawables.iter(), 2000).unwrap();
            let name = format!("img/end_winding_{ew_style}_36_2_DL.png");
            let path = std::path::Path::new(&name); // Always compare to the same reference image
            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, &|cr| {
                    for drawable in drawables.iter() {
                        drawable.draw(cr);
                    }
                });
            };
            assert!(compare_or_create(path, &callback).is_ok());
        }

        {
            let winding =
                WindingDistributed::new_minimal(36, 2, 3, 2, 1, 0, ZonePlanMethod::Tingley)
                    .unwrap();
            let drawables = ew_drawing.drawables(&winding);

            let view =
                visualization::Viewport::from_bounded_entities(drawables.iter(), 2000).unwrap();
            let name = format!("img/end_winding_{ew_style}_36_2_DL_pitch_1.png");
            let path = std::path::Path::new(&name); // Always compare to the same reference image
            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, &|cr| {
                    for drawable in drawables.iter() {
                        drawable.draw(cr);
                    }
                });
            };
            assert!(compare_or_create(path, &callback).is_ok());
        }
        {
            let winding = WindingDistributed::new(
                12,
                1,
                3,
                1,
                0,
                0,
                1,
                1,
                Connection::Star,
                0.0,
                Box::new(wire::RoundWire::default()),
                ZonePlanMethod::Tingley,
                true,
            )
            .unwrap();
            let drawables = ew_drawing.drawables(&winding);

            let view =
                visualization::Viewport::from_bounded_entities(drawables.iter(), 2000).unwrap();
            let name = format!("img/end_winding_{ew_style}_12_1_SL_concentric.png");
            let path = std::path::Path::new(&name); // Always compare to the same reference image
            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, &|cr| {
                    for drawable in drawables.iter() {
                        drawable.draw(cr);
                    }
                });
            };
            assert!(compare_or_create(path, &callback).is_ok());
        }
        {
            let winding = WindingDistributed::new(
                12,
                1,
                3,
                2,
                0,
                0,
                1,
                1,
                Connection::Star,
                0.0,
                Box::new(wire::RoundWire::default()),
                ZonePlanMethod::Tingley,
                true,
            )
            .unwrap();
            let drawables = ew_drawing.drawables(&winding);

            let view =
                visualization::Viewport::from_bounded_entities(drawables.iter(), 2000).unwrap();
            let name = format!("img/end_winding_{ew_style}_12_1_DL_concentric.png");
            let path = std::path::Path::new(&name); // Always compare to the same reference image
            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, &|cr| {
                    for drawable in drawables.iter() {
                        drawable.draw(cr);
                    }
                });
            };
            assert!(compare_or_create(path, &callback).is_ok());
        }
        {
            let winding = WindingDistributed::new(
                12,
                1,
                3,
                2,
                0,
                0,
                1,
                1,
                Connection::Star,
                0.0,
                Box::new(wire::RoundWire::default()),
                ZonePlanMethod::Tingley,
                false,
            )
            .unwrap();
            let drawables = ew_drawing.drawables(&winding);

            let view =
                visualization::Viewport::from_bounded_entities(drawables.iter(), 2000).unwrap();
            let name = format!("img/end_winding_{ew_style}_12_1_DL.png");
            let path = std::path::Path::new(&name); // Always compare to the same reference image
            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, &|cr| {
                    for drawable in drawables.iter() {
                        drawable.draw(cr);
                    }
                });
            };
            assert!(compare_or_create(path, &callback).is_ok());
        }
        {
            let winding = WindingDistributed::new(
                36,
                1,
                3,
                1,
                0,
                0,
                1,
                1,
                Connection::Star,
                0.0,
                Box::new(wire::RoundWire::default()),
                ZonePlanMethod::Tingley,
                true,
            )
            .unwrap();
            let drawables = ew_drawing.drawables(&winding);

            let view =
                visualization::Viewport::from_bounded_entities(drawables.iter(), 2000).unwrap();
            let name = format!("img/end_winding_{ew_style}_36_1_SL_concentric.png");
            let path = std::path::Path::new(&name); // Always compare to the same reference image
            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, &|cr| {
                    for drawable in drawables.iter() {
                        drawable.draw(cr);
                    }
                });
            };
            assert!(compare_or_create(path, &callback).is_ok());
        }
    }

    #[test]
    fn test_trait_object() {
        let ew_drawing = EndWindingAirGapView {
            arrowhead_length: 0.5,
            line_width: 1.0,
            slot_width: 1.0,
            tooth_width: 1.0,
            axial_length: 4.0,
            arrow_head_height: 1.0,
            start_height: 0.0,
            delta_empty_zone: 0.1,
            font_size: 14,
            end_winding_style: EndWindingStyle::Pointed {
                end_winding_coil_angle: 30.0 / 180.0 * PI,
            },
            draw_both_sides: true,
        };

        let winding = WindingDistributed::new(
            36,
            1,
            3,
            1,
            0,
            0,
            1,
            1,
            Connection::Star,
            0.0,
            Box::new(wire::RoundWire::default()),
            ZonePlanMethod::Tingley,
            true,
        )
        .unwrap();
        let wdg_trait_object: &dyn IsWinding = &winding;

        let drawables = ew_drawing.drawables(wdg_trait_object);

        let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 2000).unwrap();
        let name = format!("img/end_winding_pointed_36_1_SL_concentric.png");
        let path = std::path::Path::new(&name); // Always compare to the same reference image
        let callback = |path: &std::path::Path| {
            return view.write_to_file(path, &|cr| {
                for drawable in drawables.iter() {
                    drawable.draw(cr);
                }
            });
        };
        assert!(compare_or_create(path, &callback).is_ok());
    }

    #[test]
    fn test_tooth_coil_winding() {
        let ew_drawing = EndWindingAirGapView {
            arrowhead_length: 0.5,
            line_width: 1.0,
            slot_width: 1.0,
            tooth_width: 1.0,
            axial_length: 4.0,
            arrow_head_height: 1.0,
            start_height: 0.0,
            delta_empty_zone: 0.1,
            font_size: 14,
            end_winding_style: EndWindingStyle::Pointed {
                end_winding_coil_angle: 30.0 / 180.0 * PI,
            },
            draw_both_sides: true,
        };

        {
            let winding =
                WindingToothCoil::new_minimal(12, 5, 3, 2, ZonePlanMethod::Tingley).unwrap();
            let drawables = ew_drawing.drawables(&winding);

            let view =
                visualization::Viewport::from_bounded_entities(drawables.iter(), 2000).unwrap();
            let path = std::path::Path::new("img/end_winding_12_5_DL.png"); // Always compare to the same reference image
            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, &|cr| {
                    for drawable in drawables.iter() {
                        drawable.draw(cr);
                    }
                });
            };
            assert!(compare_or_create(path, &callback).is_ok());
        }
    }

    #[test]
    fn test_tooth_coil_winding_half() {
        let ew_drawing = EndWindingAirGapView {
            arrowhead_length: 0.5,
            line_width: 1.0,
            slot_width: 1.0,
            tooth_width: 1.0,
            axial_length: 4.0,
            arrow_head_height: 1.0,
            start_height: 0.0,
            delta_empty_zone: 0.1,
            font_size: 14,
            end_winding_style: EndWindingStyle::Pointed {
                end_winding_coil_angle: 30.0 / 180.0 * PI,
            },
            draw_both_sides: false,
        };

        {
            let winding =
                WindingToothCoil::new_minimal(12, 5, 3, 2, ZonePlanMethod::Tingley).unwrap();
            let drawables = ew_drawing.drawables(&winding);

            let view =
                visualization::Viewport::from_bounded_entities(drawables.iter(), 2000).unwrap();
            let path = std::path::Path::new("img/end_winding_12_5_DL_halfed.png"); // Always compare to the same reference image
            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, &|cr| {
                    for drawable in drawables.iter() {
                        drawable.draw(cr);
                    }
                });
            };
            assert!(compare_or_create(path, &callback).is_ok());
        }
    }

    #[test]
    fn test_coil_assembly() {
        let ew_drawing = EndWindingAirGapView {
            arrowhead_length: 0.5,
            line_width: 1.0,
            slot_width: 1.0,
            tooth_width: 1.0,
            axial_length: 4.0,
            arrow_head_height: 1.0,
            start_height: 0.0,
            delta_empty_zone: 0.1,
            font_size: 14,
            end_winding_style: EndWindingStyle::Pointed {
                end_winding_coil_angle: 30.0 / 180.0 * PI,
            },
            draw_both_sides: true,
        };

        {
            let mut coils = Coils::new();
            let coil = CoilFull::new(
                Zone::new(0, 0),
                Zone::new(11, 0),
                true,
                false,
                1,
                1,
                Box::new(RoundWire::default()),
            )
            .unwrap();

            coils.insert_coil(coil.into()).unwrap();
            let coil = CoilFull::new(
                Zone::new(1, 1),
                Zone::new(10, 1),
                true,
                false,
                1,
                1,
                Box::new(RoundWire::default()),
            )
            .unwrap();
            coils.insert_coil(coil.into()).unwrap();

            let coil = CoilFull::new(
                Zone::new(2, 0),
                Zone::new(9, 1),
                true,
                false,
                1,
                2,
                Box::new(RoundWire::default()),
            )
            .unwrap();
            coils.insert_coil(coil.into()).unwrap();

            let coil = CoilFull::new(
                Zone::new(2, 1),
                Zone::new(9, 0),
                true,
                false,
                1,
                2,
                Box::new(RoundWire::default()),
            )
            .unwrap();
            coils.insert_coil(coil.into()).unwrap();
            let winding =
                CoilAssembly::new_minimal(12, 5, 3, 2, coils, CoilLayout::DoubleVertical).unwrap();
            let drawables = ew_drawing.drawables(&winding);

            let view =
                visualization::Viewport::from_bounded_entities(drawables.iter(), 2000).unwrap();
            let path = std::path::Path::new("img/end_winding_ca_12_1_a.png"); // Always compare to the same reference image
            let callback = |path: &std::path::Path| {
                return view.write_to_file(path, &|cr| {
                    for drawable in drawables.iter() {
                        drawable.draw(cr);
                    }
                });
            };
            assert!(compare_or_create(path, &callback).is_ok());
        }
    }
}
