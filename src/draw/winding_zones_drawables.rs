use planar_geo::prelude::*;
use std::{f64::consts::SQRT_2, num::NonZeroU16};
use stem_coil_layout::Zone;
use stem_core::prelude::*;

use super::{DrawableKind, phase_color};
use crate::winding::Winding;

const INVISIBLE: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

pub struct WindingZoneDrawables<'a> {
    winding: &'a dyn Winding,
    winding_zones: WindingZones,
    zone_config: &'a ZoneDrawablesConfig,
    zone: Zone,
    phase: i32,
    drawables: [Option<(Drawable, DrawableKind)>; 4],
    arrow_circle_diameter: f64,
    arrow_tip_diameter: f64,
}

impl<'a> WindingZoneDrawables<'a> {
    pub fn new(
        winding: &'a dyn Winding,
        core: CoreRef<'_>,
        zone_config: &'a ZoneDrawablesConfig,
    ) -> Self {
        let [arrow_circle_diameter, arrow_tip_diameter] = match &zone_config.center_config {
            ZoneCenterConfig::Arrow(zone_arrow_config) => {
                let contours = core
                    .winding_zones(&winding.coil_layout())
                    .map(|c| c.contour);
                arrow_diameters(contours, zone_arrow_config.relative_diameter)
            }
            _ => [0.0, 0.0],
        };

        return WindingZoneDrawables {
            winding,
            winding_zones: core.winding_zones(&winding.coil_layout()),
            zone_config,
            zone: Zone { slot: 0, layer: 0 },
            phase: 0,
            drawables: [None, None, None, None],
            arrow_circle_diameter,
            arrow_tip_diameter,
        };
    }
}

impl<'a> Iterator for WindingZoneDrawables<'a> {
    type Item = (Drawable, DrawableKind);

    fn next(&mut self) -> Option<Self::Item> {
        for drawable in self.drawables.iter_mut() {
            if let Some(d) = drawable.take() {
                return Some((d.0, d.1));
            }
        }

        // Populate self.drawables for the next zone
        let positioned_contour = self.winding_zones.next()?;
        self.zone = positioned_contour.zone;
        self.phase = self.winding.phase_at(positioned_contour.zone);

        let turns = self.winding.turns_at(positioned_contour.zone);
        self.drawables = self.zone_config.drawables(
            positioned_contour.contour,
            self.phase,
            turns,
            self.zone,
            self.winding.phases(),
            self.arrow_circle_diameter,
            self.arrow_tip_diameter,
        );
        return self.next();
    }
}

/// Options for customizing the [`Drawable`]s produced by the
/// [`Winding::zone_drawables`] method.
///
/// The documentation of the individual fields showcases how the resulting image
/// changes based on the field values. All images have been created using
/// _examples/winding_plots.rs_.
#[derive(Clone, Debug)]
pub struct ZoneDrawablesConfig {
    /// The background color for the individual zones. See the
    /// [`ZoneBackgroundColor`] documentation for examples.
    pub background_color: ZoneBackgroundColor,
    /// Visualization options for the coil side in the zone center. See the
    /// [`ZoneCenterConfig`] documentation for examples.
    pub center_config: ZoneCenterConfig,
    /// Whether to show zones which don't contain a coil side. If set to true,
    /// empty zones are shown with a dashed contour.
    ///
    /// **Empty zones shown**
    #[doc = ""]
    #[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!("show_empty_zones",
        "docs/img/zone_drawables_config_show_empty_zones.svg")
)]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
        `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    /// ![Empty zones shown][show_empty_zones]
    ///
    /// **Empty zones hidden**
    #[doc = ""]
    #[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!("hide_empty_zones",
        "docs/img/zone_drawables_config_hide_empty_zones.svg")
)]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
        `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    /// ![Empty zones hidden][hide_empty_zones]
    pub show_empty_zones: bool,
}

impl ZoneDrawablesConfig {
    pub fn new(
        background_color: ZoneBackgroundColor,
        center_config: ZoneCenterConfig,
        show_empty_zones: bool,
    ) -> Self {
        return Self {
            background_color,
            center_config,
            show_empty_zones,
        };
    }

