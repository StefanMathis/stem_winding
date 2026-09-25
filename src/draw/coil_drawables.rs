//! Create a schematic drawing of the coils as seen from the air gap.

use std::{
    collections::HashMap,
    f64::consts::{FRAC_PI_2, PI},
    num::NonZeroU16,
};

use compare_variables::{Comparison, compare_variables};
use planar_geo::prelude::*;
use stem_core::prelude::*;

use crate::{
    coils::{Coil, CoilExt},
    draw::{EndWindingLayouter, get_phase_color},
    winding::Winding,
};

const BLACK: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

pub struct CoilDrawables<'a> {
    winding: &'a dyn Winding,
    parameters: &'a CoilDrawablesParameters,
    layer_winding_head_map: Option<HashMap<Zone, usize>>,
    colors: Vec<Color>,
    drawables: [Option<Drawable>; 8],
    index: u16,
    zone: Zone,
}

impl<'a> CoilDrawables<'a> {
    pub fn new(winding: &'a dyn Winding, parameters: &'a CoilDrawablesParameters) -> Self {
        let colors: Vec<Color> = (0..winding.phases().get())
            .map(|p| {
                get_phase_color(
                    NonZeroU16::new(p + 1).expect("is always larger than zero"),
                    winding.phases(),
                )
            })
            .collect();

        // If the end winding should be layered, create a hashmap of ranks
        let mut layer_winding_head_map: Option<HashMap<Zone, usize>> = None;
        if parameters.end_winding_style == EndWindingStyle::Layered {
            let mut map = HashMap::with_capacity(winding.number_coils());
            for coil_and_rank in EndWindingLayouter::new(winding, parameters.cyclic) {
                map.insert(
                    coil_and_rank.coil.any_zone(),
                    coil_and_rank.winding_head_layer,
                );
            }
            layer_winding_head_map = Some(map);
        }

        Self {
            winding,
            parameters,
            layer_winding_head_map,
            colors,
            drawables: [None, None, None, None, None, None, None, None],
            index: 0,
            zone: Zone { slot: 0, layer: 0 },
        }
    }
}

impl<'a> Iterator for CoilDrawables<'a> {
    type Item = (Zone, Drawable);

    fn next(&mut self) -> Option<Self::Item> {
        for drawable in self.drawables.iter_mut() {
            if let Some(d) = drawable.take() {
                return Some((self.zone, d));
            }
        }

        let layers = self.winding.layers().get();
        if self.index >= self.winding.slots().get() * layers {
            return None;
        }
        let layer = self.index.rem_euclid(layers);
        let slot = self.index / layers;
        self.zone = Zone { slot, layer };

        self.index += 1;

        match self.winding.coil_at(self.zone) {
            Some(coil) => {
                if coil.any_zone() == self.zone {
                    // Read the end winding layer
                    let winding_head_layer = self
                        .layer_winding_head_map
                        .as_ref()
                        .map(|map| {
                            map.get(&self.zone)
                                .expect("map must contain first zone of every coil")
                                .clone()
                        })
                        .unwrap_or(0);

                    let coil_drawables = self.parameters.coil(
                        coil,
                        self.winding.slots(),
                        self.winding.layers(),
                        &self.colors,
                        winding_head_layer,
                    );
                    let offset = coil_drawables.len();
                    for (i, d) in coil_drawables.into_iter().enumerate() {
                        self.drawables[i] = d;
                    }

                    if let Ok(arrowheads) = self.parameters.arrowhead_drawables(
                        coil,
                        self.winding.slots(),
                        self.winding.layers(),
                    ) {
                        // Cannot underflow, since phase is NonZeroU16, i.e. larger than 0.
                        let color_idx = usize::from(coil.phase().get()) - 1;
                        let style = self.parameters.coil_style(self.colors[color_idx]);
                        for (i, c) in arrowheads.into_iter().enumerate() {
                            self.drawables[offset + i] = Some(Drawable::new(c, style.clone()));
                        }
                    }
                }
            }
            None => {
                let contour = self.parameters.empty_zone(self.zone, layers);
                let style = self.parameters.empty_zone_style();
                self.drawables[0] = Some(Drawable::new(contour, style));
            }
        }
        return self.next();
    }
}

