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

use std::{
    f64::consts::{PI, TAU},
    num::{NonZeroU16, NonZeroUsize},
};

use crate::error::Error;
use keyring_map::{InsertionError, KeyringMap};
use num::Complex;
use stem_types::Zone;
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
    /// let wire: Box<dyn Wire> = Box::new(SffWire::default());
    ///
    /// let coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(1, 0),
    ///     true,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     wire,
    /// ).expect("zones identical");
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
    /// let wire: Box<dyn Wire> = Box::new(SffWire::default());
    ///
    /// let coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(1, 0),
    ///     true,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     wire,
    /// ).expect("zones identical");
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
    /// let wire: Box<dyn Wire> = Box::new(SffWire::default());
    ///
    /// let coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(1, 0),
    ///     true,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     wire,
    /// ).expect("zones identical");
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
    /// let wire: Box<dyn Wire> = Box::new(SffWire::default());
    ///
    /// let coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(1, 0),
    ///     true,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     wire,
    /// ).expect("zones identical");
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
    /// let wire: Box<dyn Wire> = Box::new(SffWire::default());
    ///
    /// let coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(1, 0),
    ///     true,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     wire,
    /// ).expect("zones identical");
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
    /// let wire: Box<dyn Wire> = Box::new(SffWire::default());
    ///
    /// let coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(1, 0),
    ///     true,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     wire,
    /// ).expect("zones identical");
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
    /// let full_coil_wire: Box<dyn Wire> = Box::new(SffWire::default());
    /// let full_coil: Coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(1, 0),
    ///     true,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     full_coil_wire,
    /// ).expect("zones identical").into();
    ///
    /// let mut zones = full_coil.zones();
    /// assert_eq!(zones.next(), Some(Zone::new(0, 0)));
    /// assert_eq!(zones.next(), Some(Zone::new(1, 0)));
    /// assert_eq!(zones.next(), None);
    ///
    /// let half_coil_wire: Box<dyn Wire> = Box::new(SffWire::default());
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
    /// let full_coil_wire: Box<dyn Wire> = Box::new(SffWire::default());
    /// let full_coil: Coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(1, 0),
    ///     true,
    ///     NonZeroUsize::MIN,
    ///     NonZeroU16::MIN,
    ///     full_coil_wire,
    /// ).expect("zones identical").into();
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
    /// let half_coil_wire: Box<dyn Wire> = Box::new(SffWire::default());
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
    /// Returns the number of turns.
    ///
    /// As mentioned in the [module-level documentation](crate::coils), a coil
    /// is a series of continuous wire loops placed next to each other. The
    /// number of turns is the number of such loops.
    ///
    /// The magnetomotive force produced by a coil is proportional to the number
    /// of turns multiplied by the current. This product is called the
    /// ampere-turns. Hence, for the same coil geometry and magnetic conditions,
    /// a coil with 10 turns carrying 10 amperes produces the same
    /// magnetomotive force as a coil with 100 turns carrying 1 ampere.
    fn turns(&self) -> NonZeroUsize;

    /// Set a new number of turns for the coil. See [`CoilExt::turns`] for more.
    fn set_turns(&mut self, turns: NonZeroUsize);

    /// Inverts the direction of the coil. In particular, zones with a positive
    /// phase polarity become negative, and zones with a negative phase polarity
    /// become positive.
    ///
    /// See [`ZoneAndPolarity`] for details on zone polarity.
    fn invert(&mut self);

    /// Returns the phase of the coil.
    ///
    /// Phases are numbered starting at 1, with 1 representing phase A, 2
    /// representing phase B, and so on. For example, the phases of a
    /// three-phase motor are numbered 1, 2, and 3.
    ///
    /// Phase numbering starts at 1 rather than 0 because 0 in a
    /// [`WindingTable`](crate::winding_table::WindingTable) represents an
    /// empty zone. This also provides a convenient symmetry for
    /// representing phase polarity: a zone with positive polarity for phase
    /// A is represented by `1`, while a zone with negative polarity
    /// is represented by `-1`.
    fn phase(&self) -> NonZeroU16;

    /// Set a new phase for the coil. See [`CoilExt::phase`] for more.
    fn set_phase(&mut self, phase: NonZeroU16);

    /// Returns the voltage phasor induced in the coil by a magnetic field of
    /// the given electrical `order`, normalized to the voltage induced by
    /// one turn.
    ///
    /// A voltage phasor represents the magnitude and phase of the voltage
    /// induced in the coil by a sinusoidally varying magnetic field. Each
    /// zone of the coil contributes an individual voltage phasor. The total
    /// voltage phasor induced in the coil is the complex sum of the phasors
    /// of all its zones.
    ///
    /// For a coil with `N` turns and zones with electrical angles `αᵢ`, the
    /// returned phasor is
    ///
    /// `N · Σ exp(j · αᵢ)`
    ///
    /// where zones with negative polarity contribute an additional phase shift
    /// of `π`. Each individual zone phasor therefore has unit magnitude,
    /// while the magnitude and phase of the resulting complex number
    /// describe the combined voltage induced in the coil.
    ///
    /// The `order` specifies the spatial order of the sinusoidal magnetic field
    /// in electrical coordinates, i.e. the number of sinusoidal periods per
    /// pole pair. The electrical angle of a zone is calculated as
    ///
    /// `α = order · 2π · pole_pairs / slots`
    ///
    /// where `slots` is the number of slots and `pole_pairs` is the number of
    /// pole pairs.
    ///
    /// # Examples
    ///
    /// ```
    /// use approxim::assert_abs_diff_eq;
    /// use std::num::{NonZeroU16, NonZeroUsize};
    /// use stem_winding::prelude::*;
    ///
    /// let wire: Box<dyn Wire> = Box::new(SffWire::default());
    /// let mut coil: Coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(3, 0),
    ///     true,
    ///     NonZeroUsize::new(10).expect("not zero"),
    ///     NonZeroU16::MIN,
    ///     wire,
    /// ).expect("zones identical").into();
    ///
    /// let slots = NonZeroU16::new(6).expect("not zero");
    /// let pole_pairs = NonZeroU16::new(1).expect("not zero");
    ///
    /// let phasor = coil.voltage_phasor(slots, pole_pairs, 1.0);
    /// approxim::assert_abs_diff_eq!(phasor.re, 20.0, epsilon = 1e-6);
    /// approxim::assert_abs_diff_eq!(phasor.im, 0.0, epsilon = 1e-6);
    ///
    /// coil.set_turns(NonZeroUsize::new(5).expect("not zero"));
    /// let phasor = coil.voltage_phasor(slots, pole_pairs, 1.0);
    /// approxim::assert_abs_diff_eq!(phasor.re, 10.0, epsilon = 1e-6);
    /// approxim::assert_abs_diff_eq!(phasor.im, 0.0, epsilon = 1e-6);
    /// ```
    fn voltage_phasor(&self, slots: NonZeroU16, pole_pairs: NonZeroU16, order: f64)
    -> Complex<f64>;

    /// Returns the voltage phasor induced in the coil by the specified `zone`.
    ///
    /// Unlike [`CoilExt::voltage_phasor`], which returns the combined voltage
    /// phasor induced by all zones of the coil, this method returns only the
    /// contribution of the specified zone. If the zone is not occupied by the
    /// coil, the returned phasor is `0 + 0j`. For an occupied zone, the
    /// magnitude of the returned phasor equals the number of turns of the
    /// coil. Its angle is determined by the electrical position of the zone
    /// and its polarity. See [`CoilExt::voltage_phasor`] for details.
    ///
    /// # Examples
    ///
    /// ```
    /// use approxim::assert_abs_diff_eq;
    /// use std::num::{NonZeroU16, NonZeroUsize};
    /// use stem_winding::prelude::*;
    ///
    /// let wire: Box<dyn Wire> = Box::new(SffWire::default());
    /// let mut coil: Coil = FullCoil::new(
    ///     Zone::new(0, 0),
    ///     Zone::new(3, 0),
    ///     true,
    ///     NonZeroUsize::new(10).expect("not zero"),
    ///     NonZeroU16::MIN,
    ///     wire,
    /// ).expect("zones identical").into();
    ///
    /// let slots = NonZeroU16::new(6).expect("not zero");
    /// let pole_pairs = NonZeroU16::new(1).expect("not zero");
    ///
    /// let phasor = coil.voltage_phasor_at(Zone::new(0, 0), slots, pole_pairs, 1.0);
    /// approxim::assert_abs_diff_eq!(phasor.re, 10.0, epsilon = 1e-6);
    /// approxim::assert_abs_diff_eq!(phasor.im, 0.0, epsilon = 1e-6);
    ///
    /// let phasor = coil.voltage_phasor_at(Zone::new(1, 0), slots, pole_pairs, 1.0);
    /// approxim::assert_abs_diff_eq!(phasor.re, 0.0, epsilon = 1e-6);
    /// approxim::assert_abs_diff_eq!(phasor.im, 0.0, epsilon = 1e-6);
    /// ```
    fn voltage_phasor_at(
        &self,
        zone: Zone,
        slots: NonZeroU16,
        pole_pairs: NonZeroU16,
        order: f64,
    ) -> Complex<f64>;

    /// Returns an arbitrary, but not random, [`Zone`] occupied by the coil.
    ///
    /// The returned [`Zone`] is guaranteed to be the same for every call on the
    /// same coil, but which [`Zone`] is returned is unspecified and may depend
    /// on the implementation.
    ///
    /// Since each [`Zone`] can be occupied by at most one coil, any zone of a
    /// coil can be used to uniquely identify it within a [`Coils`] container.
    fn any_zone(&self) -> Zone;

    /// Returns a reference to the underlying [`Wire`] trait object.
    fn wire(&self) -> &dyn Wire;

    /// Sets a new wire for the coil. See [`CoilExt::wire`] for more.
    fn set_wire(&mut self, wire: Box<dyn Wire>);

    /// Returns the underlying [`Wire`] trait object, consuming the coil.
    fn into_wire(self) -> Box<dyn Wire>;

    /**
    Returns the coil throw in slot pitches.

    The throw is the number of slot pitches between the two ends of a coil,
    measured along the slot direction. For a [`FullCoil`], this is the number of
    teeth / slot separators crossed when moving from the positive zone to the
    negative zone. For a [`HalfCoil`], the throw is always 0.

    In the following example, the throw of coil (a) is 2, that of coil (b) is 0,
    and that of coil (c) is 1.

    ```text
    slot | 0 | 1 | 2 | 3 | 4 | 5

           ┌───────┐       ┌───┐
           │       │   │   │   │
    coil   ▲       ▼   ▼   ▼   ▲
           │       │   │   │   |
           └───────┘       └───┘
              (a)     (b)   (c)
    ```

    A cyclic winding mounted on a [`RotCore`](stem_core::prelude::RotCore) can
    have coils that wrap around the end of the slot sequence. In this case,
    `slots` must be provided so that the throw can be calculated modulo the
    number of slots.

    If `slots` is `None`, a [`LinCore`](stem_core::prelude::LinCore) is assumed,
    and the slot indices are treated as non-cyclic. This is only relevant for a
    [`FullCoil`], as the throw of a [`HalfCoil`] is 0 by definition.

    For a [`FullCoil`], [`FullCoil::positive_slot_direction`] determines whether
    the slot index increases or decreases when moving from the
    [`FullCoil::positive_zone`] to the [`FullCoil::negative_zone`].

    For a rotary core, a decreasing slot index can wrap around from slot 0 to the
    last slot. For a linear core, wrapping is not possible, so the throw is
    calculated without wrapping.

    **Rotary core (`slots` given)**
    ```text
    slot | 0 | 1 | 2 | 3 | 4 | 5

        ──┐                   ┌──
          │                   │
    coil  ▲                   ▼
          │                   │
        ──┘                   └──
    ```
    Resulting throw is 1.

    **Linear core (`slots` not given)**
    ```text
    slot | 0 | 1 | 2 | 3 | 4 | 5

          ┌───────────────────┐
          │                   │
    coil  ▲                   ▼
          │                   │
          └───────────────────┘
    ```
    Even though the slot index should decrease, this is not possible because
    a linear core does not wrap around. Hence, the throw is 5.

    # Examples

    ```
    use std::num::{NonZeroU16, NonZeroUsize};
    use stem_winding::prelude::*;

    let wire: Box<dyn Wire> = Box::new(SffWire::default());
    let coil = FullCoil::new(
        Zone::new(0, 0),
        Zone::new(5, 0),
        false, // parameter positive_slot_direction
        NonZeroUsize::new(10).expect("not zero"),
        NonZeroU16::MIN,
        wire,
    )
    .expect("zones identical");

    // Rotary core with 6 slots
    assert_eq!(coil.throw(Some(NonZeroU16::new(6).expect("not zero"))), 1);

    // Linear core with 6 slots
    assert_eq!(coil.throw(None), 5);
    ```
    */
    fn throw(&self, slots: Option<NonZeroU16>) -> u16;

    /// Returns the resistance of the coil.
    ///
    /// The resistance of the wire forming a single turn is calculated using
    /// [`Wire::resistance`] with the given `zone_area`, `turn_length`, the
    /// number of turns, and `conditions`. The resulting resistance is then
    /// multiplied by the number of turns because the turns of a coil are
    /// connected in series.
    ///
    /// `zone_area` is the cross-sectional area available to the wire in each
    /// zone, `turn_length` is the length of one turn, and `conditions`
    /// specifies the physical conditions used to calculate the wire
    /// resistance. See [`Wire::resistance`] for details.
    fn resistance(
        &self,
        zone_area: Area,
        turn_length: Length,
        conditions: &[DynQuantity<f64>],
    ) -> ElectricalResistance {
        self.wire()
            .resistance(turn_length, zone_area, self.turns(), conditions)
            * self.turns().get() as f64
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

    fn voltage_phasor(
        &self,
        slots: NonZeroU16,
        pole_pairs: NonZeroU16,
        order: f64,
    ) -> Complex<f64> {
        match self {
            Coil::Full(coil) => coil.voltage_phasor(slots, pole_pairs, order),
            Coil::Half(coil) => coil.voltage_phasor(slots, pole_pairs, order),
        }
    }

    fn voltage_phasor_at(
        &self,
        zone: Zone,
        slots: NonZeroU16,
        pole_pairs: NonZeroU16,
        order: f64,
    ) -> Complex<f64> {
        match self {
            Coil::Full(coil) => coil.voltage_phasor_at(zone, slots, pole_pairs, order),
            Coil::Half(coil) => coil.voltage_phasor_at(zone, slots, pole_pairs, order),
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

/**
A coil with a positive and a negative zone.

This struct represents a "normal" coil where both the outgoing and the return
conductor are part of the field-creating winding, forming a closed loop. The
coil position is defined by its [`positive_zone`](FullCoil::positive_zone) and
[`negative_zone`](FullCoil::negative_zone), which cannot be identical. This
invariant is checked when building a [`FullCoil`] from [`FullCoil::new`] or by
deserializing it.

If the coil is wound onto a [`LinCore`](stem_core::prelude::LinCore), there is
only one way how to close the coil between the two zones. When wound onto a
[`RotCore`](stem_core::prelude::RotCore) however, there are two possibilities,
as shown in the following examples. The positive zone is in slot 0, the negative
zone in slot 1.

a) Slot indices increase along the end winding part of the coil.
```text
slot | 0 | 1 | 2 | 3 | 4 | 5

       ┌───────────────────┐
       │                   │
coil   ▲                   ▼
       │                   │
       └───────────────────┘
```

b) Slot indices decreases (and in this case, wraps around) along the end winding
part of the coil.
```text
slot | 0 | 1 | 2 | 3 | 4 | 5

     ──┐                   ┌──
       │                   │
coil   ▲                   ▼
       │                   │
     ──┘                   └──
```
To make the coil geometry unambiguous,
[`positive_slot_direction`](FullCoil::positive_slot_direction) sets the end
winding part to case a) if true and to case b) if false.

When interpreting the coil to form a
[`WindingTable`](crate::winding_table::WindingTable), the positive zone
represents `x` and the negative zone `-x`, where `x` is [`FullCoil::phase`].
 */
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct FullCoil {
    positive_zone: Zone,
    negative_zone: Zone,
    positive_slot_direction: bool,
    turns: NonZeroUsize,
    phase: NonZeroU16,
    wire: Box<dyn Wire>,
}

impl FullCoil {
    /**
    Creates a new [`FullCoil`] instance from its components.

    This constructor returns an error if `positive_zone == negative_zone`. For
    a detailed description of the parameters, see the documentation of
    [`FullCoil`] and [`CoilExt`].
     */
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

    /// Returns if the slot indices increase when moving along the coil end
    /// winding from the [`positive_zone`](FullCoil::positive_zone) to the
    /// [`negative_zone`](FullCoil::negative_zone) if the coil is mounted on a
    /// rotary core. Has no effect if the coil is mounted on a linear core.
    ///
    /// See the [struct](FullCoil) documentation for details.
    pub fn positive_slot_direction(&self) -> bool {
        self.positive_slot_direction
    }

    /// Sets a new slot direction for `self`.
    ///
    /// See the [struct](FullCoil) documentation for details.
    pub fn set_positive_slot_direction(&mut self, direction: bool) {
        self.positive_slot_direction = direction;
    }

    /// Returns the positive zone of `self`.
    pub fn positive_zone(&self) -> Zone {
        self.positive_zone
    }

    /// Returns the negative zone of `self`.
    pub fn negative_zone(&self) -> Zone {
        self.negative_zone
    }

    /// Returns the voltage phasor for the
    /// [`positive_zone`](FullCoil::positive_zone) of `self`.
    ///
    /// Conceptually, this method uses [`CoilExt::voltage_phasor_at`] with the
    /// zone being the [`positive_zone`](FullCoil::positive_zone) of `self`. See
    /// the docstring of [`CoilExt::voltage_phasor_at`] for details.
    pub fn voltage_phasor_positive_zone(
        &self,
        slots: NonZeroU16,
        pole_pairs: NonZeroU16,
        order: f64,
    ) -> Complex<f64> {
        let slot_angle = f64::from(self.positive_zone().slot) / f64::from(slots.get())
            * f64::from(pole_pairs.get())
            * order
            * TAU;
        return Complex::from_polar(self.turns().get() as f64, slot_angle);
    }

    /// Returns the voltage phasor for the
    /// [`negative_zone`](FullCoil::negative_zone) of `self`.
    ///
    /// Conceptually, this method uses [`CoilExt::voltage_phasor_at`] with the
    /// zone being the [`negative_zone`](FullCoil::negative_zone) of `self`. See
    /// the docstring of [`CoilExt::voltage_phasor_at`] for details.
    pub fn voltage_phasor_negative_zone(
        &self,
        slots: NonZeroU16,
        pole_pairs: NonZeroU16,
        order: f64,
    ) -> Complex<f64> {
        let slot_angle = f64::from(self.negative_zone().slot) / f64::from(slots.get())
            * f64::from(pole_pairs.get())
            * order
            * TAU
            + PI;
        return Complex::from_polar(self.turns().get() as f64, slot_angle);
    }

    /**
    Returns an iterator over all the slots "covered" by the end winding,
    including the slots of [`positive_zone`](FullCoil::positive_zone) and
    [`negative_zone`](FullCoil::negative_zone).

    As with [`CoilExt::throw`], if `slots` is given, the coil is assumed to be
    mounted on a rotary core and can potentially "wrap around". If not given,
    a linear core is assumed. See the [struct documentation](FullCoil) for
    details.

    # Examples

    ```
    use std::num::{NonZeroU16, NonZeroUsize};
    use stem_winding::prelude::*;

    let mut coil = FullCoil::new(
        Zone::new(0, 0),
        Zone::new(5, 0),
        false, // parameter positive_slot_direction
        NonZeroUsize::new(10).expect("not zero"),
        NonZeroU16::MIN,
        Box::new(SffWire::default()),
    )
    .expect("zones identical");

    // Linear core
    let covered: Vec<_> = coil.covered_slots(None).collect();
    assert_eq!(covered, vec![0, 1, 2, 3, 4, 5]);

    // Rotary core
    let covered: Vec<_> = coil.covered_slots(Some(NonZeroU16::new(6).expect("not zero"))).collect();
    assert_eq!(covered, vec![0, 5]);

    // Change direction of the end winding
    coil.set_positive_slot_direction(true);
    let covered: Vec<_> = coil.covered_slots(Some(NonZeroU16::new(6).expect("not zero"))).collect();
    assert_eq!(covered, vec![0, 1, 2, 3, 4, 5]);
    ```
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

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for FullCoil {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct FullCoilDe {
            positive_zone: Zone,
            negative_zone: Zone,
            positive_slot_direction: bool,
            turns: NonZeroUsize,
            phase: NonZeroU16,
            wire: Box<dyn Wire>,
        }

        let coil = FullCoilDe::deserialize(deserializer)?;

        FullCoil::new(
            coil.positive_zone,
            coil.negative_zone,
            coil.positive_slot_direction,
            coil.turns,
            coil.phase,
            coil.wire,
        )
        .map_err(serde::de::Error::custom)
    }
}

/// An iterator over the slots covered by the end winding of a [`FullCoil`]. Is
/// created via the [`FullCoil::covered_slots`] method, see its docstring for
/// more.
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

    fn voltage_phasor(
        &self,
        slots: NonZeroU16,
        pole_pairs: NonZeroU16,
        order: f64,
    ) -> Complex<f64> {
        return self.voltage_phasor_positive_zone(slots, pole_pairs, order)
            + self.voltage_phasor_negative_zone(slots, pole_pairs, order);
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

    fn voltage_phasor_at(
        &self,
        zone: Zone,
        slots: NonZeroU16,
        pole_pairs: NonZeroU16,
        order: f64,
    ) -> Complex<f64> {
        if self.positive_zone() == zone {
            return self.voltage_phasor_positive_zone(slots, pole_pairs, order);
        } else if self.negative_zone() == zone {
            return self.voltage_phasor_negative_zone(slots, pole_pairs, order);
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

/**
A coil with a single zone / conductor direction.

This coil represents an outgoing or a returning conductor where its counterpart
does not exist or does not participate in the magnetic field creation. An
example for the former would the squirrel cage of an induction motor: Each bar
forms a "half coil" where there is no corresponding zone which transports the
same current back. The latter case would be a yoke winding, where the coil
actually forms a full loop, but only one of its sides is at the air gap, whereas
the other one is on the outside of the yoke and the induced voltage / created
magnetic field can be neglected.
 */
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HalfCoil {
    /// The zone occupied by the coil.
    pub zone: Zone,
    /// The polarity of the zone. See [`ZoneAndPolarity`] for details.
    pub is_positive: bool,
    /// The number of turns. For the aforementioned case of the squirrel cage
    /// bar, this is 1.
    pub turns: NonZeroUsize,
    /// The phase of the coil.
    pub phase: NonZeroU16,
    /// The underlying wire.
    pub wire: Box<dyn Wire>,
}

impl HalfCoil {
    /// Returns a new [`HalfCoil`] from its components.
    ///
    /// This is a convenience wrapper over the struct constructor (all fields of
    /// [`HalfCoil`] are public).
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

    /// Returns the zone occupied by `self`.
    pub fn zone(&self) -> Zone {
        return self.zone;
    }

    /// Returns whether `self` has positive polarity. See [`ZoneAndPolarity`]
    /// for details.
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

    fn voltage_phasor(
        &self,
        slots: NonZeroU16,
        pole_pairs: NonZeroU16,
        order: f64,
    ) -> Complex<f64> {
        let slot_angle = f64::from(self.zone().slot) / f64::from(slots.get())
            * f64::from(pole_pairs.get())
            * order
            * TAU;
        let slot_angle = if self.is_positive {
            slot_angle
        } else {
            -slot_angle
        };
        return Complex::from_polar(self.turns().get() as f64, slot_angle);
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

    fn voltage_phasor_at(
        &self,
        zone: Zone,
        slots: NonZeroU16,
        pole_pairs: NonZeroU16,
        order: f64,
    ) -> Complex<f64> {
        if self.zone == zone {
            return self.voltage_phasor(slots, pole_pairs, order);
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

/**
An item returned by the [`ZoneAndPolarityIterator`] which encodes the zone and
its polarity.

If a zone is occupied by a coil, it has a polarity which describes the direction
the current goes through. When looking at the cross section of a core like in
the left image below, the current going towards the observer (circle with a
central dot) produces a counter-clockwise magnetic field and is defined to be
positive. In the neighboring slot, the current is flowing away from the observer
and produces a clockwise magnetic field, this is defined as a negative polarity.

The right image shows the same coil, but seen from the air gap. In the zone with
positive polarity, the current is flowing up; in the zone with negative polarity
it is flowing down.
 */
#[doc = ""]
#[cfg_attr(feature = "doc-images", doc = "![Coil polarity][coil_polarity]")]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("coil_polarity", "docs/img/coil_polarity.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**
This struct is created by the [`ZoneAndPolarityIterator`] iterator, which
returns the zones of a coil together with their polarity. The used definition
is completely arbitrary, but consistent throughout the stem ecosystem.
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneAndPolarity {
    /// The zone in question.
    pub zone: Zone,
    /// The polarity of [`ZoneAndPolarity::zone`], i.e. if the zone is positive
    /// or negative.
    pub is_positive: bool,
}

/// The iterator created from [`Coil::zones_and_polarities`]. See the method
/// documentation for details.
pub struct ZoneAndPolarityIterator<'a> {
    coil: &'a Coil,
    counter: usize,
}

impl<'a> ZoneAndPolarityIterator<'a> {
    /// Returns a new [`ZoneAndPolarityIterator`] for the given coil.
    pub fn new(coil: &'a Coil) -> Self {
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

/// The iterator created from [`Coil::zones`]. See the method documentation for
/// details.
pub struct ZoneIterator<'a>(ZoneAndPolarityIterator<'a>);

impl<'a> ZoneIterator<'a> {
    /// Returns a new [`ZoneIterator`] for the given coil.
    pub fn new(coil: &'a Coil) -> Self {
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