    fn drawables(
        &self,
        contour: Contour,
        phase: i32,
        turns: usize,
        zone: Zone,
        phases: NonZeroU16,
        arrow_circle_diameter: f64,
        arrow_tip_diameter: f64,
    ) -> [Option<(Drawable, DrawableKind)>; 4] {
        let mut drawables = [None, None, None, None];

        if phase == 0 && !self.show_empty_zones {
            return drawables;
        }

        // Zone contour
        let mut zone_style = stem_core::stem_slot::SLOT_STYLE;
        zone_style.background_color = self.background_color.color(phase.abs() as u16, phases);
        let mut kind = DrawableKind::Coil(zone);
        if phase == 0 {
            zone_style.line_style = LineStyle::default_dashed();
            kind = DrawableKind::EmptyZone(zone)
        }

        let centroid = contour.centroid();

        match &self.center_config {
            ZoneCenterConfig::None => {
                drawables[0] = Some((Drawable::new(contour, zone_style), kind))
            }
            ZoneCenterConfig::AmpereTurns(font_size) => {
                let text = if phase >= 0 {
                    format!("{turns}")
                } else {
                    format!("-{turns}")
                };
                zone_style.text = Some(Box::new(Text::new(
                    text,
                    Anchor::Centroid,
                    [0.0, 0.0],
                    [0.0, 0.0],
                    Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    },
                    *font_size,
                    0.0,
                )));
                drawables[0] = Some((
                    Drawable::new(contour, zone_style),
                    DrawableKind::Annotation(zone),
                ));
            }
            ZoneCenterConfig::Arrow(zone_arrow_config) => {
                drawables[0] = Some((Drawable::new(contour, zone_style), kind));
                let arrow_drawables = zone_arrow_config.arrow(
                    phase,
                    phases,
                    arrow_circle_diameter,
                    arrow_tip_diameter,
                );
                for (idx, mut drawable) in arrow_drawables.into_iter().enumerate() {
                    if let Some(d) = drawable.as_mut() {
                        d.translate(centroid);
                    }
                    drawables[idx + 1] = drawable.map(|d| (d, DrawableKind::Arrow(zone)));
                }
            }
        }

        return drawables;
    }
}

impl Default for ZoneDrawablesConfig {
    fn default() -> Self {
        Self {
            background_color: Default::default(),
            center_config: ZoneCenterConfig::None,
            show_empty_zones: true,
        }
    }
}

