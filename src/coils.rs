/*!
A [`Coil`] consists of a [`Wire`] which is wound into a series of continuous
loops which are placed next to each other so they form a single, thick loop.
When an electric current flows through it, it creates a magnetic field.

In stem, the [`Coil`] type is the fundamental building block of a
[`Winding`](crate::winding::Winding) and occupies one or more of its [`Zone`]s.
It consists of a [`Wire`] trait object and additional information like the
number of loops ([`turns`](CoilExt::turns)) or the [`phase`](CoilExt::phase) it
is connected with. Since there are multiple possible coil types ([`HalfCoil`]
and [`FullCoil`]), common functionality is factored out in the sealed
[`CoilExt`] trait.

Any [`Zone`] of a [`Winding`](crate::winding::Winding) can be occupied by at
most one coil. This allows using a [`Zone`] as an identifier for a [`Coil`]
within a [`Winding`](crate::winding::Winding). The [`Coils`] hash map allows
storing a [`Coil`] in a manner so it can be retrieved from any of its zones and
is a useful building block when defining custom
[`Winding`](crate::winding::Winding)s. All predefined windings within this crate
use [`Coils`] to provide convenient access to their individual coils.
 */

use std::num::{NonZeroU16, NonZeroUsize};

use crate::error::Error;
use keyring_map::{InsertionError, KeyringMap};
use num::Complex;
use stem_coil_layout::Zone;
use stem_wire::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/**
A convenience wrapper around a [`KeyringMap<Zone, Coil>`] for storing the
[`Coil`]s of a [`Winding`](crate::winding::Winding).

A [`Zone`] of a [`Winding`](crate::winding::Winding) can contain at most one
[`Coil`] (it can also be empty). Since a [`Coil`] can occupy multiple zones,
a multi-key hash map provides a convenient way to access the same coil using any
of its [`Zone`]s as a key. [`Coils`] is a thin wrapper around that map which
exposes a subset of the functionality of [`KeyringMap<Zone, Coil>`] to prevent
"illegal" operations (like inserting a [`Coil`] with a [`Zone`] it doesn't
occupy).

# Serialization and deserialization

[`Coils`] provides a very convenient serialization and
deserialization model: Since a [`Coil`] already knows which zones it occupies,
[`Coils`] can be serialized into and deserialized from a simple sequence of
coils, it is not necessary to use the map representation of the serialization
format:

```
use indoc::indoc;
use stem_winding::prelude::*;
use yaml_serde;

 let yaml = indoc! {"
- !Full
    positive_zone:
      slot: 0
      layer: 0
    negative_zone:
      slot: 1
      layer: 0
    positive_slot_direction: true
    turns: 1
    phase: 1
    wire:
      RoundWire:
        outer_diameter: 1 mm
        inner_diameter: 0 mm
        insulation_thickness: 0 mm
        conductor_material:
          name: Copper
          relative_permeability: 1
- !Full
    positive_zone:
      slot: 3
      layer: 0
    negative_zone:
      slot: 2
      layer: 0
    positive_slot_direction: false
    turns: 1
    phase: 2
    wire:
      RoundWire:
        outer_diameter: 1 mm
        inner_diameter: 0 mm
        insulation_thickness: 0 mm
        conductor_material:
          name: Copper
          relative_permeability: 1
"};

let coils: Coils = yaml_serde::from_str(yaml).unwrap();
assert_eq!(coils.num_coils(), 2);
assert_eq!(coils.num_zones(), 4);
```
*/
#[derive(Clone, Debug)]
pub struct Coils(KeyringMap<Zone, Coil>);

impl Default for Coils {
    fn default() -> Self {
        Self::new()
    }
}

impl Coils {
    /// Creates a new [`Coils`] container. This doesn't allocate before a
    /// [`Coil`] is inserted.
    pub fn new() -> Self {
        return Self(KeyringMap::new());
    }

    /// Creates a new empty [`Coils`] container which preallocates space for the
    /// specified number of zones and coils.
    pub fn with_capacity(num_zones: usize, num_coils: usize) -> Self {
        return Self(KeyringMap::with_capacity(num_zones, num_coils));
    }

    /// Returns the number of zones stored in the map.
    pub fn num_zones(&self) -> usize {
        self.0.num_keys()
    }

