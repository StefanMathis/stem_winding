use std::{
    collections::HashMap,
    f64::consts::{FRAC_PI_2, PI, TAU},
    num::NonZeroU16,
    sync::Arc,
};

use stem_core::{planar_geo::composite::Composite, prelude::*};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "serde")]
use dyn_quantity::deserialize_opt_quantity;

#[cfg(feature = "serde")]
use serde_mosaic::{deserialize_arc_link, serialize_arc_link};

use crate::{
    coils::{Coil, CoilExt},
    winding::Winding,
};

pub trait FromWinding<W: Winding> {
    fn from_winding(winding: &W) -> Self;
}

impl<W: Winding> FromWinding<W> for RotCore {
    fn from_winding(winding: &W) -> Self {
        use uom::typenum::P2;

        let mut yoke_radius = Length::new::<millimeter>(20.0);
        let opening_width = Length::new::<millimeter>(2.0);
        let opening_height = Length::new::<millimeter>(1.0);

        // Ratio between the angle covered by a tooth and by a slot bottom
        let ratio_tooth_slot = 3.0;

        // Angle covered by one tooth
        let slots = winding.slots().get();
        let slot_angle = TAU / slots as f64;
        let alpha = slot_angle * 1.0 / (1.0 + ratio_tooth_slot);
        let beta = slot_angle - alpha;

        // Slot bottom width
        let bottom_width = 2.0 * yoke_radius * (beta / 2.0).sin();
        let yoke_height = yoke_radius * (1.0 - (beta / 2.0).cos());

        // Scale the air gap radius by the number of slots up to 0.9.
        // The scaling formula was created "by hand" to give a good visual
        // representation, the values are chosen arbitrarily and have no deeper meaning.
        let scale_air_gap = 0.1 + 0.8 * (1.0 - 1.0 / (slots as f64).sqrt());
        let air_gap_radius = yoke_radius * scale_air_gap;

        // Calculate the slot height
        let height = yoke_radius
            - yoke_height
            - 0.5 * (4.0 * air_gap_radius.powi(P2::new()) - opening_width.powi(P2::new())).sqrt();

        // Raise yoke_radius
        yoke_radius = yoke_radius + Length::new::<millimeter>(5.0);

        // Create slot and core object (they are just used for plotting purposes)
        let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
            bottom_width,
            opening_width,
            height,
            opening_height,
            slot_angle,
            bottom_radius: Length::new::<meter>(0.0),
            top_radius: Length::new::<meter>(0.0),
            opening_radius: Length::new::<meter>(0.0),
            consider_tooth_tip_leakage: false,
        }
        .try_into()
        .expect("slot can always be created from the given values.");

        return RotCoreBuilder {
            air_gap_radius,
            yoke_radius,
            axial_length: 4.0 * bottom_width,
            axial_coil_overhang: bottom_width,
            iron_fill_factor: 1.0,
            material: Arc::new(Material::default()),
            pole_pairs: winding.pole_pairs(),
            skew_angle: 0.0,
            air_gap: Box::new(SlottedAirGap {
                slots: winding.slots(),
                starts_in_slot_middle: false,
                carter_factor_model: CarterFactorModel::MVP08,
                slot: Box::new(slot),
            }),
            flux_barrier: None,
        }
        .try_into()
        .expect("valid geometric data");
    }
}

