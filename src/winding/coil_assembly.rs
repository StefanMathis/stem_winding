use std::num::NonZeroU16;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use stem_types::{CoilLayout, Zone};

#[cfg(feature = "stem_core")]
use stem_core::prelude::*;

use crate::{
    coils::{Coil, CoilExt, Coils},
    error::Error,
    winding::{Connection, Winding},
};

/**
A winding represented as a collection of individually connected coils.

This winding type defines a winding as a collection of connected individual
coils, giving fine-grained control of all aspects (e.g. defining a different
number of turns for each coil) at the cost of increased complexity. All other
winding types can be interpreted as simplified versions of this winding type,
which is why a [`CoilAssembly`] can be derived from any other winding via its
[`From`] implementation.

Although `CoilAssembly` allows arbitrary coil configurations, each coil must
still satisfy the invariants enforced by [`CoilAssembly::insert`] (for example,
no two coils may occupy the same zone). This ensures that the resulting assembly
remains valid.

A concrete use case for this winding type is the evaluation of asymmetries:
Given a [`ToothCoilWinding`](crate::winding::ToothCoilWinding), we want to
evaluate the effect of a mistake made during the winding process, where one coil
is missing one turn. This breaks the spatial symmetry of the winding, resulting
in additional spatial harmonics of the created field:

```
use std::num::NonZeroUsize;

use stem_winding::prelude::*;
use stem_winding::winding::WindingHarmonic;

// The ideal, symmetric tooth coil winding without defects.
let mut sym_wdg = ToothCoilWinding::new(ToothCoilMinimalBuilder {
    slots: 12.try_into().expect("not zero"),
    pole_pairs: 5.try_into().expect("not zero"),
    phases: 3.try_into().expect("not zero"),
    layers: 2.try_into().expect("not zero"),
    winding_table_constructor: WindingTableConstructor::Tingley,
}).expect("valid winding topology");
sym_wdg.set_turns_per_coil(30.try_into().expect("not zero"));

// Create a CoilAssembly from it
let mut asym_wdg = CoilAssembly::from(&sym_wdg);

// Introduce the defect in one of the coils
if let Some(coil) = asym_wdg.coil_at_mut(Zone::new(0, 0)) {
   coil.set_turns(NonZeroUsize::new(coil.turns().get() - 1).expect("not zero"));
}

// Spatial orders produced by the symmetric winding
let mut sym_iter = sym_wdg.harmonics();
assert_eq!(sym_iter.next(),
    Some(
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(1),
            is_positive: true
        }));
assert_eq!(sym_iter.next(),
    Some(
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(5),
            is_positive: false
        }));
assert_eq!(sym_iter.next(),
    Some(
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(7),
            is_positive: true
        }));

// Spatial orders produced by the winding with the defect
let mut asym_iter = asym_wdg.harmonics();
assert_eq!(asym_iter.next(),
    Some(
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(1),
            is_positive: true
        }));
assert_eq!(asym_iter.next(),
    Some(
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(2), // Didn't exist for the original winding!
            is_positive: false
        }));
assert_eq!(asym_iter.next(),
    Some(
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(3), // Didn't exist for the original winding!
            is_positive: true
        }));
```

Note that [`CoilAssembly`] is a low-level winding representation and does not
provide specialized implementations of certain [`Winding`] methods, for example
[`Winding::end_winding_leakage_inductance`] or
[`Winding::end_winding_half_turn_length`]. These methods therefore use the
default implementations, which may not account for winding-specific
characteristics captured by other winding types. Consequently, the calculated
values may differ from those obtained using the original winding, even if the
coil configuration has not been modified.

# Constructors

A [`CoilAssembly`] can be derived directly from another [`Winding`], as shown
above, created using a [`CoilAssemblyBuilder`], or assembled coil by coil using
[`CoilAssembly::new`] and [`CoilAssembly::insert`]. When using the builder, each
coil is inserted individually into the resulting assembly to ensure that it
satisfies the invariants documented for [`CoilAssembly::insert`].

# Serialization and deserialization

If the `serde` feature is enabled, this winding can be deserialized from a
serialized [`CoilAssemblyBuilder`]. The serialized representation of this
winding corresponds to that of a [`CoilAssemblyBuilder`].

# Examples

This example shows how to build a [`CoilAssembly`] coil by coil and also how to
remove existing coils using [`CoilAssembly::remove`].

```
use std::num::NonZeroU16;

use stem_winding::prelude::*;

// Build the coils
let mut winding = CoilAssembly::new(
    6.try_into().expect("not zero"),
    1.try_into().expect("not zero"),
    3.try_into().expect("not zero"),
    CoilLayout::DoubleVertical,
    Default::default(),
    Vec::new(),
    Connection::Star,
).expect("valid");

winding.insert(
    FullCoil::new(
        Zone { slot: 0, layer: 0 },
        Zone { slot: 3, layer: 1 },
        true,
        10.try_into().expect("not zero"), // Number of turns
        1.try_into().expect("not zero"), // Phase
        Box::new(SffWire::default()),
    )
    .unwrap(),
).expect("valid");
winding.insert(
    FullCoil::new(
        Zone { slot: 2, layer: 0 },
        Zone { slot: 5, layer: 1 },
        true,
        12.try_into().expect("not zero"), // Number of turns
        2.try_into().expect("not zero"), // Phase
        Box::new(SffWire::default()),
    )
    .unwrap(),
).expect("valid");

// The three phases of
let ph1 = NonZeroU16::new(1).expect("not zero");
let ph2 = NonZeroU16::new(2).expect("not zero");
let ph3 = NonZeroU16::new(3).expect("not zero");

assert_eq!(winding.series_turns_per_phase(ph1), num::rational::Ratio::new(10, 1));
assert_eq!(winding.series_turns_per_phase(ph2), num::rational::Ratio::new(12, 1));

// No coil is assigned to phase 3
assert_eq!(winding.series_turns_per_phase(ph3), num::rational::Ratio::new(0, 1));

// Remove the coil belonging to phase 1.
winding.remove(Zone::new(0, 0));
assert_eq!(winding.series_turns_per_phase(ph1), num::rational::Ratio::new(0, 1));
```
*/
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "CoilAssemblyBuilder"))]
pub struct CoilAssembly {
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    phases: NonZeroU16,
    coil_layout: CoilLayout,
    coils: Coils,
    parallel_paths: Vec<NonZeroU16>,
    connection: Connection,
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
pub struct CoilAssemblyBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub coil_layout: CoilLayout,
    pub coils: Coils,
    /// Entry per phase: 0th item == Phase 1, 1st item == Phase 2, ...
    #[cfg_attr(feature = "serde", serde(default))]
    pub parallel_paths: Vec<NonZeroU16>,
    #[cfg_attr(feature = "serde", serde(default = "connection_default"))]
    pub connection: Connection,
}

