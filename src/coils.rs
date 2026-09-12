use std::num::{NonZeroU16, NonZeroUsize};

use crate::error::Error;
use compare_variables::compare_variables;
use dyn_clone::clone_box;
use keyring_map::KeyringMap;
use num::Complex;
use stem_coil_layout::Zone;
use stem_wire::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Coils(pub KeyringMap<Zone, Coil>);

impl Default for Coils {
    fn default() -> Self {
        Self::new()
    }
}

impl Coils {
    pub fn new() -> Self {
        return Self(KeyringMap::new());
    }

    pub fn with_capacity(capacity: usize) -> Self {
        return Self(KeyringMap::with_capacity(capacity, capacity));
    }

    /**
    Convenience function which inserts the given coil with its zones.
     */
    pub fn insert_coil(
        &mut self,
        coil: Coil,
    ) -> Result<(), keyring_map::InsertionError<(Vec<Zone>, Coil)>> {
        let zones: Vec<Zone> = coil.zones().collect();
        return self.0.insert_many(zones, coil);
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::{Coil, Coils, Zone};
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serializer;
    use serde::de::Error;
    use serde::ser::Serialize;
    use serde::ser::SerializeSeq;

    impl Serialize for Coils {
        // Serialize the coils as sequence
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut seq = serializer.serialize_seq(Some(self.0.num_values()))?;
            for value in self.0.values() {
                seq.serialize_element(&value)?;
            }
            seq.end()
        }
    }

    impl<'de> Deserialize<'de> for Coils {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct Visitor;

            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = Coils;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("a sequence")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::SeqAccess<'de>,
                {
                    let mut map = match seq.size_hint() {
                        Some(capacity) => Coils::with_capacity(capacity),
                        None => Coils::new(),
                    };

                    // Populate the KeyringMap from the sequence
                    while let Some(coil) = seq.next_element::<Coil>()? {
                        let zones: Vec<Zone> = coil.zones().collect();
                        match map.0.insert_many(zones, coil) {
                            Ok(_) => (),
                            Err(err) => {
                                // Create the error message
                                return Err(A::Error::custom(err.to_string()));
                            }
                        }
                    }

                    Ok(map)
                }
            }
            deserializer.deserialize_seq(Visitor {})
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Coil {
    Full(CoilFull),
    Half(CoilHalf),
}

impl Coil {
    /**
    Iterate through all zones occupied by the coil, ordered in the sequence of their occurence.
    This means that the zone "slot 3, layer 2" will always be returned before "slot 4, layer 1", regardless
    whether the zone is positive or negative. If two zones have the same slot, then the one with the lower layer is returned first.
    This ordering corresponds to the implementation of `PartialOrd` for `Zone`.
     */
    pub fn zones<'a>(&'a self) -> ZoneIterator<'a> {
        return ZoneIterator::new(self);
    }

    pub fn zones_and_polarities<'a>(&'a self) -> ZoneAndPolarityIterator<'a> {
        return ZoneAndPolarityIterator::new(self);
    }
}

pub trait CoilExt {
    fn turns(&self) -> NonZeroUsize;

    fn set_turns(&mut self, turns: NonZeroUsize);

    fn first_zone(&self) -> Zone;

    fn first_zone_is_positive(&self) -> bool;

    fn first_zone_and_polarity(&self) -> ZoneAndPolarity;

    fn set_polarity_first_zone(&mut self, positive: bool);

    fn phase(&self) -> NonZeroU16;

    fn set_phase(&mut self, phase: NonZeroU16);

    fn voltage_phasor(&self, phasor_angle: f64, ordinal: f64) -> Complex<f64>;

    fn voltage_phasor_at(
        &self,
        zone: Zone,
        phasor_angle: f64,
        ordinal: f64,
    ) -> Option<Complex<f64>>;

    fn wire(&self) -> &dyn Wire;

    fn set_wire(&mut self, wire: Box<dyn Wire>);

    fn into_wire(self) -> Box<dyn Wire>;

    /// Return the coil span in slot pitches. `slots` is the total number of
    /// winding slots.
    fn span(&self, winding_slots: NonZeroU16) -> u16;

    fn axial_overhang(&self) -> Option<Length>;

    fn end_length(&self) -> Option<Length>;