    /// Returns the number of coils stored in the map.
    pub fn num_coils(&self) -> usize {
        self.0.num_values()
    }

    /// Returns a reference to the [`Coil`] occupying this [`Zone`], if present.
    pub fn get(&self, zone: Zone) -> Option<&Coil> {
        self.0.get(&zone)
    }

    /// Returns a mutable reference to the [`Coil`] occupying this [`Zone`], if
    /// present, else `None`.
    pub fn get_mut(&mut self, zone: Zone) -> Option<&mut Coil> {
        self.0.get_mut(&zone)
    }

    /// Returns `true` if the given [`Zone`] is occupied by a [`Coil`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::{NonZeroU16, NonZeroUsize};
    /// use stem_winding::prelude::*;
    ///
    /// let wire: Box<dyn Wire> = Box::new(RoundWire::default());
    ///
    /// let coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(1, 0),
    ///     true,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     wire,
    /// ).expect("zones not identical");
    ///
    /// let mut coils = Coils::new();
    /// assert!(coils.insert(coil.into()).is_ok());
    /// assert!(coils.occupied(Zone::new(0, 0)));
    /// assert!(coils.occupied(Zone::new(1, 0)));
    /// assert!(!coils.occupied(Zone::new(2, 0)));
    /// ```
    pub fn occupied(&self, zone: Zone) -> bool {
        self.0.contains_key(&zone)
    }

    /// Removes all [`Coil`]s and their [`Zone`]s from the map while preserving
    /// its capacity.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Inserts a [`Coil`], using all of its [`Zone`]s as keys. If any of the
    /// zones is already occupied by another [`Coil`], an [`InsertionError`] is
    /// returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::{NonZeroU16, NonZeroUsize};
    /// use stem_winding::prelude::*;
    ///
    /// let wire: Box<dyn Wire> = Box::new(RoundWire::default());
    ///
    /// let coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(1, 0),
    ///     true,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     wire,
    /// ).expect("zones not identical");
    ///
    /// let mut coils = Coils::new();
    /// assert_eq!(coils.num_zones(), 0);
    /// assert_eq!(coils.num_coils(), 0);
    ///
    /// assert!(coils.insert(coil.clone().into()).is_ok());
    /// assert_eq!(coils.num_zones(), 2);
    /// assert_eq!(coils.num_coils(), 1);
    ///
    /// // Attempt to insert another coil occupying the same zones. This fails because
    /// // the zones are already occupied.
    /// assert!(coils.insert(coil.into()).is_err());
    /// ```
    pub fn insert(&mut self, coil: Coil) -> Result<(), InsertionError<(Vec<Zone>, Coil)>> {
        let zones: Vec<Zone> = coil.zones().collect();
        return self.0.insert_many(zones, coil);
    }

    /// Removes the [`Coil`] occupying the given `zone` and all of its zones
    /// from the map and returns it. If no [`Coil`] occupies the given
    /// `zone`, this method returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::{NonZeroU16, NonZeroUsize};
    /// use stem_winding::prelude::*;
    ///
    /// let wire: Box<dyn Wire> = Box::new(RoundWire::default());
    ///
    /// let coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(1, 0),
    ///     true,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     wire,
    /// ).expect("zones not identical");
    ///
    /// let mut coils = Coils::new();
    /// assert_eq!(coils.num_zones(), 0);
    /// assert_eq!(coils.num_coils(), 0);
    ///
    /// assert!(coils.insert(coil.into()).is_ok());
    /// assert_eq!(coils.num_zones(), 2);
    /// assert_eq!(coils.num_coils(), 1);
    ///
    /// assert!(coils.remove(Zone::new(1, 0)).is_some());
    /// assert_eq!(coils.num_zones(), 0);
    /// assert_eq!(coils.num_coils(), 0);
    /// ```
    pub fn remove(&mut self, zone: Zone) -> Option<Coil> {
        self.0.remove(&zone)
    }

    /// Returns an iterator over all coils.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::{NonZeroU16, NonZeroUsize};
    /// use stem_winding::prelude::*;
    ///
    /// let wire: Box<dyn Wire> = Box::new(RoundWire::default());
    ///
    /// let coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(1, 0),
    ///     true,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     wire,
    /// ).expect("zones not identical");
    ///
    /// let mut coils = Coils::new();
    /// assert!(coils.insert(coil.into()).is_ok());
    /// assert_eq!(coils.iter_coils().count(), 1);
    pub fn iter_coils(&self) -> impl Iterator<Item = &Coil> {
        self.0.values()
    }

