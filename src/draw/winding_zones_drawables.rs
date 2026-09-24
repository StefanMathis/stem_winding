use planar_geo::prelude::*;
use std::{f64::consts::SQRT_2, num::NonZeroU16};
use stem_coil_layout::Zone;
use stem_core::prelude::*;

use crate::{draw::get_phase_color, winding::Winding};

const INVISIBLE: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

pub struct WindingZoneDrawables<'a> {
    winding: &'a dyn Winding,
    winding_zones: WindingZones,
    zone_config: &'a ZoneConfig,
    zone: Zone,
    phase: i32,
    drawables: [Option<Drawable>; 4],
    arrow_circle_diameter: f64,
    arrow_tip_diameter: f64,
}

impl<'a> WindingZoneDrawables<'a> {
    pub fn new(winding: &'a dyn Winding, core: CoreRef<'_>, zone_config: &'a ZoneConfig) -> Self {
        let [arrow_circle_diameter, arrow_tip_diameter] = match &zone_config.center_config {
            Some(c) => match c {
                ZoneCenterConfig::AmpereTurns(_) => [0.0, 0.0],
                ZoneCenterConfig::Arrow(zone_arrow_config) => {
                    let contours = core
                        .winding_zones(&winding.coil_layout())
                        .map(|c| c.contour);
                    arrow_diameters(contours, zone_arrow_config.relative_diameter)
                }
            },
            None => [0.0, 0.0],
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
    type Item = (Zone, Drawable);

    fn next(&mut self) -> Option<Self::Item> {
        for drawable in self.drawables.iter_mut() {
            if let Some(d) = drawable.take() {
                return Some((self.zone, d));
            }
        }

        // Populate self.drawables for the next zone
        let pos_contour = self.winding_zones.next()?;
        self.zone = pos_contour.zone;
        self.phase = self.winding.phase_at(pos_contour.zone);

        let turns = self.winding.turns_at(pos_contour.zone);
        self.drawables = self.zone_config.drawables(
            pos_contour.contour,
            self.phase,
            turns,
            self.winding.phases(),
            self.arrow_circle_diameter,
            self.arrow_tip_diameter,
        );
        return self.next();
    }
}

/**
This struct describes the options for creating the winding shapes.
 */
#[derive(Clone, Debug)]
pub struct ZoneConfig {
    pub background_color: ZoneBackgroundColor,
    pub center_config: Option<ZoneCenterConfig>,
    pub show_empty_zones: bool,
}

impl ZoneConfig {
    pub fn new(
        background_color: ZoneBackgroundColor,
        center_config: Option<ZoneCenterConfig>,
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
        phases: NonZeroU16,
        arrow_circle_diameter: f64,
        arrow_tip_diameter: f64,
    ) -> [Option<Drawable>; 4] {
        let mut drawables = [None, None, None, None];

        if phase == 0 && !self.show_empty_zones {
            return drawables;
        }

        // Zone contour
        let mut zone_style = stem_core::stem_slot::SLOT_STYLE;
        zone_style.background_color = self.background_color.color(phase.abs() as u16, phases);
        if phase == 0 && self.show_empty_zones {
            zone_style.line_style = LineStyle::default_dashed();
        }

        let centroid = contour.centroid();

        if let Some(zone_center_config) = &self.center_config {
            match zone_center_config {
                ZoneCenterConfig::AmpereTurns(font_size) => {
                    let text = if phase > 0 {
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
                    drawables[0] = Some(Drawable::new(contour, zone_style));
                }
                ZoneCenterConfig::Arrow(zone_arrow_config) => {
                    if !zone_arrow_config.color_by_phase {
                        drawables[0] = Some(Drawable::new(contour, zone_style));
                    }

                    let arrow_drawables = zone_arrow_config.arrow(
                        phase,
                        phases,
                        arrow_circle_diameter,
                        arrow_tip_diameter,
                    );
                    let offset = !(zone_arrow_config.color_by_phase) as usize;
                    for (idx, mut drawable) in arrow_drawables.into_iter().enumerate() {
                        if let Some(d) = drawable.as_mut() {
                            d.translate(centroid);
                        }
                        drawables[idx + offset] = drawable;
                    }
                }
            }
        } else {
            drawables[0] = Some(Drawable::new(contour, zone_style));
        }

        return drawables;
    }
}

impl Default for ZoneConfig {
    fn default() -> Self {
        Self {
            background_color: Default::default(),
            center_config: None,
            show_empty_zones: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ZoneBackgroundColor {
    Phase,
    #[default]
    Default,
    None,
}

impl ZoneBackgroundColor {
    pub fn color(&self, phase: u16, phases: NonZeroU16) -> Color {
        match self {
            ZoneBackgroundColor::Phase => match NonZeroU16::new(phase) {
                Some(ph) => get_phase_color(ph, phases),
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
#[derive(Clone, Debug)]
pub enum ZoneCenterConfig {
    AmpereTurns(f64),
    Arrow(ZoneArrowConfig),
}

/**
This struct describes the options for the arrow.

# Fields
* `color_by_phase`: If true, the phase arrows are drawn in the color of the respective shape. Otherwise, they are drawn in black.
* `normalized_current`: If a current vector is given, the size and direction of the arrow will be adjusted according to the current value.
This vector must be normalized (all values between -1 and 1). Outliers will be truncated. If no vector is given, the arrow is always at its maximum size.
Its direction is then derived from the zone plan.
 */
#[derive(Clone, Debug)]
pub struct ZoneArrowConfig {
    pub color_by_phase: bool,
    pub relative_diameter: f64,
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

        let phase_color = if self.color_by_phase {
            match NonZeroU16::new(phase.abs() as u16) {
                Some(ph) => get_phase_color(ph, phases),
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
                positive_current_arrow(adj_arrow_circle_diameter, arrow_tip_diameter, phase_color)
            {
                for (idx, arrow_component) in arrow_components.into_iter().enumerate() {
                    drawables[idx] = Some(arrow_component);
                }
            }
        } else {
            if let Some(arrow_components) =
                negative_current_arrow(adj_arrow_circle_diameter, phase_color)
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
    style.line_width = 1.0;
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
    style.line_width = 1.0;
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
