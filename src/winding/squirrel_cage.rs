use std::{
    f64::consts::TAU,
    num::{NonZeroU16, NonZeroUsize},
};

use compare_variables::compare_variables;
use dyn_clone::clone_box;
use stem_coil_layout::{CoilLayout, Zone};
use stem_wire::prelude::*;

use crate::{
    coils::{Coil, CoilHalf, Coils},
    error::Error,
    winding::{Connection, Winding},
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SquirrelCageWinding {
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    end_winding_leakage_coefficient: f64,
    wire: Box<dyn Wire>,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    end_ring_width: Length,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_quantity"))]
    end_ring_height: Length,
    consider_current_displacement: bool,
    #[cfg_attr(feature = "serde", serde(skip))]
    coils: Coils,
}

impl SquirrelCageWinding {
    pub fn new<W>(builder: W) -> Result<Self, Error>
    where
        W: TryInto<SquirrelCageWinding>,
        W::Error: Into<Error>,
    {
        builder.try_into().map_err(Into::into)
    }

    pub fn new_minimal(slots: NonZeroU16, pole_pairs: NonZeroU16) -> Self {
        SquirrelCageMinimalBuilder { slots, pole_pairs }.into()
    }

    pub fn end_ring_width(&self) -> Length {
        return self.end_ring_width;
    }

    pub fn end_ring_height(&self) -> Length {
        return self.end_ring_height;
    }

    pub fn consider_current_displacement(&self) -> bool {
        return self.consider_current_displacement;
    }

    pub fn set_consider_current_displacement(&mut self, consider: bool) -> () {
        self.consider_current_displacement = consider;
    }

    pub fn wire(&self) -> &dyn Wire {
        return &*self.wire;
    }
}

impl Clone for SquirrelCageWinding {
    fn clone(&self) -> Self {
        Self {
            slots: self.slots.clone(),
            pole_pairs: self.pole_pairs.clone(),
            end_winding_leakage_coefficient: self.end_winding_leakage_coefficient.clone(),
            wire: clone_box(&*self.wire),
            end_ring_width: self.end_ring_width.clone(),
            end_ring_height: self.end_ring_height.clone(),
            consider_current_displacement: self.consider_current_displacement.clone(),
            coils: self.coils.clone(),
        }
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

    fn periodicity(&self) -> NonZeroU16 {
        return num::integer::gcd(self.slots().get(), self.pole_pairs().get())
            .try_into()
            .expect("not zero");
    }

    fn turns_at(&self, _zone: Zone) -> usize {
        1
    }

    fn phase_at(&self, zone: Zone) -> Option<i32> {
        Some(i32::from(u16::from(zone.slot)) + 1)
    }

    fn coil_layout(&self) -> CoilLayout {
        CoilLayout::Single
    }

    fn parallel_paths(&self) -> NonZeroU16 {
        NonZeroU16::MIN
    }

    /// According to the "Stabmodell" as presented in [Hut18], the number of
    /// turns per phase is 0.5 for a cage winding
    fn turns_per_phase(&self, _phase: NonZeroU16) -> num::rational::Ratio<usize> {
        num::rational::Ratio::new(1, 2)
    }

    fn connection(&self) -> Connection {
        Connection::Star
    }

    fn end_winding_leakage_coefficient(&self) -> f64 {
        self.end_winding_leakage_coefficient
    }

    /// Returns the angle between two neighbouring phases.
    fn phase_angle_difference(&self) -> f64 {
        TAU / f64::from(self.phases().get() / self.periodicity().get())
    }

    /// Returns the number of wound coils per phase (equals winding_holes in
    /// case of single-layer winding).
    fn coils_per_phase(&self) -> u16 {
        1
    }

    fn coil_groups_per_phase(&self) -> NonZeroU16 {
        NonZeroU16::MIN
    }

    fn coil_at(&self, zone: Zone) -> Option<&Coil> {
        self.coils.0.get(&zone)
    }

    fn as_dyn(&self) -> &dyn Winding {
        self
    }

    #[cfg(feature = "stem_core")]
    fn is_symmetric(&self, _core: CoreRef<'_>, _overrides: &Overrides) -> bool {
        return true;
    }

    #[cfg(feature = "stem_core")]
    fn slot_leakage_inductance(
        &self,
        _: u16,
        core: CoreRef<'_>,
        effective_air_gap: Length,
        conditions: &[InfluencingQuantity],
        overrides: &Overrides,
    ) -> Inductance {
        use uom::si::frequency::hertz;

        if let Some(slot) = core.slot() {
            // Use cached slot leakage inductance only if current displacement is not
            // considered
            if !self.consider_current_displacement() {
                if let Some(slot_leakage_inductance) = overrides.slot_leakage_inductance {
                    return slot_leakage_inductance;
                }
            }

            let leakage_factor = slot.self_inductance_leakage_coefficient(0, &self.coil_layout())
                + slot.leakage_coefficient_opening()
                + slot.leakage_coefficient_tooth_tip(effective_air_gap);

            // AC current displacement factor
            let k_x = if self.consider_current_displacement() {
                let wire_material = self
                    .wire_at(Zone::new(0, 0))
                    .unwrap()
                    .material_conductor()
                    .clone();

                let el_conductivity = 1.0 / wire_material.electrical_resistivity().get(conditions);

                let rel_permeability = wire_material.relative_permeability().get(conditions);

                let frequency = Frequency::new::<hertz>(
                    conditions
                        .into_iter()
                        .find(|value| {
                            InfluencingQuantityType::from(*value)
                                == InfluencingQuantityType::Frequency
                        })
                        .unwrap_or(&InfluencingQuantity::Frequency(Frequency::new::<hertz>(
                            0.0,
                        )))
                        .value(),
                );
                slot.current_displacement_coefficients(frequency, el_conductivity, rel_permeability)
                    .inductance_coefficient
            } else {
                1.0
            };

            // Formulae (3.66) from [Mat19]
            return *material::VACUUM_PERMEABILITY
                * core.axial_coil_length()
                * leakage_factor
                * k_x;
        } else {
            return Inductance::new::<uom::si::inductance::henry>(0.0);
        }
    }

    #[cfg(feature = "stem_core")]
    fn end_winding_half_turn_volume(
        &self,
        core: CoreRef<'_>,
        _: Zone,
        _: &Overrides,
    ) -> Option<Volume> {
        use std::f64::consts::FRAC_PI_2;
        use uom::typenum::P2;

        match core.as_lin_or_rot() {
            CoreRef::Rot(core_rot) => {
                let is_outer_part = core_rot.is_outer_part();
                let dia_ring_outer =
                    2.0 * outer_end_ring_radius(is_outer_part, core_rot.air_gap_radius(), self);
                let dia_ring_inner =
                    2.0 * inner_end_ring_radius(is_outer_part, core_rot.air_gap_radius(), self);

                return Some(
                    FRAC_PI_2
                        * (dia_ring_outer.powi(P2::new()) - dia_ring_inner.powi(P2::new()))
                        * self.end_ring_width(),
                );
            }
            CoreRef::Lin(_) => {
                return Some(
                    self.end_ring_height()
                        * self.end_ring_width()
                        * core.slot_pitch()
                        * self.slots() as f64,
                );
            }
        }
    }

    #[cfg(feature = "stem_core")]
    fn resistance(
        &self,
        _phase: NonZeroU16,
        core: CoreRef<'_>,
        conditions: &[InfluencingQuantity],
        overrides: &Overrides,
    ) -> ElectricalResistance {
        use std::f64::consts::PI;
        use uom::si::frequency::hertz;

        let electrical_resistivity = self
            .wire()
            .material_conductor()
            .electrical_resistivity()
            .get(conditions);

        // Use resistance constant only if current displacement is not considered
        if !self.consider_current_displacement() {
            if let Some(resistance_constant) = overrides.resistance_constant {
                return resistance_constant * electrical_resistivity;
            }
        }

        let end_winding_area = self.end_ring_width() * self.end_ring_height();
        let resistivity = self
            .wire()
            .material_conductor()
            .electrical_resistivity()
            .get(conditions);

        let resistance_ring_segment = match core.as_lin_or_rot() {
            CoreRef::Rot(core_rot) => {
                let is_outer_part = core_rot.is_outer_part();
                let outer_end_ring_rad =
                    outer_end_ring_radius(is_outer_part, core_rot.air_gap_radius(), self);
                let inner_end_ring_rad =
                    inner_end_ring_radius(is_outer_part, core_rot.air_gap_radius(), self);
                let ring_coefficient =
                    end_ring_resistance_coefficient(outer_end_ring_rad, inner_end_ring_rad, self);

                // Ring segment resistance according to [Mat19], (3.54), substituting
                // 1/conductivity with electrical resistivity
                ring_coefficient * resistivity * (PI * (outer_end_ring_rad + inner_end_ring_rad))
                    / end_winding_area
                    / self.slots() as f64
            }
            CoreRef::Lin(_) => resistivity * core.slot_pitch() / end_winding_area,
        };

        // Formula (3.55) in [Mat19]
        let end_ring_resistance = resistance_ring_segment
            / (2.0
                * (PI * self.pole_pairs() as f64 / self.slots() as f64)
                    .sin()
                    .powi(2));

        // AC current displacement factor
        let k_r = if self.consider_current_displacement() {
            let wire_material = self.wire().material_conductor().clone();

            let el_conductivity = 1.0 / wire_material.electrical_resistivity().get(conditions);

            let rel_permeability = wire_material.relative_permeability().get(conditions);

            let frequency = Frequency::new::<hertz>(
                conditions
                    .into_iter()
                    .find(|value| {
                        InfluencingQuantityType::from(*value) == InfluencingQuantityType::Frequency
                    })
                    .unwrap_or(&InfluencingQuantity::Frequency(Frequency::new::<hertz>(
                        0.0,
                    )))
                    .value(),
            );
            core.current_displacement_coefficients(frequency, el_conductivity, rel_permeability)
                .resistance_coefficient
        } else {
            1.0
        };

        let bar_resistance =
            self.wire()
                .resistance(core.zone_area(), core.axial_coil_length(), conditions);
        let bar_overhang_resistance =
            self.wire()
                .resistance(core.zone_area(), core.axial_coil_overhang(), conditions);

        return end_ring_resistance + bar_overhang_resistance + bar_resistance * k_r;
    }

    // Calculate the end winding inductance according to [Mat19], eq. (3.70) and
    // (3.71).
    #[cfg(feature = "stem_core")]
    fn end_winding_leakage_inductance(
        &self,
        _: u16,
        core: CoreRef<'_>,
        overrides: &Overrides,
    ) -> Inductance {
        use std::f64::consts::PI;

        let end_winding_half_turn_length = self
            .end_winding_half_turn_length(core, Zone::new(0, 0), overrides)
            .expect("cannot fail");
        let ring_segment_inductance = *material::VACUUM_PERMEABILITY
            * self.end_winding_leakage_coefficient()
            * end_winding_half_turn_length
            / self.pole_pairs() as f64;

        let poles_per_slot = self.pole_pairs() as f64 / self.slots() as f64;
        return ring_segment_inductance / (2.0 * (PI * poles_per_slot).sin());
    }

    #[cfg(feature = "stem_core")]
    fn end_winding_half_turn_length(
        &self,
        core: CoreRef<'_>,
        _: Zone,
        overrides: &Overrides,
    ) -> Option<Length> {
        use std::f64::consts::PI;
        if let Some(end_winding_half_turn_length) = overrides.end_winding_half_turn_length {
            return Some(end_winding_half_turn_length);
        }

        match core.as_lin_or_rot() {
            CoreRef::Rot(core_rot) => {
                let is_outer_part = core_rot.is_outer_part();
                let outer_end_ring_rad =
                    outer_end_ring_radius(is_outer_part, core_rot.air_gap_radius(), self);
                let inner_end_ring_rad =
                    inner_end_ring_radius(is_outer_part, core_rot.air_gap_radius(), self);
                return Some(PI * (outer_end_ring_rad + inner_end_ring_rad) / self.slots() as f64);
            }
            CoreRef::Lin(core_lin) => return Some(core_lin.width() / self.slots() as f64),
        }
    }

    #[cfg(feature = "stem_core")]
    fn slot_shapes(&self, slot: &dyn slot::IsSlot) -> Vec<Shape> {
        // Check if the slot opening is filled. This is the case for cage windings with
        // a bar wire.
        return slot.shapes(
            self.coil_layout(),
            (&self.wire as &dyn std::any::Any)
                .downcast_ref::<wire::BarWire>()
                .is_some(),
        );
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SquirrelCageBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub end_winding_leakage_coefficient: f64,
    pub wire: Box<dyn Wire>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub end_ring_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    pub end_ring_height: Length,
    pub consider_current_displacement: bool,
}

impl TryFrom<SquirrelCageBuilder> for SquirrelCageWinding {
    type Error = Error;

    fn try_from(builder: SquirrelCageBuilder) -> Result<Self, Self::Error> {
        compare_variables!(0.0 <= builder.end_winding_leakage_coefficient)?;

        let mut coils = Coils::with_capacity(u16::from(builder.slots).into());
        for slot in 0..u16::from(builder.slots) {
            let zone = Zone::new(slot, 0);
            let coil: Coil = CoilHalf::new(
                zone,
                true,
                NonZeroUsize::MIN,
                NonZeroU16::new(slot + 1).expect("is non-zero"),
                clone_box(&*builder.wire),
            )
            .into();
            coils.0.insert(zone, coil);
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

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SquirrelCageMinimalBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
}

impl From<SquirrelCageMinimalBuilder> for SquirrelCageWinding {
    fn from(builder: SquirrelCageMinimalBuilder) -> Self {
        let mut coils = Coils::with_capacity(u16::from(builder.slots).into());
        for slot in 0..u16::from(builder.slots) {
            let zone = Zone::new(slot, 0);
            let coil: Coil = CoilHalf::new(
                zone,
                true,
                NonZeroUsize::MIN,
                NonZeroU16::new(slot + 1).expect("is non-zero"),
                Box::new(SffWire::default()),
            )
            .into();
            coils.0.insert(zone, coil);
        }

        SquirrelCageWinding {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            end_winding_leakage_coefficient: 0.0,
            wire: Box::new(SffWire::default()),
            end_ring_width: Length::new::<meter>(0.0),
            end_ring_height: Length::new::<meter>(0.0),
            consider_current_displacement: true,
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

#[cfg(feature = "stem_core")]
fn outer_end_ring_radius(
    is_outer_part: bool,
    air_gap_radius: Length,
    winding: &SquirrelCageWinding,
) -> Length {
    if is_outer_part {
        return air_gap_radius + winding.end_ring_height();
    } else {
        return air_gap_radius;
    };
}

#[cfg(feature = "stem_core")]
fn inner_end_ring_radius(
    is_outer_part: bool,
    air_gap_radius: Length,
    winding: &SquirrelCageWinding,
) -> Length {
    if is_outer_part {
        return air_gap_radius;
    } else {
        return air_gap_radius - winding.end_ring_height();
    };
}

/// Correction factor according to [Tri36].
#[cfg(feature = "stem_core")]
fn end_ring_resistance_coefficient(
    outer_end_ring_radius: Length,
    inner_end_ring_radius: Length,
    winding: &SquirrelCageWinding,
) -> f64 {
    return f64::from(u16::from(winding.pole_pairs()))
        * f64::from(
            (2.0 * outer_end_ring_radius) / (outer_end_ring_radius + inner_end_ring_radius),
        )
        * (1.0 - f64::from(inner_end_ring_radius / outer_end_ring_radius))
        * (1.0
            + f64::from(inner_end_ring_radius / outer_end_ring_radius)
                .powi(2 * i32::from(u16::from(winding.pole_pairs()))))
        / (1.0
            - f64::from(inner_end_ring_radius / outer_end_ring_radius)
                .powi(2 * i32::from(u16::from(winding.pole_pairs()))));
}

#[cfg(test)]
#[cfg(feature = "stem_core")]
mod tests {
    use std::sync::Arc;

    use stem_wire::prelude::*;

    use super::*;

    #[test]
    fn test_end_ring_resistance() {
        let mut material = Material::default();
        material
            .set_electrical_resistivity(ElectricalResistivity::new::<ohm_meter>(1.7857e-8).into());

        let winding: SquirrelCageWinding = SquirrelCageBuilder {
            slots: NonZeroU16::new(28).expect("not zero"),
            pole_pairs: NonZeroU16::new(2).expect("not zero"),
            end_winding_leakage_coefficient: 0.0,
            wire: Box::new(SffWire::new(Arc::new(material), 1.0, 1.0).unwrap()),
            end_ring_width: Length::new::<millimeter>(11.0),
            end_ring_height: Length::new::<millimeter>(11.2),
            consider_current_displacement: true,
        }
        .try_into()
        .expect("valid winding");

        // Check the end ring
        let outer_end_ring_radius_ =
            outer_end_ring_radius(false, Length::new::<millimeter>(54.4), &winding);
        approxim::assert_abs_diff_eq!(
            outer_end_ring_radius_.get::<millimeter>(),
            54.4, // Expected resistance in Ohm
            epsilon = 0.0001
        );

        let inner_end_ring_radius_ =
            inner_end_ring_radius(false, Length::new::<millimeter>(54.4), &winding);
        approxim::assert_abs_diff_eq!(
            inner_end_ring_radius_.get::<millimeter>(),
            43.2, // Expected resistance in Ohm
            epsilon = 0.0001
        );

        approxim::assert_abs_diff_eq!(
            end_ring_resistance_coefficient(
                outer_end_ring_radius_,
                inner_end_ring_radius_,
                &winding
            ),
            1.06516, // Expected resistance in Ohm
            epsilon = 0.0001
        );
    }
}
