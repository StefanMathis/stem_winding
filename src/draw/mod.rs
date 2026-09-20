pub mod cross_section;
use std::num::NonZeroU16;

use colorgrad::Gradient;
pub use cross_section::*;
use stem_core::planar_geo::draw::Color;

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
