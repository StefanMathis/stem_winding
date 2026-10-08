//! Helpers for calculating physical properties of a [`Winding`].
//!
//! This module provides implementations for some [`Winding`] methods gated
//! behind the `stem_core` feature, as well as [`CoilProperties`] and
//! supporting functions for calculating properties such as end-winding
//! half-turn lengths.
//!
//! Users only need to interact with this module directly when implementing
//! [`Winding`] for their own type and wanting to reuse functionality provided
//! here, such as the end-winding half-turn length calculations.

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

/// Constructs a core with a slot configuration matching a [`Winding`].
///
/// The resulting core is intended as convenient scaffolding for use cases where
/// a core is required by an API but its detailed geometry is not important,
/// such as visualizing winding zones, writing tests, or experimenting with a
/// winding.
///
/// The constructed core uses sensible default geometry and has the same number
/// of slots as the winding. It should not be assumed to represent a physically
/// meaningful machine geometry.
///
/// This trait serves a similar purpose to [`From`], but is required because
/// [`LinCore`] and [`RotCore`] are defined in another crate and therefore
/// cannot implement `From<&W>` directly.
pub trait FromWinding<W: Winding> {
    /// Constructs a core with a slot configuration matching the specified
    /// [`Winding`].
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
    pub end_winding_half_turn_lengths: &'a HashMap<Zone, Length>,
}

impl<'a> Iterator for CoilPropertyIterator<'a> {
    type Item = CoilProperties<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.coils
            .next()
            .map(|coil| {
                self.winding.coil_properties_at(
                    self.core,
                    coil.any_zone(),
                    self.end_winding_half_turn_lengths,
                )
            })
            .flatten()
    }
}

/// Provides the context required to calculate physical properties of a
/// [`Coil`].
///
/// A `CoilProperties` instance combines a [`Coil`] from a [`Winding`] with a
/// [`CoreRef`] to allow calculating physical properties like the coil
/// [resistance](CoilProperties::resistance) or
/// [volume](CoilProperties::volume).
///
/// As with the corresponding methods from [Winding], predefined end-winding
/// half-turn lengths can be supplied for individual zones; for zones without
/// an override, the lengths are calculated on demand.
///
/// An optional end-winding volume override can be supplied to represent the
/// total conductor volume of the entire end winding of the [`Winding`]. This
/// allows winding types whose end winding cannot be adequately represented as a
/// collection of individual half turns, such as squirrel-cage windings with
/// conducting end rings. When such an override is provided, its volume is
/// distributed equally among all coil sides of the winding for the per-zone
/// [`end_winding_half_turn_volume`](Self::end_winding_half_turn_volume)
/// calculation.
///
/// The properties are calculated on demand by the methods of this type rather
/// than being stored as precomputed values. This allows the same
/// `CoilProperties` instance to be used to evaluate properties under
/// different environmental conditions where applicable.
///
/// `CoilProperties` is created by [`Winding::coil_properties_at`].
pub struct CoilProperties<'a> {
    coil: &'a Coil,
    winding: &'a dyn Winding,
    core: CoreRef<'a>,
    end_winding_half_turn_lengths: &'a HashMap<Zone, Length>,
    end_winding_volume: Option<Volume>,
}

impl<'a> CoilProperties<'a> {
    /// Creates a new [`CoilProperties`] from its components.
    ///
    /// This method should only be used to implement
    /// [`Winding::coil_properties_at`], ensuring that the specified [`Coil`] is
    /// part of the [`Winding`]. To create [`CoilProperties`] for an existing
    /// winding, use [`Winding::coil_properties_at`] instead.
    ///
    /// `end_winding_half_turn_lengths` can be used to override the calculated
    /// end-winding half-turn length for individual [`Zone`]s.
    ///
    /// If an `end_winding_volume` override is provided, it is interpreted as
    /// the total conductor volume of the end winding of the winding. The
    /// volume is distributed equally among all coil sides of the winding so
    /// that it can be used by the per-zone
    /// [`end_winding_half_turn_volume`](Self::end_winding_half_turn_volume)
    /// calculation.
    ///
    /// This allows winding types whose end winding cannot be represented as a
    /// collection of individual half turns, such as squirrel-cage windings with
    /// conducting end rings, to provide their total end-winding volume while
    /// retaining the per-zone API.
    pub fn new(
        coil: &'a Coil,
        winding: &'a dyn Winding,
        core: CoreRef<'a>,
        end_winding_half_turn_lengths: &'a HashMap<Zone, Length>,
        end_winding_volume: Option<Volume>,
    ) -> Self {
        Self {
            coil,
            winding,
            core,
            end_winding_half_turn_lengths,
            end_winding_volume,
        }
    }