    /// Returns an iterator over all zones and the coils occupying them.
    ///
    /// Since multiple zones can point to the same coil, some cois can be
    /// returned multiple times and the total number of returned pairs is equal
    /// to the number of zones.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::{NonZeroU16, NonZeroUsize};
    /// use stem_winding::prelude::*;
    ///
    /// let wire: Box<dyn Wire> = Box::new(RoundWire::default());
    ///
    /// let coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(1, 0),
    ///     true,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     wire,
    /// ).expect("zones not identical");
    ///
    /// let mut coils = Coils::new();
    /// assert!(coils.insert(coil.into()).is_ok());
    /// assert_eq!(coils.iter().count(), 2);
    pub fn iter(&self) -> impl Iterator<Item = (&Zone, &Coil)> {
        self.0.iter()
    }

    /// Returns an iterator over all zones.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::{NonZeroU16, NonZeroUsize};
    /// use stem_winding::prelude::*;
    ///
    /// let wire: Box<dyn Wire> = Box::new(RoundWire::default());
    ///
    /// let coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(1, 0),
    ///     true,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     wire,
    /// ).expect("zones not identical");
    ///
    /// let mut coils = Coils::new();
    /// assert!(coils.insert(coil.into()).is_ok());
    /// assert_eq!(coils.iter_zones().count(), 2);
    pub fn iter_zones(&self) -> impl Iterator<Item = &Zone> {
        self.0.keys()
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
                        Some(capacity) => Coils::with_capacity(capacity, capacity),
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

/// An enum representing all possible coil variants.
///
/// See the [module-level docs](crate::coils) for more.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Coil {
    /// A [`FullCoil`].
    Full(FullCoil),
    /// A [`HalfCoil`].
    Half(HalfCoil),
}

impl Coil {
    /// Returns an iterator over all [`Zone`]s of the coil.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::{NonZeroU16, NonZeroUsize};
    /// use stem_winding::prelude::*;
    ///
    /// let full_coil_wire: Box<dyn Wire> = Box::new(RoundWire::default());
    /// let full_coil: Coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(1, 0),
    ///     true,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     full_coil_wire,
    /// ).expect("zones not identical").into();
    ///
    /// let mut zones = full_coil.zones();
    /// assert_eq!(zones.next(), Some(Zone::new(0, 0)));
    /// assert_eq!(zones.next(), Some(Zone::new(1, 0)));
    /// assert_eq!(zones.next(), None);
    ///
    /// let half_coil_wire: Box<dyn Wire> = Box::new(RoundWire::default());
    /// let half_coil: Coil = HalfCoil::new(
    ///     Zone::new(0, 0),
    ///     true,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     half_coil_wire,
    /// ).into();
    ///
    /// let mut zones = half_coil.zones();
    /// assert_eq!(zones.next(), Some(Zone::new(0, 0)));
    /// assert_eq!(zones.next(), None);
    /// ```
    pub fn zones<'a>(&'a self) -> ZoneIterator<'a> {
        return ZoneIterator::new(self);
    }

    /// Returns an iterator over all [`Zone`]s occupied by the coil and their
    /// phase polarities. The iterator produces [`ZoneAndPolarity`] instances;
    /// see its documentation for more information.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::{NonZeroU16, NonZeroUsize};
    /// use stem_winding::prelude::*;
    ///
    /// let full_coil_wire: Box<dyn Wire> = Box::new(RoundWire::default());
    /// let full_coil: Coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(1, 0),
    ///     true,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     full_coil_wire,
    /// ).expect("zones not identical").into();
    ///
    /// let zones: Vec<_> = full_coil.zones_and_polarities().collect();
    ///
    /// assert_eq!(zones.len(), 2);
    /// assert!(zones.contains(&ZoneAndPolarity {
    ///    zone: Zone::new(0, 0),
    ///    is_positive: true,
    /// }));
    /// assert!(zones.contains(&ZoneAndPolarity {
    ///    zone: Zone::new(1, 0),
    ///    is_positive: false,
    /// }));
    ///
    /// let half_coil_wire: Box<dyn Wire> = Box::new(RoundWire::default());
    /// let half_coil: Coil = HalfCoil::new(
    ///     Zone::new(0, 0),
    ///     false,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     half_coil_wire,
    /// ).into();
    ///
    /// let zones: Vec<_> = half_coil.zones_and_polarities().collect();
    ///
    /// assert_eq!(zones.len(), 2);
    /// assert!(zones.contains(&ZoneAndPolarity {
    ///    zone: Zone::new(0, 0),
    ///    is_positive: false,
    /// }));
    /// ```
    pub fn zones_and_polarities<'a>(&'a self) -> ZoneAndPolarityIterator<'a> {
        return ZoneAndPolarityIterator::new(self);
    }

    /// Returns a reference to the wrapped [`FullCoil`], if the enum variant is
    /// [`Coil::Full`].
    pub fn full(&self) -> Option<&FullCoil> {
        match self {
            Coil::Full(full_coil) => Some(full_coil),
            Coil::Half(_) => None,
        }
    }

    /// Returns a reference to the wrapped [`HalfCoil`], if the enum variant is
    /// [`Coil::Half`].
    pub fn half(&self) -> Option<&HalfCoil> {
        match self {
            Coil::Full(_) => None,
            Coil::Half(half_coil) => Some(half_coil),
        }
    }
}