impl<W: Winding> FromWinding<W> for LinCore {
    fn from_winding(winding: &W) -> Self {
        // Define some constructor values
        let height = Length::new::<millimeter>(20.0);
        let opening_width = Length::new::<millimeter>(2.0);
        let opening_height = Length::new::<millimeter>(1.0);
        let bottom_width = match winding.coil_layout() {
            CoilLayout::SingleFilled => Length::new::<millimeter>(8.2),
            CoilLayout::Single => Length::new::<millimeter>(8.2),
            CoilLayout::DoubleVertical => Length::new::<millimeter>(8.2),
            CoilLayout::DoubleHorizontal => Length::new::<millimeter>(16.4),
            CoilLayout::Quadruple => Length::new::<millimeter>(16.4),
            CoilLayout::MultiVertical(_) => Length::new::<millimeter>(8.2),
        };
        let core_height = 1.3 * height;
        let core_width =
            (Length::new::<millimeter>(6.8) + bottom_width) * winding.slots().get() as f64;

        let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
            bottom_width,
            opening_width,
            height,
            opening_height,
            slot_angle: 0.0,
            bottom_radius: Length::new::<meter>(0.0),
            top_radius: Length::new::<meter>(0.0),
            opening_radius: Length::new::<meter>(0.0),
            consider_tooth_tip_leakage: false,
        }
        .try_into()
        .expect("slot can always be created from the given values.");

        return LinCoreBuilder {
            height: core_height,
            width: core_width,
            axial_length: 4.0 * bottom_width,
            axial_coil_overhang: bottom_width,
            skew_angle: 0.0,
            iron_fill_factor: 1.0,
            material: Arc::new(Material::default()),
            pole_pairs: winding.pole_pairs(),
            air_gap: Box::new(SlottedAirGap {
                slots: winding.slots(),
                starts_in_slot_middle: false,
                carter_factor_model: CarterFactorModel::MVP08,
                slot: Box::new(slot),
            }),
            flux_barrier: None,
        }
        .try_into()
        .expect("valid geometric data");
    }
}

/// A decomposition of the phase resistance of a [`Winding`] into a
/// geometry-dependent resistance constant and the electrical resistivity of the
/// winding material. It can be created using the
/// [`Winding::resistance_decomposition`] trait method.
///
/// The phase resistance is calculated as
///
/// `R = k_R * rho`,
///
/// where R is the phase resistance, `k_R` is
/// [`resistance_constant`](ResistanceDecomposition::resistance_constant), and
/// `rho` is the electrical resistivity of
/// [`material`](ResistanceDecomposition::material).
///
/// The resistance constant depends on the winding and core geometry but not on
/// the electrical resistivity. This allows the decomposition to be calculated
/// once and reused to efficiently evaluate the resistance under different
/// environmental conditions.
///
/// # Examples
///
/// This example uses arbitrary values for the fields; in a real-world usage,
/// it is recommended to use [`Winding::resistance_decomposition`].
///
/// ```
/// use std::str::FromStr;
/// use std::sync::Arc;
///
/// use approxim::assert_abs_diff_eq;
/// use stem_winding::prelude::*;
/// use stem_winding::var_quantity::unary::FirstOrderTaylor;
///
/// // Define a material with an electrical resistivity which changes linearly
/// // with the temperature.
/// let copper = {
///     let mut material = Material::default();
///     material.electrical_resistivity = VarQuantity::Function(
///         QuantityFunction::new(Box::new(
///         FirstOrderTaylor::new(
///         DynQuantity::from_str("1 m/S").expect("parseable"),
///         DynQuantity::from_str("0.4 % / K").expect("parseable"),
///         DynQuantity::from_str("20.0 °C").expect("parseable"),
///         )
///         .expect("units match"),
///     ))
///     .expect("units match"),
///     );
///     Arc::new(material)
/// };
///
/// let decomposed = ResistanceDecomposition {
///     resistance_constant: ReciprocalLength::new::<reciprocal_meter>(10.0),
///     material: copper,
/// };
///
/// // 20 °C
/// let conditions = [DynQuantity::from(ThermodynamicTemperature::new::<degree_celsius>(20.0))];
/// assert_abs_diff_eq!(
///     decomposed.resistance(&conditions).get::<ohm>(),
///     10.0,
///     epsilon = 0.0001
/// );
///
/// // 120 °C -> Resistance has increased by 40 %
/// let conditions = [DynQuantity::from(ThermodynamicTemperature::new::<degree_celsius>(120.0))];
/// assert_abs_diff_eq!(
///     decomposed.resistance(&conditions).get::<ohm>(),
///     14.0,
///     epsilon = 0.0001
/// );
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ResistanceDecomposition {
    /// The resistance constant `k_R` summarizing the phase winding coil
    /// geometry influence on the resistance.
    ///
    /// Its unit is reciprocal length (1 / Length).
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub resistance_constant: ReciprocalLength,
    #[cfg_attr(
        feature = "serde",
        serde(
            serialize_with = "serialize_arc_link",
            deserialize_with = "deserialize_arc_link"
        )
    )]
    /// The material from which the electrical resistivity `rho` is obtained.
    pub material: std::sync::Arc<Material>,
}