#[cfg(feature = "serde")]
fn connection_default() -> Connection {
    Connection::Star
}

impl TryFrom<CoilAssemblyBuilder> for CoilAssembly {
    type Error = Error;

    fn try_from(builder: CoilAssemblyBuilder) -> Result<Self, Self::Error> {
        let mut winding = CoilAssembly {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            coils: Default::default(),
            coil_layout: builder.coil_layout,
            parallel_paths: builder.parallel_paths,
            connection: builder.connection,
        };

        for coil in builder.coils.into_iter() {
            winding.insert(coil)?;
        }

        return Ok(winding);
    }
}

impl CoilAssembly {
    pub fn new(
        slots: NonZeroU16,
        pole_pairs: NonZeroU16,
        phases: NonZeroU16,
        coil_layout: CoilLayout, // Defines layers
        coils: Coils,
        parallel_paths: Vec<NonZeroU16>,
        connection: Connection,
    ) -> Result<Self, Error> {
        let mut winding = CoilAssembly {
            slots,
            pole_pairs,
            phases,
            coils: Default::default(),
            coil_layout,
            parallel_paths,
            connection,
        };

        // Insert the coils one after another and see if there are conflicts
        for coil in coils.into_iter() {
            winding.insert(coil)?;
        }
        return Ok(winding);
    }

