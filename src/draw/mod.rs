use std::num::NonZeroU16;

use colorgrad::Gradient;
use stem_coil_layout::Zone;
use stem_core::planar_geo::draw::*;

pub mod winding_zones_drawables;
pub use winding_zones_drawables::*;

pub mod end_winding_layouter;
pub use end_winding_layouter::*;

pub mod coil_drawables;
pub use coil_drawables::*;

/// A description for the [`Drawable`]s created by the [`CoilDrawables`] and
/// [`WindingZoneDrawables`] iterators.
///
/// The [`CoilDrawables`] and [`WindingZoneDrawables`] iterators return tuples
/// `(Drawable, DrawableKind)` with the latter specifying what the [`Drawable`]
/// represents. This can be used to adjust the [`Drawable`]. For example, if the
/// line width of the coil lines for `Zone {slot: 1, layer: 1}` should be set to
/// twice that of the other coil lines, the iterator could be adapted like this:
///
/// ```
/// use stem_winding::prelude::*;
///
/// let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
///     slots: 12.try_into().expect("not zero"),
///     pole_pairs: 5.try_into().expect("not zero"),
///     phases: 3.try_into().expect("not zero"),
///     layers: 2.try_into().expect("not zero"),
///     winding_table_constructor: WindingTableConstructor::Tingley
/// }
/// .try_into()
/// .unwrap();
/// let core = LinCore::from_winding(&winding);
/// let params = CoilDrawablesParameters::from(&core);
///
/// // Adaptation of the iterator
/// let iter = winding.coil_drawables(&params).map(|(mut drawable, kind)| {
///     if let DrawableKind::Coil(zone) = kind {
///         if zone == Zone::new(1, 1) {
///             drawable.style.line_width = 2.0 * drawable.style.line_width;
///         }
///     }
///     drawable
/// });
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DrawableKind {
    /// A tooth and its slot index.
    Tooth(u16),
    /// An annotation for the coil side in the given zone, such as the ampere
    /// turns at the center of the coil area produced by
    /// [`ZoneCenterConfig::AmpereTurns`]. The [`Style`] of the associated
    /// [`Drawable`] contains a [`Text`].
    Annotation(Zone),
    /// The coil side in the given zone.
    Coil(Zone),
    /// An arrow indicating the polarity or current direction of the coil side
    /// in the given zone.
    Arrow(Zone),
    /// An empty zone that does not contain a coil side.
    EmptyZone(Zone),
}

impl DrawableKind {
    /// Returns the [`Zone`] associated with the [`DrawableKind`].
    ///
    /// The [`DrawableKind::Tooth`] variant does not correspond to a [`Zone`],
    /// hence this function returns `None`. For all other variants, it returns
    /// the [`Zone`] associated with the [`DrawableKind`].
    pub fn zone(&self) -> Option<Zone> {
        match self {
            DrawableKind::Tooth(_) => None,
            DrawableKind::Annotation(zone) => Some(*zone),
            DrawableKind::Coil(zone) => Some(*zone),
            DrawableKind::Arrow(zone) => Some(*zone),
            DrawableKind::EmptyZone(zone) => Some(*zone),
        }
    }
}

/// Returns the color for the specified phase based on the Turbo colormap.
///
/// The colors are based on Google's **Turbo** colormap, a smooth rainbow
/// colormap designed for scientific and engineering visualization. It
/// progresses through blue, cyan, green, yellow, orange, and red, providing
/// visually distinct colors for different values. Turbo was designed as an
/// improved alternative to the traditional Jet colormap, reducing its
/// characteristic banding and abrupt color transitions. [\(1\)](#phase_color_1)
///
/// Each phase is assigned the color at the center of its corresponding
/// interval, so the first and last phases do not map to the endpoints of the
/// color scale.
///
/// This function is used in the implementation of the [`CoilDrawables`] and
/// [`WindingZoneDrawables`] iterators for the phase colors.
///
///
/// # Literature
///
/// <a id="phase_color_1">\(1\)</a>
/// Mikhailov, A.: *Turbo, An Improved Rainbow Colormap for Visualization*,
/// Google Research, 2019.
///
/// # Examples
///
/// ```
/// use std::num::NonZeroU16;
///
/// use approxim::assert_abs_diff_eq;
/// use stem_winding::draw::phase_color;
///
/// // Color scheme goes from blue over green to red:
///
/// // Phase 1
/// let color = phase_color(
///     NonZeroU16::new(1).unwrap(),
///     NonZeroU16::new(3).unwrap()
/// );
/// assert_abs_diff_eq!(color.r, 0.2235, epsilon = 1e-3);
/// assert_abs_diff_eq!(color.g, 0.5294, epsilon = 1e-3);
/// assert_abs_diff_eq!(color.b, 0.9764, epsilon = 1e-3);
///
/// // Phase 2
/// let color = phase_color(
///     NonZeroU16::new(2).unwrap(),
///     NonZeroU16::new(3).unwrap()
/// );
/// assert_abs_diff_eq!(color.r, 0.5843, epsilon = 1e-3);
/// assert_abs_diff_eq!(color.g, 0.9843, epsilon = 1e-3);
/// assert_abs_diff_eq!(color.b, 0.3176, epsilon = 1e-3);
///
/// // Phase 3
/// let color = phase_color(
///     NonZeroU16::new(3).unwrap(),
///     NonZeroU16::new(3).unwrap()
/// );
/// assert_abs_diff_eq!(color.r, 0.8980, epsilon = 1e-3);
/// assert_abs_diff_eq!(color.g, 0.2823, epsilon = 1e-3);
/// assert_abs_diff_eq!(color.b, 0.0745, epsilon = 1e-3);
/// ```
pub fn phase_color(phase: NonZeroU16, phases: NonZeroU16) -> Color {
    let position = (f32::from(phase.get()) - 0.5) / f32::from(phases.get());
    let c = colorgrad::preset::turbo().at(position);
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