impl ResistanceDecomposition {
    /// Calculates the phase resistance for the specified environmental
    /// conditions.
    ///
    /// The electrical resistivity is obtained from [`Material`] for the
    /// specified conditions and multiplied by [`Self::resistance_constant`].
    /// See the [struct documentation](ResistanceDecomposition) for an example.
    pub fn resistance(&self, conditions: &[DynQuantity<f64>]) -> ElectricalResistance {
        let electrical_resistivity = self.material.electrical_resistivity().get(conditions);
        return self.resistance_constant * electrical_resistivity;
    }
}

/**
This struct allows to overwrite certain calculated properties of an electric motor. This is useful if e.g.
the phase resistance is known from measurements and a simulation should use this value instead of a calculated one.

Some properties depend on other properties, for example the phase resistance constant
depends on the end winding length. These situations are resolved as follows:
MOVE THIS TO FIELD DOCSTRINGS

1) phase resistance constant is None, end winding length is None
When querying the phase resistance, the end winding length is calculated, then the phase resistance constant
(and subsequently the phase resistance) is calculated.

2) phase resistance constant is None, end winding length is Some
When querying the phase resistance, the cached end winding length is used to calculate the phase resistance constant

3) phase resistance constant is Some, end winding length is None
When querying the phase resistance, the cached phase resistance constant is used (the end winding length is not calculated).

4) phase resistance constant is Some, end winding length is Some
When querying the phase resistance, the cached phase resistance constant is used (the cached end winding length is ignored).

The default value for all properties is `None`.
 */
#[derive(Clone, Default, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Overrides {
    /// Overrides for individual coils, characterised by one of their zones.
    /// If more than one zone of a single coil is specified, the value which
    /// corresponds to the first zone returned by [`CoilExt::zones`] is used.
    /// See [`Winding::end_winding_half_turn_length`] on which zone corresponds
    /// to which end winding.
    #[cfg_attr(feature = "serde", serde(default))]
    pub end_winding_half_turn_lengths: HashMap<Zone, Length>,

    /**
    This value is a machine constant containing all non-changing parts of the phase resistance.
    The phase resistance is calculated as `resistance = resistance_constant * electric_resistivity`.
     */
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "deserialize_opt_quantity")
    )]
    pub resistance_constant: Option<ReciprocalLength>,

    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "deserialize_opt_quantity")
    )]
    pub main_inductance: Option<Inductance>,

    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "deserialize_opt_quantity")
    )]
    pub slot_leakage_inductance: Option<Inductance>,

    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "deserialize_opt_quantity")
    )]
    pub end_winding_leakage_inductance: Option<Inductance>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub air_gap_leakage_factor: Option<f64>,
}

impl Overrides {
    /**
    Clear the cache by setting all values to None.
     */
    pub fn clear(&mut self) {
        self.end_winding_half_turn_lengths = Default::default();
        self.resistance_constant = None;
        self.main_inductance = None;
        self.slot_leakage_inductance = None;
        self.end_winding_leakage_inductance = None;
        self.air_gap_leakage_factor = None;
    }

    pub fn set_end_winding_half_turn_length(
        &mut self,
        coil: &Coil,
        end_winding_half_turn_length: Length,
    ) {
        for zone in coil.zones() {
            self.end_winding_half_turn_lengths
                .insert(zone, end_winding_half_turn_length);
        }
    }

