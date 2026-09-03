use dyn_clone::clone_box;
use num::rational::Ratio;
use planar_geo::shape::Shape;
use slot::CoilLayout;
use std::f64::consts::TAU;
use wire::{IsWire, SffWire};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};


use material::{InfluencingQuantity, InfluencingQuantityType, IsQuantityFunction};


use magnetic_core::{CoreRef, IsCoreRef};


use stem_primitives::Overrides;

#[cfg(feature = "serde")]
use deserialize_untagged_verbose_error::DeserializeAlias;

#[cfg(feature = "serde")]
use dyn_quantity::deserialize_quantity;

use uom::si::{f64::*, length::meter};

use compare_variables::compare_variables;

use crate::{Coil, CoilHalf, Coils, Connection, Winding, Zone};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, DeserializeAlias))]
#[cfg_attr(feature = "serde", serde(try_from = "serde_impl::WindingCageVariants"))]
#[cfg_attr(feature = "serde", deserialize_alias(name = "NewFull"))]
pub struct SquirrelCageWinding {
    slots: u16,                           // Inner of slots
    pole_pairs: u16,                      // Inner of pole pairs
    end_winding_leakage_coefficient: f64, // End winding flux leakage coefficient
    wire: Box<dyn IsWire>,                // Wire motor
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    end_ring_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    end_ring_height: Length,
    consider_current_displacement: bool,
    coils: Coils,
}