    fn resistance(
        &self,
        zone_area: Area,
        length: Length,
        conditions: &[DynQuantity<f64>],
    ) -> ElectricalResistance {
        self.wire()
            .resistance(length, zone_area, self.turns(), conditions)
            * usize::from(self.turns()) as f64
    }
}

impl CoilExt for Coil {
    fn turns(&self) -> NonZeroUsize {
        match self {
            Coil::Full(coil) => coil.turns(),
            Coil::Half(coil) => coil.turns(),
        }
    }

    fn set_turns(&mut self, turns: NonZeroUsize) {
        match self {
            Coil::Full(coil) => coil.set_turns(turns),
            Coil::Half(coil) => coil.set_turns(turns),
        }
    }

    fn phase(&self) -> NonZeroU16 {
        match self {
            Coil::Full(coil) => coil.phase(),
            Coil::Half(coil) => coil.phase(),
        }
    }

    fn set_phase(&mut self, phase: NonZeroU16) {
        match self {
            Coil::Full(coil) => coil.set_phase(phase),
            Coil::Half(coil) => coil.set_phase(phase),
        }
    }

    fn voltage_phasor(&self, phasor_angle: f64, ordinal: f64) -> Complex<f64> {
        match self {
            Coil::Full(coil) => coil.voltage_phasor(phasor_angle, ordinal),
            Coil::Half(coil) => coil.voltage_phasor(phasor_angle, ordinal),
        }
    }

    fn voltage_phasor_at(
        &self,
        zone: Zone,
        phasor_angle: f64,
        ordinal: f64,
    ) -> Option<Complex<f64>> {
        match self {
            Coil::Full(coil) => coil.voltage_phasor_at(zone, phasor_angle, ordinal),
            Coil::Half(coil) => coil.voltage_phasor_at(zone, phasor_angle, ordinal),
        }
    }

    fn wire(&self) -> &dyn Wire {
        match self {
            Coil::Full(coil) => coil.wire(),
            Coil::Half(coil) => coil.wire(),
        }
    }

    fn set_wire(&mut self, wire: Box<dyn Wire>) {
        match self {
            Coil::Full(coil) => coil.set_wire(wire),
            Coil::Half(coil) => coil.set_wire(wire),
        }
    }

    fn into_wire(self) -> Box<dyn Wire> {
        match self {
            Coil::Full(coil) => coil.into_wire(),
            Coil::Half(coil) => coil.into_wire(),
        }
    }

    fn span(&self, winding_slots: NonZeroU16) -> u16 {
        match self {
            Coil::Full(coil) => coil.span(winding_slots),
            Coil::Half(coil) => coil.span(winding_slots),
        }
    }

    fn first_zone(&self) -> Zone {
        match self {
            Coil::Full(coil) => coil.first_zone(),
            Coil::Half(coil) => coil.first_zone(),
        }
    }

    fn first_zone_is_positive(&self) -> bool {
        match self {
            Coil::Full(coil) => coil.first_zone_is_positive(),
            Coil::Half(coil) => coil.first_zone_is_positive(),
        }
    }

    fn first_zone_and_polarity(&self) -> ZoneAndPolarity {
        match self {
            Coil::Full(coil) => coil.first_zone_and_polarity(),
            Coil::Half(coil) => coil.first_zone_and_polarity(),
        }
    }

    fn set_polarity_first_zone(&mut self, positive: bool) {
        match self {
            Coil::Full(coil) => coil.set_polarity_first_zone(positive),
            Coil::Half(coil) => coil.set_polarity_first_zone(positive),
        }
    }

    fn axial_overhang(&self) -> Option<Length> {
        match self {
            Coil::Full(coil) => coil.axial_overhang(),
            Coil::Half(coil) => coil.axial_overhang(),
        }
    }

    fn end_length(&self) -> Option<Length> {
        match self {
            Coil::Full(coil) => coil.end_length(),
            Coil::Half(coil) => coil.end_length(),
        }
    }
}

impl From<CoilFull> for Coil {
    fn from(value: CoilFull) -> Self {
        return Self::Full(value);
    }
}

impl From<CoilHalf> for Coil {
    fn from(value: CoilHalf) -> Self {
        return Self::Half(value);
    }
}

// =========================================