/**
This struct contains the parameters needed to create the air gap view of the end winding.
 */
#[derive(Debug, Clone)]
pub struct CoilDrawablesParameters {
    pub arrowhead_length: f64,
    pub line_width: f64,
    pub slot_width: f64,
    pub tooth_width: f64,
    pub axial_length: f64,
    pub start_height: f64,
    pub arrow_head_height: f64,
    pub axial_coil_overhang: f64,
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
    pub cyclic: bool,
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

impl Default for CoilDrawablesParameters {
    fn default() -> Self {
        Self {
            arrowhead_length: 0.5,
            line_width: 1.0,
            slot_width: 1.0,
            tooth_width: 1.0,
            axial_length: 4.0,
            axial_coil_overhang: 0.8,
            start_height: 0.0,
            arrow_head_height: 1.0,
            delta_empty_zone: 0.1,
            font_size: 14,
            end_winding_style: EndWindingStyle::Pointed {
                end_winding_coil_angle: 30.0 / 180.0 * PI,
            },
            draw_both_sides: true,
            cyclic: true,
        }
    }
}

impl From<CoreRef<'_>> for CoilDrawablesParameters {
    fn from(core: CoreRef<'_>) -> CoilDrawablesParameters {
        let mut tooth_width = core
            .tooth_width_at(0.5 * core.tooth_height())
            .get::<meter>();
        let slot_width = if let Some(slot) = core.slot() {
            slot.width_at(0.5 * slot.height()).get::<meter>()
        } else {
            tooth_width = tooth_width * 0.5;
            0.5 * tooth_width
        };
        let cyclic = core.rot().is_some();

        return Self {
            arrowhead_length: 0.5,
            line_width: 1.0,
            slot_width,
            tooth_width,
            axial_length: core.axial_length().get::<meter>(),
            axial_coil_overhang: 1.2 * core.axial_length().get::<meter>(),
            arrow_head_height: 0.25 * core.axial_length().get::<meter>(),
            start_height: 0.0,
            delta_empty_zone: 0.1,
            font_size: 14,
            end_winding_style: EndWindingStyle::Pointed {
                end_winding_coil_angle: 30.0 / 180.0 * PI,
            },
            draw_both_sides: true,
            cyclic,
        };
    }
}

impl CoilDrawablesParameters {
    pub fn max_coil_height(&self, winding: &dyn Winding) -> f64 {
        let layers = winding.layers();
        match self.end_winding_style {
            EndWindingStyle::Pointed {
                end_winding_coil_angle,
            } => {
                let total_len = f64::from(winding.slots().get()) * self.slot_and_tooth_width();

                // Find the coil with the largest throw; it determines the
                // height of the end winding
                let max_dist = winding.coils().fold(0.0f64, |acc, c| match c {
                    Coil::Full(full_coil) => {
                        let x1 = self.horizontal_coil_position(full_coil.positive_zone(), layers);
                        let x2 = self.horizontal_coil_position(full_coil.negative_zone(), layers);

                        let zone_dist = if self.cyclic {
                            if full_coil.positive_slot_direction() {
                                if full_coil.positive_zone() < full_coil.negative_zone() {
                                    x2 - x1
                                } else {
                                    total_len - (x1 - x2)
                                }
                            } else {
                                if full_coil.positive_zone() < full_coil.negative_zone() {
                                    total_len - (x2 - x1)
                                } else {
                                    x1 - x2
                                }
                            }
                        } else {
                            (x1 - x2).abs()
                        };
                        acc.max(zone_dist)
                    }
                    Coil::Half(_) => acc,
                });
                let height = 0.5 * max_dist * end_winding_coil_angle.sin();
                return height + 0.5 * self.axial_coil_length();
            }
            EndWindingStyle::Layered => {
                // Find the highest rank
                let highest_rank = EndWindingLayouter::new(winding, self.cyclic)
                    .fold(0usize, |acc, car| acc.max(car.winding_head_layer));

                let layer_dist = self.slot_width / (layers.get() + 1) as f64;
                return 0.5 * self.axial_coil_length() + layer_dist * (highest_rank + 1) as f64;
            }
        }
    }