impl SquirrelCageWinding {
    pub fn new(
        slots: u16,
        pole_pairs: u16,
        end_winding_leakage_coefficient: f64,
        wire: Box<dyn IsWire>,
        end_ring_width: Length,
        end_ring_height: Length,
        consider_current_displacement: bool,
    ) -> stem_primitives::Result<Self> {
        // Create the coils vector
        let mut coils = Coils::with_capacity(slots.into());
        for slot in 0..slots {
            let zone = Zone::new(slot, 0);
            let coil: Coil = CoilHalf::new(zone, true, 1, slot + 1, clone_box(&*wire)).into();
            coils.0.insert(zone, coil);
        }

        let winding = Self {
            slots,
            pole_pairs,
            end_winding_leakage_coefficient,
            wire,
            end_ring_width,
            end_ring_height,
            consider_current_displacement,
            coils,
        };
        return winding.check();
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

    pub fn wire(&self) -> &dyn IsWire {
        return &*self.wire;
    }

    /// Check if self is properly defined (no illegal parameter values)
    fn check(self) -> stem_primitives::Result<Self> {
        compare_variables!(0 < self.slots)?;
        compare_variables!(0 < self.pole_pairs)?;
        compare_variables!(0.0 <= self.end_winding_leakage_coefficient)?;
        return Ok(self);
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

impl Default for SquirrelCageWinding {
    fn default() -> Self {
        return SquirrelCageWinding::new(
            28,
            2,
            0.0,
            Box::new(SffWire::default()),
            Length::new::<meter>(0.0),
            Length::new::<meter>(0.0),
            true,
        )
        .unwrap();
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Winding for SquirrelCageWinding {
    fn phases(&self) -> u16 {
        return self.slots;
    }

    fn slots(&self) -> u16 {
        return self.slots;
    }

    fn pole_pairs(&self) -> u16 {
        return self.pole_pairs;
    }

    fn layers(&self) -> u16 {
        return 1;
    }

    fn periodicity(&self) -> u16 {
        return num::integer::gcd(self.slots(), self.pole_pairs());
    }

    fn turns_at(&self, _zone: Zone) -> usize {
        return 1;
    }

    fn phase_at(&self, zone: Zone) -> Option<i32> {
        return Some(zone.slot as i32 + 1);
    }

    fn coil_layout(&self) -> CoilLayout {
        return CoilLayout::Single;
    }

    fn parallel_paths(&self) -> u16 {
        return 1;
    }

    /// According to the "Stabmodell" as presented in [Hut18], the number of
    /// turns per phase is 0.5 for a cage winding
    fn turns_per_phase(&self, _phase: u16) -> Ratio<usize> {
        return Ratio::new(1, 2);
    }

    fn connection(&self) -> Connection {
        return Connection::Star;
    }

    fn end_winding_leakage_coefficient(&self) -> f64 {
        return self.end_winding_leakage_coefficient;
    }

    /// Returns the angle between two neighbouring phases.
    fn phase_angle_difference(&self) -> f64 {
        return TAU / ((self.phases() / self.periodicity()) as f64);
    }

    /// Returns the number of wound coils per phase (equals winding_holes in
    /// case of single-layer winding).
    fn coils_per_phase(&self) -> u16 {
        return 1;
    }

    fn coil_groups_per_phase(&self) -> u16 {
        return 1;
    }

    fn coil_at(&self, zone: Zone) -> Option<&Coil> {
        return self.coils.0.get(&zone);
    }

    fn as_dyn(&self) -> &dyn Winding {
        self
    }

    
    fn is_symmetric(&self, _core: CoreRef<'_>, _overrides: &Overrides) -> bool {
        return true;
    }

    
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

    
    fn resistance(
        &self,
        _phase: u16,
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

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
struct New {
    slots: u16,                           // Inner of slots
    pole_pairs: u16,                      // Inner of pole pairs
    end_winding_leakage_coefficient: f64, // End winding flux leakage coefficient
    wire: Box<dyn IsWire>,                // Wire motor
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    end_ring_width: Length,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_quantity"))]
    end_ring_height: Length,
    consider_current_displacement: bool,
}

impl TryFrom<New> for SquirrelCageWinding {
    type Error = stem_primitives::Error;

    fn try_from(value: New) -> Result<Self, Self::Error> {
        return Self::new(
            value.slots,
            value.pole_pairs,
            value.end_winding_leakage_coefficient,
            value.wire,
            value.end_ring_width,
            value.end_ring_height,
            value.consider_current_displacement,
        );
    }
}

#[derive(constructor)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[constructor(
    target = "SquirrelCageWinding",
    fn_name = "new_minimal",
    error = "stem_primitives::Error"
)]
struct NewMinimal {
    pub slots: u16,      // Inner of slots
    pub pole_pairs: u16, // Inner of pole pairs
}

impl TryFrom<NewMinimal> for SquirrelCageWinding {
    type Error = stem_primitives::Error;

    fn try_from(value: NewMinimal) -> Result<Self, Self::Error> {
        return Self::new(
            value.slots,
            value.pole_pairs,
            0.0,
            Box::new(SffWire::default()),
            Length::new::<meter>(0.0),
            Length::new::<meter>(0.0),
            true,
        );
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;
    use deserialize_untagged_verbose_error::{DeserializeUntaggedVerboseError, TryFromEnum};

    #[derive(TryFromEnum, DeserializeUntaggedVerboseError)]
    #[try_from_enum(target = "SquirrelCageWinding", error = "stem_primitives::Error")]
    pub(super) enum WindingCageVariants {
        #[try_from_enum(convert_with = "check")]
        NewFull(NewFull),
        New(New),
        NewMinimal(NewMinimal),
    }

    fn check(alias: NewFull) -> stem_primitives::Result<SquirrelCageWinding> {
        return SquirrelCageWinding::from(alias).check();
    }
}

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
fn end_ring_resistance_coefficient(
    outer_end_ring_radius: Length,
    inner_end_ring_radius: Length,
    winding: &SquirrelCageWinding,
) -> f64 {
    return winding.pole_pairs() as f64
        * f64::from(
            (2.0 * outer_end_ring_radius) / (outer_end_ring_radius + inner_end_ring_radius),
        )
        * (1.0 - f64::from(inner_end_ring_radius / outer_end_ring_radius))
        * (1.0
            + f64::from(inner_end_ring_radius / outer_end_ring_radius)
                .powi(2 * winding.pole_pairs() as i32))
        / (1.0
            - f64::from(inner_end_ring_radius / outer_end_ring_radius)
                .powi(2 * winding.pole_pairs() as i32));
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use magnetic_core::{CarterFactorModel, CoreRot};
    use material::Material;
    use slot::SlotTrapezoidSemi;
    use uom::si::electrical_resistance::ohm;
    use uom::si::electrical_resistivity::ohm_meter;
    use uom::si::frequency::hertz;
    use uom::si::inductance::henry;
    use uom::si::length::millimeter;
    use uom::si::thermodynamic_temperature::degree_celsius;

    use super::*;

    #[test]
    fn test_cage_winding_s28_fb_v1() {
        let mut material = Material::default();
        material.set_electrical_resistivity(material::IsQuantityFunction::Constant(
            ElectricalResistivity::new::<ohm_meter>(1.7857e-8),
        ));

        let rotor_winding = SquirrelCageWinding::new(
            28,
            2,
            0.0,
            Box::new(SffWire::new(Arc::new(material), 1.0, 1.0).unwrap()),
            Length::new::<millimeter>(11.0),
            Length::new::<millimeter>(11.2),
            true,
        )
        .unwrap();
        let slot = SlotTrapezoidSemi::new(
            Length::new::<millimeter>(6.76),
            Length::new::<millimeter>(1.5),
            Length::new::<millimeter>(1.5),
            Length::new::<millimeter>(6.79),
            Length::new::<millimeter>(5.54),
            Length::new::<millimeter>(0.75),
            -0.2243994752564138,
            1.6829960644231035,
            1.611245917561955,
            Length::new::<millimeter>(0.0),
            Length::new::<millimeter>(0.0),
            Length::new::<millimeter>(0.0),
            Length::new::<millimeter>(0.0),
            Length::new::<millimeter>(0.0),
            true,
        )
        .unwrap();

        let core: CoreRot = magnetic_core::CoreRotBuilder {
            air_gap_radius: Length::new::<millimeter>(54.4),
            yoke_radius: Length::new::<millimeter>(19.0),
            axial_length: Length::new::<millimeter>(165.0),
            axial_coil_overhang: Length::new::<millimeter>(0.0),
            iron_fill_factor: 0.95,
            material: Arc::new(Material::default()),
            pole_pairs: 2,
            skew_angle: 0.0,
            air_gap: Box::new(magnetic_core::AirGapSlotted {
                slots: 28,
                starts_in_slot_middle: true,
                carter_factor_model: CarterFactorModel::Bin12,
                slot: Box::new(slot),
            }),
            flux_barrier: None,
        }
        .try_into()
        .expect("valid magnetic core");

        // Check the end ring
        let outer_end_ring_radius_ =
            outer_end_ring_radius(core.is_outer_part(), core.air_gap_radius(), &rotor_winding);
        approxim::assert_abs_diff_eq!(
            outer_end_ring_radius_.get::<millimeter>(),
            54.4, // Expected resistance in Ohm
            epsilon = 0.0001
        );

        let inner_end_ring_radius_ =
            inner_end_ring_radius(core.is_outer_part(), core.air_gap_radius(), &rotor_winding);
        approxim::assert_abs_diff_eq!(
            inner_end_ring_radius_.get::<millimeter>(),
            43.2, // Expected resistance in Ohm
            epsilon = 0.0001
        );

        approxim::assert_abs_diff_eq!(
            end_ring_resistance_coefficient(
                outer_end_ring_radius_,
                inner_end_ring_radius_,
                &rotor_winding
            ),
            1.06516, // Expected resistance in Ohm
            epsilon = 0.0001
        );

        approxim::assert_abs_diff_eq!(
            rotor_winding
                .resistance(
                    1,
                    core.as_lin_or_rot(),
                    &[
                        InfluencingQuantity::Temperature(ThermodynamicTemperature::new::<
                            degree_celsius,
                        >(20.0)),
                        InfluencingQuantity::Frequency(Frequency::new::<hertz>(50.0)),
                    ],
                    &Default::default(),
                )
                .get::<ohm>(),
            8.624087e-5, // Expected resistance in Ohm
            epsilon = 1e-10
        );

        approxim::assert_abs_diff_eq!(
            rotor_winding
                .slot_leakage_inductance(
                    1,
                    core.as_lin_or_rot(),
                    Length::new::<millimeter>(1.0),
                    &[],
                    &Default::default(),
                )
                .get::<henry>(),
            2.2416022e-7, // Expected value in H
            epsilon = 1e-12
        );
    }
}