/**
# End winding geometry / coil orientation

The property `clockwise` defines the geometry of the end winding. If it is set to true, the end winding coil starts at `positive_zone`,
then the slot index increases until `negative_zone` is reached (possibly crossing zero and wrapping around). If it is set to false,
the end winding coil starts at `positive_zone`, then the slot index decreases until `negative_zone` is reached (possibly crossing zero and wrapping around).
In geometrical terms, this means that the tooth between the first and the last slot has been crossed (for a round, closed core).
When looking at the coil motor from the air gap, the "true" direction results in a clockwise coil, hence the name of this property.

The following example illustrates this:
```text
slot | 0 | 1 | 2 | 3 |

        ---     ---
       |   |   |   |
coil   ^   v   v   ^
       |   |   |   |
        ---     ---
        (a)     (b)
```
In this example, coil (a) is clockwise (`clockwise = true`), while coil (b) is counterclockwise (hence `clockwise = false`)
 */
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CoilFull {
    first_zone: Zone,
    second_zone: Zone,
    first_zone_is_positive: bool,
    clockwise: bool,
    turns: NonZeroUsize,
    // WindingTable represents phases as signed i32 values. Using u16 here
    // ensures that every phase number, with either polarity, fits in i32.
    phase: NonZeroU16,
    wire: Box<dyn Wire>,
    #[cfg_attr(feature = "serde", serde(default))]
    axial_overhang: Option<Length>,
    #[cfg_attr(feature = "serde", serde(default))]
    end_length: Option<Length>,
}

impl CoilFull {
    /// Returns an instance of `Coil`
    pub fn new(
        first_zone: Zone,
        second_zone: Zone,
        first_zone_is_positive: bool,
        clockwise: bool,
        turns: NonZeroUsize,
        phase: NonZeroU16,
        wire: Box<dyn Wire>,
    ) -> Result<Self, Error> {
        return Self::with_coil_end_lengths(
            first_zone,
            second_zone,
            first_zone_is_positive,
            clockwise,
            turns,
            phase,
            wire,
            None,
            None,
        );
    }

    /// Returns an instance of `Coil`
    pub fn with_positive_and_negative_zones(
        positive_zone: Zone,
        negative_zone: Zone,
        clockwise: bool,
        turns: NonZeroUsize,
        phase: NonZeroU16,
        wire: Box<dyn Wire>,
    ) -> Result<Self, Error> {
        return Self::with_coil_end_lengths(
            positive_zone,
            negative_zone,
            true,
            clockwise,
            turns,
            phase,
            wire,
            None,
            None,
        );
    }

    /**
    Build a coil with a specified axial overhang and end winding length.
    Both values are specified for a half-turn (see drawing below)
    If those values are set to None, this function is equivalent to `CoilFull::new`.

    The ASCII art below visualizes the axial coil overhang with equal signs (=)
    and the end winding length with box drawing characters (─).
    If the space of a single character equals one mm, the return value of `axial_overhang` would be 3 mm and
    `end_winding_half_turn_length` would be 7 mm.

    ```text
         ┌──────┐
    ┌──==│      │=──┐
    │    │ Core │   │ <-- Coil
    └──==│      │=──┘
         └──────┘
    ```
     */
    pub fn with_coil_end_lengths(
        first_zone: Zone,
        second_zone: Zone,
        first_zone_is_positive: bool,
        clockwise: bool,
        turns: NonZeroUsize,
        phase: NonZeroU16,
        wire: Box<dyn Wire>,
        axial_overhang: Option<Length>,
        end_length: Option<Length>,
    ) -> Result<Self, Error> {
        if first_zone == second_zone {
            return Err(Error::EqualCoilZones(first_zone));
        }

        let zero = Length::new::<meter>(0.0);
        if let Some(value) = axial_overhang {
            compare_variables!(val zero <= value)?;
        }
        if let Some(value) = end_length {
            compare_variables!(val zero <= value)?;
        }
        return Ok(CoilFull {
            first_zone,
            second_zone,
            first_zone_is_positive,
            clockwise,
            turns,
            phase,
            wire,
            axial_overhang,
            end_length,
        });
    }

    /// Returns the side of the coil where a positive current creates a
    /// clockwise rotating field ("goes into the drawing area")
    pub fn positive_zone(&self) -> Zone {
        if self.first_zone_is_positive {
            return self.first_zone;
        } else {
            return self.second_zone;
        }
    }