    /// Returns a reference to the underlying [`Coil`].
    pub fn coil(&self) -> &'a Coil {
        self.coil
    }

    /// Returns a reference to the [`Winding`] containing the coil.
    pub fn winding(&self) -> &'a dyn Winding {
        self.winding
    }

    /// Returns a reference to the [`CoreRef`] used for the property
    /// calculations.
    pub fn core(&self) -> CoreRef<'a> {
        self.core
    }

    /// Returns the end-winding half-turn length overrides.
    pub fn end_winding_half_turn_lengths_overrides(&self) -> &'a HashMap<Zone, Length> {
        self.end_winding_half_turn_lengths
    }

    /// Returns the end-winding volume override.
    ///
    /// As described in the struct documentation, this is the total conductor
    /// volume of the entire end winding of [`CoilProperties::winding`].
    pub fn end_winding_volume_override(&self) -> Option<Volume> {
        self.end_winding_volume
    }

    /// Returns the conductor material of the coil.
    pub fn material(&self) -> &Material {
        self.coil.wire().material()
    }

    /// Returns the length of the axial half turn of the [`Coil`].
    ///
    /// The axial half-turn length is the sum of [`CoreExt::axial_coil_length`]
    /// and the [`CoreExt::axial_coil_overhang`]:
    ///
    /// `length = axial_coil_length + axial_coil_overhang`.
    pub fn axial_half_turn_length(&self) -> Length {
        self.core.axial_coil_length() + self.core.axial_coil_overhang()
    }

    /// Returns the conductor volume of the axial half turn associated with the
    /// specified [`Zone`].
    ///
    /// The volume is calculated as the effective conductor cross-sectional area
    /// multiplied by the axial half-turn length:
    ///
    /// `volume = effective_conductor_area * axial_half_turn_length`.
    ///
    /// The effective conductor cross-sectional area is calculated from the area
    /// of the winding zone ([`CoreExt::winding_zone_at`]) and the number of
    /// turns of the coil using the
    /// [`Wire::effective_conductor_area`](stem_wire::wire::Wire::effective_conductor_area).
    /// The axial half-turn length is obtained from
    /// [CoilProperties::axial_half_turn_length].
    ///
    /// Returns zero if the specified zone does not belong to this coil or if no
    /// winding zone contour is available for the zone
    pub fn axial_half_turn_volume(&self, zone: Zone) -> Volume {
        let zone_area = Area::new::<square_meter>(
            self.core
                .winding_zone_at(&self.winding.coil_layout(), zone)
                .map(|c| c.area())
                .unwrap_or(0.0),
        );
        let cross_section = self
            .coil
            .wire()
            .effective_conductor_area(zone_area, self.coil.turns());
        return cross_section * self.axial_half_turn_length();
    }

    /// Returns the total conductor volume of the axial coil parts.
    ///
    /// The volume is calculated by summing the [conductor volume of the
    /// axial half turn](CoilProperties::axial_half_turn_volume)
    /// associated with each [`Zone`] of the coil.
    pub fn axial_volume(&self) -> Volume {
        self.coil
            .zones()
            .map(|zone| self.end_winding_half_turn_volume(zone))
            .sum()
    }

    /// Returns the end-winding half-turn length for the specified [`Zone`].
    ///
    /// If an override is provided for the zone in
    /// [`CoilProperties::end_winding_half_turn_lengths`], that value is
    /// returned. Otherwise, the length is calculated using
    /// [`Winding::end_winding_half_turn_length`].
    ///
    /// Returns zero if the specified zone does not belong to this coil.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
    ///     slots: 12.try_into().expect("not zero"),
    ///     pole_pairs: 5.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 1.try_into().expect("not zero"),
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    /// }
    /// .try_into()
    /// .unwrap();
    ///
    /// let core = RotCore::from_winding(&winding);
    ///
    /// // Override one of the coil half turn lengths
    /// let mut end_winding_half_turn_lengths = HashMap::new();
    /// end_winding_half_turn_lengths.insert(Zone::new(11, 0), Length::new::<meter>(0.42));
    ///
    /// let coil_properties = winding.coil_properties_at(
    ///     core.as_core_ref(),
    ///     Zone::new(0, 0),
    ///     &end_winding_half_turn_lengths
    /// ).expect("coil exists");
    ///
    /// assert_abs_diff_eq!(
    ///     coil_properties.end_winding_half_turn_length(Zone::new(0, 0)).get::<meter>(),
    ///     0.01396,
    ///     epsilon = 0.0001
    /// );
    /// assert_abs_diff_eq!(
    ///     coil_properties.end_winding_half_turn_length(Zone::new(11, 0)).get::<meter>(),
    ///     0.42,
    ///     epsilon = 0.0001
    /// );
    ///
    /// // This zone does not belong to the coil -> Returns zero.
    /// assert_abs_diff_eq!(
    ///     coil_properties.end_winding_half_turn_length(Zone::new(2, 0)).get::<meter>(),
    ///     0.0,
    ///     epsilon = 0.0001
    /// );
    /// ```
    pub fn end_winding_half_turn_length(&self, zone: Zone) -> Length {
        if !self.coil().zones().any(|z| z == zone) {
            return Length::new::<meter>(0.0);
        }
        self.end_winding_half_turn_lengths
            .get(&zone)
            .cloned()
            .unwrap_or_else(|| self.winding.end_winding_half_turn_length(self.core, zone))
    }

    /// Returns the conductor volume of the end-winding half turn associated
    /// with the specified [`Zone`].
    ///
    /// The volume is calculated as the effective conductor cross-sectional area
    /// multiplied by the end-winding half-turn length:
    ///
    /// `volume = effective_conductor_area * end_winding_half_turn_length`.
    ///
    /// The effective conductor cross-sectional area is calculated from the area
    /// of the winding zone ([`CoreExt::winding_zone_at`]) and the number of
    /// turns of the coil using the
    /// [`Wire::effective_conductor_area`](stem_wire::wire::Wire::effective_conductor_area).
    /// The end-winding half-turn length is obtained from
    /// [`Self::end_winding_half_turn_length`], including any override
    /// supplied for the zone.
    ///
    /// Returns zero if the specified zone does not belong to this coil or if no
    /// winding zone contour is available for the zone.
    pub fn end_winding_half_turn_volume(&self, zone: Zone) -> Volume {
        if !self.coil.zones().any(|z| z == zone) {
            return Volume::new::<cubic_meter>(0.0);
        }

        if let Some(end_winding_volume) = self.end_winding_volume {
            let num_coil_sides = self
                .winding
                .coils_iter()
                .map(|coil| coil.zones().count())
                .sum::<usize>();

            return end_winding_volume / num_coil_sides as f64;
        }

        let zone_area = Area::new::<square_meter>(
            self.core
                .winding_zone_at(&self.winding.coil_layout(), zone)
                .map(|c| c.area())
                .unwrap_or(0.0),
        );
        let cross_section = self
            .coil
            .wire()
            .effective_conductor_area(zone_area, self.coil.turns());
        return cross_section * self.end_winding_half_turn_length(zone);
    }

    /// Returns the total conductor volume of the coil's end winding.
    ///
    /// The volume is calculated by summing the [conductor volume of the
    /// end-winding half turn](CoilProperties::end_winding_half_turn_volume)
    /// associated with each [`Zone`] of the coil.
    pub fn end_winding_volume(&self) -> Volume {
        self.coil
            .zones()
            .map(|zone| self.end_winding_half_turn_volume(zone))
            .sum()
    }

    /// Returns the total conductor volume of the [`Coil`].
    ///
    /// The volume is calculated by summing the conductor volume associated with
    /// each [`Zone`] of the coil. For each zone, the conductor volume is
    /// calculated from the effective conductor cross-sectional area and the
    /// total conductor length:
    ///
    /// `volume = effective_conductor_area * turn_length`.
    ///
    /// The turn length consists of the axial coil length, axial coil overhang,
    /// and end-winding half-turn length:
    ///
    /// `turn_length = axial_coil_length + axial_coil_overhang +
    /// end_winding_half_turn_length`.
    ///
    /// The effective conductor cross-sectional area is calculated from the
    /// winding zone area and the number of turns of the coil. Zones for
    /// which no winding zone contour is available are omitted from the
    /// calculation.
    pub fn volume(&self) -> Volume {
        let mut volume = Volume::new::<cubic_meter>(0.0);
        for zone in self.coil.zones() {
            volume += self.end_winding_half_turn_volume(zone) + self.axial_half_turn_volume(zone);
        }
        return volume;
    }

    /// Returns the resistance of the [`Coil`].
    ///
    /// The coil resistance is calculated by summing the resistance
    /// contributions of its [`Zone`]s for which a winding zone contour can
    /// be obtained from `core` via [`CoreExt::winding_zone_at`]. The turn
    /// length is calculated as
    ///
    /// `turn_length = axial_coil_length + axial_coil_overhang +
    /// end_winding_half_turn_length`.
    ///
    /// The `axial_coil_length` and `axial_coil_overhang` are obtained from
    /// [`CoilProperties::core`] via [`CoreExt::axial_coil_length`] and
    /// [`CoreExt::axial_coil_overhang`]. The `end_winding_half_turn_length` is
    /// taken from `self.end_winding_half_turn_lengths` map if an override
    /// is provided for the zone. Otherwise, it is calculated using
    /// [`CoilProperties::end_winding_half_turn_length`].
    ///
    /// The specified environmental `conditions` are forwarded to
    /// [`CoilExt::resistance`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::str::FromStr;
    /// use std::sync::Arc;
    /// use std::collections::HashMap;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    /// use stem_winding::var_quantity::unary::FirstOrderTaylor;
    ///
    /// // Define a material with an electrical resistivity which changes linearly
    /// // with the temperature.
    /// let copper = {
    ///     let mut material = Material::default();
    ///     material.electrical_resistivity = VarQuantity::new(
    ///         FirstOrderTaylor::new(
    ///             DynQuantity::from_str("1 / 56 m/MS").expect("parseable"),
    ///             DynQuantity::from_str("0.393 % / K").expect("parseable"),
    ///             DynQuantity::from_str("20.0 °C").expect("parseable"),
    ///         )
    ///         .expect("units match")
    ///     ).expect("units match");
    ///     Arc::new(material)
    /// };
    ///
    /// let winding: ToothCoilWinding = ToothCoilBuilder {
    ///     slots: 12.try_into().expect("not zero"),
    ///     pole_pairs: 5.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 1.try_into().expect("not zero"),
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    ///     turns_per_coil: 30.try_into().expect("not zero"),
    ///     parallel_paths: 1.try_into().expect("not zero"),
    ///     connection: Connection::Star,
    ///     end_winding_leakage_coefficient: 0.0,
    ///     wire: Box::new(SffWire::new(
    ///         copper,
    ///         0.5, // slot_fill_factor_conductor
    ///         0.6, // slot_fill_factor_overall
    ///     ).expect("valid inputs"))
    /// }
    /// .try_into()
    /// .unwrap();
    ///
    /// let core = RotCore::from_winding(&winding);
    /// let end_winding_half_turn_lengths = HashMap::new();
    ///
    /// let coil_properties = winding.coil_properties_at(
    ///     core.as_core_ref(),
    ///     Zone::new(0,0),
    ///     &end_winding_half_turn_lengths
    /// ).expect("coil exists");
    ///
    /// assert_abs_diff_eq!(
    ///     coil_properties.resistance(&[DynQuantity::from_str("20 °C").expect("parseable")]).get::<ohm>(),
    ///     0.10108,
    ///     epsilon = 0.0001
    /// );
    /// assert_abs_diff_eq!(
    ///     coil_properties.resistance(&[DynQuantity::from_str("120 °C").expect("parseable")]).get::<ohm>(),
    ///     0.10108,
    ///     epsilon = 0.0001
    /// );
    /// ```
    pub fn resistance(&self, conditions: &[DynQuantity<f64>]) -> ElectricalResistance {
        let mut resistance = ElectricalResistance::new::<ohm>(0.0);
        for zone in self.coil.zones() {
            if let Some(zone_contour) = self.core.winding_zone_at(&self.winding.coil_layout(), zone)
            {
                let zone_area = Area::new::<square_meter>(zone_contour.area());
                let length = self.core.axial_coil_length()
                    + self.core.axial_coil_overhang()
                    + self.end_winding_half_turn_length(zone);
                resistance += self.coil.resistance(zone_area, length, conditions);
            }
        }
        resistance
    }

    /// Returns the [`ResistanceDecomposition`] required to calculate the coil
    /// [`resistance`](CoilProperties::resistance) as the product
    ///
    /// `R = k_R * rho`,
    ///
    /// where `R` is the coil resistance, `k_R` is a resistance constant, and
    /// `rho` is the electrical resistivity of the winding material.
    ///
    /// The resistance constant depends only on the coil and core geometry and
    /// has the unit reciprocal length (`1 / Length`). Separating it from
    /// the electrical resistivity allows it to be calculated once and
    /// reused, for example when evaluating the resistance of the same coil
    /// at different temperatures.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::str::FromStr;
    /// use std::sync::Arc;
    /// use std::collections::HashMap;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    /// use stem_winding::var_quantity::unary::FirstOrderTaylor;
    ///
    /// // Define a material with an electrical resistivity which changes linearly
    /// // with the temperature.
    /// let copper = {
    ///     let mut material = Material::default();
    ///     material.electrical_resistivity = VarQuantity::new(
    ///         FirstOrderTaylor::new(
    ///             DynQuantity::from_str("1 / 56 m/MS").expect("parseable"),
    ///             DynQuantity::from_str("0.393 % / K").expect("parseable"),
    ///             DynQuantity::from_str("20.0 °C").expect("parseable"),
    ///         )
    ///         .expect("units match")
    ///     ).expect("units match");
    ///     Arc::new(material)
    /// };
    ///
    /// let winding: ToothCoilWinding = ToothCoilBuilder {
    ///     slots: 12.try_into().expect("not zero"),
    ///     pole_pairs: 5.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 1.try_into().expect("not zero"),
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    ///     turns_per_coil: 30.try_into().expect("not zero"),
    ///     parallel_paths: 1.try_into().expect("not zero"),
    ///     connection: Connection::Star,
    ///     end_winding_leakage_coefficient: 0.0,
    ///     wire: Box::new(SffWire::new(
    ///         copper,
    ///         0.5, // slot_fill_factor_conductor
    ///         0.6, // slot_fill_factor_overall
    ///     ).expect("valid inputs"))
    /// }
    /// .try_into()
    /// .unwrap();
    ///
    /// let core = RotCore::from_winding(&winding);
    /// let end_winding_half_turn_lengths = HashMap::new();
    ///
    /// let coil_properties = winding.coil_properties_at(
    ///     core.as_core_ref(),
    ///     Zone::new(0,0),
    ///     &end_winding_half_turn_lengths
    /// ).expect("coil exists");
    ///
    /// let decomposition = coil_properties.resistance_decomposition();
    ///
    /// let conditions = [DynQuantity::from_str("20 °C").expect("parseable")];
    /// assert_abs_diff_eq!(
    ///     coil_properties.resistance(&conditions).get::<ohm>(),
    ///     decomposition.resistance(&conditions).get::<ohm>(),
    ///     epsilon = 0.0001
    /// );
    ///
    /// let conditions = [DynQuantity::from_str("20 °C").expect("parseable")];
    /// assert_abs_diff_eq!(
    ///     coil_properties.resistance(&conditions).get::<ohm>(),
    ///     decomposition.resistance(&conditions).get::<ohm>(),
    ///     epsilon = 0.0001
    /// );
    /// ```
    pub fn resistance_decomposition(&self) -> ResistanceDecomposition {
        let material = self.coil.wire().material_arc().clone();
        let resistance = self.resistance(&[]);
        let electrical_resistivity = material.electrical_resistivity().get(&[]);
        return ResistanceDecomposition {
            resistance_constant: resistance / electrical_resistivity,
            material,
        };
    }
}