    pub fn set_same_end_winding_half_turn_length<'a, I: Iterator<Item = &'a Coil>>(
        &mut self,
        coils: I,
        end_winding_half_turn_length: Length,
    ) {
        for coil in coils {
            self.set_end_winding_half_turn_length(coil, end_winding_half_turn_length);
        }
    }
}

pub struct CoilPropertyIterator<'a> {
    pub coils: crate::iterators::CoilsIterator<'a>,
    pub winding: &'a dyn Winding,
    pub core: CoreRef<'a>,
    pub overrides: &'a Overrides,
}

impl<'a> Iterator for CoilPropertyIterator<'a> {
    type Item = CoilProperties<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.coils.next().map(|coil| CoilProperties {
            coil,
            winding: self.winding,
            core: self.core.clone(),
            overrides: self.overrides,
        })
    }
}

pub struct CoilProperties<'a> {
    pub coil: &'a Coil,
    pub winding: &'a dyn Winding,
    pub core: CoreRef<'a>,
    pub overrides: &'a Overrides,
}

impl<'a> CoilProperties<'a> {
    pub fn coil(&self) -> &'a Coil {
        return self.coil;
    }

    /**
    Return the end winding length of a half-turn. A half turn starts in the middle of the end winding
    on one side and ends in the middle of the end winding of the other side.

    If the space of a single character in the ASCII drawing below equals one mm, the return value of
    this function would be 7 mm (│ + ┌ + 4*─ + ┐).

                // Lookup from overrides. If there is no entry, calculate the value instead.

    ```text
       ┌──────┐
    ┌──│      │──┐
    │  │ Core │  │ <-- Coil
    └──│      │──┘
       └──────┘
    ```
     */
    pub fn end_winding_half_turn_length(&self) -> Length {
        let zone = self.coil().any_zone();
        self.overrides
            .end_winding_half_turn_lengths
            .get(&zone)
            .cloned()
            .unwrap_or_else(|| self.winding.end_winding_half_turn_length(self.core, zone))
    }

    pub fn end_winding_half_turn_volume(&self) -> Volume {
        self.winding.end_winding_half_turn_volume(
            self.core.clone(),
            self.coil.zones().next().expect("has at least one zone"),
            Some(self.end_winding_half_turn_length()),
        )
    }

    pub fn end_winding_volume(&self) -> Volume {
        match self.coil() {
            Coil::Full(full_coil) => {
                return 2.0 * self.end_winding_half_turn_volume() * full_coil.turns().get() as f64;
            }
            Coil::Half(coil_half) => {
                return self.end_winding_half_turn_volume() * coil_half.turns().get() as f64;
            }
        }
    }

    pub fn volume(&self) -> Volume {
        let mut volume = Volume::new::<cubic_meter>(0.0);
        for zone in self.coil.zones() {
            if let Some(zone_contour) = self.core.winding_zone_at(&self.winding.coil_layout(), zone)
            {
                let zone_area = Area::new::<square_meter>(zone_contour.area());
                let cross_section = self
                    .coil
                    .wire()
                    .effective_conductor_area(zone_area, self.coil.turns());

                let length = self.core.axial_coil_length()
                    + self.core.axial_coil_overhang()
                    + self.end_winding_half_turn_length();

                volume += length * cross_section;
            }
        }

        return volume;
    }

    pub fn mass(&self) -> Mass {
        let mass_density = self.coil.wire().material().mass_density().get(&[]);
        return self.volume() * mass_density;
    }

    pub fn heat_capacity(&self) -> HeatCapacity {
        let specific_heat_capacity = self.coil.wire().material().heat_capacity().get(&[]);
        return self.mass() * specific_heat_capacity;
    }
}