mod private {
    /// Sealed trait for [`CoilExt`](super::CoilExt).
    pub trait Sealed {}
}

/**
A trait for functionality which is shared between the different [`Coil`]
variants.

This trait provides functionality common to [`Coil`] and its variants such as
accessing or changing the number of turns or the coil phase. It is not meant to
be implemented by external types and is therefore sealed.
 */
pub trait CoilExt: private::Sealed {
    fn turns(&self) -> NonZeroUsize;

    fn set_turns(&mut self, turns: NonZeroUsize);

    fn invert(&mut self);

    fn phase(&self) -> NonZeroU16;

    fn set_phase(&mut self, phase: NonZeroU16);

    fn voltage_phasor(&self, phasor_angle: f64, ordinal: f64) -> Complex<f64>;

    fn voltage_phasor_at(&self, zone: Zone, phasor_angle: f64, ordinal: f64) -> Complex<f64>;

    /// Arbitrary - but not random - zone of the coil. Since each zone is
    /// occupied by exactly one coil, this is a unique identifier / "hash" for
    /// the coil.
    fn any_zone(&self) -> Zone;

    fn wire(&self) -> &dyn Wire;

    fn set_wire(&mut self, wire: Box<dyn Wire>);

    fn into_wire(self) -> Box<dyn Wire>;

    /// Returns the coil throw in slot pitches.
    ///
    /// slots specifies the number of slots for a cyclic winding structure. If
    /// None, a linear winding structure is assumed and no wrapping around the
    /// slot sequence is possible.
    fn throw(&self, slots: Option<NonZeroU16>) -> u16;

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

impl private::Sealed for Coil {}

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

    fn any_zone(&self) -> Zone {
        match self {
            Coil::Full(coil) => coil.any_zone(),
            Coil::Half(coil) => coil.any_zone(),
        }
    }

    fn voltage_phasor(&self, phasor_angle: f64, ordinal: f64) -> Complex<f64> {
        match self {
            Coil::Full(coil) => coil.voltage_phasor(phasor_angle, ordinal),
            Coil::Half(coil) => coil.voltage_phasor(phasor_angle, ordinal),
        }
    }

