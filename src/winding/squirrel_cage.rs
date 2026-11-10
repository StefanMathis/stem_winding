//! Provides the [`SquirrelCageWinding`] as well as its builder structs.
//!
//! In addition to those structs, this module also holds the helper structs
//! [`InductanceWithCurrentDisplacement`] and
//! [`ResistanceWithCurrentDisplacement`], which are part of the [`VarQuantity`]
//! returned from [`SquirrelCageWinding::slot_leakage_inductance`] and
//! [`SquirrelCageWinding::resistance`] respectively. The helper functions
//! [`end_ring_radii`] and [`end_ring_resistance_coefficient`] participate in
//! the calculation of these physical quantities as well.
//!
//! Usually, a user only interacts with [`SquirrelCageWinding`] and the builder
//! structs [`SquirrelCageBuilder`] and [`SquirrelCageMinimalBuilder`] directly.

use std::{
    num::{NonZeroU16, NonZeroUsize},
    sync::Arc,
};

use compare_variables::compare_variables;
use stem_types::*;
use stem_wire::prelude::*;

use crate::{
    coils::{Coil, Coils, HalfCoil},
    error::Error,
    winding::{Connection, Winding},
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "stem_core")]
use stem_core::prelude::*;

/// Part of the [`VarQuantity`] expression returned by
/// [`SquirrelCageWinding::slot_leakage_inductance`] if the current displacement
/// is considered ([`SquirrelCageWinding::consider_current_displacement`] is
/// true).
///
/// This struct is not meant to be created directly. It calculates the current
/// displacement factor for a [`SquirrelCageWinding`] using the
/// [`CurrentDisplacementCalculator`] returned by
/// [`Slot::current_displacement_coefficients`] and multiplies it with the slot
/// leakage inductance. The product is returned from
/// [`InductanceWithCurrentDisplacement::eval`].
#[derive(PartialEq, Clone)]
#[cfg(feature = "stem_core")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InductanceWithCurrentDisplacement {
    calculator: CurrentDisplacementCalculator,
    el_resistivity: VarQuantity<ElectricalResistivity>,
    rel_permeability: VarQuantity<f64>,
    inductance: Inductance,
}

#[cfg(feature = "stem_core")]
#[cfg_attr(feature = "serde", typetag::serde)]
impl QuantityFunction for InductanceWithCurrentDisplacement {
    fn eval(&self, conditions: &[DynQuantity<f64>]) -> DynQuantity<f64> {
        let el_conductivity = 1.0 / self.el_resistivity.eval(conditions);
        let rel_permeability = self.rel_permeability.eval(conditions);

        let frequency = conditions
            .into_iter()
            .find_map(|value| Frequency::try_from(*value).ok())
            .unwrap_or(Frequency::new::<hertz>(0.0));

        // Create a mutable clone of the calculator so we can call eval.
        let mut calculator = self.calculator.clone();

        // Current displacement factor, only applied for the bar inductance
        let k_x = calculator
            .eval(frequency, el_conductivity, rel_permeability)
            .inductance;

        DynQuantity::from(self.inductance * k_x)
    }

    fn dyn_eq(&self, other: &dyn QuantityFunction) -> bool {
        (other as &dyn std::any::Any).downcast_ref::<Self>() == Some(self)
    }
}

/// Part of the [`VarQuantity`] expression returned by
/// [`SquirrelCageWinding::resistance`] if the current displacement
/// is considered ([`SquirrelCageWinding::consider_current_displacement`] is
/// true).
///
/// This struct is not meant to be created directly. It calculates the current
/// displacement factor for a [`SquirrelCageWinding`] using the
/// [`CurrentDisplacementCalculator`] returned by
/// [`Slot::current_displacement_coefficients`] and multiplies it with the bar
/// resistance. The product is returned from
/// [`ResistanceWithCurrentDisplacement::eval`].
#[derive(PartialEq, Clone)]
#[cfg(feature = "stem_core")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ResistanceWithCurrentDisplacement {
    calculator: CurrentDisplacementCalculator,
    el_resistivity: VarQuantity<ElectricalResistivity>,
    rel_permeability: VarQuantity<f64>,
    end_ring_resistance: VarQuantity<ElectricalResistance>,
    bar_overhang_resistance: VarQuantity<ElectricalResistance>,
    bar_resistance: VarQuantity<ElectricalResistance>,
}