/// Estimates the mean length of a single wire in the end winding of a
/// tooth-coil winding, approximating the half-turn as a semicircle spanning the
/// two winding-zone centroids.
/// Example. Tooth coil winding
pub fn end_winding_leakage_inductance_semicircle<W: Winding>(
    winding: &W,
    core: CoreRef<'_>,
    end_winding_half_turn_length: Option<Length>,
    end_winding_leakage_coefficient: f64,
) -> Inductance {
    let end_winding_half_turn_length = end_winding_half_turn_length
        .unwrap_or_else(|| winding.end_winding_half_turn_length(core, Zone { slot: 0, layer: 0 }));
    *VACUUM_PERMEABILITY * end_winding_leakage_coefficient * f64::from(winding.slots().get())
        / (f64::from(winding.layers().get()) * f64::from(winding.phases().get()))
        * winding.turns_in_slot(0).pow(2) as f64
        / f64::from(winding.parallel_paths(NonZeroU16::MIN).get()).powi(2)
        * (end_winding_half_turn_length + core.axial_coil_overhang())
}

/// Tooth coil winding, symmetric winding
pub fn end_winding_leakage_inductance_distributed<W: Winding>(
    winding: &W,
    core: CoreRef<'_>,
    end_winding_half_turn_length: Option<Length>,
    end_winding_leakage_coefficient: f64,
) -> Inductance {
    let end_winding_half_turn_length = end_winding_half_turn_length
        .unwrap_or_else(|| winding.end_winding_half_turn_length(core, Zone { slot: 0, layer: 0 }));
    2.0 * end_winding_leakage_coefficient
        * *VACUUM_PERMEABILITY
        * winding
            .series_turns_per_phase(NonZeroU16::MIN)
            .to_integer()
            .pow(2) as f64
        * (end_winding_half_turn_length + core.axial_coil_overhang())
        / f64::from(winding.pole_pairs().get())
}

// Calculate the end winding inductance according to [Mat19], eq. (3.70) and
// (3.71).
pub fn end_winding_leakage_inductance_cage<W: Winding>(
    winding: &W,
    core: CoreRef<'_>,
    end_winding_half_turn_length: Option<Length>,
    end_winding_leakage_coefficient: f64,
) -> Inductance {
    use std::f64::consts::PI;
    let end_winding_half_turn_length = end_winding_half_turn_length
        .unwrap_or_else(|| winding.end_winding_half_turn_length(core, Zone { slot: 0, layer: 0 }));
    let ring_segment_inductance =
        *VACUUM_PERMEABILITY * end_winding_leakage_coefficient * end_winding_half_turn_length
            / f64::from(winding.pole_pairs().get());

    let poles_per_slot = f64::from(winding.pole_pairs().get()) / f64::from(winding.slots().get());
    return ring_segment_inductance / (2.0 * (PI * poles_per_slot).sin());
}

/// Approximates the length of an end-winding half turn for a [`FullCoil`] at
/// the specified `zone` with a semi-circle.
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Approximation of the end winding geometry as a semicircle][cad_end_winding_semicircle]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "cad_end_winding_semicircle",
        "docs/img/cad_end_winding_semicircle.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
///
/// The half-turn length is approximated as a semicircle connecting the
/// centroids of the two winding zones of the coil:
///
/// `ew_turn_length = π * bending_radius`
///
/// with `2 * bending_radius` being the distance between the centroids.
///
/// If the coil at `zone` is not a [`FullCoil`], this function returns zero.
/// The returned length corresponds to the centerline of the conductor path.
pub fn end_winding_half_turn_length_semicircle<W: Winding + ?Sized>(
    winding: &W,
    core: CoreRef<'_>,
    zone: Zone,
) -> Length {
    use std::f64::consts::FRAC_PI_2;

    let coil = match winding.coil_at(zone) {
        Some(c) => c,
        None => return Length::new::<meter>(0.0),
    };

    match coil {
        Coil::Full(full_coil) => {
            let pos_contour =
                match core.winding_zone_at(&winding.coil_layout(), full_coil.positive_zone()) {
                    Some(c) => c,
                    None => return Length::new::<meter>(0.0),
                };
            let neg_contour =
                match core.winding_zone_at(&winding.coil_layout(), full_coil.negative_zone()) {
                    Some(c) => c,
                    None => return Length::new::<meter>(0.0),
                };

            let [xp, yp] = pos_contour.centroid();
            let [xn, yn] = neg_contour.centroid();
            FRAC_PI_2 * Length::new::<meter>(((xp - xn).powi(2) + (yp - yn).powi(2)).sqrt())
        }
        Coil::Half(_) => Length::new::<meter>(0.0),
    }
}