    /// Returns the side of the coil where a positive current creates a
    /// counterclockwise rotating field ("goes out of the drawing area").
    pub fn negative_zone(&self) -> Zone {
        if self.first_zone_is_positive {
            return self.second_zone;
        } else {
            return self.first_zone;
        }
    }

    pub fn clockwise(&self) -> bool {
        return self.clockwise;
    }

    pub fn set_clockwise(&mut self, clockwise: bool) {
        self.clockwise = clockwise;
    }

    /**
    Calculates the normalized voltage phasor of the outward coil side for the given harmonic ordinal.

    See the documentation of `voltage_phasor` for examples.
    */
    pub fn voltage_phasor_positive_zone(&self, phasor_angle: f64, ordinal: f64) -> Complex<f64> {
        // Get the angle of the ρ-th zone. The reference is the first zone
        // of phase 1, which always equals the first beam of the phasor star
        // (alpha_rho=0 = 0°) If the zone is negative, the beam direction needs
        // to be inverted (sign-function)
        let slot_angle = self.positive_zone().slot as f64 * phasor_angle * ordinal;
        return usize::from(self.turns()) as f64 * (Complex::new(0.0, slot_angle)).exp();
    }

    /**
    Calculates the normalized voltage phasor of the return coil side for the given harmonic ordinal.
    If the coil is a half-coil, this value is zero by definition.

    See the documentation of `voltage_phasor` for examples.
    */
    pub fn voltage_phasor_negative_zone(&self, phasor_angle: f64, ordinal: f64) -> Complex<f64> {
        // Get the angle of the ρ-th zone. The reference is the first zone
        // of phase 1, which always equals the first beam of the phasor star
        // (alpha_rho=0 = 0°) If the zone is negative, the beam direction needs
        // to be inverted (sign-function)
        let slot_angle = self.positive_zone().slot as f64 * phasor_angle * ordinal;
        return usize::from(self.turns()) as f64 * (Complex::new(0.0, slot_angle)).exp();
    }

    pub fn second_zone(&self) -> Zone {
        return self.second_zone;
    }

    /**
    Returns an iterator over the slots "covered" by the end winding of the coil,
    starting at the first zone slot and stopping at the second zone slot.
     */
    pub fn covered_slots(&self, slots: NonZeroU16) -> CoveredSlots {
        let ascending = if self.clockwise() {
            self.first_zone_is_positive
        } else {
            !self.first_zone_is_positive
        };
        return CoveredSlots {
            second_slot: self.second_zone.slot,
            slots: slots.get(),
            slot: self.first_zone.slot,
            ascending,
            exhausted: false,
        };
    }
}

pub struct CoveredSlots {
    second_slot: u16,
    slots: u16,
    slot: u16,
    ascending: bool,
    exhausted: bool,
}

impl Iterator for CoveredSlots {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }
        if self.slot == self.second_slot {
            self.exhausted = true;
            return Some(self.slot);
        } else {
            let returned_slot = self.slot;
            if self.ascending {
                self.slot += 1;
                if self.slot == self.slots {
                    self.slot = 0;
                }
            } else {
                match self.slot.checked_sub(1) {
                    Some(val) => self.slot = val,
                    None => self.slot = self.slots - 1,
                }
            }
            return Some(returned_slot);
        }
    }
}

impl Clone for CoilFull {
    fn clone(&self) -> Self {
        Self {
            first_zone: self.first_zone.clone(),
            second_zone: self.second_zone.clone(),
            clockwise: self.clockwise.clone(),
            turns: self.turns.clone(),
            phase: self.phase.clone(),
            wire: clone_box(&*self.wire),
            first_zone_is_positive: self.first_zone_is_positive.clone(),
            axial_overhang: self.axial_overhang.clone(),
            end_length: self.end_length.clone(),
        }
    }
}

impl CoilExt for CoilFull {
    fn turns(&self) -> NonZeroUsize {
        return self.turns;
    }

    fn set_turns(&mut self, turns: NonZeroUsize) {
        self.turns = turns;
    }

    fn phase(&self) -> NonZeroU16 {
        return self.phase;
    }