#[cfg_attr(feature = "serde", typetag::serde)]
#[cfg(feature = "stem_core")]
impl QuantityFunction for ResistanceWithCurrentDisplacement {
    fn eval(&self, conditions: &[DynQuantity<f64>]) -> DynQuantity<f64> {
        let el_conductivity = 1.0 / self.el_resistivity.eval(conditions);
        let rel_permeability = self.rel_permeability.eval(conditions);

        let frequency = conditions
            .into_iter()
            .find_map(|value| Frequency::try_from(*value).ok())
            .unwrap_or(Frequency::new::<hertz>(0.0));

        // Create a mutable clone of the calculator so we can call eval.
        let mut calculator = self.calculator.clone();

        // Current displacement factor, only applied for the bar resistance
        let k_r = calculator
            .eval(frequency, el_conductivity, rel_permeability)
            .resistance;

        DynQuantity::from(
            self.end_ring_resistance.eval(conditions)
                + self.bar_overhang_resistance.eval(conditions)
                + self.bar_resistance.eval(conditions) * k_r,
        )
    }

    fn dyn_eq(&self, other: &dyn QuantityFunction) -> bool {
        (other as &dyn std::any::Any).downcast_ref::<Self>() == Some(self)
    }
}

/**
A symmetric [`Winding`] consisting of bars which are short-circuited at both
ends by an end ring.

Squirrel-cage windings are usually found in the rotors of asynchronous machines.
They are not fed directly from a voltage source, but create their current system
from the voltage induced by the stator winding field. This makes them very
robust and allows economical mass-production via casting, making the
asynchronous motors using them the standard drives for applications such as
pumps or fan drives, where the motor runs directly on the power grid and
properties such as reliability and low price are most important.

The individual coils of a squirrel-cage winding are [`HalfCoil`]s which are
connected to each other via an end ring. Each bar has its own phase and the
current is phase-shifted by `2 * π * p / N` to its neighboring coils
(implemented in [`SquirrelCageWinding::phase_angle`] where `p` is the number of
[`pole_pairs`](Winding::pole_pairs) and `N` is the number of
[`slots`](Winding::slots)). The number of pole pairs is defined by that of the
stator field, hence the same [`SquirrelCageWinding`] might have different pole
pair numbers depending on the stator winding it is paired with.

The following image shows the coil layout for a 12-slot squirrel cage winding
with each color representing a different phase:
 */
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Coils of a 12 / 1 squirrel cage winding][coils_squirrel_cage_winding]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "coils_squirrel_cage_winding",
        "docs/img/coils_squirrel_cage_winding.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

_Image created with `/examples/winding_plots.rs`, function `squirrel_cage_winding()`._

# Properties

With regards to its [`Winding`] implementation, a [`SquirrelCageWinding`] has
the following properties:
- It always fulfills the symmetric conditions of the [`Winding::is_symmetric`]
method. This means that its [`base_winding_count`](Winding::base_winding_count)
is calculated in constant time using the [`base_winding_count_symmetric`]
function.
- The number of turns for all coils is 1. This means that its
[`series_turns_per_phase`](Winding::series_turns_per_phase) is 1/2, since each
coil only covers half a turn, see [`HalfCoil`].
- The number of [`phases`](Winding::phases) is equal to the number of
[`slots`](Winding::slots). If the winding has more than one pole pair, phases in
different coils may have the same phase shift and be in "sync". They are still
considered to be different phases. Therefore, [`Winding::base_winding_count`]
returns always 1.
- All coils use the same [`wire`](SquirrelCageWinding::wire).
- It it is always single-layered.
- Its end winding turn length is determined by the ring dimensions
[`end_ring_width`](SquirrelCageWinding::end_ring_width) and
[`end_ring_height`](SquirrelCageWinding::end_ring_height). The exact calculation
is described in the documentation of
[`SquirrelCageWinding::end_winding_half_turn_length`] (only available if the
`stem_core` feature is enabled).
- Its end winding leakage inductance is calculated with the specialized
[`SquirrelCageWinding::end_winding_leakage_inductance`] function (only available
if the `stem_core` feature is enabled).
- Its nominal winding factor is always 1 for any phase or spatial order.

# Constructors

A [`SquirrelCageWinding`] can be built from one of its builder structs
([`SquirrelCageBuilder`] or [`SquirrelCageMinimalBuilder`]) via its [`TryFrom`]
implementation or with the [`new`](SquirrelCageWinding::new) method. Some of the
builder field values need to uphold certain invariants; for example the
[`end_winding_leakage_coefficient`](SquirrelCageWinding::end_winding_leakage_coefficient)
must be positive. This is checked during the building process, see the
documentation of [`SquirrelCageBuilder`] for details. Note that
[`SquirrelCageMinimalBuilder`] can be converted infallibly via [`From`].

# Serialization and deserialization

If the `serde` feature is enabled, this winding can be deserialized from a
serialized [`SquirrelCageBuilder`] or [`SquirrelCageMinimalBuilder`].
The serialized representation of this winding corresponds to that of a
[`SquirrelCageBuilder`].