/// Returns the end-winding leakage inductance for a symmetric tooth-coil
/// winding.
///
/// This function implements a formula for tooth-coil windings provided by
/// the author's PhD supervisor, Prof. Dr.-Ing. Gerhard Huth:
///
/// `L_ew = μ0 * λ_ew * l_ew * N * w_s² / (l * m * a²)`
///
/// where `μ0` is the [`VACUUM_PERMEABILITY`], `λ_ew` is the
/// `end_winding_leakage_coefficient`, `l_ew` is the
/// `end_winding_half_turn_length`, `N` is the number of [`Winding::slots`],
/// `w_s` is the number of turns per slot, `l` is the number of winding layers,
/// `m` is the number of [`Winding::phases`], and `a` is the number of
/// [`Winding::parallel_paths`].
///
/// The formula assumes that the winding is symmetric as defined by
/// [`Winding::is_symmetric`]. If no `end_winding_half_turn_length` is supplied,
/// the mean half-turn length of all coils in the winding is used.
///
/// Huth proposes a value of `0.25` for the `end_winding_leakage_coefficient`.
///
/// The formula was provided directly by Prof. Dr.-Ing. Gerhard Huth; no
/// published source for it is currently available.
pub fn end_winding_leakage_inductance_tooth_coil<W: Winding>(
    winding: &W,
    core: CoreRef<'_>,
    end_winding_half_turn_length: Option<Length>,
    end_winding_leakage_coefficient: f64,
) -> Inductance {
    let end_winding_half_turn_length = end_winding_half_turn_length.unwrap_or_else(|| {
        let mut end_winding_half_turn_length = Length::new::<meter>(0.0);
        let mut counter = 0.0;
        for coil in winding.coils_iter() {
            for zone in coil.zones() {
                end_winding_half_turn_length += winding.end_winding_half_turn_length(core, zone);
                counter += 1.0;
            }
        }
        // Mean end winding half turn length
        end_winding_half_turn_length / counter
    });

    *VACUUM_PERMEABILITY * end_winding_leakage_coefficient * f64::from(winding.slots().get())
        / (f64::from(winding.layers().get()) * f64::from(winding.phases().get()))
        * winding.turns_in_slot(0).pow(2) as f64
        / f64::from(winding.parallel_paths(NonZeroU16::MIN).get()).powi(2)
        * (end_winding_half_turn_length + core.axial_coil_overhang())
}