    fn voltage_phasor_at(&self, zone: Zone, phasor_angle: f64, ordinal: f64) -> Complex<f64> {
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

    fn throw(&self, slots: Option<NonZeroU16>) -> u16 {
        match self {
            Coil::Full(coil) => coil.throw(slots),
            Coil::Half(coil) => coil.throw(slots),
        }
    }

    fn invert(&mut self) {
        match self {
            Coil::Full(coil) => coil.invert(),
            Coil::Half(coil) => coil.invert(),
        }
    }
}

impl From<FullCoil> for Coil {
    fn from(value: FullCoil) -> Self {
        return Self::Full(value);
    }
}

impl From<HalfCoil> for Coil {
    fn from(value: HalfCoil) -> Self {
        return Self::Half(value);
    }
}

// =========================================

/**
# End winding geometry / coil orientation

The property positive_slot_direction defines the direction of the end winding
between first_zone and second_zone for a round, closed core. Such a core
provides two possible paths around the circumference, one following increasing
slot indices and one following decreasing slot indices.

If positive_slot_direction is true, the end winding proceeds from
first_zone to second_zone with increasing slot indices. If it is false,
it proceeds with decreasing slot indices. In either case, the slot indices wrap
around when crossing the end of the slot sequence.

positive_slot_direction = starting at positive_zone, does the end winding proceed in the direction of increasing slot indices?

For a linear core, there is only one possible path between two zones, so
positive_slot_direction has no geometrical significance.

For a round, closed core, the two directions correspond to the two possible
ways of routing the end winding around the core. For example, connecting slots
0 and 1 can either take the short path directly from 0 to 1 or the long path
wrapping around the other side of the core:

Arrow going up: Positive in this diagram

```text
slot | 0 | 1 | 2 | 3 | 4 | 5

     ──┐   ┌───┐   ┌───┐   ┌──
       │   │   │   │   │   │
coil   ▲   ▲   ▼   ▼   ▲   ▼
       │   │   │   │   |   │
     ──┘   └───┘   └───┘   └──
     (a)    (b)     (c)    (a)
```

(a): positive_slot_direction = false
(b): positive_slot_direction = true
(c): positive_slot_direction = false

In the example, coil (a) connects slots 0 and 1 in the positive slot direction. Coil (b) connects slots 2 and 1 in the negative slot direction. Coil (c) connects slots 4 and 0 by wrapping around the end of the slot sequence.
 */
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FullCoil {
    positive_zone: Zone,
    negative_zone: Zone,
    positive_slot_direction: bool,
    turns: NonZeroUsize,
    // WindingTable represents phases as signed i32 values. Using u16 here
    // ensures that every phase number, with either polarity, fits in i32.
    phase: NonZeroU16,
    wire: Box<dyn Wire>,
}

impl FullCoil {
    /// Returns an instance of `Coil`
    pub fn new(
        positive_zone: Zone,
        negative_zone: Zone,
        positive_slot_direction: bool,
        turns: NonZeroUsize,
        phase: NonZeroU16,
        wire: Box<dyn Wire>,
    ) -> Result<Self, Error> {
        if positive_zone == negative_zone {
            return Err(Error::EqualCoilZones(positive_zone));
        }

        return Ok(FullCoil {
            positive_zone,
            negative_zone,
            positive_slot_direction,
            turns,
            phase,
            wire,
        });
    }

    pub fn positive_slot_direction(&self) -> bool {
        self.positive_slot_direction
    }

    pub fn positive_zone(&self) -> Zone {
        self.positive_zone
    }

    pub fn negative_zone(&self) -> Zone {
        self.negative_zone
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

    /**
    Returns an iterator over the slots "covered" by the end winding of the coil,
    starting at the first zone slot and stopping at the second zone slot.
     */
    pub fn covered_slots(&self, slots: Option<NonZeroU16>) -> CoveredSlots {
        return CoveredSlots {
            second_slot: self.negative_zone.slot,
            slots,
            slot: self.positive_zone.slot,
            positive_slot_direction: self.positive_slot_direction,
            exhausted: false,
        };
    }
}

pub struct CoveredSlots {
    second_slot: u16,
    slots: Option<NonZeroU16>,
    slot: u16,
    positive_slot_direction: bool,
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
            // Linear or rotary core?
            match self.slots {
                Some(slots) => {
                    let returned_slot = self.slot;
                    if self.positive_slot_direction {
                        self.slot += 1;
                        if self.slot == slots.get() {
                            self.slot = 0;
                        }
                    } else {
                        match self.slot.checked_sub(1) {
                            Some(val) => self.slot = val,
                            None => self.slot = slots.get() - 1,
                        }
                    }
                    return Some(returned_slot);
                }
                None => {
                    let slot = self.slot;
                    if self.second_slot > self.slot {
                        self.slot += 1;
                    } else {
                        self.slot -= 1;
                    }
                    return Some(slot);
                }
            }
        }
    }
}

impl private::Sealed for FullCoil {}

impl CoilExt for FullCoil {
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
    use winding::{FullCoil, CoilExt, Zone, phasor_angle};
    use wire::RoundWire;
    use approxim;