    pub fn bounding_box(&self, winding: &dyn Winding, with_teeth: bool) -> BoundingBox {
        let ymax = self.max_coil_height(winding);
        let ymin = if self.draw_both_sides { -ymax } else { 0.0 };
        let mut xmin = 0.0;
        let mut xmax =
            self.slot_and_tooth_width() * f64::from(winding.slots().get()) + self.tooth_width;
        if !with_teeth {
            xmin += 0.5 * self.tooth_width;
            xmax -= 0.5 * self.tooth_width;
        }

        BoundingBox::try_new(xmin, xmax, ymin, ymax).unwrap_or(BoundingBox::new(0.0, 0.0, 0.0, 0.0))
    }

    pub fn validate(&self) -> Result<(), Comparison<f64>> {
        compare_variables!(self.arrowhead_length >= 0.0)?;
        compare_variables!(self.line_width >= 0.0)?;
        compare_variables!(self.tooth_width >= 0.0)?;
        compare_variables!(self.axial_length >= 0.0)?;
        compare_variables!(self.delta_empty_zone >= 0.0)?;
        return Ok(());
    }

    pub fn tooth_drawable(&self, slot: u16) -> Drawable {
        let lower = if self.draw_both_sides {
            -0.5 * self.axial_length
        } else {
            0.0
        };

        // Top-left coordinates
        let lower_left = [self.slot_and_tooth_width() * f64::from(slot), lower];
        let upper_right = [lower_left[0] + self.tooth_width, 0.5 * self.axial_length];

        return Drawable::new(
            Contour::rectangle(lower_left, upper_right),
            Style {
                line_color: BLACK,
                background_color: stem_core::GRAY,
                line_width: 0.5,
                line_style: LineStyle::Solid,
                line_cap: LineCap::Round,
                line_join: LineJoin::Miter,
                text: None,
            },
        );
    }

    pub fn coil_style(&self, color: Color) -> Style {
        Style {
            line_color: color,
            background_color: color,
            line_width: self.line_width,
            line_style: LineStyle::Solid,
            line_cap: LineCap::Round,
            line_join: LineJoin::Miter,
            text: None,
        }
    }

    pub fn empty_zone_style(&self) -> Style {
        Style {
            line_color: BLACK,
            background_color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
            line_width: 0.5,
            line_style: LineStyle::default_dashed(),
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            text: None,
        }
    }

    pub fn slot_and_tooth_width(&self) -> f64 {
        return self.tooth_width + self.slot_width;
    }

    /**
    Create a drawable for the given empty zone. The total number of layers is also needed
    in order to set the correct zone width.
     */
    pub fn empty_zone(&self, zone: Zone, layers: u16) -> Contour {
        // Calculate the width of a layer
        let layer_width = self.slot_width / (layers as f64);
        let zone_delta = self.delta_empty_zone * layer_width;

        let lower = if self.draw_both_sides {
            -0.5 * self.axial_length
        } else {
            0.0
        };

        let lower_left = [
            self.slot_and_tooth_width() * (zone.slot as f64)
                + self.tooth_width
                + zone_delta
                + layer_width * (zone.layer as f64),
            lower,
        ];

        let upper_right = [
            lower_left[0] + layer_width - 2.0 * zone_delta,
            0.5 * self.axial_length,
        ];

        return Contour::rectangle(lower_left, upper_right);
    }