/// Returns the end winding leakage inductance for a distributed, symmetric
/// winding.
///
/// This function implements
/// [\[1\]](#end_winding_leakage_inductance_distributed_1), eq. (3.7.24):
///
/// `L_ew = 2 * μ0 * λ_ew * l_ew * w² / p`
///
/// where `μ0` is the [`VACUUM_PERMEABILITY`], `λ_ew` is the
/// `end_winding_leakage_coefficient`, `l_ew` is the
/// `end_winding_half_turn_length`, `w` is the
/// [`Winding::series_turns_per_phase`] and `p` is the [`Winding::pole_pairs`].
///
/// The implementation of this formula assumes that the winding is symmetric as
/// defined in [`Winding::is_symmetric`]. If no `end_winding_half_turn_length`
/// is supplied, the mean half-turn length of all coils in the winding is used.
///
/// The `end_winding_leakage_coefficient` depends on the specific winding
/// topology and cannot be calculated for the general case. Table 3.7.2 of
/// [\[1\]](#end_winding_leakage_inductance_distributed_1) provides the
/// following reference values, where `m` is the number of phases:
///
/// |   | Stator (m = 3) | Rotor (m = 3) | Stator (m = 1) |
/// |---|---|---|---|
/// | Single layer | 0.3 | 0.25 | 0.12 |
/// | Double layer | 0.25 | 0.2 | 0.17 |
/// | Squirrel cage |  | 0.05 |  |
///
/// # Literature
/// <a id="end_winding_leakage_inductance_distributed_1">\[1\]</a>
/// Müller, G., Vogt, K. and Ponick, B.: Berechnung elektrischer Maschinen,
/// 6th edition, Wiley-VCH, 2008
pub fn end_winding_leakage_inductance_distributed<W: Winding>(
    winding: &W,
    core: CoreRef<'_>,
    end_winding_half_turn_length: Option<Length>,
    end_winding_leakage_coefficient: f64,
) -> Inductance {
    let end_winding_half_turn_length = end_winding_half_turn_length.unwrap_or_else(|| {
        let mut end_winding_half_turn_length = Length::new::<meter>(0.0);
        let mut counter = 0.0;
        for coil in winding.coils_iter() {
            for zone in coil.zones() {
                end_winding_half_turn_length += winding.end_winding_half_turn_length(core, zone);
                counter += 1.0;
            }
        }
        // Mean end winding half turn length
        end_winding_half_turn_length / counter
    });

    2.0 * end_winding_leakage_coefficient
        * *VACUUM_PERMEABILITY
        * winding
            .series_turns_per_phase(NonZeroU16::MIN)
            .to_integer()
            .pow(2) as f64
        * (end_winding_half_turn_length + core.axial_coil_overhang())
        / f64::from(winding.pole_pairs().get())
}