# Examples

The following example creates a 28-slot squirrel-cage winding which is paired
with a stator winding with two pole pairs.

```
use approxim::assert_abs_diff_eq;

use stem_winding::prelude::*;

let builder = SquirrelCageMinimalBuilder {
    slots: 28.try_into().expect("not zero"),
    pole_pairs: 2.try_into().expect("not zero"),
};

// Note that the minimal builder is infallible!
let winding = SquirrelCageWinding::from(builder);

// The base winding is always 1, since each slot has a distinct phase.
assert_eq!(winding.base_winding_count().get(), 1);

// Winding factor is 1 for any spatial order
assert_abs_diff_eq!(
    winding.winding_factor(1.try_into().expect("not zero"), SpatialOrder::Electrical(1)),
    1.0,
    epsilon = 1e-3
);
assert_abs_diff_eq!(
    winding.winding_factor(1.try_into().expect("not zero"), SpatialOrder::Electrical(5)),
    1.0,
    epsilon = 1e-3
);

// Squirrel-cage windings have a very low air gap leakage factor compared to
// convential winding types where the coils of a phase are distributed over
// multiple slots.
assert_abs_diff_eq!(
    winding.air_gap_leakage_factor(1.try_into().expect("not zero")),
    0.016955,
    epsilon = 1e-5
);
```
*/
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(into = "SquirrelCageBuilder"))]
pub struct SquirrelCageWinding {
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    end_winding_leakage_coefficient: f64,
    wire: Arc<dyn Wire>,
    end_ring_width: Length,
    end_ring_height: Length,
    consider_current_displacement: bool,
    coils: Coils,
}

impl SquirrelCageWinding {
    /// Creates a new winding.
    ///
    /// This is a wrapper around `T::try_into`. The builder structs provided by
    /// the module implement [`TryInto<SquirrelCageWinding>`], see their
    /// respective documentation for details on the failure modes.
    pub fn new<W>(builder: W) -> Result<Self, Error>
    where
        W: TryInto<SquirrelCageWinding>,
        W::Error: Into<Error>,
    {
        builder.try_into().map_err(Into::into)
    }

    /// Returns a [`SquirrelCageWinding`] where all values except the number of
    /// `slots` and `pole_pairs` are set to sensible defaults.
    ///
    /// This is a wrapper around creating a [`SquirrelCageMinimalBuilder`] and
    /// immediately converting it, see the builder documentation for details on
    /// the defaults.
    pub fn new_minimal(slots: NonZeroU16, pole_pairs: NonZeroU16) -> Self {
        SquirrelCageMinimalBuilder { slots, pole_pairs }.into()
    }

    /// The end ring width in axial direction.
    pub fn end_ring_width(&self) -> Length {
        return self.end_ring_width;
    }

    /// The end ring height in radial direction.
    pub fn end_ring_height(&self) -> Length {
        return self.end_ring_height;
    }

    /// Whether the influence of current displacement is considered when
    /// calculating the [`SquirrelCageWinding::resistance`]
    /// and [`SquirrelCageWinding::slot_leakage_inductance`].
    ///
    /// As discussed in detail in [`Slot::current_displacement_coefficients`],
    /// current displacement effects lead to an effective increase of the coil
    /// resistance inside a slot and decrease of the slot leakage inductance. If
    /// this value is set to true, these effects are considered when calculating
    /// the [`SquirrelCageWinding::resistance`] and the
    /// [`SquirrelCageWinding::slot_leakage_inductance`]. If the `core` argument
    /// to these functions is not slotted, the current displacement coefficients
    /// become one and `consider_current_displacement` has no effect.
    ///
    /// The analytical approximation employed by the
    /// [`CurrentDisplacementCalculator`] assumes that the whole slot is filled
    /// by a single wire and that the permeability of the core material is
    /// magnitudes higher than that of the wire material. The first assumption
    /// can be seen as true if the surface area of the wire matches that of
    /// the slot or is not much smaller. This means that the winding can
    /// e.g. have a [`RectangularWire`] while the slot is slightly
    /// trapezoidal as long as the wire covers most of the slot. The more
    /// the wire deviates from that assumption, the larger the error becomes
    /// and the factor should eventually be set to false. Likewise, if the
    /// core material has only a slightly higher permeability than the wire
    /// material, the analytical model delivers inaccurate results and the
    /// current displacement consideration should therefore be set to false.
    pub fn consider_current_displacement(&self) -> bool {
        return self.consider_current_displacement;
    }

