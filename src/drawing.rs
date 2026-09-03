use rayon::prelude::*;
use std::f64::consts::{PI, SQRT_2};

use nalgebra::{Point2, Vector2};
use planar_geo::prelude::*;

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

    /**
    First value equals the arrow outline diameter, second one the arrow tip diameter
     */
    pub(crate) fn default_arrow_dimensions(&self, slot_shapes: &[Shape]) -> [f64; 2] {
        match &self.center_config {
            Some(center_config) => match center_config {
                ZoneCenterConfig::AmpereTurns => [0.0, 0.0],
                ZoneCenterConfig::Arrow(config) => {
                    arrow_diameters(slot_shapes, config.relative_diameter)
                }
            },
            None => [0.0, 0.0],
        }
    }

    /**
    Return the number of shapes representing a zone
     */
    pub(crate) fn number_shapes(&self) -> usize {
        match self.center_config.as_ref() {
            Some(center_config) => match center_config {
                ZoneCenterConfig::AmpereTurns => 1,
                ZoneCenterConfig::Arrow(_) => 4,
            },
            None => return 1,
        }
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

/**
This struct describes the inner element of the zone shape.

# Variants
* `AmpereTurns`: Write the number of ampere turns in the shape middle, taking into account the polarity of the zone according to the zone plan.
* `Arrow`: Draw an arrow inside the zone shape which is defined by a `ZoneArrowConfig` struct.
 */
#[derive(Clone, Debug)]
pub enum ZoneCenterConfig {
    AmpereTurns,
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

pub(crate) fn add_annotation_shape(
    shapes: &mut Vec<Shape>,
    center: Point2<f64>,
    default_dia_circle: f64,
    default_dia_tip: f64,
    phase: i32,
    center_config: &ZoneCenterConfig,
) {
    match center_config {
        ZoneCenterConfig::AmpereTurns => {
            // No-op
        }
        ZoneCenterConfig::Arrow(zone_arrow_config) => {
            // Modify the size of the arrow outline
            match zone_arrow_config.normalized_current.as_ref() {
                Some(currents) => {
                    let current = currents[(phase.abs() - 1) as usize].clamp(-1.0, 1.0);
                    let weighted_current: f64 = phase.signum() as f64 * current;
                    let dia_arrow =
                        default_dia_circle * zone_arrow_config.relative_diameter * current.abs();

                    // If the current is approximately zero, don't draw anything
                    if approxim::abs_diff_eq!(weighted_current, 0.0, epsilon = 1e-9) {
                        // No-op
                    } else if weighted_current > 0.0 {
                        let mut arrow_shapes = positive_current_arrow(dia_arrow, default_dia_tip);
                        arrow_shapes.iter_mut().for_each(|shape| {
                            shape.translate(Vector2::new(center.x, center.y));
                        });
                        shapes.extend(arrow_shapes.into_iter());
                    } else {
                        let mut arrow_shapes = negative_current_arrow(dia_arrow);
                        arrow_shapes.iter_mut().for_each(|shape| {
                            shape.translate(Vector2::new(center.x, center.y));
                        });
                        shapes.extend(arrow_shapes.into_iter());
                    }
                }
                None => {
                    let dia_arrow = default_dia_circle * zone_arrow_config.relative_diameter;
                    let dia_tip = default_dia_tip * zone_arrow_config.relative_diameter;
                    if phase > 0 {
                        let mut arrow_shapes = positive_current_arrow(dia_arrow, dia_tip);
                        arrow_shapes.iter_mut().for_each(|shape| {
                            shape.translate(Vector2::new(center.x, center.y));
                        });
                        shapes.extend(arrow_shapes.into_iter());
                    } else {
                        let mut arrow_shapes = negative_current_arrow(dia_arrow);
                        arrow_shapes.iter_mut().for_each(|shape| {
                            shape.translate(Vector2::new(center.x, center.y));
                        });
                        shapes.extend(arrow_shapes.into_iter());
                    }
                }
            };
        }
    }
}

/// Create an arrow symbolizing a positive current flow "out of the drawing area".
fn positive_current_arrow(diameter_arrow: f64, diameter_arrow_tip: f64) -> [Shape; 2] {
    // Arrow circle
    let arc = ArcSegment::from_center_radius_start_sweep_angle(
        Point2::new(0.0, 0.0),
        diameter_arrow / 2.0,
        0.0,
        2.0_f64 * PI,
        DEFAULT_EPSILON,
        DEFAULT_MAX_RELATIVE,
    )
    .unwrap();

    let outline = Shape::from_contour_and_holes(
        SegmentChain::new(vec![arc.into()], DEFAULT_EPSILON, DEFAULT_MAX_RELATIVE)
            .unwrap()
            .into(),
        None,
    );

    // Tip circle
    let arc = ArcSegment::from_center_radius_start_sweep_angle(
        Point2::new(0.0, 0.0),
        diameter_arrow_tip / 2.0,
        0.0,
        2.0_f64 * PI,
        DEFAULT_EPSILON,
        DEFAULT_MAX_RELATIVE,
    )
    .unwrap();

    let tip = Shape::from_contour_and_holes(
        SegmentChain::new(vec![arc.into()], DEFAULT_EPSILON, DEFAULT_MAX_RELATIVE)
            .unwrap()
            .into(),
        None,
    );

    return [outline, tip];
}

/// Create an arrow symbolizing a negative current flow "into the drawing area".
fn negative_current_arrow(diameter_arrow: f64) -> [Shape; 3] {
    // Arrow circle
    let arc = ArcSegment::from_center_radius_start_sweep_angle(
        Point2::new(0.0, 0.0),
        diameter_arrow / 2.0,
        0.0,
        2.0_f64 * PI,
        DEFAULT_EPSILON,
        DEFAULT_MAX_RELATIVE,
    )
    .unwrap();

    let outline = Shape::from_contour_and_holes(
        SegmentChain::new(vec![arc.into()], DEFAULT_EPSILON, DEFAULT_MAX_RELATIVE)
            .unwrap()
            .into(),
        None,
    );

    // Create the "x"

    // From upper right to lower left
    let points = [
        Point2::new(
            diameter_arrow / (2.0 * SQRT_2),
            diameter_arrow / (2.0 * SQRT_2),
        ),
        Point2::new(
            -diameter_arrow / (2.0 * SQRT_2),
            -diameter_arrow / (2.0 * SQRT_2),
        ),
    ];
    let first_line = Shape::from_contour_and_holes(
        SegmentChain::from_points(points.as_slice(), DEFAULT_EPSILON, DEFAULT_MAX_RELATIVE)
            .unwrap()
            .into(),
        None,
    );

    // From upper left to lower right
    let points = [
        Point2::new(
            -diameter_arrow / (2.0 * SQRT_2),
            diameter_arrow / (2.0 * SQRT_2),
        ),
        Point2::new(
            diameter_arrow / (2.0 * SQRT_2),
            -diameter_arrow / (2.0 * SQRT_2),
        ),
    ];
    let second_line = Shape::from_contour_and_holes(
        SegmentChain::from_points(points.as_slice(), DEFAULT_EPSILON, DEFAULT_MAX_RELATIVE)
            .unwrap()
            .into(),
        None,
    );

    return [outline, first_line, second_line];
}

/**
Returns [arrow circle diameter, arrow tip diameter]
 */
pub(crate) fn arrow_diameters(slot_shapes: &[Shape], relative_diameter: f64) -> [f64; 2] {
    // Get the maximum arrow diameter by identifying the maximum circle which can be fitted in the slot when its center equals the centroid.
    let mut dia_arrow = std::f64::INFINITY;

    for shape in slot_shapes.iter() {
        let center = shape.contour().centroid();
        let bb = shape.bounding_box();

        // Maximum horizontal diameter
        let horizontal_line = SegmentChain::from_points(
            [
                Point2::new(bb.xmin() - 1.0, center.y),
                Point2::new(bb.xmax() + 1.0, center.y),
            ]
            .as_slice(),
            DEFAULT_EPSILON,
            DEFAULT_MAX_RELATIVE,
        )
        .unwrap();

        let intersections_horizontal: Vec<_> = shape
            .contour()
            .intersections_par(&horizontal_line, DEFAULT_EPSILON, DEFAULT_MAX_RELATIVE)
            .collect();
        let width =
            (intersections_horizontal[0].point.x - intersections_horizontal[1].point.x).abs();

        // Maximum vertical diameter
        let vertical_line = SegmentChain::from_points(
            [
                Point2::new(center.x, bb.ymin() - 1.0),
                Point2::new(center.x, bb.ymax() + 1.0),
            ]
            .as_slice(),
            DEFAULT_EPSILON,
            DEFAULT_MAX_RELATIVE,
        )
        .unwrap();
        let intersections_vertical: Vec<_> = shape
            .contour()
            .intersections_par(&vertical_line, DEFAULT_EPSILON, DEFAULT_MAX_RELATIVE)
            .collect();
        let heigth = (intersections_vertical[0].point.y - intersections_vertical[1].point.y).abs();

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
    use cairo_viewport::*;
    use material::Material;
    use slot::{
        SlotTrapezoidSemi,
        is_slot::{angle_bottom_no_slope, angle_top_no_slope},
    };

    use crate::*;
    use magnetic_core::{CarterFactorModel, CoreLin, IsCoreRef};
    use uom::si::{f64::*, length::millimeter};

    use super::*;

    #[test]
    fn test_test_arrows() {
        let mut shapes: Vec<Shape> = Vec::new();
        let arrow_pos = positive_current_arrow(1.0, 0.1);
        shapes.extend(arrow_pos.into_iter());

        let mut arrow_neg = negative_current_arrow(2.0);
        for shape in arrow_neg.iter_mut() {
            shape.translate(Vector2::new(3.0, 0.0));
        }
        shapes.extend(arrow_neg.into_iter());

        let mut style_pos = Style::default();
        style_pos.line_color = visualization::color::BLACK;
        style_pos.line_width = 1.0;
        let mut style_pos_filled = style_pos.clone();
        style_pos_filled.background_color = visualization::color::BLACK;

        let mut style_neg = Style::default();
        style_neg.line_color = visualization::color::RED;
        style_neg.line_width = 1.0;

        let styles = [
            style_pos,
            style_pos_filled,
            style_neg.clone(),
            style_neg.clone(),
            style_neg,
        ];

        let drawables: Vec<DrawableShape> = shapes
            .into_iter()
            .zip(styles.into_iter())
            .map(|(shape, style)| (shape, style).into())
            .collect();

        let view = visualization::Viewport::from_bounded_entities(drawables.iter(), 500).unwrap();
        let path = std::path::Path::new("img/arrow_shapes.png"); // Always compare to the same reference image
        let callback = move |path: &std::path::Path| {
            return view.write_to_file(path, &|cr| {
                for drawable in drawables.iter() {
                    drawable.draw(cr);
                }
            });
        };
        assert!(compare_or_create(path, &callback).is_ok());
    }

    #[test]
    fn test_default_arrow_dimensions() {
        let core = create_core();
        let winding = WindingToothCoil::new_minimal(12, 5, 3, 2, WindingTableMethod::Tingley).unwrap();
        let single_slot_shapes = winding.slot_shapes(core.slot().unwrap());

        let zone_config = ZoneConfig::new(
            ZoneBackgroundColor::Phase,
            Some(ZoneCenterConfig::Arrow(ZoneArrowConfig::new(
                false, 0.8, None,
            ))),
            true,
        );
        let [circle, tip] = zone_config.default_arrow_dimensions(single_slot_shapes.as_slice());
        approxim::assert_abs_diff_eq!(circle, 3.2e-3, epsilon = 1e-6);
        approxim::assert_abs_diff_eq!(tip, 3.2e-4, epsilon = 1e-6);
    }

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

        return magnetic_core::CoreLinBuilder {
            height: Length::new::<millimeter>(25.0),
            width: Length::new::<millimeter>(150.0),
            axial_length: Length::new::<millimeter>(100.0),
            axial_coil_overhang: Length::new::<millimeter>(0.0),
            skew_angle: 0.0,
            iron_fill_factor: 1.0,
            material: std::sync::Arc::new(Material::default()),
            pole_pairs: 5,
            air_gap: Box::new(magnetic_core::AirGapSlotted {
                slots: 12,
                starts_in_slot_middle: true,
                carter_factor_model: CarterFactorModel::PS62,
                slot: Box::new(slot),
            }),
            flux_barrier: None,
        }
        .try_into()
        .unwrap();
    }
}
