use std::{
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
            axial_length: Length::new::<millimeter>(1.0),
            axial_coil_overhang: Length::new::<millimeter>(0.0),
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
            axial_length: Length::new::<millimeter>(1.0),
            axial_coil_overhang: Length::new::<millimeter>(0.0),
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

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ResistanceComponents {
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
    pub material: std::sync::Arc<Material>,
}

impl ResistanceComponents {
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
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "deserialize_opt_quantity")
    )]
    pub end_winding_half_turn_length: Option<Length>,

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
        self.end_winding_half_turn_length = None;
        self.resistance_constant = None;
        self.main_inductance = None;
        self.slot_leakage_inductance = None;
        self.end_winding_leakage_inductance = None;
        self.air_gap_leakage_factor = None;
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

    ```text
       ┌──────┐
    ┌──│      │──┐
    │  │ Core │  │ <-- Coil
    └──│      │──┘
       └──────┘
    ```
     */
    pub fn end_winding_half_turn_length(&self) -> Length {
        return self
            .winding
            .end_winding_half_turn_length(
                self.core.clone(),
                self.coil.zones().next().expect("has at least one zone"),
                self.overrides,
            )
            .expect("must contain a coil");
    }

    pub fn end_winding_half_turn_volume(&self) -> Volume {
        return self
            .winding
            .end_winding_half_turn_volume(
                self.core.clone(),
                self.coil.zones().next().expect("has at least one zone"),
                self.overrides,
            )
            .expect("must contain a coil");
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
                    + self
                        .winding
                        .axial_coil_overhang(self.core, zone)
                        .unwrap_or(Length::new::<meter>(0.0))
                    + self
                        .winding
                        .end_winding_half_turn_length(self.core, zone, self.overrides)
                        .unwrap_or(Length::new::<meter>(0.0));

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
/// Tooth coil winding, symmetric winding
pub fn end_winding_leakage_inductance_semicircle<W: Winding>(
    winding: &W,
    core: CoreRef<'_>,
    overrides: &Overrides,
) -> Inductance {
    *VACUUM_PERMEABILITY
        * winding.end_winding_leakage_coefficient()
        * f64::from(winding.slots().get())
        / (f64::from(winding.layers().get()) * f64::from(winding.phases().get()))
        * winding.turns_in_slot(0).pow(2) as f64
        / f64::from(winding.parallel_paths().get()).powi(2)
        * (winding
            .end_winding_half_turn_length(core, Zone::new(0, 0), overrides)
            .unwrap_or(Length::new::<meter>(0.0))
            + core.axial_coil_overhang())
}

/// Tooth coil winding, symmetric winding
pub fn end_winding_leakage_inductance_distributed<W: Winding>(
    winding: &W,
    core: CoreRef<'_>,
    overrides: &Overrides,
) -> Inductance {
    2.0 * winding.end_winding_leakage_coefficient()
        * *VACUUM_PERMEABILITY
        * winding.turns_per_phase(NonZeroU16::MIN).to_integer().pow(2) as f64
        * (winding
            .end_winding_half_turn_length(core, Zone::new(0, 0), overrides)
            .unwrap_or(Length::new::<meter>(0.0))
            + core.axial_coil_overhang())
        / winding.pole_pairs().get() as f64
}

// Calculate the end winding inductance according to [Mat19], eq. (3.70) and
// (3.71).
pub fn end_winding_leakage_inductance_cage<W: Winding>(
    winding: &W,
    core: CoreRef<'_>,
    overrides: &Overrides,
) -> Inductance {
    use std::f64::consts::PI;

    let end_winding_half_turn_length = winding
        .end_winding_half_turn_length(core, Zone::new(0, 0), overrides)
        .unwrap_or(Length::new::<meter>(0.0));
    let ring_segment_inductance = *VACUUM_PERMEABILITY
        * winding.end_winding_leakage_coefficient()
        * end_winding_half_turn_length
        / f64::from(winding.pole_pairs().get());

    let poles_per_slot = f64::from(winding.pole_pairs().get()) / f64::from(winding.slots().get());
    return ring_segment_inductance / (2.0 * (PI * poles_per_slot).sin());
}

/// Estimates the mean length of a single wire in the end winding of a
/// tooth-coil winding, approximating the half-turn as a semicircle spanning the
/// two winding-zone centroids.
/// Tooth coil winding
/// The end winding is approximated by the centerline of the conductor path.
pub fn end_winding_half_turn_length_semicircle<W: Winding>(
    winding: &W,
    core: CoreRef<'_>,
    zone: Zone,
) -> Option<Length> {
    use std::f64::consts::FRAC_PI_2;

    match winding.coil_at(zone)? {
        Coil::Full(full_coil) => {
            let pos_contour =
                core.winding_zone_at(&winding.coil_layout(), full_coil.positive_zone())?;
            let neg_contour =
                core.winding_zone_at(&winding.coil_layout(), full_coil.negative_zone())?;

            let [xp, yp] = pos_contour.centroid();
            let [xn, yn] = neg_contour.centroid();
            Some(FRAC_PI_2 * Length::new::<meter>(((xp - xn).powi(2) + (yp - yn).powi(2)).sqrt()))
        }
        Coil::Half(_) => Some(Length::new::<meter>(0.0)),
    }
}

/// Approximates the mean length of a single wire in the end winding of a
/// tooth-coil winding as a circular arc at the mean radius of the two coil
/// sides. The arc angle is determined from the coil throw.
///
/// This approximation follows the common mean-radius/coil-pitch approach used
/// for analytical end-winding length calculations [1].
/// https://ansyshelp.ansys.com/public/account/secured?returnurl=/Views/Secured/MotorCAD/v252/en/Motor-CAD_UG/MotorCAD/topics/end_winding_length_calculation.html?utm_source=chatgpt.com
/// Gundogdu, T. and Komurgoz, G. (2020), Comparative study on performance characteristics of PM and reluctance machines equipped with overlapping, semi-overlapping, and non-overlapping windings. IET Electric Power Applications, 14: 991-1001. https://doi.org/10.1049/iet-epa.2019.0743
/// The end winding is approximated by the centerline of the conductor path.
pub fn end_winding_half_turn_length_circular_arc<W: Winding>(
    winding: &W,
    core: &RotCore,
    zone: Zone,
) -> Option<Length> {
    match winding.coil_at(zone)? {
        Coil::Full(full_coil) => {
            let pos_contour =
                core.winding_zone_at(&winding.coil_layout(), full_coil.positive_zone())?;
            let neg_contour =
                core.winding_zone_at(&winding.coil_layout(), full_coil.negative_zone())?;

            let [xp, yp] = pos_contour.centroid();
            let [xn, yn] = neg_contour.centroid();

            let r1 = (xp.powi(2) + yp.powi(2)).sqrt();
            let r2 = (xn.powi(2) + yn.powi(2)).sqrt();

            let slots = winding.slots().get();
            let throw = full_coil.throw(Some(winding.slots()));

            let delta_theta = 2.0 * PI * f64::from(throw) / f64::from(slots);
            let mean_radius = (r1 + r2) / 2.0;

            // Approximation of the bending radii where the coil is bent from
            // the axial direction into the cross-section plane. See drawing in
            // docstring.
            let slot_pitch_angle = TAU / f64::from(slots);
            let [x0, y0] = core
                .winding_zone_at(&CoilLayout::SingleFilled, Zone { slot: 0, layer: 0 })?
                .centroid();
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

            Some(Length::new::<meter>(
                mean_radius * delta_theta + (FRAC_PI_2 - 1.0) * (b1 + b2),
            ))
        }
        Coil::Half(_) => Some(Length::new::<meter>(0.0)),
    }
}

/// Estimates the mean length of a single wire in the end winding of a linear
/// distributed winding.
///
/// The end winding is approximated by a straight section connecting the two
/// winding-zone centroids, with a quarter-circle at each end to account for
/// the transition from the winding zone to the end winding. The radius of
/// each quarter-circle is approximated by the distance between the winding-zone
/// centroid and the center of the adjacent tooth.
/// The resulting length is therefore approximated as
///
/// `d + π * r`
///
/// where `d` is the distance between the two winding-zone centroids and `r`
/// is the bend radius described above.
/// Special case: coil throw = 0 (start and stop slot are identical) => d * π/2
/// The end winding is approximated by the centerline of the conductor path.
pub fn end_winding_half_turn_length_straight<W: Winding>(
    winding: &W,
    core: &LinCore,
    zone: Zone,
) -> Option<Length> {
    match winding.coil_at(zone)? {
        Coil::Full(full_coil) => {
            let pos_contour =
                core.winding_zone_at(&winding.coil_layout(), full_coil.positive_zone())?;
            let neg_contour =
                core.winding_zone_at(&winding.coil_layout(), full_coil.negative_zone())?;

            let [xp, yp] = pos_contour.centroid();
            let [xn, yn] = neg_contour.centroid();
            let d = ((xp - xn).powi(2) + (yp - yn).powi(2)).sqrt();

            // Approximation of the bending radii where the coil is bent from
            // the axial direction into the cross-section plane. See drawing in
            // docstring.
            let slot_pitch = core.slot_pitch().get::<meter>();
            let [x0, _] = core
                .winding_zone_at(&CoilLayout::SingleFilled, Zone { slot: 0, layer: 0 })?
                .centroid();
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
            Some(Length::new::<meter>(d + (FRAC_PI_2 - 1.0) * (b1 + b2)))
        }
        Coil::Half(_) => Some(Length::new::<meter>(0.0)),
    }
}