    /// Enables / disables the consideration of the current displacement
    /// influence on [`SquirrelCageWinding::resistance`]
    /// and [`SquirrelCageWinding::slot_leakage_inductance`].
    ///
    /// See [`SquirrelCageWinding::consider_current_displacement`] for details.
    pub fn set_consider_current_displacement(&mut self, consider: bool) -> () {
        self.consider_current_displacement = consider;
    }

    /// Returns the [`Wire`] used by all coils of `self`.
    ///
    /// As stated in the struct documentation, all coils of a
    /// [`SquirrelCageWinding`] use the same wire by definition.
    pub fn wire(&self) -> &dyn Wire {
        return &*self.wire;
    }

    /// Returns the coefficient used for the
    /// [`end_winding_leakage_inductance`](SquirrelCageWinding::end_winding_leakage_inductance)
    /// calculation.
    ///
    /// See the winding-specific documentation of
    /// [`SquirrelCageWinding::end_winding_leakage_inductance`] for details.
    pub fn end_winding_leakage_coefficient(&self) -> f64 {
        self.end_winding_leakage_coefficient
    }

    /// Sets a new number of pole pairs for the winding and returns the old one.
    ///
    /// As discussed in the [`SquirrelCageWinding`] struct documentation, the
    /// number of pole pairs of this winding type is determined by the magnetic
    /// stator field inducing the voltages. Hence, its number of pole pairs can
    /// change if that of the exciting magnetic field changes (for example, if
    /// the stator winding is pole-switchable like in a Dahlander connection).
    pub fn set_pole_pairs(&mut self, pole_pairs: NonZeroU16) -> NonZeroU16 {
        let old = self.pole_pairs;
        self.pole_pairs = pole_pairs;
        return old;
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Winding for SquirrelCageWinding {
    fn phases(&self) -> NonZeroU16 {
        self.slots
    }

    fn slots(&self) -> NonZeroU16 {
        self.slots
    }

    fn pole_pairs(&self) -> NonZeroU16 {
        self.pole_pairs
    }

    fn layers(&self) -> NonZeroU16 {
        NonZeroU16::MIN
    }

    fn base_winding_count(&self) -> NonZeroU16 {
        NonZeroU16::MIN
    }

    fn turns_at(&self, _zone: Zone) -> usize {
        1
    }

    fn phase_at(&self, zone: Zone) -> i32 {
        i32::from(zone.slot) + 1
    }

    fn coil_layout(&self) -> CoilLayout {
        CoilLayout::Single
    }

    fn parallel_paths(&self, _phase: NonZeroU16) -> NonZeroU16 {
        NonZeroU16::MIN
    }

    fn phase_angle(&self, phase: NonZeroU16) -> f64 {
        // For a squirrel-cage with N slots and p pole pairs, the electrical angle
        // advances by delta = 2 * π * p / N.
        std::f64::consts::TAU * f64::from(self.pole_pairs().get()) / f64::from(self.slots().get())
            * f64::from(phase.get() - 1)
    }

    fn series_turns_per_phase(&self, _phase: NonZeroU16) -> num::rational::Ratio<usize> {
        num::rational::Ratio::new(1, 2)
    }

    fn connection(&self) -> Connection {
        Connection::Star
    }

    fn coil_at(&self, zone: Zone) -> Option<&Coil> {
        self.coils.get(zone)
    }

    fn as_dyn(&self) -> &dyn Winding {
        self
    }

    #[cfg(feature = "stem_core")]
    fn is_symmetric(&self, _core: CoreRef<'_>) -> bool {
        return true;
    }

    #[cfg(feature = "stem_core")]
    fn slot_leakage_inductance(
        &self,
        core: CoreRef<'_>,
        _phase: NonZeroU16,
        effective_air_gap: Length,
    ) -> VarQuantity<Inductance> {
        /*
        [Mat19]: Mathis, S.: Permanentmagneterregte Line-Start-Antriebe in
        Ferrittechnik, PhD thesis, Shaker, 2019, URL:
        https://kluedo.ub.rptu.de/frontdoor/index/index/docId/8192
         */

        if let Some(slot) = core.slot() {
            let leakage_factor = slot.self_inductance_leakage_coefficient(0, &self.coil_layout())
                + slot.leakage_coefficient_opening()
                + slot.leakage_coefficient_tooth_tip(effective_air_gap);

            // Formulae (3.66) from [Mat19]
            let inductance = *VACUUM_PERMEABILITY * core.axial_coil_length() * leakage_factor;

            // AC current displacement factor
            if self.consider_current_displacement() {
                VarQuantity::new(InductanceWithCurrentDisplacement {
                    calculator: core.current_displacement_coefficients(),
                    el_resistivity: self.wire().material().electrical_resistivity().clone(),
                    rel_permeability: self.wire().material().relative_permeability().clone(),
                    inductance,
                })
                .expect("returns an inductance")
            } else {
                VarQuantity::from(inductance)
            }
        } else {
            Default::default()
        }
    }

    #[cfg(feature = "stem_core")]
    fn coil_properties_at<'a>(
        &'a self,
        core: CoreRef<'a>,
        zone: Zone,
    ) -> Option<crate::core_support::CoilProperties<'a>> {
        let coil = self.coil_at(zone)?;

        use std::f64::consts::PI;
        use uom::typenum::P2;

        let end_winding_volume = match core {
            CoreRef::Rot(rot_core) => {
                let [r_outer, r_inner] = end_ring_radii(
                    self.end_ring_height(),
                    rot_core.air_gap_radius(),
                    rot_core.is_outer(),
                );
                4.0 * PI
                    * (r_outer.powi(P2::new()) - r_inner.powi(P2::new()))
                    * self.end_ring_width()
            }
            CoreRef::Lin(lin_core) => {
                2.0 * self.end_ring_height() * self.end_ring_width() * lin_core.width()
            }
        };

        return Some(crate::core_support::CoilProperties::new(
            coil,
            self.as_dyn(),
            core,
            Some(end_winding_volume),
        ));
    }

