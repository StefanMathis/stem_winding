//! Create a schematic drawing of the coils as seen from the air gap.

use crate::EndWindingLayouter;
use cairo_viewport::*;
use magnetic_core::{CoreRef, IsCoreRef};
use planar_geo::prelude::*;
use smallvec::SmallVec;
use std::{
    collections::HashMap,
    f64::consts::{FRAC_PI_2, PI},
    mem::MaybeUninit,
};
use winding::{Coil, IsCoil, IsWinding, Zone};

/**
This struct contains the parameters needed to create the air gap view of the end winding.
 */
#[derive(Debug, Clone)]
pub struct EndWindingAirGapView {
    pub arrowhead_length: f64,
    pub line_width: f64,
    pub slot_width: f64,
    pub tooth_width: f64,
    pub axial_length: f64,
    pub start_height: f64,
    pub arrow_head_height: f64,
    /**
    Relative margin between the border of an empty zone and the tooth.
    The absolute margin is calculated as `delta_empty_zone * slot_width / layers`.
     */
    pub delta_empty_zone: f64,
    pub font_size: u32,
    pub end_winding_style: EndWindingStyle,
    /**
    If true, draw both the positive and the negative side. If false, just draw the positive side
     */
    pub draw_both_sides: bool,
}

/**
If this value is set to true, the end winding is represented by horizontal lines which
are layered by an `EndWindingLayouter`
 */
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EndWindingStyle {
    Pointed {
        /**
        In case of an angled winding head representation,
        this value is the angle in rad between the lines
        and the horizontal axis.
         */
        end_winding_coil_angle: f64,
    },
    Layered,
}

impl Default for EndWindingAirGapView {
    fn default() -> Self {
        Self {
            arrowhead_length: 0.5,
            line_width: 1.0,
            slot_width: 1.0,
            tooth_width: 1.0,
            axial_length: 4.0,
            start_height: 0.0,
            arrow_head_height: 1.0,
            delta_empty_zone: 0.1,
            font_size: 14,
            end_winding_style: EndWindingStyle::Pointed {
                end_winding_coil_angle: 30.0 / 180.0 * PI,
            },
            draw_both_sides: true,
        }
    }
}

impl TryFrom<CoreRef<'_>> for EndWindingAirGapView {
    type Error = &'static str;

    fn try_from(core: CoreRef<'_>) -> Result<EndWindingAirGapView, Self::Error> {
        use magnetic_core::uom::si::length::meter;
        let slot = core.slot().ok_or("core is not slotted")?;
        return Ok(Self {
            arrowhead_length: 0.5,
            line_width: 1.0,
            slot_width: slot.bottom_width().get::<meter>(),
            tooth_width: core.tooth_width().get::<meter>(),
            axial_length: core.axial_length().get::<meter>(),
            arrow_head_height: 0.25 * core.axial_length().get::<meter>(),
            start_height: 0.0,
            delta_empty_zone: 0.1,
            font_size: 14,
            end_winding_style: EndWindingStyle::Pointed {
                end_winding_coil_angle: 30.0 / 180.0 * PI,
            },
            draw_both_sides: true,
        });
    }
}