    fn set_phase(&mut self, phase: NonZeroU16) {
        self.phase = phase;
    }

    /**
    Calculates the normalized voltage phasor of the given coil for the given harmonic ordinal.

    The winding topology is defined by the phasor angle, which can be calculated with the free function `phasor_angle` or the winding method of the same name.

    The term "normalized phasor voltage" means that the induced voltage is assumed to have an amplitude of 1 V per turn.

    ```
    use winding::{CoilFull, CoilExt, Zone, phasor_angle};
    use wire::RoundWire;
    use approxim;

    let outward_side = Zone {slot: 1, layer: 0};
    let return_side = Zone {slot: 7, layer: 0};
    let coil = CoilFull::new(outward_side, return_side, true, true, 10, 1, Box::new(RoundWire::default())).unwrap();

    // Calculate the electrical phasor angle between two slots for a 6-slot winding with one pole pair.
    let angle = phasor_angle(6, 1);

    // Calculate the phasor of the fundamental harmonic
    let phasor = coil.voltage_phasor(angle, 1.0);
    approxim::assert_abs_diff_eq!(phasor.norm(), 20.0);
    approxim::assert_abs_diff_eq!(phasor.to_polar().0, 20.0); // Radius
    approxim::assert_abs_diff_eq!(phasor.to_polar().1, 1.047197, epsilon = 1e-6); // Angle

    // Look at the coil sides
    let phasor_outward = coil.voltage_phasor_positive_zone(angle, 1.0);
    approxim::assert_abs_diff_eq!(phasor_outward.re, 5.0, epsilon = 1e-6);
    approxim::assert_abs_diff_eq!(phasor_outward.im, 8.660254, epsilon = 1e-6);
    approxim::assert_abs_diff_eq!(phasor_outward.to_polar().1, 1.047197, epsilon = 1e-6);

    let phasor_return = coil.voltage_phasor_negative_zone(angle, 1.0);
    approxim::assert_abs_diff_eq!(phasor_return.re, 5.0, epsilon = 1e-6);
    approxim::assert_abs_diff_eq!(phasor_return.im, 8.660254, epsilon = 1e-6);
    approxim::assert_abs_diff_eq!(phasor_return.to_polar().1, 1.047197, epsilon = 1e-6);

    ```
    */
    fn voltage_phasor(&self, phasor_angle: f64, ordinal: f64) -> Complex<f64> {
        return self.voltage_phasor_positive_zone(phasor_angle, ordinal)
            + self.voltage_phasor_negative_zone(phasor_angle, ordinal);
    }

    fn wire(&self) -> &dyn Wire {
        return &*self.wire;
    }

    fn set_wire(&mut self, wire: Box<dyn Wire>) {
        self.wire = wire;
    }

    fn into_wire(self) -> Box<dyn Wire> {
        return self.wire;
    }

    /**
    Return the coil throw. `slots` is the total number of winding slots.
    This function performs wrapping subtraction, meaning it won't panic if `slots`
    is a nonsensical value (e.g. smaller than one of the two coil slots). It will however
    return a nonsensical result.
     */
    fn span(&self, slots: NonZeroU16) -> u16 {
        let slots = slots.get();

        let (minuend, subtrahend) = if self.clockwise() {
            (self.negative_zone().slot, self.positive_zone().slot)
        } else {
            (self.positive_zone().slot, self.negative_zone().slot)
        };

        // Cover special case
        if minuend == subtrahend {
            return (!self.clockwise()) as u16 * slots;
        }
        let result = (slots + minuend).wrapping_sub(subtrahend);
        if result > slots {
            return result.wrapping_sub(slots);
        } else {
            return result;
        }
    }

    fn voltage_phasor_at(
        &self,
        zone: Zone,
        phasor_angle: f64,
        ordinal: f64,
    ) -> Option<Complex<f64>> {
        if self.positive_zone() == zone {
            return Some(self.voltage_phasor_positive_zone(phasor_angle, ordinal));
        } else if self.negative_zone() == zone {
            return Some(self.voltage_phasor_negative_zone(phasor_angle, ordinal));
        } else {
            return None;
        }
    }

    fn first_zone(&self) -> Zone {
        return self.first_zone;
    }

    fn first_zone_is_positive(&self) -> bool {
        return self.first_zone_is_positive;
    }