    #[cfg(feature = "stem_core")]
    fn resistance(
        &self,
        core: CoreRef<'_>,
        phase: NonZeroU16,
    ) -> VarQuantity<ElectricalResistance> {
        /*
        [Mat19]: Mathis, S.: Permanentmagneterregte Line-Start-Antriebe in
        Ferrittechnik, PhD thesis, Shaker, 2019, URL:
        https://kluedo.ub.rptu.de/frontdoor/index/index/docId/8192
         */

        use std::f64::consts::PI;

        let end_winding_area = self.end_ring_width() * self.end_ring_height();
        let resistivity = self.wire().material().electrical_resistivity();

        // Conversion factor from segment to ring. See [Mat19], (3.54).
        let segment_to_ring = 2.0
            * (PI * f64::from(self.pole_pairs().get()) / f64::from(self.slots().get()))
                .sin()
                .powi(2);

        let end_ring_resistance: VarQuantity<ElectricalResistance> = match core {
            CoreRef::Rot(core_rot) => {
                let [outer_rad, inner_rad] = end_ring_radii(
                    self.end_ring_height(),
                    core_rot.air_gap_radius(),
                    core_rot.is_outer(),
                );
                let ring_coefficient =
                    end_ring_resistance_coefficient(outer_rad, inner_rad, self.pole_pairs());

                // Ring segment resistance according to [Mat19], (3.54), substituting
                // 1/conductivity with electrical resistivity
                VarQuantity::from(
                    ring_coefficient * (PI * (outer_rad + inner_rad))
                        / end_winding_area
                        / f64::from(self.slots().get())
                        / segment_to_ring,
                ) * resistivity
            }
            CoreRef::Lin(_) => {
                VarQuantity::from(core.slot_pitch() / end_winding_area / segment_to_ring)
                    * resistivity
            }
        };

        let zone_area = Area::new::<square_meter>(
            core.winding_zone_at(
                &self.coil_layout(),
                Zone {
                    slot: phase.get(),
                    layer: 0,
                },
            )
            .map(|c| c.area())
            .unwrap_or(0.0),
        );

        let bar_resistance =
            self.wire()
                .resistance(core.axial_coil_length(), zone_area, NonZeroUsize::MIN);
        let bar_overhang_resistance =
            self.wire()
                .resistance(core.axial_coil_overhang(), zone_area, NonZeroUsize::MIN);

        // AC current displacement factor
        if self.consider_current_displacement() {
            VarQuantity::new(ResistanceWithCurrentDisplacement {
                calculator: core.current_displacement_coefficients(),
                el_resistivity: self.wire().material().electrical_resistivity().clone(),
                rel_permeability: self.wire().material().relative_permeability().clone(),
                end_ring_resistance,
                bar_overhang_resistance,
                bar_resistance,
            })
            .expect("returns a resistance")
        } else {
            end_ring_resistance + bar_overhang_resistance + bar_resistance
        }
    }