/// The background color of the zone [`Drawable`]s produced by the
/// [`Winding::zone_drawables`] method.
///
/// This enum is used to define the [`ZoneDrawablesConfig`] which is given as an
/// argument to [`Winding::zone_drawables`]. See the variant documentation for
/// examples.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ZoneBackgroundColor {
    /// If a coil occupies the zone, its phase is used to dermine the
    /// background color from the [`phase_color`] function. If the zone in
    /// question does not contain a coil side, it has no background color.
    #[doc = ""]
    #[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!("phase_color",
        "docs/img/zone_drawables_config_phase.svg")
)]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
        `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    /// ![Individual phase color][phase_color]
    Phase,
    /// The [`ORANGE`](stem_slot::ORANGE) color from the [stem_slot] crate which
    /// is used as the "default" color when visualizing a slot and its zones.
    #[doc = ""]
    #[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!("slot_color",
        "docs/img/zone_drawables_config_default.svg")
)]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
        `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    /// ![Default slot color][slot_color]
    #[default]
    Default,
    /// No background color.
    #[doc = ""]
    #[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!("no_bg_color",
        "docs/img/zone_drawables_config_none.svg")
)]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
        `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    /// ![No background color example][no_bg_color]
    None,
}

impl ZoneBackgroundColor {
    pub fn color(&self, phase: u16, phases: NonZeroU16) -> Color {
        match self {
            ZoneBackgroundColor::Phase => match NonZeroU16::new(phase) {
                Some(ph) => phase_color(ph, phases),
                None => Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                },
            },
            ZoneBackgroundColor::Default => stem_slot::SLOT_STYLE.background_color,
            ZoneBackgroundColor::None => INVISIBLE,
        }
    }
}

/**
This struct describes the inner element of the zone shape.

# Variants
* `AmpereTurns`: Write the number of ampere turns in the shape middle, taking into account the polarity of the zone according to the zone plan.
* `Arrow`: Draw an arrow inside the zone shape which is defined by a `ZoneArrowConfig` struct.
 */
/// Visualization of coil side properties in the zone center.
///
/// This enum is used to define the [`ZoneDrawablesConfig`] which is given as an
/// argument to [`Winding::zone_drawables`]. See the variant documentation for
/// examples.
#[derive(Clone, Debug)]
pub enum ZoneCenterConfig {
    /// No information in the zone center.
    #[doc = ""]
    #[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!("no_zone_config",
        "docs/img/zone_drawables_config_no_zone_config.svg")
)]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
        `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    /// ![No zone center configuration][no_zone_config]
    None,
    /// Shows the ampere-turns of the coil side for a current of one ampere.
    ///
    /// Since the ampere-turns are the product of the directed current times the
    /// number of coil turns inside the zone, the shown number is either +turns
    /// or -turns, depending on the current direction / polarity of the coil
    /// side. The field value defines the font size.
    #[doc = ""]
    #[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!("ampere_turns",
        "docs/img/zone_drawables_config_ampere_turns.svg")
)]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
        `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    /// ![Ampere turns][ampere_turns]
    AmpereTurns(f64),
    /// Shows the current going through the coil as an arrow which either enters
    /// or exits the image plane (depending on the product of current sign and
    /// zone polarity). See the documentation of [`ZoneArrowConfig`] for
    /// examples.
    Arrow(ZoneArrowConfig),
}

/// Visualization configuration for the zone arrow.
///
/// The fields of this struct determine the appearance of the zone arrow. This
/// is best showcased by comparing two examples for a winding with the following
/// [`WindingTable`](crate::winding_table::WindingTable):
///
/// ```text
/// L \ S │   0   1   2   3   4   5
/// ──────┼────────────────────────
///   0   │   1  -3   2   0   0   0
///   1   │   0   0   0  -1   3  -2
/// ```
///
/// **Example 1**
/// ```ignore
/// ZoneArrowConfig {
///     color_by_phase: true,
///     relative_diameter: 0.6,
///     normalized_current: None,
/// }
/// ```
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![ZoneArrowConfig example 1][color_by_phase]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "color_by_phase",
        "docs/img/zone_drawables_config_zone_arrow_color_by_phase.svg"
    )
)]
///
/// **Example 2**
/// ```ignore
/// ZoneArrowConfig {
///     color_by_phase: false,
///     relative_diameter: 0.8,
///     normalized_current: [0.5, 0.5, -1.0],
/// }
/// ```
#[cfg_attr(
    feature = "doc-images",
    doc = "![ZoneArrowConfig example 2][black_normalized_current]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "black_normalized_current",
        "docs/img/zone_drawables_config_zone_arrow_black_normalized_current.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