    fn first_zone_and_polarity(&self) -> ZoneAndPolarity {
        return ZoneAndPolarity {
            zone: self.first_zone,
            positive: self.first_zone_is_positive,
        };
    }

    fn set_polarity_first_zone(&mut self, positive: bool) {
        self.first_zone_is_positive = positive;
    }

    fn axial_overhang(&self) -> Option<Length> {
        return self.axial_overhang;
    }

    fn end_length(&self) -> Option<Length> {
        return self.end_length;
    }
}

// ================================================================

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CoilHalf {
    zone: Zone,
    positive: bool,
    turns: NonZeroUsize,
    phase: NonZeroU16,
    wire: Box<dyn Wire>,
    #[cfg_attr(feature = "serde", serde(default))]
    axial_overhang: Option<Length>,
    #[cfg_attr(feature = "serde", serde(default))]
    end_length: Option<Length>,
}

impl CoilHalf {
    /// Returns an instance of `Coil`
    pub fn new(
        zone: Zone,
        positive: bool,
        turns: NonZeroUsize,
        phase: NonZeroU16,
        wire: Box<dyn Wire>,
    ) -> Self {
        Self::with_coil_end_lengths(zone, positive, turns, phase, wire, None, None)
    }

    pub fn with_coil_end_lengths(
        zone: Zone,
        positive: bool,
        turns: NonZeroUsize,
        phase: NonZeroU16,
        wire: Box<dyn Wire>,
        axial_overhang: Option<Length>,
        end_length: Option<Length>,
    ) -> Self {
        let zero_length = Length::new::<meter>(0.0);
        if let Some(value) = axial_overhang {
            compare_variables!(zero_length <= value).unwrap();
        }
        if let Some(value) = end_length {
            compare_variables!(zero_length <= value).unwrap();
        }
        return CoilHalf {
            zone,
            positive,
            turns,
            phase,
            wire,
            axial_overhang,
            end_length,
        };
    }

    pub fn zone(&self) -> Zone {
        return self.zone;
    }
}

impl Clone for CoilHalf {
    fn clone(&self) -> Self {
        Self {
            zone: self.zone.clone(),
            positive: self.positive.clone(),
            turns: self.turns.clone(),
            phase: self.phase.clone(),
            wire: clone_box(&*self.wire),
            axial_overhang: self.axial_overhang.clone(),
            end_length: self.end_length.clone(),
        }
    }
}

impl CoilExt for CoilHalf {
    fn turns(&self) -> NonZeroUsize {
        return self.turns;
    }

    fn set_turns(&mut self, turns: NonZeroUsize) {
        self.turns = turns;
    }

    fn phase(&self) -> NonZeroU16 {
        return self.phase;
    }

    fn set_phase(&mut self, phase: NonZeroU16) {
        self.phase = phase;
    }

    fn voltage_phasor(&self, phasor_angle: f64, ordinal: f64) -> Complex<f64> {
        let slot_angle = self.zone().slot as f64 * phasor_angle * ordinal;
        let slot_angle = if self.first_zone_is_positive() {
            slot_angle
        } else {
            -slot_angle
        };
        return usize::from(self.turns()) as f64 * (Complex::new(0.0, slot_angle)).exp();
    }

    fn wire(&self) -> &dyn Wire {
        return &*self.wire;
    }

    fn set_wire(&mut self, wire: Box<dyn Wire>) {
        self.wire = wire;
    }

    fn into_wire(self) -> Box<dyn Wire> {
        return self.wire;
    }

    fn span(&self, _slots: NonZeroU16) -> u16 {
        return 0;
    }

    fn voltage_phasor_at(
        &self,
        zone: Zone,
        phasor_angle: f64,
        ordinal: f64,
    ) -> Option<Complex<f64>> {
        if self.zone == zone {
            return Some(self.voltage_phasor(phasor_angle, ordinal));
        } else {
            return None;
        }
    }

    fn first_zone(&self) -> Zone {
        return self.zone;
    }

    fn first_zone_is_positive(&self) -> bool {
        return self.positive;
    }

    fn first_zone_and_polarity(&self) -> ZoneAndPolarity {
        return ZoneAndPolarity {
            zone: self.zone,
            positive: self.positive,
        };
    }