    /// Returns the end winding leakage inductance for a squirrel-cage winding.
    ///
    /// This function implements
    /// [\[1\]](#end_winding_leakage_inductance_cage_1), eq. (3.70) and eq.
    /// (3.71) in a generalized form:
    ///
    /// `L_ew,seg = μ0 * λ_ew * l_ew / p`
    ///
    /// and
    ///
    /// `L_ew = L_ew,seg / (2 * sin²(π * p / N))`
    ///
    /// where `μ0` is the [`VACUUM_PERMEABILITY`], `λ_ew` is the
    /// `end_winding_leakage_coefficient`, `l_ew` is the
    /// `end_winding_half_turn_length`, `p` is the number of
    /// [`Winding::pole_pairs`], `N` is the number of [`Winding::slots`] and
    /// `L_ew,seg` is the inductance of a ring segment.
    ///
    /// In eq. (3.70), `l_ew` is written out as `π * (D_ri,a + D_ri,i) / (2 *
    /// N)` where `D_ri,a` and `D_ri,i` are the outer and inner diameters of
    /// the end winding ring. This expression is specific to a rotary core,
    /// which is why this implementation uses the general formula.
    ///
    /// A value of 0.35 for the `end_winding_leakage_coefficient` is a
    /// reasonable default.
    ///
    /// # Literature
    /// <a id="end_winding_leakage_inductance_cage_1">\[1\]</a>
    /// Mathis, S.: Permanentmagneterregte Line-Start-Antriebe in Ferrittechnik,
    /// PhD thesis, Shaker, 2019, URL: https://kluedo.ub.rptu.de/frontdoor/index/index/docId/8192
    #[cfg(feature = "stem_core")]
    fn end_winding_leakage_inductance(
        &self,
        core: CoreRef<'_>,
        phase: NonZeroU16,
        spatial_order: SpatialOrder,
    ) -> VarQuantity<Inductance> {
        use std::f64::consts::PI;
        let _ = phase;
        let _ = spatial_order;

        let ring_segment_inductance = *VACUUM_PERMEABILITY
            * self.end_winding_leakage_coefficient()
            * self.end_winding_half_turn_length(core, Zone::new(0, 0))
            / f64::from(self.pole_pairs().get());

        let poles_per_slot = f64::from(self.pole_pairs().get()) / f64::from(self.slots().get());
        return (ring_segment_inductance / (2.0 * (PI * poles_per_slot).sin().powi(2))).into();
    }

    /// Returns the length of one half turn of the end winding associated with
    /// the specified [`Zone`].
    ///
    /// The [`SquirrelCageWinding`] overrides the default implementation of
    /// [`Winding::end_winding_half_turn_length`]:
    /// - [`LinCore::width`] / [`Winding::slots`] for a [`LinCore`]
    /// - `π * (ri + ra) / N` where `ri` and `ra` are created via
    ///   [`end_ring_radii`] and `N` is [`Winding::slots`].
    ///   [\[1\]](#end_winding_half_turn_length_1), eq. (3.70)
    ///
    /// # Literature
    /// <a id="end_winding_half_turn_length_1">\[1\]</a>
    /// Mathis, S.: Permanentmagneterregte Line-Start-Antriebe in Ferrittechnik,
    /// PhD thesis, Shaker, 2019, URL: https://kluedo.ub.rptu.de/frontdoor/index/index/docId/8192
    #[cfg(feature = "stem_core")]
    fn end_winding_half_turn_length(&self, core: CoreRef<'_>, zone: Zone) -> Length {
        let _ = zone;
        use std::f64::consts::PI;
        match core {
            CoreRef::Rot(core_rot) => {
                let [outer_rad, inner_rad] = end_ring_radii(
                    self.end_ring_height(),
                    core_rot.air_gap_radius(),
                    core_rot.is_outer(),
                );
                return PI * (outer_rad + inner_rad) / f64::from(self.slots().get());
            }
            CoreRef::Lin(core_lin) => {
                return core_lin.width() / f64::from(self.slots().get());
            }
        }
    }
}

/**
A builder struct for a [`SquirrelCageWinding`].

This builder can be used to create a [`SquirrelCageWinding`] via its
[`new`](SquirrelCageWinding::new) method or via its [`TryFrom`] implementation.
Creating the winding fails if the conditions stated on the documentation of the
individual fields are not fulfilled (for example,
[`SquirrelCageWinding::end_winding_leakage_coefficient`] must not be negative).
 */
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SquirrelCageBuilder {
    /// Number of slots. Corresponds to [`Winding::slots`] in the final winding.
    pub slots: NonZeroU16,
    /// Number of pole pairs. Corresponds to [`Winding::pole_pairs`] in the
    /// final winding.
    pub pole_pairs: NonZeroU16,
    /// Coefficient used to calculate
    /// [`Winding::end_winding_leakage_inductance`], see
    /// [`SquirrelCageWinding::end_winding_leakage_coefficient`]. Must not be
    /// negative (`end_winding_leakage_coefficient >= 0`).
    pub end_winding_leakage_coefficient: f64,
    /// The wire used for all coils of the winding.
    pub wire: Arc<dyn Wire>,
    /// The axial width of the end ring. Must be positive (`end_ring_width >
    /// 0`).
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    pub end_ring_width: Length,
    /// The (radial) height of the end ring. Must be positive (`end_ring_width >
    /// 0`).
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    pub end_ring_height: Length,
    /// Whether to consider or to neglect the influence of current displacement
    /// on the resistance and inductance of the winding. See
    /// [`SquirrelCageWinding::consider_current_displacement`] for an in-depth
    /// discussion.
    pub consider_current_displacement: bool,
}

