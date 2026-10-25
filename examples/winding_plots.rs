use std::f64::consts::PI;
use std::num::{NonZeroU16, NonZeroUsize};
use std::path::PathBuf;
use std::sync::Arc;

use cairo_viewport::bounding_box::ToBoundingBox;
use cairo_viewport::{BoundingBox, SideLength, Viewport};
use planar_geo::Transformation;
use planar_geo::draw::*;
use stem_winding::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    zone_polarity()?;
    winding_table_reference_img()?;
    phd_stator()?;
    zone_drawables_config()?;
    coil_drawables_method()?;
    zone_drawables_method()?;

    // Winding-specific plots
    let zone_config = ZoneDrawablesConfig {
        background_color: ZoneBackgroundColor::Phase,
        center_config: ZoneCenterConfig::Arrow(ZoneArrowConfig {
            color_by_phase: false,
            relative_diameter: 0.8,
            normalized_current: None,
        }),
        show_empty_zones: true,
    };
    tooth_coil_winding(&zone_config)?;

    return Ok(());
}

fn winding_table_reference_img() -> Result<(), Box<dyn std::error::Error>> {
    let winding: DistributedWinding = DistributedMinimalBuilder {
        slots: 6.try_into()?,
        pole_pairs: 1.try_into()?,
        phases: 3.try_into()?,
        layers: 2.try_into()?,
        coil_span_reduction: 1,
        zone_span_variation: 0,
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()?;

    let core = LinCore::from_winding(&winding);

    let mut drawables: Vec<Drawable> = vec![core.drawable().into()];

    // Zone view
    let config = ZoneDrawablesConfig {
        background_color: ZoneBackgroundColor::Phase,
        center_config: ZoneCenterConfig::Arrow(ZoneArrowConfig {
            color_by_phase: false,
            relative_diameter: 0.8,
            normalized_current: None,
        }),
        show_empty_zones: true,
    };
    drawables.extend(
        winding
            .zone_drawables(core.as_core_ref(), &config)
            .map(|z| z.0),
    );
    drawables
        .iter_mut()
        .for_each(|d| d.line_reflection([0.0, 0.0], [1.0, 0.0]));

    let mut bb = core.drawable().bounding_box();
    let ymin = bb.ymin();
    bb.try_set_ymin(-bb.ymax());
    bb.try_set_ymax(-ymin);
    bb.scale(1.01);

    let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(&format!("docs/img/winding_table_reference_img.svg"));
    let view = Viewport::from_bounding_box(&bb, SideLength::Long(800));
    view.write_to_file(&fp, |cr| {
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.paint()?;

        for d in drawables.iter() {
            d.draw(cr)?;
        }

        return Ok(());
    })?;
    return Ok(());
}

fn zone_polarity() -> Result<(), Box<dyn std::error::Error>> {
    let mut coils = Coils::new();
    coils.insert(
        FullCoil::new(
            Zone::new(0, 0),
            Zone::new(1, 0),
            true,
            NonZeroUsize::MIN,
            NonZeroU16::MIN,
            Box::new(SffWire::default()),
        )
        .expect("zones not identical")
        .into(),
    )?;

    let coil_assembly = CoilAssembly::new_minimal(
        2.try_into()?,
        NonZeroU16::MIN,
        NonZeroU16::MIN,
        CoilLayout::Single,
        coils,
    )?;

    let core = LinCore::from_winding(&coil_assembly);

    let mut drawables: Vec<Drawable> = vec![core.drawable().into()];

    // Zone view
    let config = ZoneDrawablesConfig {
        background_color: ZoneBackgroundColor::Phase,
        center_config: ZoneCenterConfig::Arrow(ZoneArrowConfig {
            color_by_phase: false,
            relative_diameter: 0.8,
            normalized_current: None,
        }),
        show_empty_zones: true,
    };
    drawables.extend(
        coil_assembly
            .zone_drawables(core.as_core_ref(), &config)
            .map(|z| z.0),
    );

    // Coil view
    let xshift = core.width().get::<meter>() * 1.5;
    let yshift = 0.5 * core.height().get::<meter>();

    let mut params = CoilDrawablesConfig::from(&core);
    params.line_width = 2.0;
    drawables.extend(
        coil_assembly
            .coil_drawables(&params)
            .map(|(mut drawable, _)| {
                drawable.translate([xshift, yshift]);
                drawable
            }),
    );
    drawables
        .iter_mut()
        .for_each(|d| d.line_reflection([0.0, 0.0], [1.0, 0.0]));

    let mut bb =
        BoundingBox::from_bounded_entities(drawables.iter()).expect("has at least one element");
    bb.scale(1.02);

    let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&format!("docs/img/coil_polarity.svg"));
    let view = Viewport::from_bounding_box(&bb, SideLength::Long(800));
    view.write_to_file(&fp, |cr| {
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.paint()?;

        for d in drawables.iter() {
            d.draw(cr)?;
        }

        return Ok(());
    })?;
    return Ok(());
}