///
/// While the influence of the [`ZoneArrowConfig::color_by_phase`] parameter is
/// obvious, the other two deserve a detailed discussion. In example 1,
/// [`ZoneArrowConfig::relative_diameter`] is set to 0.6. Since there is no
/// current, all arrow circles have the same diameter and the shown polarity
/// corresponds to that of the coil side.
///
/// In contrast, in example 2 the arrow circle diameter is determined as the
/// product of the [`ZoneArrowConfig::normalized_current`] and the relative
/// diameter. For phase 1 and 2, the normalized current is 0.5, meaning that the
/// actual diameter is 0.4 (0.8 * 0.5); for phase 3  it is 0.8 (0.8 * 1).
/// Furthermore, the arrow polarity has been inverted for phase 3, because its
/// phase current is negative.
#[derive(Clone, Debug)]
pub struct ZoneArrowConfig {
    /// If true, the geometric objects (lines, circles) forming the arrow are
    /// colored with the [`phase_color`] function (using the coil phase as
    /// the input). If false, the objects are drawn in black.
    pub color_by_phase: bool,
    /// Relative diameter of the arrow circle. Is clamped to [0, 1]. 0 means no
    /// arrow, 1 means that the arrow diameter is either the
    /// [`BoundingBox::height`] or [`BoundingBox::width`] of the zone contour,
    /// whichever is smaller. Correspondingly, a value of 0.5 will result in a
    /// diameter half that value.
    pub relative_diameter: f64,
    /// Normalized current value for each phase which should be between [-1, 1].
    /// If specified, the coil phase is used to index into the contained
    /// vector (phase 1 corresponding to the 0th entry, phase 2
    /// corresponding to the 1st entry and so on). The absolute value at
    /// that index is clamped to [0, 1] and multiplied
    /// with the [`ZoneArrowConfig::relative_diameter`] to get an arrow diameter
    /// whose size corresponds to the current. If the value is negative, the
    /// arrow polarity is inverted. See the [`ZoneArrowConfig`] documentation
    /// for an example.
    pub normalized_current: Option<Vec<f64>>,
}

impl ZoneArrowConfig {
    pub fn new(
        color_by_phase: bool,
        relative_diameter: f64,
        normalized_current: Option<Vec<f64>>,
    ) -> Self {
        return Self {
            color_by_phase,
            relative_diameter,
            normalized_current,
        };
    }

    fn arrow(
        &self,
        phase: i32,
        phases: NonZeroU16,
        arrow_circle_diameter: f64,
        arrow_tip_diameter: f64,
    ) -> [Option<Drawable>; 3] {
        let mut drawables = [None, None, None];

        // Get the normalized current value for the phase. If the phase is zero,
        // the normalized current is also zero.
        let rel_curr = match (phase.abs() as usize).checked_sub(1) {
            Some(idx) => {
                &self
                    .normalized_current
                    .as_ref()
                    .map(|v| v.get(idx))
                    .flatten()
                    .cloned()
                    .unwrap_or(1.0)
                    * f64::from(phase.signum())
            }
            None => 0.0,
        };
        let adj_arrow_circle_diameter = arrow_circle_diameter * rel_curr.abs();

        let phase_col = if self.color_by_phase {
            match NonZeroU16::new(phase.abs() as u16) {
                Some(ph) => phase_color(ph, phases),
                None => INVISIBLE,
            }
        } else {
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            }
        };

        if rel_curr > 0.0 {
            if let Some(arrow_components) =
                positive_current_arrow(adj_arrow_circle_diameter, arrow_tip_diameter, phase_col)
            {
                for (idx, arrow_component) in arrow_components.into_iter().enumerate() {
                    drawables[idx] = Some(arrow_component);
                }
            }
        } else {
            if let Some(arrow_components) =
                negative_current_arrow(adj_arrow_circle_diameter, phase_col)
            {
                for (idx, arrow_component) in arrow_components.into_iter().enumerate() {
                    drawables[idx] = Some(arrow_component);
                }
            }
        }

        return drawables;
    }
}

impl Default for ZoneArrowConfig {
    fn default() -> Self {
        Self {
            color_by_phase: false,
            relative_diameter: 0.8,
            normalized_current: None,
        }
    }
}

/// Create an arrow symbolizing a positive current flow "out of the drawing
/// area".
fn positive_current_arrow(
    diameter_arrow: f64,
    diameter_arrow_tip: f64,
    color: Color,
) -> Option<[Drawable; 2]> {
    let mut style = Style::default();
    style.background_color = INVISIBLE;
    style.line_color = color;
    style.line_width = 1.0;

    // Arrow circle
    let arc = Contour::circle([0.0, 0.0], diameter_arrow / 2.0);
    let outline = Drawable::new(arc, style.clone());

    // Tip circle
    let arc = Contour::circle([0.0, 0.0], diameter_arrow_tip / 2.0);
    style.background_color = color;
    let shape = Shape::try_from(arc).ok()?;
    let tip = Drawable::new(shape, style);

    return Some([outline, tip]);
}