    let outward_side = Zone {slot: 1, layer: 0};
    let return_side = Zone {slot: 7, layer: 0};
    let coil = FullCoil::new(outward_side, return_side, true, true, 10, 1, Box::new(RoundWire::default())).unwrap();

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

    fn any_zone(&self) -> Zone {
        self.positive_zone
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

    fn throw(&self, slots: Option<NonZeroU16>) -> u16 {
        let positive = self.positive_zone().slot;
        let negative = self.negative_zone().slot;

        let Some(slots) = slots else {
            return self
                .positive_zone()
                .slot
                .abs_diff(self.negative_zone().slot);
        };
        let slots = slots.get();

        // Treat the special case of the slots being equal
        if positive == negative {
            if self.positive_zone() > self.negative_zone() {
                if self.positive_slot_direction() {
                    return slots;
                }
                return 0;
            } else {
                if self.positive_slot_direction() {
                    return 0;
                }
                return slots;
            }
        }

        let (minuend, subtrahend) = if self.positive_slot_direction() {
            (negative, positive)
        } else {
            (positive, negative)
        };

        (slots + minuend).wrapping_sub(subtrahend) % slots
    }

    fn voltage_phasor_at(&self, zone: Zone, phasor_angle: f64, ordinal: f64) -> Complex<f64> {
        if self.positive_zone() == zone {
            return self.voltage_phasor_positive_zone(phasor_angle, ordinal);
        } else if self.negative_zone() == zone {
            return self.voltage_phasor_negative_zone(phasor_angle, ordinal);
        } else {
            return Complex::new(0.0, 0.0);
        }
    }

    fn invert(&mut self) {
        self.positive_slot_direction = !self.positive_slot_direction;
        let tmp = self.positive_zone;
        self.positive_zone = self.negative_zone;
        self.negative_zone = tmp;
    }
}

impl From<FullCoil> for Box<dyn Wire> {
    fn from(value: FullCoil) -> Self {
        value.wire
    }
}

// ================================================================

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HalfCoil {
    zone: Zone,
    is_positive: bool,
    turns: NonZeroUsize,
    phase: NonZeroU16,
    wire: Box<dyn Wire>,
}

impl HalfCoil {
    pub fn new(
        zone: Zone,
        is_positive: bool,
        turns: NonZeroUsize,
        phase: NonZeroU16,
        wire: Box<dyn Wire>,
    ) -> Self {
        Self {
            zone,
            is_positive,
            turns,
            phase,
            wire,
        }
    }

    pub fn zone(&self) -> Zone {
        return self.zone;
    }

    pub fn is_positive(&self) -> bool {
        self.is_positive
    }
}

impl private::Sealed for HalfCoil {}

impl CoilExt for HalfCoil {
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

    fn any_zone(&self) -> Zone {
        self.zone
    }

    fn voltage_phasor(&self, phasor_angle: f64, ordinal: f64) -> Complex<f64> {
        let slot_angle = self.zone().slot as f64 * phasor_angle * ordinal;
        let slot_angle = if self.is_positive {
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

    fn throw(&self, _slots: Option<NonZeroU16>) -> u16 {
        return 0;
    }

    fn voltage_phasor_at(&self, zone: Zone, phasor_angle: f64, ordinal: f64) -> Complex<f64> {
        if self.zone == zone {
            return self.voltage_phasor(phasor_angle, ordinal);
        } else {
            return Complex::new(0.0, 0.0);
        }
    }

    fn invert(&mut self) {
        self.is_positive = !self.is_positive;
    }
}

impl From<HalfCoil> for Box<dyn Wire> {
    fn from(value: HalfCoil) -> Self {
        value.wire
    }
}

// ================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneAndPolarity {
    pub zone: Zone,
    pub is_positive: bool,
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
                    return Some(ZoneAndPolarity {
                        zone: coil.positive_zone(),
                        is_positive: true,
                    });
                }
                2 => {
                    return Some(ZoneAndPolarity {
                        zone: coil.negative_zone(),
                        is_positive: false,
                    });
                }
                _ => return None,
            },
            Coil::Half(coil) => match self.counter {
                1 => {
                    return Some(ZoneAndPolarity {
                        zone: coil.zone(),
                        is_positive: coil.is_positive(),
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