/// Approximates the length of an end-winding half turn for a [`FullCoil`] at
/// the specified `zone` with a circular arc along the front side of the `core`.
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Approximation of the end winding geometry for a rotary core][cad_end_winding_circular_arc]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "cad_end_winding_circular_arc",
        "docs/img/cad_end_winding_circular_arc.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/// As shown in the image, the end winding coil geometry is composed of two
/// straight parts whose length is equal to `coil_diameter`, two quarter-circles
/// and a circular arc between the zone centroids.
///
/// The straight parts model the overlap with other coils. Since this end
/// winding approximation is meant to be used for distributed windings, it can
/// be expected that the end winding has to cross at least one other coil and
/// therefore has to cover the `coil_diameter` distance twice.
///
/// The bending radii are assumed to be the distance between the zone centroids
/// and the teeth middle. The quarter-circle length is `π/2 * radius`, but the
/// bending correspondingly also shortens the circular arc, so that the full
/// contribution of a bending becomes `(π/2 - 1) * radius`.
///
/// The circular arc goes from the centroids centers and its readius is the mean
/// radius of the two zones `eq_center_radius`.
///
/// If the coil at `zone` is not a [`FullCoil`], this function returns zero.
/// The returned length corresponds to the centerline of the conductor path.
///
/// This function is the equivalent of [`end_winding_half_turn_length_straight`]
/// for rotary cores.
///
/// This analytical approach is inspired by
/// [\[1\]](#end_winding_half_turn_length_circular_arc_1),
/// [\[2\]](#end_winding_half_turn_length_circular_arc_2).
///
/// # Literature
///
/// <a id="end_winding_half_turn_length_circular_arc_1">[1]</a>
/// [ANSYS Motor-CAD: End winding length calculation](https://ansyshelp.ansys.com/public/account/secured?returnurl=/Views/Secured/MotorCAD/v252/en/Motor-CAD_UG/MotorCAD/topics/end_winding_length_calculation.html)
///
/// <a id="end_winding_half_turn_length_circular_arc_2">[2]</a>
/// Gundogdu, T. and Komurgoz, G.: Comparative study on performance
/// characteristics of PM and reluctance machines equipped with overlapping,
/// semi-overlapping, and non-overlapping windings. IET Electric Power
/// Applications, 14, 991–1001, 2020.
/// <https://doi.org/10.1049/iet-epa.2019.0743>
pub fn end_winding_half_turn_length_circular_arc<W: Winding + ?Sized>(
    winding: &W,
    core: &RotCore,
    zone: Zone,
) -> Length {
    let coil = match winding.coil_at(zone) {
        Some(c) => c,
        None => return Length::new::<meter>(0.0),
    };

    match coil {
        Coil::Full(full_coil) => {
            let pos_contour =
                match core.winding_zone_at(&winding.coil_layout(), full_coil.positive_zone()) {
                    Some(c) => c,
                    None => return Length::new::<meter>(0.0),
                };
            let neg_contour =
                match core.winding_zone_at(&winding.coil_layout(), full_coil.negative_zone()) {
                    Some(c) => c,
                    None => return Length::new::<meter>(0.0),
                };

            let [xp, yp] = pos_contour.centroid();
            let [xn, yn] = neg_contour.centroid();

            let r1 = (xp.powi(2) + yp.powi(2)).sqrt();
            let r2 = (xn.powi(2) + yn.powi(2)).sqrt();

            let slots = winding.slots().get();
            let throw = full_coil.throw(Some(winding.slots()));

            let theta = 2.0 * PI * f64::from(throw) / f64::from(slots);
            let mean_radius = (r1 + r2) / 2.0;

            // Approximation of the coil diameter. This diameter has to be
            // travelled up and down if the coil overlaps at least one other
            // coil, which is usually the case for non tooth-coil windings.
            // The coil is approximated as a circle which covers the same area
            // as the mean value of the two contours:
            // coil_dia = 2 * sqrt((Apos + Aneg) / (2 * PI))
            let coil_dia = 2.0 * ((pos_contour.area() + neg_contour.area()) / (TAU)).sqrt();

            // Approximation of the bending radii where the coil is bent from
            // the axial direction into the cross-section plane. See drawing in
            // docstring.
            let slot_pitch_angle = TAU / f64::from(slots);
            let [x0, y0] = core
                .winding_zone_at(&CoilLayout::SingleFilled, Zone { slot: 0, layer: 0 })
                .map(|c| c.centroid())
                .unwrap_or([0.0, 0.0]); // This cannot happen, because the first zone always exists.
            let angle_slot_0 = y0.atan2(-x0);
            let offset = 0.5 * slot_pitch_angle - angle_slot_0;

            let s1 = full_coil.positive_zone().slot;
            let s2 = full_coil.negative_zone().slot;
            let angle_zone1 = yp.atan2(-xp);
            let angle_zone2 = yn.atan2(-xn);
            let (d1, d2) = if full_coil.positive_slot_direction() {
                let angle_tooth1 = f64::from(s1 + 1) * slot_pitch_angle - offset;
                let angle_tooth2 = f64::from(s2) * slot_pitch_angle - offset;
                (angle_tooth1 - angle_zone1, angle_zone2 - angle_tooth2)
            } else {
                let angle_tooth1 = f64::from(s1) * slot_pitch_angle - offset;
                let angle_tooth2 = f64::from(s2 + 1) * slot_pitch_angle - offset;
                (angle_zone1 - angle_tooth1, angle_tooth2 - angle_zone2)
            };

            // Bending radius is roughly the distance from zone centroid to tooth
            // middle on the circle which goes through the respective zone centroid.
            let b1 = r1 * d1.rem_euclid(TAU);
            let b2 = r2 * d2.rem_euclid(TAU);

            Length::new::<meter>(
                mean_radius * theta + (FRAC_PI_2 - 1.0) * (b1 + b2) + 2.0 * coil_dia,
            )
        }
        Coil::Half(_) => Length::new::<meter>(0.0),
    }
}