fn phd_stator() -> Result<(), Box<dyn std::error::Error>> {
    let winding: DistributedWinding = DistributedMinimalBuilder {
        slots: 36.try_into().expect("not zero"),
        pole_pairs: 2.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 1.try_into().expect("not zero"),
        coil_span_reduction: 0,
        zone_span_variation: 0,
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()?;

    // Create the core of the machine
    let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
        bottom_width: Length::new::<millimeter>(9.2),
        opening_width: Length::new::<millimeter>(2.0),
        height: Length::new::<millimeter>(17.75),
        opening_height: Length::new::<millimeter>(2.0),
        slot_angle: PI / 18.0,
        bottom_radius: Length::new::<millimeter>(2.0),
        top_radius: Length::new::<millimeter>(2.0),
        opening_radius: Length::new::<millimeter>(0.5),
        consider_tooth_tip_leakage: false,
    }
    .try_into()?;
    let core: RotCore = RotCoreBuilder {
        air_gap_radius: Length::new::<millimeter>(55.0),
        yoke_radius: Length::new::<millimeter>(85.0),
        axial_length: Length::new::<millimeter>(165.0),
        axial_coil_overhang: Length::new::<millimeter>(0.0),
        iron_fill_factor: 0.95,
        material: Arc::new(Material::default()),
        pole_pairs: 2.try_into().expect("not zero"),
        skew_angle: 0.0,
        air_gap: Box::new(SlottedAirGap::new(
            36.try_into().expect("not zero"),
            true,
            CarterFactorModel::Bin12,
            Box::new(slot),
        )),
        flux_barrier: None,
    }
    .try_into()?;

    let mut drawables: Vec<Drawable> = vec![core.drawable().into()];

    // Zone view
    let config = ZoneDrawablesConfig {
        background_color: ZoneBackgroundColor::Phase,
        center_config: ZoneCenterConfig::Arrow(ZoneArrowConfig {
            color_by_phase: false,
            relative_diameter: 0.8,
            normalized_current: None,
        }),
        show_empty_zones: true,
    };
    drawables.extend(
        winding
            .zone_drawables(core.as_core_ref(), &config)
            .map(|z| z.0),
    );
    drawables
        .iter_mut()
        .for_each(|d| d.line_reflection([0.0, 0.0], [1.0, 0.0]));

    let mut bb =
        BoundingBox::from_bounded_entities(drawables.iter()).expect("has at least one element");
    bb.scale(1.02);

    let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&format!("docs/img/phd_stator.svg"));
    let view = Viewport::from_bounding_box(&bb, SideLength::Long(600));
    view.write_to_file(&fp, |cr| {
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.paint()?;

        for d in drawables.iter() {
            d.draw(cr)?;
        }

        return Ok(());
    })?;
    return Ok(());
}