impl From<SquirrelCageWinding> for SquirrelCageBuilder {
    fn from(value: SquirrelCageWinding) -> Self {
        Self {
            slots: value.slots,
            pole_pairs: value.pole_pairs,
            end_winding_leakage_coefficient: value.end_winding_leakage_coefficient,
            wire: value.wire,
            end_ring_width: value.end_ring_width,
            end_ring_height: value.end_ring_height,
            consider_current_displacement: value.consider_current_displacement,
        }
    }
}

impl TryFrom<SquirrelCageBuilder> for SquirrelCageWinding {
    type Error = Error;

    fn try_from(builder: SquirrelCageBuilder) -> Result<Self, Self::Error> {
        compare_variables!(0.0 <= builder.end_winding_leakage_coefficient)?;

        let zero_length = Length::new::<meter>(0.0);
        compare_variables!(zero_length < builder.end_ring_width)?;
        compare_variables!(zero_length < builder.end_ring_height)?;

        let cap = u16::from(builder.slots).into();
        let mut coils = Coils::with_capacity(cap, cap);
        for slot in 0..u16::from(builder.slots) {
            let zone = Zone::new(slot, 0);
            let coil: Coil = HalfCoil::new(
                zone,
                true,
                NonZeroUsize::MIN,
                NonZeroU16::new(slot + 1).expect("is non-zero"),
                builder.wire.clone(),
            )
            .into();
            coils.insert(coil)?;
        }

        return Ok(SquirrelCageWinding {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            end_winding_leakage_coefficient: builder.end_winding_leakage_coefficient,
            wire: builder.wire,
            end_ring_width: builder.end_ring_width,
            end_ring_height: builder.end_ring_height,
            consider_current_displacement: builder.consider_current_displacement,
            coils,
        });
    }
}

/**
A minimal builder struct for a [`SquirrelCageWinding`].

This struct is useful if one is interested in a qualitative analysis for the
specified topology (e.g. determining the winding factor or the harmonics).
Compared to the fully-featured [`SquirrelCageBuilder`], values such as the
[`SquirrelCageWinding::end_winding_leakage_coefficient`] are set to sensible
defaults. For quantitative calculations (e.g. the [`Winding::resistance`]), use
[`SquirrelCageBuilder`] instead.

This builder is infallible, hence a [`From`] implementation exists for
[`SquirrelCageWinding`].

# Examples

```
use stem_winding::prelude::*;

let builder = SquirrelCageMinimalBuilder {
    slots: 28.try_into().expect("not zero"),
    pole_pairs: 2.try_into().expect("not zero"),
};
let winding = SquirrelCageWinding::from(builder);
assert_eq!(winding.consider_current_displacement(), false);
```
 */
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SquirrelCageMinimalBuilder {
    /// Number of slots. Corresponds to [`Winding::slots`] in the final winding.
    pub slots: NonZeroU16,
    /// Number of pole pairs. Corresponds to [`Winding::pole_pairs`] in the
    /// final winding.
    pub pole_pairs: NonZeroU16,
}

impl From<SquirrelCageWinding> for SquirrelCageMinimalBuilder {
    fn from(value: SquirrelCageWinding) -> Self {
        Self {
            slots: value.slots,
            pole_pairs: value.pole_pairs,
        }
    }
}

impl From<SquirrelCageMinimalBuilder> for SquirrelCageWinding {
    fn from(builder: SquirrelCageMinimalBuilder) -> Self {
        let cap = u16::from(builder.slots).into();
        let mut coils = Coils::with_capacity(cap, cap);
        for slot in 0..u16::from(builder.slots) {
            let zone = Zone::new(slot, 0);
            let coil: Coil = HalfCoil::new(
                zone,
                true,
                NonZeroUsize::MIN,
                NonZeroU16::new(slot + 1).expect("is non-zero"),
                Arc::new(SffWire::default()),
            )
            .into();
            coils
                .insert(coil)
                .expect("each coil occupies individual zones");
        }

        SquirrelCageWinding {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            end_winding_leakage_coefficient: 0.0,
            wire: Arc::new(SffWire::default()),
            end_ring_width: Length::new::<meter>(1.0),
            end_ring_height: Length::new::<meter>(1.0),
            consider_current_displacement: false,
            coils,
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for SquirrelCageWinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(deserialize_untagged_verbose_error::DeserializeUntaggedVerboseError)]
        enum SquirrelCageEnum {
            SquirrelCageBuilder(SquirrelCageBuilder),
            SquirrelCageMinimalBuilder(SquirrelCageMinimalBuilder),
        }
        let w = SquirrelCageEnum::deserialize(deserializer)?;
        match w {
            SquirrelCageEnum::SquirrelCageBuilder(w) => {
                w.try_into().map_err(serde::de::Error::custom)
            }
            SquirrelCageEnum::SquirrelCageMinimalBuilder(w) => Ok(w.into()),
        }
    }
}