    /**
    Create the drawables (coil lines, numbers) for the given coil.
    At maximum, two coil lines and four numbers can be created
     */
    pub fn coil(
        &self,
        coil: &Coil,
        slots: NonZeroU16,
        layers: NonZeroU16,
        colors: &[Color],
        winding_head_layer: usize,
    ) -> [Option<Drawable>; 6] {
        // Cannot underflow, since phase is NonZeroU16, i.e. larger than 0.
        let color_idx = usize::from(coil.phase().get()) - 1;
        let phase_color = colors.get(color_idx).cloned().unwrap_or(BLACK);

        let mut coil_style = Style::default();
        coil_style.line_width = self.line_width;
        coil_style.line_style = LineStyle::Solid;
        coil_style.line_color = phase_color.clone();

        match coil {
            Coil::Half(coil_half) => {
                // Create the straight conductor sections inside the core slots for the outward
                // and the return conductor
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

                // Create the straight conductor sections inside the core slots for the outward
                // and the return conductor
                let x_outward = self.horizontal_coil_position(pos_zone, layers);
                let x_return = self.horizontal_coil_position(neg_zone, layers);

                return self.coil_drawable_from_coordinates(
                    slots,
                    layers,
                    x_outward,
                    x_return,
                    coil_style,
                    coil_full.positive_slot_direction(),
                    winding_head_layer,
                    Some(AnnotationInfo {
                        slots,
                        layers,
                        pos_zone,
                        neg_zone,
                        positive_slot_direction: coil_full.positive_slot_direction(),
                    }),
                );
            }
        }
    }