/// Returns the end winding leakage inductance for a squirrel-cage winding.
///
/// This function implements
/// [\[1\]](#end_winding_leakage_inductance_distributed_1), eq. (3.70) and eq.
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
/// In eq. (3.70), `l_ew` is written out as `π * (D_ri,a + D_ri,i) / (2 * N)`
/// where `D_ri,a` and `D_ri,i` are the outer and inner diameters of the end
/// winding ring. This expression is specific to a rotary core, which is why
/// this implementation uses the general formula.
///
/// A value of 0.35 for the `end_winding_leakage_coefficient` is a reasonable
/// default.
///
/// # Literature
/// <a id="end_winding_leakage_inductance_cage_1">\[1\]</a>
/// Mathis, S.: Permanentmagneterregte Line-Start-Antriebe in Ferrittechnik,
/// PhD thesis, Shaker, 2019
pub fn end_winding_leakage_inductance_cage<W: Winding>(
    winding: &W,
    core: CoreRef<'_>,
    end_winding_half_turn_length: Option<Length>,
    end_winding_leakage_coefficient: f64,
) -> Inductance {
    use std::f64::consts::PI;
    let end_winding_half_turn_length = end_winding_half_turn_length.unwrap_or_else(|| {
        let mut end_winding_half_turn_length = Length::new::<meter>(0.0);
        let mut counter = 0.0;
        for coil in winding.coils_iter() {
            for zone in coil.zones() {
                end_winding_half_turn_length += winding.end_winding_half_turn_length(core, zone);
                counter += 1.0;
            }
        }
        // Mean end winding half turn length
        end_winding_half_turn_length / counter
    });

    let ring_segment_inductance =
        *VACUUM_PERMEABILITY * end_winding_leakage_coefficient * end_winding_half_turn_length
            / f64::from(winding.pole_pairs().get());

    let poles_per_slot = f64::from(winding.pole_pairs().get()) / f64::from(winding.slots().get());
    return ring_segment_inductance / (2.0 * (PI * poles_per_slot).sin().powi(2));
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