/// Returns the `[outer radius, inner radius]` values for an end ring with the
/// specified [`end_ring_height`](SquirrelCageWinding::end_ring_height) and
/// [`air_gap_radius`](RotCore::air_gap_radius).
///
/// `is_outer` specifies whether the cage is mounted on an outer or an inner
/// core, see [`RotCore::is_outer`].
///
/// # Examples
///
/// ```
/// use approxim::assert_abs_diff_eq;
/// use stem_winding::prelude::*;
/// use stem_winding::winding::squirrel_cage::end_ring_radii;
///
/// let end_ring_height = Length::new::<millimeter>(11.2);
/// let [outer_rad, inner_rad] = end_ring_radii(end_ring_height, Length::new::<millimeter>(54.4), false);
/// assert_abs_diff_eq!(outer_rad.get::<millimeter>(), 54.4, epsilon = 0.0001);
/// assert_abs_diff_eq!(inner_rad.get::<millimeter>(), 43.2, epsilon = 0.0001);
/// ```
#[cfg(feature = "stem_core")]
pub fn end_ring_radii(
    end_ring_height: Length,
    air_gap_radius: Length,
    is_outer: bool,
) -> [Length; 2] {
    if is_outer {
        return [air_gap_radius + end_ring_height, air_gap_radius];
    } else {
        return [air_gap_radius, air_gap_radius - end_ring_height];
    };
}

/// Returns the correction factor for the end winding resistance.
///
/// Since the current is not distributed evenly across an end winding ring of a
/// [`SquirrelCageWinding`], its effective resistance is higher than the nominal
/// one calculated purely from the geometry and material data. In
/// [\[1\]](#end_ring_resistance_coefficient_1), a correction factor `k` is
/// proposed:
///
/// `k = p/2 (1 - ri/ra)` * [1 + (ri/ra)^p] / [1 - (ri/ra)^p]`
///
/// where `p` is [`Winding::pole_pairs`] and the inner and outer ring radii `ri`
/// and `ra` can be obtained from [`end_ring_radii`].
///
/// This factor is used in the implementation of
/// [`SquirrelCageWinding::resistance`].
///
/// # Literature
/// <a id="end_ring_resistance_coefficient_1">\[1\]</a>
/// Trickey, P. H.: "Induction motor resistance ring width" in Electrical
/// Engineering 55.2 (1936), p. 144-150.
///
/// # Examples
///
/// ```
/// use std::num::NonZeroU16;
///
/// use approxim::assert_abs_diff_eq;
/// use stem_winding::prelude::*;
/// use stem_winding::winding::squirrel_cage::{end_ring_radii, end_ring_resistance_coefficient};
///
/// let end_ring_height = Length::new::<millimeter>(11.2);
/// let [outer_rad, inner_rad] = end_ring_radii(end_ring_height, Length::new::<millimeter>(54.4), false);
/// let coeff = end_ring_resistance_coefficient(outer_rad, inner_rad, NonZeroU16::new(2).unwrap());
/// assert_abs_diff_eq!(coeff, 1.06516, epsilon = 0.0001);
/// ```
#[cfg(feature = "stem_core")]
pub fn end_ring_resistance_coefficient(
    outer_end_ring_radius: Length,
    inner_end_ring_radius: Length,
    pole_pairs: NonZeroU16,
) -> f64 {
    return f64::from(u16::from(pole_pairs))
        * f64::from(
            (2.0 * outer_end_ring_radius) / (outer_end_ring_radius + inner_end_ring_radius),
        )
        * (1.0 - f64::from(inner_end_ring_radius / outer_end_ring_radius))
        * (1.0
            + f64::from(inner_end_ring_radius / outer_end_ring_radius)
                .powi(2 * i32::from(u16::from(pole_pairs))))
        / (1.0
            - f64::from(inner_end_ring_radius / outer_end_ring_radius)
                .powi(2 * i32::from(u16::from(pole_pairs))));
}