    fn set_polarity_first_zone(&mut self, positive: bool) {
        self.positive = positive;
    }

    fn axial_overhang(&self) -> Option<Length> {
        return self.axial_overhang;
    }

    fn end_length(&self) -> Option<Length> {
        return self.end_length;
    }
}

// ================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneAndPolarity {
    pub zone: Zone,
    pub positive: bool,
}

pub struct ZoneAndPolarityIterator<'a> {
    coil: &'a Coil,
    counter: usize,
}
impl<'a> ZoneAndPolarityIterator<'a> {
    fn new(coil: &'a Coil) -> Self {
        return ZoneAndPolarityIterator { coil, counter: 0 };
    }
}
impl<'a> Iterator for ZoneAndPolarityIterator<'a> {
    type Item = ZoneAndPolarity;
    fn next(&mut self) -> Option<ZoneAndPolarity> {
        self.counter += 1;
        match self.coil {
            Coil::Full(coil) => match self.counter {
                1 => {
                    if coil.first_zone() < coil.second_zone() {
                        return Some(ZoneAndPolarity {
                            zone: coil.first_zone(),
                            positive: coil.first_zone_is_positive(),
                        });
                    } else {
                        return Some(ZoneAndPolarity {
                            zone: coil.second_zone(),
                            positive: !coil.first_zone_is_positive(),
                        });
                    }
                }
                2 => {
                    if coil.first_zone() < coil.second_zone() {
                        return Some(ZoneAndPolarity {
                            zone: coil.second_zone(),
                            positive: !coil.first_zone_is_positive(),
                        });
                    } else {
                        return Some(ZoneAndPolarity {
                            zone: coil.first_zone(),
                            positive: coil.first_zone_is_positive(),
                        });
                    }
                }
                _ => return None,
            },
            Coil::Half(coil) => match self.counter {
                1 => {
                    return Some(ZoneAndPolarity {
                        zone: coil.zone(),
                        positive: coil.first_zone_is_positive(),
                    });
                }
                _ => return None,
            },
        }
    }
}

pub struct ZoneIterator<'a>(ZoneAndPolarityIterator<'a>);

impl<'a> ZoneIterator<'a> {
    fn new(coil: &'a Coil) -> Self {
        return ZoneIterator(ZoneAndPolarityIterator::new(coil));
    }
}

impl<'a> Iterator for ZoneIterator<'a> {
    type Item = Zone;

    fn next(&mut self) -> Option<Zone> {
        let zone_and_polarity = self.0.next()?;
        return Some(zone_and_polarity.zone);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stem_wire::round::RoundWire;

    #[test]
    fn test_coil_orientation() {
        // Tooth coil
        let coil = CoilFull::new(
            Zone::new(0, 0),
            Zone::new(1, 0),
            true,
            true,
            NonZeroUsize::MIN,
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .unwrap();
        assert_eq!(coil.span(NonZeroU16::new(12).expect("not zero")), 1);

        // Another tooth coil which wraps around (11 -> 0)
        let coil = CoilFull::new(
            Zone::new(11, 0),
            Zone::new(0, 0),
            true,
            true,
            NonZeroUsize::MIN,
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .unwrap();
        assert_eq!(coil.span(NonZeroU16::new(12).expect("not zero")), 1);

        // Extremely long coil
        let coil = CoilFull::new(
            Zone::new(0, 0),
            Zone::new(1, 0),
            true,
            false,
            NonZeroUsize::MIN,
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .unwrap();
        assert_eq!(coil.span(NonZeroU16::new(12).expect("not zero")), 11);

        // Both coil zones occupy the same slot
        let coil = CoilFull::new(
            Zone::new(0, 0),
            Zone::new(0, 1),
            true,
            true,
            NonZeroUsize::MIN,
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .unwrap();
        assert_eq!(coil.span(NonZeroU16::new(12).expect("not zero")), 0);

        // Both coil zones occupy the same slot
        let coil = CoilFull::new(
            Zone::new(0, 0),
            Zone::new(0, 1),
            true,
            false,
            NonZeroUsize::MIN,
            NonZeroU16::MIN,
            Box::new(RoundWire::default()),
        )
        .unwrap();
        assert_eq!(coil.span(NonZeroU16::new(12).expect("not zero")), 12);
    }
}
