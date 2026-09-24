use std::num::NonZeroU16;

use colorgrad::Gradient;
use stem_core::planar_geo::draw::Color;

pub mod winding_zones_drawables;
pub use winding_zones_drawables::*;

pub mod end_winding_layouter;
pub use end_winding_layouter::*;

pub mod coil_drawables;
pub use coil_drawables::*;

/**
Returns the color for a phase

Based on the "turbo()" color scheme
 */
pub fn get_phase_color(phase: NonZeroU16, phases: NonZeroU16) -> Color {
    let position = (f32::from(phase.get()) - 0.5) / f32::from(phases.get());
    let c = colorgrad::preset::turbo().at(position);
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