fn zone_drawables_config() -> Result<(), Box<dyn std::error::Error>> {
    let mut winding = CoilAssembly::new(
        6.try_into().expect("not zero"),
        1.try_into().expect("not zero"),
        3.try_into().expect("not zero"),
        CoilLayout::DoubleVertical,
        Default::default(),
        Vec::new(),
        Connection::Star,
    )?;
    winding.insert(
        FullCoil::new(
            Zone { slot: 0, layer: 0 },
            Zone { slot: 3, layer: 1 },
            true,
            1.try_into().expect("not zero"),
            1.try_into().expect("not zero"),
            Box::new(SffWire::default()),
        )
        .unwrap(),
    )?;
    winding.insert(
        FullCoil::new(
            Zone { slot: 4, layer: 1 },
            Zone { slot: 1, layer: 0 },
            true,
            1.try_into().expect("not zero"),
            3.try_into().expect("not zero"),
            Box::new(SffWire::default()),
        )
        .unwrap(),
    )?;
    winding.insert(
        FullCoil::new(
            Zone { slot: 2, layer: 0 },
            Zone { slot: 5, layer: 1 },
            true,
            1.try_into().expect("not zero"),
            2.try_into().expect("not zero"),
            Box::new(SffWire::default()),
        )
        .unwrap(),
    )?;

    let core = LinCore::from_winding(&winding);

    let side_length = SideLength::Long(400);

    {
        for background_color in [
            ZoneBackgroundColor::Phase,
            ZoneBackgroundColor::Default,
            ZoneBackgroundColor::None,
        ]
        .into_iter()
        {
            let config = ZoneDrawablesConfig {
                background_color,
                center_config: ZoneCenterConfig::Arrow(ZoneArrowConfig {
                    color_by_phase: false,
                    relative_diameter: 0.8,
                    normalized_current: None,
                }),
                show_empty_zones: true,
            };
            let mut drawables: Vec<Drawable> = vec![core.drawable().into()];
            drawables.extend(
                winding
                    .zone_drawables(core.as_core_ref(), &config)
                    .map(|z| z.0),
            );
            drawables
                .iter_mut()
                .for_each(|d| d.line_reflection([0.0, 0.0], [1.0, 0.0]));

            let mut bb = BoundingBox::from_bounded_entities(drawables.iter())
                .expect("has at least one element");
            bb.scale(1.02);

            let filename = match background_color {
                ZoneBackgroundColor::Phase => "phase",
                ZoneBackgroundColor::Default => "default",
                ZoneBackgroundColor::None => "none",
            };

            let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(&format!("docs/img/zone_drawables_config_{filename}.svg"));
            let view = Viewport::from_bounding_box(&bb, side_length);
            view.write_to_file(&fp, |cr| {
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.paint()?;

                for d in drawables.iter() {
                    d.draw(cr)?;
                }

                return Ok(());
            })?;
        }
    }
    {
        for show_empty_zones in [true, false].into_iter() {
            let config = ZoneDrawablesConfig {
                background_color: ZoneBackgroundColor::Phase,
                center_config: ZoneCenterConfig::Arrow(ZoneArrowConfig {
                    color_by_phase: false,
                    relative_diameter: 0.8,
                    normalized_current: None,
                }),
                show_empty_zones,
            };
            let mut drawables: Vec<Drawable> = vec![core.drawable().into()];
            drawables.extend(
                winding
                    .zone_drawables(core.as_core_ref(), &config)
                    .map(|z| z.0),
            );
            drawables
                .iter_mut()
                .for_each(|d| d.line_reflection([0.0, 0.0], [1.0, 0.0]));

            let mut bb = BoundingBox::from_bounded_entities(drawables.iter())
                .expect("has at least one element");
            bb.scale(1.02);

            let filename = if show_empty_zones {
                "show_empty_zones"
            } else {
                "hide_empty_zones"
            };

            let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(&format!("docs/img/zone_drawables_config_{filename}.svg"));
            let view = Viewport::from_bounding_box(&bb, side_length);
            view.write_to_file(&fp, |cr| {
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.paint()?;

                for d in drawables.iter() {
                    d.draw(cr)?;
                }

                return Ok(());
            })?;
        }
    }
    {
        let config = ZoneDrawablesConfig {
            background_color: ZoneBackgroundColor::Phase,
            center_config: ZoneCenterConfig::None,
            show_empty_zones: true,
        };
        let mut drawables: Vec<Drawable> = vec![core.drawable().into()];
        drawables.extend(
            winding
                .zone_drawables(core.as_core_ref(), &config)
                .map(|z| z.0),
        );
        drawables
            .iter_mut()
            .for_each(|d| d.line_reflection([0.0, 0.0], [1.0, 0.0]));

        let mut bb =
            BoundingBox::from_bounded_entities(drawables.iter()).expect("has at least one element");
        bb.scale(1.02);

        let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&format!(
            "docs/img/zone_drawables_config_no_zone_config.svg"
        ));
        let view = Viewport::from_bounding_box(&bb, side_length);
        view.write_to_file(&fp, |cr| {
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.paint()?;

            for d in drawables.iter() {
                d.draw(cr)?;
            }

            return Ok(());
        })?;
    }
    {
        let config = ZoneDrawablesConfig {
            background_color: ZoneBackgroundColor::Phase,
            center_config: ZoneCenterConfig::AmpereTurns(12.0),
            show_empty_zones: true,
        };
        let mut drawables: Vec<Drawable> = vec![core.drawable().into()];
        drawables.extend(
            winding
                .zone_drawables(core.as_core_ref(), &config)
                .map(|z| z.0),
        );
        drawables
            .iter_mut()
            .for_each(|d| d.line_reflection([0.0, 0.0], [1.0, 0.0]));

        let mut bb =
            BoundingBox::from_bounded_entities(drawables.iter()).expect("has at least one element");
        bb.scale(1.02);

        let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(&format!("docs/img/zone_drawables_config_ampere_turns.svg"));
        let view = Viewport::from_bounding_box(&bb, side_length);
        view.write_to_file(&fp, |cr| {
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.paint()?;

            for d in drawables.iter() {
                d.draw(cr)?;
            }

            return Ok(());
        })?;
    }
    {
        let config = ZoneDrawablesConfig {
            background_color: ZoneBackgroundColor::None,
            center_config: ZoneCenterConfig::Arrow(ZoneArrowConfig {
                color_by_phase: true,
                relative_diameter: 0.6,
                normalized_current: None,
            }),
            show_empty_zones: false,
        };
        let mut drawables: Vec<Drawable> = vec![core.drawable().into()];
        drawables.extend(
            winding
                .zone_drawables(core.as_core_ref(), &config)
                .map(|z| z.0),
        );
        drawables
            .iter_mut()
            .for_each(|d| d.line_reflection([0.0, 0.0], [1.0, 0.0]));

        let mut bb =
            BoundingBox::from_bounded_entities(drawables.iter()).expect("has at least one element");
        bb.scale(1.02);

        let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&format!(
            "docs/img/zone_drawables_config_zone_arrow_color_by_phase.svg"
        ));
        let view = Viewport::from_bounding_box(&bb, side_length);
        view.write_to_file(&fp, |cr| {
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.paint()?;

            for d in drawables.iter() {
                d.draw(cr)?;
            }

            return Ok(());
        })?;
    }
    {
        let config = ZoneDrawablesConfig {
            background_color: ZoneBackgroundColor::None,
            center_config: ZoneCenterConfig::Arrow(ZoneArrowConfig {
                color_by_phase: false,
                relative_diameter: 0.8,
                normalized_current: Some(vec![0.5, 0.5, -1.0]),
            }),
            show_empty_zones: false,
        };
        let mut drawables: Vec<Drawable> = vec![core.drawable().into()];
        drawables.extend(
            winding
                .zone_drawables(core.as_core_ref(), &config)
                .map(|z| z.0),
        );
        drawables
            .iter_mut()
            .for_each(|d| d.line_reflection([0.0, 0.0], [1.0, 0.0]));

        let mut bb =
            BoundingBox::from_bounded_entities(drawables.iter()).expect("has at least one element");
        bb.scale(1.02);

        let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&format!(
            "docs/img/zone_drawables_config_zone_arrow_black_normalized_current.svg"
        ));
        let view = Viewport::from_bounding_box(&bb, side_length);
        view.write_to_file(&fp, |cr| {
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.paint()?;

            for d in drawables.iter() {
                d.draw(cr)?;
            }

            return Ok(());
        })?;
    }

    return Ok(());
}