impl EndWindingAirGapView {
    pub fn drawables(&self, winding: &dyn IsWinding) -> Vec<DrawableGeometry> {
        let mut drawables: Vec<DrawableGeometry> = (0..(winding.slots() + 1))
            .into_iter()
            .map(|tooth| self.tooth(tooth).into())
            .collect();

        let colors: Vec<Color> =
            color_gradient_iterator(winding.phases() as usize, colorgrad::turbo()).collect();

        // If the end winding should be layered, create a hashmap of ranks
        let mut layer_winding_head_map: Option<HashMap<Zone, usize>> = None;
        if self.end_winding_style == EndWindingStyle::Layered {
            let mut map = HashMap::with_capacity(winding.number_coils());
            for coil_and_rank in EndWindingLayouter::new(winding) {
                map.insert(
                    coil_and_rank.coil.first_zone(),
                    coil_and_rank.winding_head_layer,
                );
            }
            layer_winding_head_map = Some(map);
        }

        for slot in 0..winding.slots() {
            for layer in 0..winding.layers() {
                let zone = Zone::new(slot, layer);
                match winding.coil_at(zone) {
                    Some(coil) => {
                        if coil.first_zone() == zone {
                            // Read the end winding layer
                            let winding_head_layer = layer_winding_head_map
                                .as_ref()
                                .map(|map| {
                                    map.get(&zone)
                                        .expect("map must contain first zone of every coil")
                                        .clone()
                                })
                                .unwrap_or(0);

                            let (coil_lines, annotations) = self.coil(
                                coil,
                                winding.slots(),
                                winding.layers(),
                                &colors,
                                winding_head_layer,
                            );
                            let arrowheads = self.arrowhead_drawables(
                                coil,
                                winding.slots(),
                                winding.layers(),
                                colors.as_slice(),
                            );
                            for line in coil_lines.into_iter() {
                                drawables.push(line.into());
                            }
                            for head in arrowheads.into_iter() {
                                drawables.push(head.into());
                            }
                            for annotation in annotations.into_iter() {
                                drawables.push(annotation.into());
                            }
                        }
                    }
                    None => drawables.push(self.empty_zone(zone, winding.layers()).into()),
                }
            }
        }

        return drawables;
    }

    pub fn slot_and_tooth_width(&self) -> f64 {
        return self.tooth_width + self.slot_width;
    }

    /**
    Create a drawable for the given empty zone. The total number of layers is also needed
    in order to set the correct zone width.
     */
    pub fn empty_zone(&self, zone: Zone, layers: u16) -> DrawableShape {
        let mut style = Style::default();
        style.line_color = BLACK;
        style.line_style = LineStyle::default_dashed();

        // Calculate the width of a layer
        let layer_width = self.slot_width / (layers as f64);
        let zone_delta = self.delta_empty_zone * layer_width;

        let lower = if self.draw_both_sides {
            -0.5 * self.axial_length
        } else {
            0.0
        };

        let lower_left = Point2::new(
            self.slot_and_tooth_width() * (zone.slot as f64)
                + self.tooth_width
                + zone_delta
                + layer_width * (zone.layer as f64),
            lower,
        );

        let upper_right = Point2::new(
            lower_left.x + layer_width - 2.0 * zone_delta,
            0.5 * self.axial_length,
        );

        let path = Contour::rectangle(lower_left, upper_right).unwrap();
        return DrawableShape::new(path.into(), style);
    }

    /**
    Create a drawable for the tooth with the given index.
    The first tooth comes before the first slot.
     */
    pub fn tooth(&self, tooth: u16) -> DrawableShape {
        let mut style = Style::default();
        style.line_color = BLACK;
        style.background_color = GRAY;

        let lower = if self.draw_both_sides {
            -0.5 * self.axial_length
        } else {
            0.0
        };

        // Top-left coordinates
        let lower_left = Point2::new(self.slot_and_tooth_width() * (tooth as f64), lower);
        let upper_right = Point2::new(lower_left.x + self.tooth_width, 0.5 * self.axial_length);

        let path = Contour::rectangle(lower_left, upper_right).unwrap();
        return DrawableShape::new(path.into(), style);
    }