/// Create an arrow symbolizing a negative current flow "into the drawing area".
fn negative_current_arrow(diameter_arrow: f64, color: Color) -> Option<[Drawable; 3]> {
    let mut style = Style::default();
    style.line_color = color;
    style.line_width = 1.0;
    style.background_color = INVISIBLE;

    // Arrow circle
    let arc = Contour::circle([0.0, 0.0], diameter_arrow / 2.0);
    let shape = Shape::try_from(arc).ok()?;
    let outline = Drawable::new(shape, style.clone());

    // Create the "x"

    // From upper right to lower left
    let ls = LineSegment::new(
        [
            diameter_arrow / (2.0 * SQRT_2),
            diameter_arrow / (2.0 * SQRT_2),
        ],
        [
            -diameter_arrow / (2.0 * SQRT_2),
            -diameter_arrow / (2.0 * SQRT_2),
        ],
    )
    .ok()?;
    let first_line = Drawable::new(ls, style.clone());

    // From upper left to lower right
    // From upper right to lower left
    let ls = LineSegment::new(
        [
            -diameter_arrow / (2.0 * SQRT_2),
            diameter_arrow / (2.0 * SQRT_2),
        ],
        [
            diameter_arrow / (2.0 * SQRT_2),
            -diameter_arrow / (2.0 * SQRT_2),
        ],
    )
    .ok()?;
    let second_line = Drawable::new(ls, style.clone());

    return Some([outline, first_line, second_line]);
}

/**
Returns [arrow circle diameter, arrow tip diameter]
 */
fn arrow_diameters<I: Iterator<Item = Contour>>(contours: I, relative_diameter: f64) -> [f64; 2] {
    // Get the maximum arrow diameter by identifying the maximum circle which can be
    // fitted in the slot when its center equals the centroid.
    let mut dia_arrow = std::f64::INFINITY;

    for contour in contours {
        let center = contour.centroid();
        let bb = contour.bounding_box();

        // Maximum horizontal diameter
        let horizontal_line =
            match LineSegment::new([bb.xmin() - 1.0, center[1]], [bb.xmax() + 1.0, center[1]]) {
                Ok(ls) => ls,
                Err(_) => continue,
            };

        let intersections_horizontal: Vec<_> = contour.intersections_par(&horizontal_line);
        let width =
            (intersections_horizontal[0].point[0] - intersections_horizontal[1].point[0]).abs();

        // Maximum vertical diameter
        let vertical_line =
            match LineSegment::new([center[0], bb.ymin() - 1.0], [center[0], bb.ymax() + 1.0]) {
                Ok(ls) => ls,
                Err(_) => continue,
            };

        let intersections_vertical: Vec<_> = contour.intersections_par(&vertical_line);
        let heigth =
            (intersections_vertical[0].point[1] - intersections_vertical[1].point[1]).abs();

        let temp_dia_arrow = if width > heigth {
            heigth * relative_diameter.clamp(0.0, std::f64::INFINITY)
        } else {
            width * relative_diameter.clamp(0.0, std::f64::INFINITY)
        };
        if temp_dia_arrow < dia_arrow {
            dia_arrow = temp_dia_arrow;
        }
    }
    let dia_tip = dia_arrow * 0.1;

    return [dia_arrow, dia_tip];
}

#[cfg(test)]
mod tests {
    use super::*;
    use cairo_viewport::{SideLength, Viewport, compare_or_create};

    #[test]
    fn test_arrows() {
        let mut drawables: Vec<Drawable> = Vec::new();
        let arrow_pos = positive_current_arrow(
            1.0,
            0.1,
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        )
        .unwrap();
        drawables.extend(arrow_pos.into_iter());

        let mut arrow_neg = negative_current_arrow(
            2.0,
            Color {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        )
        .unwrap();
        for d in arrow_neg.iter_mut() {
            d.translate([3.0, 0.0]);
        }
        drawables.extend(arrow_neg.into_iter());

        let view =
            Viewport::from_bounded_entities(drawables.iter(), SideLength::Long(500)).unwrap();
        let path = std::path::Path::new("tests/img/arrow_shapes.png");

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
    }
}
