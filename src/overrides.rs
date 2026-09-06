use stem_wire::stem_material::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "serde")]
use dyn_quantity::deserialize_opt_quantity;

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