    /**
    Create the drawables (coil lines, numbers) for the given coil.
    At maximum, two coil lines and four numbers can be created
     */
    pub fn coil(
        &self,
        coil: &Coil,
        slots: u16,
        layers: u16,
        colors: &[Color],
        winding_head_layer: usize,
    ) -> (
        SmallVec<[DrawableSegmentChain; 2]>,
        SmallVec<[DrawableShape; 4]>,
    ) {
        let phase_color = colors
            .get((coil.phase() - 1) as usize)
            .cloned()
            .unwrap_or(BLACK);

        let mut coil_style = Style::default();
        coil_style.line_width = self.line_width;
        coil_style.line_style = LineStyle::Solid;
        coil_style.line_color = phase_color.clone();

        match coil {
            Coil::Half(coil_half) => {
                // Create the straight conductor sections inside the core slots for the outward and the return conductor
                let x_pos = self.horizontal_coil_position(coil_half.zone(), layers);

                return self.coil_drawable_from_coordinates(
                    slots,
                    layers,
                    x_pos,
                    x_pos,
                    coil_style,
                    true,
                    winding_head_layer,
                    None,
                );
            }
            Coil::Full(coil_full) => {
                let pos_zone = coil_full.positive_zone();
                let neg_zone = coil_full.negative_zone();

                // Create the straight conductor sections inside the core slots for the outward and the return conductor
                let x_outward = self.horizontal_coil_position(pos_zone, layers);
                let x_return = self.horizontal_coil_position(neg_zone, layers);

                return self.coil_drawable_from_coordinates(
                    slots,
                    layers,
                    x_outward,
                    x_return,
                    coil_style,
                    coil_full.clockwise(),
                    winding_head_layer,
                    Some(AnnotationInfo {
                        slots,
                        layers,
                        pos_zone,
                        neg_zone,
                        clockwise: coil_full.clockwise(),
                    }),
                );
            }
        }
    }

    pub fn coil_drawable_from_coordinates(
        &self,
        slots: u16,
        layers: u16,
        x_outward: f64,
        x_return: f64,
        coil_style: Style,
        clockwise: bool,
        winding_head_layer: usize,
        annotation_info: Option<AnnotationInfo>,
    ) -> (
        SmallVec<[DrawableSegmentChain; 2]>,
        SmallVec<[DrawableShape; 4]>,
    ) {
        let mut lines = SmallVec::new();
        let separated_coil =
            (clockwise && x_return < x_outward) || (!clockwise && x_return > x_outward);

        let (pos_lines, center_annotations) = match self.end_winding_style {
            EndWindingStyle::Pointed {
                end_winding_coil_angle,
            } => (
                self.coil_head_pointed(
                    slots,
                    x_outward,
                    x_return,
                    end_winding_coil_angle,
                    clockwise,
                ),
                false,
            ),
            EndWindingStyle::Layered => (
                self.coil_head_layered(
                    slots,
                    layers,
                    x_outward,
                    x_return,
                    separated_coil,
                    winding_head_layer,
                ),
                true,
            ),
        };

        // If the end winding crosses the upper or lower limit (middle of first and last tooth respectively), separate the connection in two halfes.
        let color = coil_style.line_color.clone();

        let annotations: SmallVec<[DrawableShape; 4]> = if separated_coil {
            if let Some(info) = annotation_info {
                let coil_cutoff_left = self.coil_cutoff_left();
                let coil_cutoff_right = self.coil_cutoff_right(slots);
                let coil_height_at_cutoff = pos_lines[0].segments()[0].start().y;

                let annotations = info.end_winding_annotations(
                    coil_height_at_cutoff,
                    coil_cutoff_left,
                    coil_cutoff_right,
                    center_annotations,
                    color,
                    self.font_size.into(),
                );

                if self.draw_both_sides {
                    SmallVec::from_buf(mirror_positive_annotations(annotations))
                } else {
                    let mut vec = SmallVec::new();
                    let [a, b] = annotations;
                    vec.push(a);
                    vec.push(b);
                    vec
                }
            } else {
                SmallVec::new()
            }
        } else {
            SmallVec::new()
        };

        for mut pos_line in pos_lines.into_iter() {
            let mut neg_line = pos_line.clone();

            if self.draw_both_sides {
                neg_line.line_reflection(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0));
                neg_line.reverse();
                pos_line.append(neg_line, DEFAULT_EPSILON, DEFAULT_MAX_RELATIVE);
                if !separated_coil {
                    pos_line.close(DEFAULT_EPSILON, DEFAULT_MAX_RELATIVE);
                }
            }

            // Convert to a drawable shape
            lines.push(DrawableSegmentChain::new(pos_line, coil_style.clone()));
        }

