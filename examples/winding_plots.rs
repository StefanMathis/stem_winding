use std::num::{NonZeroU16, NonZeroUsize};
use std::path::PathBuf;

use cairo_viewport::bounding_box::ToBoundingBox;
use cairo_viewport::{BoundingBox, SideLength, Viewport};
use planar_geo::Transformation;
use planar_geo::draw::*;
use stem_winding::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    zone_polarity()?;
    winding_table_reference_img()?;
    phd_stator()?;
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
        center_config: Some(ZoneCenterConfig::Arrow(ZoneArrowConfig {
            color_by_phase: false,
            relative_diameter: 0.8,
            normalized_current: None,
        })),
        show_empty_zones: true,
    };
    drawables.extend(
        winding
            .zone_drawables(core.as_core_ref(), &config)
            .map(|z| z.1),
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
            Box::new(RoundWire::default()),
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
        center_config: Some(ZoneCenterConfig::Arrow(ZoneArrowConfig {
            color_by_phase: false,
            relative_diameter: 0.8,
            normalized_current: None,
        })),
        show_empty_zones: true,
    };
    drawables.extend(
        coil_assembly
            .zone_drawables(core.as_core_ref(), &config)
            .map(|z| z.1),
    );

    // Coil view
    let xshift = core.width().get::<meter>() * 1.5;
    let yshift = 0.5 * core.height().get::<meter>();

    let mut params = CoilDrawablesParameters::from(&core);
    params.line_width = 2.0;
    drawables.extend(
        coil_assembly
            .coil_drawables(&params)
            .map(|(_, mut drawable)| {
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