    pub fn coil_drawable_from_coordinates(
        &self,
        slots: NonZeroU16,
        layers: NonZeroU16,
        x_outward: f64,
        x_return: f64,
        coil_style: Style,
        positive_slot_direction: bool,
        winding_head_layer: usize,
        annotation_info: Option<AnnotationInfo>,
    ) -> [Option<Drawable>; 6] {
        let separated_coil = (positive_slot_direction && x_return < x_outward)
            || (!positive_slot_direction && x_return > x_outward);

        let (pos_lines, center_annotations) = match self.end_winding_style {
            EndWindingStyle::Pointed {
                end_winding_coil_angle,
            } => (
                self.coil_head_pointed(
                    slots,
                    x_outward,
                    x_return,
                    end_winding_coil_angle,
                    positive_slot_direction,
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

        // If the end winding crosses the upper or lower limit (middle of first and last
        // tooth respectively), separate the connection in two halfes.
        let color = coil_style.line_color.clone();

        let mut drawables = [None, None, None, None, None, None];

        if separated_coil {
            if let Some(info) = annotation_info {
                let coil_cutoff_left = self.coil_cutoff_left();
                let coil_cutoff_right = self.coil_cutoff_right(slots);
                let coil_height_at_cutoff =
                    (pos_lines[0].as_ref()).map_or(0.0, |ps| ps[0].start()[1]);

                let annotations = info.end_winding_annotations(
                    coil_height_at_cutoff,
                    coil_cutoff_left,
                    coil_cutoff_right,
                    center_annotations,
                    color,
                    self.font_size.into(),
                );

                if self.draw_both_sides {
                    for (i, annotation) in duplicate_positive_annotations(annotations)
                        .into_iter()
                        .enumerate()
                    {
                        drawables[i + 2] = Some(Drawable::from(annotation));
                    }
                } else {
                    for (i, annotation) in annotations.into_iter().enumerate() {
                        drawables[i + 2] = Some(Drawable::from(annotation));
                    }
                }
            }
        }

        for (i, mut line) in pos_lines.into_iter().enumerate() {
            if let Some(mut pos_line) = line.take() {
                let mut neg_line = pos_line.clone();

                if self.draw_both_sides {
                    neg_line.line_reflection([0.0, 0.0], [1.0, 0.0]);
                    neg_line.reverse();
                    pos_line.append(&mut neg_line);
                    if !separated_coil {
                        pos_line.close();
                    }
                }
                drawables[i] = Some(Drawable::new(pos_line, coil_style.clone()));
            }
        }

        return drawables;
    }

    fn coil_head_pointed(
        &self,
        slots: NonZeroU16,
        x_outward: f64,
        x_return: f64,
        end_winding_coil_angle: f64,
        positive_slot_direction: bool,
    ) -> [Option<Polysegment>; 2] {
        let mut polysegments = [None, None];

        // Clamp the input values
        let x_outward = self.clamp_x(x_outward, slots);
        let x_return = self.clamp_x(x_return, slots);
        let coil_cutoff_left = self.coil_cutoff_left();
        let coil_cutoff_right = self.coil_cutoff_right(slots);

        let half_axial_coil_length = 0.5 * self.axial_coil_length();
        let total_width = self.total_width(slots);
        let ew_angle_sin = end_winding_coil_angle.sin();

        // If the end winding crosses the upper or lower limit (middle of first and last
        // tooth respectively), separate the connection in two halfes.

        // Case 1: End winding crosses the last tooth and continues at the first tooth
        let last_to_first = positive_slot_direction && x_return < x_outward;

        // Case 2: End winding crosses the first tooth and continues at the last tooth
        let first_to_last = !positive_slot_direction && x_return > x_outward;

        if last_to_first || first_to_last {
            // End winding crossing

            let x_return_extrap = if last_to_first {
                x_return + (total_width - self.tooth_width)
            } else {
                x_return - (total_width - self.tooth_width)
            };
            let total_end_winding_width = (x_outward - x_return_extrap).abs();
            let middle_vertex = get_middle_vertex(x_outward, x_return_extrap, ew_angle_sin);

            let (x1a, x1b, x2a, x2b, x2c) = if last_to_first {
                // End winding is ascending
                if middle_vertex[0] > coil_cutoff_right {
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
                        middle_vertex[0],
                        x_outward,
                    )
                }
            } else {
                // End winding direction is descending
                if middle_vertex[0] < coil_cutoff_left {
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
                        middle_vertex[0],
                        x_outward,
                    )
                }
            };

            let height_at_cutoff = (x1b - x1a).abs() * ew_angle_sin;

            // Create the lines for the end winding
            polysegments[0] = Some(Polysegment::from_points(&[
                [x1b, height_at_cutoff + half_axial_coil_length],
                [x1a, half_axial_coil_length],
                [x1a, self.start_height],
            ]));

            polysegments[1] = Some(Polysegment::from_points(&[
                [x2a, height_at_cutoff + half_axial_coil_length],
                [x2b, middle_vertex[1] + half_axial_coil_length],
                [x2c, half_axial_coil_length],
                [x2c, self.start_height],
            ]));
        } else {
            // End winding is fully located between first and last tooth
            if x_outward != x_return {
                let middle_vertex = get_middle_vertex(x_outward, x_return, ew_angle_sin);
                polysegments[0] = Some(Polysegment::from_points(&[
                    [x_outward, self.start_height],
                    [x_outward, half_axial_coil_length],
                    [middle_vertex[0], middle_vertex[1] + half_axial_coil_length],
                    [x_return, half_axial_coil_length],
                    [x_return, self.start_height],
                ]));
            }
        }
        return polysegments;
    }

    fn coil_head_layered(
        &self,
        slots: NonZeroU16,
        layers: NonZeroU16,
        x_outward: f64,
        x_return: f64,
        separated_coil: bool,
        winding_head_layer: usize,
    ) -> [Option<Polysegment>; 2] {
        let mut polysegments = [None, None];
        let layer_dist = self.slot_width / (layers.get() + 1) as f64;
        let layer_height = 0.5 * self.axial_coil_length() + layer_dist * winding_head_layer as f64;

        let [x_left, x_right] = if x_outward < x_return {
            [x_outward, x_return]
        } else {
            [x_return, x_outward]
        };

        if separated_coil {
            // First chain
            let mut ps1 = Polysegment::new();
            if let Ok(line) = LineSegment::new(
                [self.coil_cutoff_left(), layer_height + layer_dist],
                [x_left - layer_dist, layer_height + layer_dist],
            ) {
                ps1.push_back(line.into());
            }
            if let Ok(arc) = ArcSegment::from_start_center_angle(
                [x_left - layer_dist, layer_height + layer_dist],
                [x_left - layer_dist, layer_height],
                -FRAC_PI_2,
            ) {
                ps1.push_back(arc.into());
            }
            if let Ok(line) = LineSegment::new([x_left, layer_height], [x_left, self.start_height])
            {
                ps1.push_back(line.into());
            }
            polysegments[0] = Some(ps1);

            // Second chain
            let mut ps2 = Polysegment::new();
            if let Ok(line) = LineSegment::new(
                [self.coil_cutoff_right(slots), layer_height + layer_dist],
                [x_right + layer_dist, layer_height + layer_dist],
            ) {
                ps2.push_back(line.into());
            }
            if let Ok(arc) = ArcSegment::from_start_center_angle(
                [x_right + layer_dist, layer_height + layer_dist],
                [x_right + layer_dist, layer_height],
                FRAC_PI_2,
            ) {
                ps2.push_back(arc.into());
            }
            if let Ok(line) =
                LineSegment::new([x_right, layer_height], [x_right, self.start_height])
            {
                ps2.push_back(line.into());
            }
            polysegments[1] = Some(ps2);
        } else {
            let mut ps = Polysegment::new();
            if let Ok(line) = LineSegment::new([x_left, self.start_height], [x_left, layer_height])
            {
                ps.push_back(line.into());
            }
            if let Ok(arc) = ArcSegment::from_start_center_angle(
                [x_left, layer_height],
                [x_left + layer_dist, layer_height],
                -FRAC_PI_2,
            ) {
                ps.push_back(arc.into());
            }
            if let Ok(line) = LineSegment::new(
                [x_left + layer_dist, layer_height + layer_dist],
                [x_right - layer_dist, layer_height + layer_dist],
            ) {
                ps.push_back(line.into());
            }
            if let Ok(arc) = ArcSegment::from_start_center_angle(
                [x_right - layer_dist, layer_height + layer_dist],
                [x_right - layer_dist, layer_height],
                -FRAC_PI_2,
            ) {
                ps.push_back(arc.into());
            }
            if let Ok(line) =
                LineSegment::new([x_right, layer_height], [x_right, self.start_height])
            {
                ps.push_back(line.into());
            }
            polysegments[0] = Some(ps);
        }
        return polysegments;
    }

    pub fn arrowhead_drawables(
        &self,
        coil: &Coil,
        slots: NonZeroU16,
        layers: NonZeroU16,
    ) -> Result<[Contour; 2], planar_geo::error::Error> {
        let mut arrows = [
            Contour::new(Polysegment::new()),
            Contour::new(Polysegment::new()),
        ];

        let arrow_head_size =
            ArrowHeadSize::SideLength(0.9 * self.slot_width / f64::from(layers.get()));

        for (idx, zap) in coil.zones_and_polarities().enumerate() {
            let x = self.horizontal_coil_position(zap.zone, layers);
            arrows[idx] = self.arrowhead_drawable_from_coordinates(
                slots,
                x,
                self.arrow_head_height,
                arrow_head_size,
                zap.is_positive,
            )?;
        }
        return Ok(arrows);
    }

    pub fn arrowhead_drawable_from_coordinates(
        &self,
        slots: NonZeroU16,
        x: f64,
        y: f64,
        arrow_head_size: ArrowHeadSize,
        positive_arrow: bool,
    ) -> Result<Contour, planar_geo::error::Error> {
        // Clamp the input values
        let x = self.clamp_x(x, slots);

        // Positive phase: Arrow points up
        // Negative phase: Arrow points down
        if self.draw_both_sides {
            if positive_arrow {
                return Contour::arrow_from_head_length_angle(
                    [x, -y],
                    self.arrowhead_length,
                    -FRAC_PI_2,
                    0.0,
                    arrow_head_size,
                );
            } else {
                return Contour::arrow_from_head_length_angle(
                    [x, y],
                    self.arrowhead_length,
                    FRAC_PI_2,
                    0.0,
                    arrow_head_size,
                );
            }
        } else {
            if positive_arrow {
                let height = arrow_head_size.height();
                return Contour::arrow_from_head_length_angle(
                    [x, y - height],
                    self.arrowhead_length,
                    -FRAC_PI_2,
                    0.0,
                    arrow_head_size,
                );
            } else {
                return Contour::arrow_from_head_length_angle(
                    [x, y],
                    self.arrowhead_length,
                    FRAC_PI_2,
                    0.0,
                    arrow_head_size,
                );
            }
        };
    }

    pub fn cage_end_ring(&self, slots: u16) -> Result<[LineSegment; 2], planar_geo::error::Error> {
        // Draw an connecting "beam" over the conductors
        let x_start = 0.5 * self.tooth_width;
        let x_stop = slots as f64 * self.slot_and_tooth_width() + 0.5 * self.tooth_width;

        let y = 0.5 * (self.axial_length + self.slot_width);

        let lower = LineSegment::new([x_start, y], [x_stop, y])?;
        let upper = LineSegment::new([x_start, -y], [x_stop, -y])?;

        return Ok([lower, upper]);
    }

    /**
    Return the horizontal position of a coil line in the given zone.
     */
    pub fn horizontal_coil_position(&self, zone: Zone, layers: NonZeroU16) -> f64 {
        return self.tooth_width
            + self.slot_and_tooth_width() * zone.slot as f64
            + self.slot_width * (1.0 / layers.get() as f64 * (zone.layer as f64 + 0.5));
    }

    pub fn total_width(&self, slots: NonZeroU16) -> f64 {
        return self.slot_and_tooth_width() * slots.get() as f64 + self.tooth_width;
    }

    pub fn coil_cutoff_left(&self) -> f64 {
        return 0.5 * self.tooth_width;
    }

    pub fn coil_cutoff_right(&self, slots: NonZeroU16) -> f64 {
        return self.total_width(slots) - 0.5 * self.tooth_width;
    }

    pub fn axial_coil_length(&self) -> f64 {
        return self.axial_length + self.axial_coil_overhang;
    }

    pub fn clamp_x(&self, x: f64, slots: NonZeroU16) -> f64 {
        return x.clamp(self.coil_cutoff_left(), self.coil_cutoff_right(slots));
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AnnotationInfo {
    pub slots: NonZeroU16,
    pub layers: NonZeroU16,
    pub pos_zone: Zone,
    pub neg_zone: Zone,
    pub positive_slot_direction: bool,
}

impl AnnotationInfo {
    fn left_and_right_zone(&self) -> [Zone; 2] {
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
    ) -> [Text; 2] {
        // Placeholders which don't actually allocate strings
        let placeholder = Text::new(
            String::new(),
            Anchor::Center,
            [0.0, 0.0],
            [0.0, 0.0],
            color,
            font_size,
            0.0,
        );
        let mut annotations = [placeholder.clone(), placeholder];

        let left = [coil_cutoff_left, coil_height_at_cutoff];
        let right = [coil_cutoff_right, coil_height_at_cutoff];
        let [left_zone, right_zone] = self.left_and_right_zone();

        let anchors_and_slots = if center_annotations {
            [
                (Anchor::Left, right_zone.slot),
                (Anchor::Right, left_zone.slot),
            ]
        } else {
            if self.slots.get() == left_zone.slot + right_zone.slot + 1 {
                /*
                According to the slots, the coil head is separated exactly in the middle.
                Now check if the separation is slightly off-center due to the layer combination.
                 */
                match left_zone
                    .layer
                    .cmp(&(self.layers.get() - 1 - right_zone.layer))
                {
                    std::cmp::Ordering::Less => [
                        (Anchor::BottomLeft, right_zone.slot),
                        (Anchor::TopRight, left_zone.slot),
                    ],
                    std::cmp::Ordering::Equal => [
                        (Anchor::BottomLeft, right_zone.slot),
                        (Anchor::BottomRight, left_zone.slot),
                    ],
                    std::cmp::Ordering::Greater => [
                        (Anchor::TopLeft, right_zone.slot),
                        (Anchor::BottomRight, left_zone.slot),
                    ],
                }
            } else if self.slots.get() > left_zone.slot + right_zone.slot {
                [
                    (Anchor::BottomLeft, right_zone.slot),
                    (Anchor::TopRight, left_zone.slot),
                ]
            } else {
                [
                    (Anchor::TopLeft, right_zone.slot),
                    (Anchor::BottomRight, left_zone.slot),
                ]
            }
        };

        let fixed_offset = [2.0, 2.0];

        // When a number is placed to the right of an end winding line,
        // it needs to be located in the bottom left and vice versa!
        for (elem, (anchor, slot)) in annotations.iter_mut().zip(anchors_and_slots.into_iter()) {
            let position = if slot == left_zone.slot { right } else { left };
            *elem = coil_wrap_around_annotation(
                (slot + 1).to_string(),
                anchor,
                color.clone(),
                font_size,
                position,
                fixed_offset,
            );
        }

        return annotations;
    }
}

fn duplicate_positive_annotations(annotations_pos: [Text; 2]) -> [Text; 4] {
    let placeholder = Text::new(
        String::new(),
        Anchor::Center,
        [0.0, 0.0],
        [0.0, 0.0],
        Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        },
        0.0,
        0.0,
    );
    let mut annotations = [
        placeholder.clone(),
        placeholder.clone(),
        placeholder.clone(),
        placeholder,
    ];

    for (idx, annotation_pos) in annotations_pos.into_iter().enumerate() {
        let mut annotation_neg = annotation_pos.clone();

        // Mirror along the x-axis
        annotation_neg.scaled_anchor_offset[1] = -annotation_neg.scaled_anchor_offset[1];
        annotation_neg.fixed_anchor_offset[1] = -annotation_neg.fixed_anchor_offset[1];

        // Mirror about the origin
        annotation_neg.anchor = match annotation_neg.anchor {
            Anchor::TopLeft => Anchor::BottomLeft,
            Anchor::TopRight => Anchor::BottomRight,
            Anchor::BottomLeft => Anchor::TopLeft,
            Anchor::BottomRight => Anchor::TopRight,
            _ => annotation_neg.anchor,
        };

        annotations[idx + 2] = annotation_neg;
        annotations[idx] = annotation_pos;
    }

    return annotations;
}

fn get_middle_vertex(x1: f64, x2: f64, ew_angle_sin: f64) -> [f64; 2] {
    let xmean = 0.5 * (x1 + x2);
    let delta_x = if x1 > x2 { x1 - xmean } else { x2 - xmean };
    let ymean = delta_x * ew_angle_sin;
    return [xmean, ymean];
}

fn coil_wrap_around_annotation(
    text: String,
    anchor: Anchor,
    color: Color,
    font_size: f64,
    position: [f64; 2],
    fixed_offset: [f64; 2],
) -> Text {
    let (x, y) = match anchor {
        Anchor::Centroid => (0.0, 0.0),
        Anchor::Center => (0.0, 0.0),
        Anchor::TopLeft => (-fixed_offset[0], -fixed_offset[1]),
        Anchor::Top => (0.0, -fixed_offset[1]),
        Anchor::TopRight => (fixed_offset[0], -fixed_offset[1]),
        Anchor::Right => (fixed_offset[0], 0.0),
        Anchor::BottomRight => (fixed_offset[0], fixed_offset[1]),
        Anchor::Bottom => (0.0, fixed_offset[1]),
        Anchor::BottomLeft => (-fixed_offset[0], fixed_offset[1]),
        Anchor::Left => (-fixed_offset[0], 0.0),
    };

    return Text::new(text, anchor, [x, y], position, color, font_size, 0.0);
}