        return (lines, annotations);
    }

    fn coil_head_pointed(
        &self,
        slots: u16,
        x_outward: f64,
        x_return: f64,
        end_winding_coil_angle: f64,
        clockwise: bool,
    ) -> SmallVec<[SegmentChain; 2]> {
        let mut vector = SmallVec::new();

        // Clamp the input values
        let x_outward = self.clamp_x(x_outward, slots);
        let x_return = self.clamp_x(x_return, slots);
        let coil_cutoff_left = self.coil_cutoff_left();
        let coil_cutoff_right = self.coil_cutoff_right(slots);

        let half_axial_coil_length = 0.5 * self.axial_coil_length();
        let total_width = self.total_width(slots);
        let ew_angle_sin = end_winding_coil_angle.sin();

        // If the end winding crosses the upper or lower limit (middle of first and last tooth respectively), separate the connection in two halfes.
        let last_to_first = clockwise && x_return < x_outward; // Case 1: End winding crosses the last tooth and continues at the first tooth
        let first_to_last = !clockwise && x_return > x_outward; // Case 2: End winding crosses the first tooth and continues at the last tooth

        if last_to_first || first_to_last {
            // End winding crossing

            let x_return_extrap = if last_to_first {
                x_return + (total_width - self.tooth_width)
            } else {
                x_return - (total_width - self.tooth_width)
            };
            let total_end_winding_width = (x_outward - x_return_extrap).abs();
            let middle_vertex = calc_middle_vertex(x_outward, x_return_extrap, ew_angle_sin);

            let (x1a, x1b, x2a, x2b, x2c) = if last_to_first {
                // End winding is ascending
                if middle_vertex.x > coil_cutoff_right {
                    // End winding middle is behind the last tooth
                    (
                        x_outward,
                        coil_cutoff_right,
                        coil_cutoff_left,
                        x_return - 0.5 * total_end_winding_width,
                        x_return,
                    )
                } else {
                    // End winding middle is before the last tooth
                    (
                        x_return,
                        coil_cutoff_left,
                        coil_cutoff_right,
                        middle_vertex.x,
                        x_outward,
                    )
                }
            } else {
                // End winding direction is descending
                if middle_vertex.x < coil_cutoff_left {
                    // End winding middle is before the first tooth
                    (
                        x_outward,
                        coil_cutoff_left,
                        coil_cutoff_right,
                        x_return + 0.5 * total_end_winding_width,
                        x_return,
                    )
                } else {
                    // End winding middle is behind the first tooth
                    (
                        x_return,
                        coil_cutoff_right,
                        coil_cutoff_left,
                        middle_vertex.x,
                        x_outward,
                    )
                }
            };

            let height_at_cutoff = (x1b - x1a).abs() * ew_angle_sin;

            // Create the lines for the end winding
            if let Ok(line) = SegmentChain::from_points(
                &[
                    Point2::new(x1b, height_at_cutoff + half_axial_coil_length),
                    Point2::new(x1a, half_axial_coil_length),
                    Point2::new(x1a, self.start_height),
                ],
                DEFAULT_EPSILON,
                DEFAULT_MAX_RELATIVE,
            ) {
                vector.push(line)
            }

            if let Ok(line) = SegmentChain::from_points(
                &[
                    Point2::new(x2a, height_at_cutoff + half_axial_coil_length),
                    Point2::new(x2b, middle_vertex.y + half_axial_coil_length),
                    Point2::new(x2c, half_axial_coil_length),
                    Point2::new(x2c, self.start_height),
                ],
                DEFAULT_EPSILON,
                DEFAULT_MAX_RELATIVE,
            ) {
                vector.push(line);
            };
        } else {
            // End winding is fully located between first and last tooth
            if x_outward != x_return {
                let middle_vertex = calc_middle_vertex(x_outward, x_return, ew_angle_sin);
                if let Ok(line) = SegmentChain::from_points(
                    &[
                        Point2::new(x_outward, self.start_height),
                        Point2::new(x_outward, half_axial_coil_length),
                        Point2::new(middle_vertex.x, middle_vertex.y + half_axial_coil_length),
                        Point2::new(x_return, half_axial_coil_length),
                        Point2::new(x_return, self.start_height),
                    ],
                    DEFAULT_EPSILON,
                    DEFAULT_MAX_RELATIVE,
                ) {
                    vector.push(line);
                }
            }
        }
        return vector;
    }

    fn coil_head_layered(
        &self,
        slots: u16,
        layers: u16,
        x_outward: f64,
        x_return: f64,
        separated_coil: bool,
        winding_head_layer: usize,
    ) -> SmallVec<[SegmentChain; 2]> {
        let mut lines = SmallVec::new();
        let layer_dist = self.slot_width / (layers + 1) as f64;
        let layer_height = 0.5 * self.axial_length + layer_dist * winding_head_layer as f64;

        let [x_left, x_right] = if x_outward < x_return {
            [x_outward, x_return]
        } else {
            [x_return, x_outward]
        };

        if separated_coil {
            // First chain
            let mut chain = Vec::with_capacity(3);
            if let Ok(line) = LineSegment::new(
                Point2::new(self.coil_cutoff_left(), layer_height + layer_dist),
                Point2::new(x_left - layer_dist, layer_height + layer_dist),
                DEFAULT_EPSILON,
                DEFAULT_MAX_RELATIVE,
            ) {
                chain.push(line.into());
            }
            if let Ok(arc) = ArcSegment::from_start_radius_stop(
                Point2::new(x_left - layer_dist, layer_height + layer_dist),
                Point2::new(x_left - layer_dist, layer_height),
                Point2::new(x_left, layer_height),
                false,
                DEFAULT_EPSILON,
                DEFAULT_MAX_RELATIVE,
            ) {
                chain.push(arc.into());
            }
            if let Ok(line) = LineSegment::new(
                Point2::new(x_left, layer_height),
                Point2::new(x_left, self.start_height),
                DEFAULT_EPSILON,
                DEFAULT_MAX_RELATIVE,
            ) {
                chain.push(line.into());
            }
            if let Ok(chain) = SegmentChain::new(chain, DEFAULT_EPSILON, DEFAULT_MAX_RELATIVE) {
                lines.push(chain);
            }

            // Second chain
            let mut chain = Vec::with_capacity(3);
            if let Ok(line) = LineSegment::new(
                Point2::new(self.coil_cutoff_right(slots), layer_height + layer_dist),
                Point2::new(x_right + layer_dist, layer_height + layer_dist),
                DEFAULT_EPSILON,
                DEFAULT_MAX_RELATIVE,
            ) {
                chain.push(line.into());
            }
            if let Ok(arc) = ArcSegment::from_start_radius_stop(
                Point2::new(x_right + layer_dist, layer_height + layer_dist),
                Point2::new(x_right + layer_dist, layer_height),
                Point2::new(x_right, layer_height),
                true,
                DEFAULT_EPSILON,
                DEFAULT_MAX_RELATIVE,
            ) {
                chain.push(arc.into());
            }
            if let Ok(line) = LineSegment::new(
                Point2::new(x_right, layer_height),
                Point2::new(x_right, self.start_height),
                DEFAULT_EPSILON,
                DEFAULT_MAX_RELATIVE,
            ) {
                chain.push(line.into());
            }
            if let Ok(chain) = SegmentChain::new(chain, DEFAULT_EPSILON, DEFAULT_MAX_RELATIVE) {
                lines.push(chain);
            }
        } else {
            let mut chain = Vec::with_capacity(5);
            if let Ok(line) = LineSegment::new(
                Point2::new(x_left, self.start_height),
                Point2::new(x_left, layer_height),
                DEFAULT_EPSILON,
                DEFAULT_MAX_RELATIVE,
            ) {
                chain.push(line.into());
            }
            if let Ok(arc) = ArcSegment::from_start_radius_stop(
                Point2::new(x_left, layer_height),
                Point2::new(x_left + layer_dist, layer_height),
                Point2::new(x_left + layer_dist, layer_height + layer_dist),
                false,
                DEFAULT_EPSILON,
                DEFAULT_MAX_RELATIVE,
            ) {
                chain.push(arc.into());
            }
            if let Ok(line) = LineSegment::new(
                Point2::new(x_left + layer_dist, layer_height + layer_dist),
                Point2::new(x_right - layer_dist, layer_height + layer_dist),
                DEFAULT_EPSILON,
                DEFAULT_MAX_RELATIVE,
            ) {
                chain.push(line.into());
            }
            if let Ok(arc) = ArcSegment::from_start_radius_stop(
                Point2::new(x_right - layer_dist, layer_height + layer_dist),
                Point2::new(x_right - layer_dist, layer_height),
                Point2::new(x_right, layer_height),
                false,
                DEFAULT_EPSILON,
                DEFAULT_MAX_RELATIVE,
            ) {
                chain.push(arc.into());
            }
            if let Ok(line) = LineSegment::new(
                Point2::new(x_right, layer_height),
                Point2::new(x_right, self.start_height),
                DEFAULT_EPSILON,
                DEFAULT_MAX_RELATIVE,
            ) {
                chain.push(line.into());
            }

            if let Ok(chain) = SegmentChain::new(chain, DEFAULT_EPSILON, DEFAULT_MAX_RELATIVE) {
                lines.push(chain);
            }
        }
        return lines;
    }

    pub fn arrowhead_drawables(
        &self,
        coil: &Coil,
        slots: u16,
        layers: u16,
        colors: &[Color],
    ) -> SmallVec<[DrawableShape; 2]> {
        let phase_color = colors
            .get((coil.phase() - 1) as usize)
            .cloned()
            .unwrap_or(BLACK);
        let mut arrows = SmallVec::new();

        let arrow_head_size = ArrowHeadSize::SideLength(0.9 * self.slot_width / f64::from(layers));

        for zap in coil.zones_and_polarities() {
            let x = self.horizontal_coil_position(zap.zone, layers);

            arrows.push(self.arrowhead_drawable_from_coordinates(
                slots,
                x,
                self.arrow_head_height,
                arrow_head_size,
                phase_color.clone(),
                zap.positive,
            ))
        }
        return arrows;
    }

    pub fn arrowhead_drawable_from_coordinates(
        &self,
        slots: u16,
        x: f64,
        y: f64,
        arrow_head_size: ArrowHeadSize,
        color: Color,
        positive_arrow: bool,
    ) -> DrawableShape {
        let mut style = Style::default();
        style.line_color = color.clone();
        style.background_color = color;

        // Clamp the input values
        let x = self.clamp_x(x, slots);

        // Positive phase: Arrow points up
        // Negative phase: Arrow points down
        let arrow = if self.draw_both_sides {
            if positive_arrow {
                Contour::arrow_from_head_length_angle(
                    Point2::new(x, -y),
                    self.arrowhead_length,
                    -FRAC_PI_2,
                    arrow_head_size,
                )
                .unwrap()
            } else {
                Contour::arrow_from_head_length_angle(
                    Point2::new(x, y),
                    self.arrowhead_length,
                    FRAC_PI_2,
                    arrow_head_size,
                )
                .unwrap()
            }
        } else {
            if positive_arrow {
                let height = arrow_head_size.height();
                Contour::arrow_from_head_length_angle(
                    Point2::new(x, y - height),
                    self.arrowhead_length,
                    -FRAC_PI_2,
                    arrow_head_size,
                )
                .unwrap()
            } else {
                Contour::arrow_from_head_length_angle(
                    Point2::new(x, y),
                    self.arrowhead_length,
                    FRAC_PI_2,
                    arrow_head_size,
                )
                .unwrap()
            }
        };

        let shape = Shape::from_contour_and_holes(arrow, None);

        return DrawableShape::new(shape, style);
    }

    pub fn cage_end_ring(&self, slots: u16) -> [DrawableSegmentChain; 2] {
        // Draw an connecting "beam" over the conductors
        let x_start = 0.5 * self.tooth_width;
        let x_stop = slots as f64 * self.slot_and_tooth_width() + 0.5 * self.tooth_width;
        let mut shape_style = Style::default();
        shape_style.line_color = BLACK;

        let y = 0.5 * (self.axial_length + self.slot_width);

        let line = SegmentChain::from_points(
            &[Point2::new(x_start, y), Point2::new(x_stop, y)],
            DEFAULT_EPSILON,
            DEFAULT_MAX_RELATIVE,
        )
        .unwrap();

        let lower = DrawableSegmentChain::new(line, shape_style.clone());

        let line = SegmentChain::from_points(
            &[Point2::new(x_start, -y), Point2::new(x_stop, -y)],
            DEFAULT_EPSILON,
            DEFAULT_MAX_RELATIVE,
        )
        .unwrap();

        let upper = DrawableSegmentChain::new(line, shape_style.clone());

        return [lower, upper];
    }

    /**
    Return the horizontal position of a coil line in the given zone.
     */
    pub fn horizontal_coil_position(&self, zone: Zone, layers: u16) -> f64 {
        return self.tooth_width
            + self.slot_and_tooth_width() * zone.slot as f64
            + self.slot_width * (1.0 / layers as f64 * (zone.layer as f64 + 0.5));
    }

    pub fn total_width(&self, slots: u16) -> f64 {
        return self.slot_and_tooth_width() * slots as f64 + self.tooth_width;
    }

    pub fn coil_cutoff_left(&self) -> f64 {
        return 0.5 * self.tooth_width;
    }

    pub fn coil_cutoff_right(&self, slots: u16) -> f64 {
        return self.total_width(slots) - 0.5 * self.tooth_width;
    }

    pub fn axial_coil_length(&self) -> f64 {
        return self.axial_length * 1.2;
    }

    pub fn clamp_x(&self, x: f64, slots: u16) -> f64 {
        return x.clamp(self.coil_cutoff_left(), self.coil_cutoff_right(slots));
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AnnotationInfo {
    pub slots: u16,
    pub layers: u16,
    pub pos_zone: Zone,
    pub neg_zone: Zone,
    pub clockwise: bool,
}

impl AnnotationInfo {
    pub fn left_and_right_zone(&self) -> [Zone; 2] {
        if self.pos_zone < self.neg_zone {
            return [self.pos_zone, self.neg_zone];
        } else {
            return [self.neg_zone, self.pos_zone];
        }
    }

    /**
    Return the end winding annotations in the following order:
    Top right, top left
     */
    fn end_winding_annotations(
        &self,
        coil_height_at_cutoff: f64,
        coil_cutoff_left: f64,
        coil_cutoff_right: f64,
        center_annotations: bool,
        color: Color,
        font_size: f64,
    ) -> [DrawableShape; 2] {
        let mut annotations: [MaybeUninit<DrawableShape>; 2] = [const { MaybeUninit::uninit() }; 2];

        let fixed_offset = Vector2::new(2.0, 2.0);

        let left = Vector2::new(coil_cutoff_left, coil_height_at_cutoff);
        let right = Vector2::new(coil_cutoff_right, coil_height_at_cutoff);
        let [left_zone, right_zone] = self.left_and_right_zone();

        let anchors_and_slots = if center_annotations {
            [
                (Anchor::Right, right_zone.slot),
                (Anchor::Left, left_zone.slot),
            ]
        } else {
            if self.slots == left_zone.slot + right_zone.slot + 1 {
                /*
                According to the slots, the coil head is separated exactly in the middle.
                Now check if the separation is slightly off-center due to the layer combination.
                 */
                match left_zone.layer.cmp(&(self.layers - 1 - right_zone.layer)) {
                    std::cmp::Ordering::Less => [
                        (Anchor::TopRight, right_zone.slot),
                        (Anchor::BottomLeft, left_zone.slot),
                    ],
                    std::cmp::Ordering::Equal => [
                        (Anchor::TopRight, right_zone.slot),
                        (Anchor::TopLeft, left_zone.slot),
                    ],
                    std::cmp::Ordering::Greater => [
                        (Anchor::BottomRight, right_zone.slot),
                        (Anchor::TopLeft, left_zone.slot),
                    ],
                }
            } else if self.slots > left_zone.slot + right_zone.slot {
                [
                    (Anchor::TopRight, right_zone.slot),
                    (Anchor::BottomLeft, left_zone.slot),
                ]
            } else {
                [
                    (Anchor::BottomRight, right_zone.slot),
                    (Anchor::TopLeft, left_zone.slot),
                ]
            }
        };

        // When a number is placed to the right of an end winding line,
        // it needs to be located in the bottom left and vice versa!
        for (elem, (anchor, slot)) in (&mut annotations[..])
            .into_iter()
            .zip(anchors_and_slots.into_iter())
        {
            let position = if slot == left_zone.slot { right } else { left };
            elem.write(coil_wrap_around_annotation(
                (slot + 1).to_string(),
                anchor,
                color.clone(),
                font_size,
                position,
                fixed_offset,
            ));
        }

        return unsafe { std::mem::transmute::<_, [DrawableShape; 2]>(annotations) };
    }
}

// Local helper functions
// ====================================================================================================

fn mirror_positive_annotations(annotations_pos: [DrawableShape; 2]) -> [DrawableShape; 4] {
    let mut annotations: [MaybeUninit<DrawableShape>; 4] = [const { MaybeUninit::uninit() }; 4];
    for (idx, elem) in (&mut annotations[..]).into_iter().enumerate() {
        if idx < 2 {
            elem.write(annotations_pos[idx].clone());
        } else {
            let mut annotation_neg = annotations_pos[idx - 2].clone();

            annotation_neg.line_reflection(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0));

            if let Some(txt) = &mut annotation_neg.style.text {
                txt.fixed_anchor_offset =
                    Vector2::new(txt.fixed_anchor_offset.x, -txt.fixed_anchor_offset.y);
                txt.anchor = match txt.anchor {
                    Anchor::TopLeft => Anchor::BottomLeft,
                    Anchor::TopRight => Anchor::BottomRight,
                    Anchor::BottomLeft => Anchor::TopLeft,
                    Anchor::BottomRight => Anchor::TopRight,
                    _ => txt.anchor,
                };
            }
            elem.write(annotation_neg);
        }
    }

    return unsafe { std::mem::transmute::<_, [DrawableShape; 4]>(annotations) };
}

