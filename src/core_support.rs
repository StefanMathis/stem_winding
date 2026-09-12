use std::{f64::consts::TAU, sync::Arc};

use stem_core::prelude::*;
use stem_wire::stem_material::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "serde")]
use dyn_quantity::deserialize_opt_quantity;

use crate::winding::Winding;

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
        let core_width = (Length::new::<millimeter>(6.8) + bottom_width) * winding.slots() as f64;

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
    pub(crate) material: std::sync::Arc<Material>, // core material
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
    pub coils: crate::CoilsIterator<'a>,
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
            .end_winding_half_turn_length(self.core.clone(), self.coil.first_zone(), self.overrides)
            .expect("must contain a coil");
    }

    pub fn end_winding_half_turn_volume(&self) -> Volume {
        return self
            .winding
            .end_winding_half_turn_volume(self.core.clone(), self.coil.first_zone(), self.overrides)
            .expect("must contain a coil");
    }

    pub fn end_winding_volume(&self) -> Volume {
        match self.coil() {
            Coil::Full(coil_full) => {
                return 2.0 * self.end_winding_half_turn_volume() * coil_full.turns() as f64;
            }
            Coil::Half(coil_half) => {
                return self.end_winding_half_turn_volume() * coil_half.turns() as f64;
            }
        }
    }

    pub fn volume(&self) -> Volume {
        let winding_area = self.core.zone_area();
        let first_zone = self.coil.first_zone();
        let cross_section = self
            .coil
            .wire()
            .cross_section(winding_area, self.coil.turns());
        let coil_length = self.core.axial_coil_length()
            + self
                .winding
                .axial_coil_overhang(self.core.clone(), first_zone)
                .expect("must contain a coil");
        let multiplier = match self.coil() {
            Coil::Full(_) => 2.0,
            Coil::Half(_) => 1.0,
        };
        return (cross_section * coil_length + self.end_winding_half_turn_volume())
            * multiplier
            * self.coil.turns() as f64;
    }

    pub fn mass(&self) -> Mass {
        let mass_density = self
            .coil
            .wire()
            .material_conductor()
            .mass_density()
            .get(&[]);
        return self.volume() * mass_density;
    }

    pub fn heat_capacity(&self) -> HeatCapacity {
        let specific_heat_capacity = self
            .coil
            .wire()
            .material_conductor()
            .heat_capacity()
            .get(&[]);
        return self.mass() * specific_heat_capacity;
    }
}
