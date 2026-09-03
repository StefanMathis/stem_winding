use std::{f64::consts::TAU, sync::Arc};

use stem_core::prelude::*;
use stem_core::stem_slot::semi_trapezoid::SemiTrapezoidWithoutSlopesBuilder;

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
        let slots = winding.slots();
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