    pub fn new_minimal(
        slots: NonZeroU16,
        pole_pairs: NonZeroU16,
        phases: NonZeroU16,
        coil_layout: CoilLayout,
        coils: Coils,
    ) -> Result<Self, Error> {
        return Self::new(
            slots,
            pole_pairs,
            phases,
            coil_layout,
            coils,
            Vec::new(),
            Connection::Star,
        );
    }

    // Try to insert a coil
    pub fn insert<C: Into<Coil>>(&mut self, coil: C) -> Result<(), Error> {
        let coil: Coil = coil.into();

        let coil_phase = coil.phase().get();
        let winding_phases = self.phases().get();
        let winding_slots = self.slots().get();
        let winding_layers = self.layers().get();
        compare_variables::compare_variables!(coil_phase <= winding_phases)?;

        for zone in coil.zones() {
            compare_variables::compare_variables!(zone.slot < winding_slots)?;
            compare_variables::compare_variables!(zone.layer < winding_layers)?;
        }

        return self.coils.insert(coil).map_err(Error::from);
    }

    /// Try to remove the coil occupying the given zone
    pub fn remove(&mut self, zone: Zone) -> Option<Coil> {
        return self.coils.remove(zone);
    }

    /// Remove all coils from the coil assembly
    pub fn clear_coils(&mut self) {
        self.coils.clear();
    }

    pub fn set_pole_pairs(&mut self, pole_pairs: NonZeroU16) {
        self.pole_pairs = pole_pairs;
    }

    /// Like [`Winding::coil_at`], but returns a mutable reference to the coil.
    pub fn coil_at_mut(&mut self, zone: Zone) -> Option<&mut Coil> {
        self.coils.get_mut(zone)
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Winding for CoilAssembly {
    fn phases(&self) -> NonZeroU16 {
        self.phases
    }

    fn slots(&self) -> NonZeroU16 {
        self.slots
    }

    fn pole_pairs(&self) -> NonZeroU16 {
        self.pole_pairs
    }

    fn layers(&self) -> NonZeroU16 {
        self.coil_layout().layers()
    }

    fn coil_at(&self, zone: Zone) -> Option<&Coil> {
        return self.coils.get(zone);
    }

    fn coil_layout(&self) -> CoilLayout {
        self.coil_layout
    }

    fn parallel_paths(&self, phase: NonZeroU16) -> NonZeroU16 {
        self.parallel_paths
            .get(usize::from(phase.get() - 1))
            .cloned()
            .unwrap_or(NonZeroU16::MIN)
    }

    fn connection(&self) -> Connection {
        self.connection
    }

    fn as_dyn(&self) -> &dyn Winding {
        self
    }
}

impl<W: Winding + ?Sized> From<&W> for CoilAssembly {
    fn from(winding: &W) -> Self {
        // Build the hashmap
        let mut coils = Coils::with_capacity(winding.num_coils(), winding.num_coils());
        for coil in winding.coils_iter() {
            coils
                .insert(coil.clone())
                .expect("two coils occupy the same zone. This is a bug.")
        }
        let parallel_paths = (1..winding.phases().get() + 1)
            .map(|phase| winding.parallel_paths(NonZeroU16::new(phase).unwrap_or(NonZeroU16::MIN)))
            .collect();

        return CoilAssembly {
            slots: winding.slots(),
            pole_pairs: winding.pole_pairs(),
            phases: winding.phases(),
            coils,
            coil_layout: winding.coil_layout(),
            parallel_paths,
            connection: winding.connection(),
        };
    }
}