/// Approximates the length of an end-winding half turn for a [`FullCoil`] at
/// the specified `zone` with a straight line along the front side of the
/// `core`.
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Approximation of the end winding geometry for a linear core][cad_end_winding_straight]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "cad_end_winding_straight",
        "docs/img/cad_end_winding_straight.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/// As shown in the image, the end winding coil geometry is composed of two
/// straight parts whose length is equal to `coil_diameter`, two quarter-circles
/// and a straight line between the zone centroids.
///
/// The straight parts model the overlap with other coils. Since this end
/// winding approximation is meant to be used for distributed windings, it can
/// be expected that the end winding has to cross at least one other coil and
/// therefore has to cover the `coil_diameter` distance twice.
///
/// The bending radii are assumed to be the distance between the zone centroids
/// and the teeth middle. The quarter-circle length is `π/2 * radius`, but the
/// bending correspondingly also shortens the circular arc, so that the full
/// contribution of a bending becomes `(π/2 - 1) * radius`.
///
/// If the coil at `zone` is not a [`FullCoil`], this function returns zero.
/// The returned length corresponds to the centerline of the conductor path.
///
/// This function is the equivalent of
/// [`end_winding_half_turn_length_circular_arc`] for linear cores.
///
/// This analytical approach is inspired by
/// [\[1\]](#end_winding_half_turn_length_circular_arc_1),
/// [\[2\]](#end_winding_half_turn_length_circular_arc_2).
///
/// # Literature
///
/// <a id="end_winding_half_turn_length_circular_arc_1">[1]</a>
/// [ANSYS Motor-CAD: End winding length calculation](https://ansyshelp.ansys.com/public/account/secured?returnurl=/Views/Secured/MotorCAD/v252/en/Motor-CAD_UG/MotorCAD/topics/end_winding_length_calculation.html)
///
/// <a id="end_winding_half_turn_length_circular_arc_2">[2]</a>
/// Gundogdu, T. and Komurgoz, G.: Comparative study on performance
/// characteristics of PM and reluctance machines equipped with overlapping,
/// semi-overlapping, and non-overlapping windings. IET Electric Power
/// Applications, 14, 991–1001, 2020.
/// <https://doi.org/10.1049/iet-epa.2019.0743>
pub fn end_winding_half_turn_length_straight<W: Winding + ?Sized>(
    winding: &W,
    core: &LinCore,
    zone: Zone,
) -> Length {
    let coil = match winding.coil_at(zone) {
        Some(c) => c,
        None => return Length::new::<meter>(0.0),
    };

    match coil {
        Coil::Full(full_coil) => {
            let pos_contour =
                match core.winding_zone_at(&winding.coil_layout(), full_coil.positive_zone()) {
                    Some(c) => c,
                    None => return Length::new::<meter>(0.0),
                };
            let neg_contour =
                match core.winding_zone_at(&winding.coil_layout(), full_coil.negative_zone()) {
                    Some(c) => c,
                    None => return Length::new::<meter>(0.0),
                };

            let [xp, yp] = pos_contour.centroid();
            let [xn, yn] = neg_contour.centroid();
            let d = ((xp - xn).powi(2) + (yp - yn).powi(2)).sqrt();

            // Approximation of the coil diameter. This diameter has to be
            // travelled up and down if the coil overlaps at least one other
            // coil, which is usually the case for non tooth-coil windings.
            // The coil is approximated as a circle which covers the same area
            // as the mean value of the two contours:
            // coil_dia = 2 * sqrt((Apos + Aneg) / (2 * PI))
            let coil_dia = 2.0 * ((pos_contour.area() + neg_contour.area()) / (TAU)).sqrt();

            // Approximation of the bending radii where the coil is bent from
            // the axial direction into the cross-section plane. See drawing in
            // docstring.
            let slot_pitch = core.slot_pitch().get::<meter>();
            let [x0, _] = core
                .winding_zone_at(&CoilLayout::SingleFilled, Zone { slot: 0, layer: 0 })
                .map(|c| c.centroid())
                .unwrap_or([0.0, 0.0]); // This cannot happen, because the first zone always exists.
            let offset = 0.5 * slot_pitch - x0;

            let s1 = full_coil.positive_zone().slot;
            let s2 = full_coil.negative_zone().slot;
            let (b1, b2) = match s1.cmp(&s2) {
                std::cmp::Ordering::Less => {
                    let tooth1_center = f64::from(s1 + 1) * slot_pitch - offset;
                    let tooth2_center = f64::from(s2) * slot_pitch - offset;
                    (tooth1_center - xp, xn - tooth2_center)
                }
                std::cmp::Ordering::Equal => (0.0, d), // Resulting in a semi-circle
                std::cmp::Ordering::Greater => {
                    let tooth1_center = f64::from(s1) * slot_pitch - offset;
                    let tooth2_center = f64::from(s2 + 1) * slot_pitch - offset;
                    (xp - tooth1_center, tooth2_center - xn)
                }
            };

            /*
            d: Euclidian distance between the contour centers
            FRAC_PI_2: Quarter circle length
            -1: Subtract the length of the bending radii from d
            */
            Length::new::<meter>(d + (FRAC_PI_2 - 1.0) * (b1 + b2) + 2.0 * coil_dia)
        }
        Coil::Half(_) => Length::new::<meter>(0.0),
    }
}