fn zone_drawables_method() -> Result<(), Box<dyn std::error::Error>> {
    let side_length = SideLength::Long(600);

    let winding: DistributedWinding = DistributedMinimalBuilder {
        slots: 12.try_into()?,
        pole_pairs: 1.try_into()?,
        phases: 3.try_into()?,
        layers: 2.try_into()?,
        coil_span_reduction: 1,
        zone_span_variation: 0,
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()?;
    let core = LinCore::from_winding(&winding);

    let config = ZoneDrawablesConfig {
        background_color: ZoneBackgroundColor::None,
        center_config: ZoneCenterConfig::Arrow(ZoneArrowConfig {
            color_by_phase: true,
            relative_diameter: 0.8,
            normalized_current: None,
        }),
        show_empty_zones: true,
    };
    let mut drawables: Vec<Drawable> = vec![core.drawable().into()];
    drawables.extend(
        winding
            .zone_drawables(core.as_core_ref(), &config)
            .map(|z| z.0),
    );
    drawables
        .iter_mut()
        .for_each(|d| d.line_reflection([0.0, 0.0], [1.0, 0.0]));

    let mut bb =
        BoundingBox::from_bounded_entities(drawables.iter()).expect("has at least one element");
    bb.scale(1.02);

    let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(&format!("docs/img/zone_drawable_method_1.svg"));
    let view = Viewport::from_bounding_box(&bb, side_length);
    view.write_to_file(&fp, |cr| {
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.paint()?;

        for d in drawables.iter() {
            d.draw(cr)?;
        }

        return Ok(());
    })?;

    let config = ZoneDrawablesConfig {
        background_color: ZoneBackgroundColor::Phase,
        center_config: ZoneCenterConfig::AmpereTurns(12.0),
        show_empty_zones: true,
    };
    let mut drawables: Vec<Drawable> = vec![core.drawable().into()];
    drawables.extend(
        winding
            .zone_drawables(core.as_core_ref(), &config)
            .map(|z| z.0),
    );
    drawables
        .iter_mut()
        .for_each(|d| d.line_reflection([0.0, 0.0], [1.0, 0.0]));

    let mut bb =
        BoundingBox::from_bounded_entities(drawables.iter()).expect("has at least one element");
    bb.scale(1.02);

    let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(&format!("docs/img/zone_drawable_method_2.svg"));
    let view = Viewport::from_bounding_box(&bb, side_length);
    view.write_to_file(&fp, |cr| {
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.paint()?;

        for d in drawables.iter() {
            d.draw(cr)?;
        }

        return Ok(());
    })?;
    return Ok(());
}

fn coil_drawables_method() -> Result<(), Box<dyn std::error::Error>> {
    let side_length = SideLength::Long(600);

    let winding: DistributedWinding = DistributedMinimalBuilder {
        slots: 12.try_into()?,
        pole_pairs: 1.try_into()?,
        phases: 3.try_into()?,
        layers: 2.try_into()?,
        coil_span_reduction: 1,
        zone_span_variation: 0,
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()?;
    let core = RotCore::from_winding(&winding);

    let mut config = CoilDrawablesConfig::from(&core);
    let mut bb = config.bounding_box(&winding);
    bb.scale(1.08);

    let drawables = winding.coil_drawables(&config).map(|z| z.0);

    let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(&format!("docs/img/coil_drawable_method_1.svg"));
    let view = Viewport::from_bounding_box(&bb, side_length);
    view.write_to_file(&fp, |cr| {
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.paint()?;

        for d in drawables {
            d.draw(cr)?;
        }

        return Ok(());
    })?;

    config.end_winding_style = EndWindingStyle::Layered;
    let mut bb = config.bounding_box(&winding);
    bb.scale(1.08);

    let drawables = winding.coil_drawables(&config).map(|z| z.0);

    let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(&format!("docs/img/coil_drawable_method_2.svg"));
    let view = Viewport::from_bounding_box(&bb, side_length);
    view.write_to_file(&fp, |cr| {
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.paint()?;

        for d in drawables {
            d.draw(cr)?;
        }

        return Ok(());
    })?;
    return Ok(());
}

fn tooth_coil_winding(zone_config: &ZoneDrawablesConfig) -> Result<(), Box<dyn std::error::Error>> {
    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 12.try_into()?,
        pole_pairs: 5.try_into()?,
        phases: 3.try_into()?,
        layers: 2.try_into()?,
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();
    let core = LinCore::from_winding(&winding);

    {
        let side_length = SideLength::Long(600);

        let mut coil_config = CoilDrawablesConfig::from(&core);
        coil_config.cyclic = true;

        let drawables = winding.coil_drawables(&coil_config);
        let bb = coil_config.bounding_box(&winding);
        let w = bb.width();
        let h = bb.height();
        let bb = BoundingBox::new(
            bb.xmin() - 0.03 * w,
            bb.xmax() + 0.03 * w,
            -bb.ymax() - 0.08 * h,
            -bb.ymin() + 0.08 * h,
        );

        let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(&format!("docs/img/coils_tooth_coil_winding.svg"));
        let view = Viewport::from_bounding_box(&bb, side_length);
        view.write_to_file(&fp, |cr| {
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.paint()?;

            for (mut d, k) in drawables {
                if matches!(k, DrawableKind::Arrow(_)) {
                    d.line_reflection([0.0, 0.0], [1.0, 0.0]);
                }
                d.draw(cr)?;
            }

            return Ok(());
        })?;
    }
    {
        let side_length = SideLength::Long(600);

        let drawables = winding
            .zone_drawables(core.as_core_ref(), &zone_config)
            .map(|z| z.0);
        let bb = core.shape().bounding_box();
        let w = bb.width();
        let h = bb.height();
        let bb = BoundingBox::new(
            bb.xmin() - 0.002 * w,
            bb.xmax() + 0.002 * w,
            -bb.ymax() - 0.01 * h,
            -bb.ymin() + 0.01 * h,
        );

        let fp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(&format!("docs/img/table_tooth_coil_winding.svg"));
        let view = Viewport::from_bounding_box(&bb, side_length);
        view.write_to_file(&fp, |cr| {
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.paint()?;
            cr.scale(1.0, -1.0);

            core.drawable().draw(cr)?;

            for d in drawables {
                d.draw(cr)?;
            }

            return Ok(());
        })?;
    }

    return Ok(());
}