fn calc_middle_vertex(x1: f64, x2: f64, ew_angle_sin: f64) -> Point2<f64> {
    let xmean = 0.5 * (x1 + x2);
    let delta_x = if x1 > x2 { x1 - xmean } else { x2 - xmean };
    let ymean = delta_x * ew_angle_sin;
    return Point2::new(xmean, ymean);
}

fn coil_wrap_around_annotation(
    text: String,
    anchor: Anchor,
    color: Color,
    font_size: f64,
    position: Vector2<f64>,
    fixed_offset: Vector2<f64>,
) -> DrawableShape {
    let (x, y) = match anchor {
        Anchor::Centroid => (0.0, 0.0),
        Anchor::Center => (0.0, 0.0),
        Anchor::TopLeft => (fixed_offset.x, fixed_offset.y),
        Anchor::Top => (0.0, fixed_offset.y),
        Anchor::TopRight => (-fixed_offset.x, fixed_offset.y),
        Anchor::Right => (-fixed_offset.x, 0.0),
        Anchor::BottomRight => (-fixed_offset.x, -fixed_offset.y),
        Anchor::Bottom => (0.0, -fixed_offset.y),
        Anchor::BottomLeft => (fixed_offset.x, -fixed_offset.y),
        Anchor::Left => (fixed_offset.x, 0.0),
    };
    let fixed_offset = Vector2::new(x, y);

    let text = Text::new(
        text,
        anchor,
        fixed_offset,
        Vector2::new(0.0, 0.0),
        color,
        font_size,
        false,
        0.0,
    );
    let mut shape = text.text_box(Point2::new(0.0, 0.0));
    shape.translate(position);
    let style: Style = text.into();
    return DrawableShape::new(shape, style);
}
