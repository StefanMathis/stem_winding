[serialization and deserialization]: https://stefanmathis.github.io/stem_book/serialization_and_deserialization.html
[stem book]: https://stefanmathis.github.io/stem_book/
[stem_core]: https://crates.io/crates/stem_core

> **Feedback welcome!**  
> Found a bug, missing docs, or have a feature request?  
> Please open an issue on [GitHub](https://github.com/StefanMathis/stem_winding.git).

![Integer-slot distributed winding on a rotary core][phd_stator.svg]

This crate provides tools for defining and analyzing AC windings of electrical
machines. Its functionality is built around the [`Winding`] trait, which
provides a common interface for describing winding topologies and evaluating
their properties.

The crate offers three levels of functionality:

- **Machine theory analysis:** Define winding topologies using [`Coil`]s,
  generate [`WindingTable`]s, and evaluate properties such as winding factors,
  harmonic content, air-gap leakage factors, and cogging and ripple torque
  orders.
- **Physical properties** (requires the `stem_core` feature): Calculate
  quantities such as phase resistance and inductance, and determine the coil
  geometry within a machine core.
- **Visualization** (requires the `cairo` feature): Visualize winding tables and
  coil layouts within machine geometry.

# Machine theory analysis

The following example defines a 12-slot, 10-pole, three-phase double-layer
tooth-coil winding using the predefined [`ToothCoilWinding`]. The corresponding
winding table is shown below. It was generated using this crate with the
`stem_core` and `cairo` features enabled.

![12-slot, 10-pole double-layer tooth-coil winding table][table_tooth_coil_winding.svg]

```rust
use std::num::NonZeroU16;
use approxim::assert_abs_diff_eq;

use stem_winding::prelude::*;

// The crate offers a bunch of winding table construction algorithms. For this
// particular one, we use the Tingley algorithm, which is suitable for
// three-phase windings with one or two layers. For other topologies, it might
// be necessary to use other construction algorithms.
let winding_table_constructor = WindingTableConstructor::Tingley;

// Since we are only interested in a theoretical analysis, some data such as the
// used wire is irrelevant here. Hence, we use a builder struct which fills in
// the missing information with sensible defaults.
let winding = ToothCoilWinding::new(ToothCoilMinimalBuilder {
    slots: NonZeroU16::new(12).expect("not zero"),
    pole_pairs: NonZeroU16::new(5).expect("not zero"),
    phases: NonZeroU16::new(3).expect("not zero"),
    layers: NonZeroU16::new(2).expect("not zero"),
    winding_table_constructor,
}).expect("given values create a symmetric, valid winding");

// Investigate the spatial orders of the field created by this winding. This
// method returns an infinite iterator, starting at the lowest absolute order.
let mut harmonics = winding.harmonics();

// First subharmonic (one sinusoidal wave over the entire air gap)
assert_eq!(harmonics.next().unwrap().spatial_order, SpatialOrder::Mechanical(1));

// Fundamental harmonic (one sinusoidal wave per pole pair).
// Its interaction with the corresponding rotor field contributes to the average torque.
assert_eq!(harmonics.next().unwrap().spatial_order, SpatialOrder::Mechanical(5));

// Calculate the winding factors of phase 1 for these harmonics
let phase = NonZeroU16::new(1).expect("not zero");
assert_abs_diff_eq!(winding.winding_factor(phase, SpatialOrder::Mechanical(1)), 0.066, epsilon = 1e-3);
assert_abs_diff_eq!(winding.winding_factor(phase, SpatialOrder::Mechanical(5)), 0.933, epsilon = 1e-3);

// Calculate the air-gap leakage factor, which characterizes the portion of
// the air-gap field that does not contribute to useful torque. A low value
// is generally desirable.
assert_abs_diff_eq!(winding.air_gap_leakage_factor(phase), 0.968, epsilon = 1e-3);

// Determine the lowest spatial order for the cogging and the ripple torque.
// Higher values are generally desirable, as they tend to result in smoother torque.
assert_eq!(winding.lowest_cogging_torque_order(), 60);
assert_eq!(winding.lowest_ripple_torque_order(), 30);
```

All methods demonstrated above are provided by the [`Winding`] trait and are
therefore available for any type implementing it.

# Physical properties

When the `stem_core` feature is enabled, this crate integrates with
[stem_core] to account for the geometry of the machine core when calculating
physical quantities such as phase resistance, inductance, or coil mass. These
calculations provide the physical parameters needed for machine simulations.

The following example demonstrates this for a 36-slot, four-pole, single-layer
distributed winding placed in a rotary core as shown in the opening
illustration.

```rust
use std::str::FromStr;
use std::sync::Arc;
use std::num::{NonZeroU16, NonZeroUsize};
use approxim::assert_abs_diff_eq;

use stem_winding::prelude::*;
use stem_winding::var_quantity::unary::FirstOrderTaylor;

// Define a wire with an electrical resistivity specified at 20 °C and a
// linear temperature coefficient of 0.4 %/K.
let wire = {
    let mut material = Material::default();
    material.electrical_resistivity = VarQuantity::new(
        FirstOrderTaylor::new(
            DynQuantity::from_str("1 / 56 m/MS").expect("parseable"),
            DynQuantity::from_str("0.4 % / K").expect("parseable"),
            DynQuantity::from_str("20.0 °C").expect("parseable"),
        )
        .expect("units match")
    ).expect("units match");
    Arc::new(SffWire::new( Arc::new(material), 0.4, 0.5).expect("valid inputs"))
};

// Instead of using the new method from the previous examples, all winding
// builders also implement TryInto for a direct conversion.
let winding: DistributedWinding = DistributedBuilder {
    slots: NonZeroU16::new(36).expect("not zero"),
    pole_pairs: NonZeroU16::new(2).expect("not zero"),
    phases: NonZeroU16::new(3).expect("not zero"),
    layers: NonZeroU16::new(1).expect("not zero"),
    coil_throw_reduction: 0, // No short-pitching
    zone_shift: 0, // No zone shift
    winding_table_constructor: WindingTableConstructor::Tingley,
    turns_per_coil: NonZeroUsize::new(36).expect("not zero"),
    parallel_paths: NonZeroU16::new(1).expect("not zero"),
    connection: Connection::Star,
    end_winding_leakage_coefficient: 0.25,
    wire,
    concentric_coils: false,
    end_winding_half_turn_length_override: None,
}
.try_into()
.expect("given values create a symmetric, valid winding");

// We use a rotary core generated from the winding with default dimensions.
// For practical simulations, the core should be defined using the actual
// machine geometry, as demonstrated in the stem_core documentation.
let core = RotCore::from_winding(&winding);

// Assert that the winding is symmetric.
assert!(winding.is_symmetric(core.as_core_ref()));

// Calculate the phase resistance. Since the winding is symmetric, it
// suffices to evaluate one phase. The returned VarQuantity can be evaluated
// under different conditions, such as different operating temperatures.
let phase = NonZeroU16::new(1).expect("not zero");
let resistance = winding.resistance(core.as_core_ref(), phase);

let conditions = &[ThermodynamicTemperature::new::<degree_celsius>(20.0).into()];
assert_abs_diff_eq!(resistance.eval(conditions).get::<ohm>(), 4.141, epsilon = 1e-3);

let conditions = &[ThermodynamicTemperature::new::<degree_celsius>(120.0).into()];
assert_abs_diff_eq!(resistance.eval(conditions).get::<ohm>(), 5.798, epsilon = 1e-3);

// Get the slot leakage inductance for the fundamental spatial order
// (see also the documentation of the stem_slot crate).
// The air gap is needed to approximate the leakage flux at the tooth tips.
let air_gap = Length::new::<millimeter>(0.6);
let slot_leakage_inductance = winding.slot_leakage_inductance(
    core.as_core_ref(),
    phase,
    air_gap,
).eval(&[]);
assert_abs_diff_eq!(slot_leakage_inductance.get::<millihenry>(), 0.332, epsilon = 1e-3);

// Get the total phase inductance (all leakage inductances + main inductance)
// for the fundamental (pole pair) spatial order.
let inductance = winding.inductance(
    core.as_core_ref(),
    phase,
    SpatialOrder::Electrical(1),
    air_gap,
).eval(&[]);
assert_abs_diff_eq!(inductance.get::<millihenry>(), 7.652, epsilon = 1e-3);

// Get the volume of one particular coil.
let coil_props = winding.coil_properties_at(core.as_core_ref(), Zone::new(0, 0)).expect("coil at this location");
assert_abs_diff_eq!(coil_props.volume().get::<cubic_centimeter>(), 0.336, epsilon = 0.001);

// Calculate the total mass of all coils.
let mass: Mass = winding
    .coil_properties_iter(core.as_core_ref())
    .map(|cp| cp.volume() * cp.material().mass_density().eval(&[])).sum();
assert_abs_diff_eq!(mass.get::<gram>(), 6.055, epsilon = 0.001);
```

# Visualization

If the `cairo` feature is enabled, this crate provides two ways to visualize a
winding. The first shows the coil sides arranged in a winding table within the
machine core, as demonstrated in the previous examples. This representation is
provided by [`Winding::zone_drawables`].

Alternatively, [`Winding::coil_drawables`] visualizes the coils as viewed from
the air gap. This is particularly useful for examining the end windings and
their overlap.

The following image shows the coils of the 12-slot, 10-pole double-layer
tooth-coil winding from the first example.

![Coils of a 12-slot, 10-pole double-layer tooth-coil winding][coils_tooth_coil_winding.svg]

The executable example at `examples/winding_plots.rs` demonstrates both
visualization methods.

# Predefined winding types

In addition to the [`Winding`] trait, this crate provides several predefined
winding types implementing this trait:

- [`DistributedWinding`]: The coils of each phase span multiple slots or teeth
  and overlap one another. Distributed windings typically offer a relatively
  sinusoidal back-EMF and smooth torque, but require more intricate end-winding
  geometries, increasing manufacturing complexity.
- [`ToothCoilWinding`]: Each coil is wound around a single tooth, resulting in
  short and simple end windings that are well suited to automated manufacturing.
  However, tooth-coil windings can exhibit significant back-EMF harmonics and
  less smooth torque than comparable distributed windings.
- [`QuadrupleLayerToothCoilWinding`]: A variant of the
  [`ToothCoilWinding`] with two coils per tooth and four coil sides per slot.
  This arrangement provides additional degrees of freedom when designing the
  electromagnetic properties of the winding.
- [`DistributedToothCoilWinding`]: A hybrid of [`DistributedWinding`] and
  [`ToothCoilWinding`] that combines distributed coil arrangements with short
  and simple end windings. A trade-off is its comparatively low winding factor.
- [`SquirrelCageWinding`]: Typically used in induction motor rotors, this
  winding type generates its current system through electromagnetic induction
  rather than being supplied directly by an external source.
- [`CoilAssembly`]: A low-level representation consisting of individual
  [`Coil`]s. It allows arbitrary coil configurations, including asymmetric
  windings for investigating the effects of manufacturing deviations.

# Implementing your own winding

Extensibility is a core design principle of the `stem` ecosystem. To define a
custom winding type, simply implement the [`Winding`] trait. The trait provides
default implementations for many methods, giving custom types access to the
same analysis interface as the predefined winding types. These implementations
can be overridden whenever winding-specific behavior is required.

The following example demonstrates this using a deliberately simple winding
consisting of exactly one [`Coil`].

```rust
use std::sync::Arc;
use std::num::NonZeroU16;
use approxim::assert_abs_diff_eq;

use serde::{Deserialize, Serialize};

use stem_winding::prelude::*;

// A winding consisting of exactly one coil. All windings must be cloneable.
// The serde feature additionally requires Serialize and Deserialize.
#[derive(Clone, Deserialize, Serialize)] 
struct SingleCoilWinding(Coil);

// Required for serializing trait objects when the serde feature is enabled.
#[typetag::serde]
impl Winding for SingleCoilWinding {
    fn phases(&self) -> NonZeroU16 {
        NonZeroU16::new(1).expect("not zero")
    }

    fn slots(&self) -> NonZeroU16 {
        NonZeroU16::new(2).expect("not zero")
    }

    fn pole_pairs(&self) -> NonZeroU16 {
        NonZeroU16::new(1).expect("not zero")
    }

    fn parallel_paths(&self, phase: NonZeroU16) -> NonZeroU16 {
        let _ = phase;
        NonZeroU16::new(1).expect("not zero")
    }

    fn connection(&self) -> Connection {
        Connection::Star
    }

    fn coil_layout(&self) -> CoilLayout {
        CoilLayout::Single
    }

    /// Returns the coil occupying the given [`Zone`], if any.
    fn coil_at(&self, zone: Zone) -> Option<&Coil> {
        for z in self.0.zones() {
            if z == zone {
                return Some(&self.0)
            }
        }
        return None;
    }

    fn as_dyn(&self) -> &dyn Winding {
        self
    }
}

// A single coil occupying both slots of the winding
let winding = SingleCoilWinding(FullCoil::new(
    Zone { slot: 0, layer: 0 },
    Zone { slot: 1, layer: 0 },
    true,
    1.try_into().expect("not zero"),
    1.try_into().expect("not zero"),
    Arc::new(SffWire::default()),
)
.unwrap().into());

let phase = NonZeroU16::new(1).expect("not zero");

// Expected winding factor is 1, since we have a winding over two slots with one pole pair
assert_abs_diff_eq!(winding.winding_factor(phase, SpatialOrder::Mechanical(1)), 1.0, epsilon = 1e-3);

// The air-gap leakage factor is relatively high for this simple winding topology.
assert_abs_diff_eq!(winding.air_gap_leakage_factor(phase), 1.467, epsilon = 1e-3);
```

# Serialization and deserialization

If the `serde` feature is enabled, all windings and their components (coils, 
electrical [`Connection`] enums etc.) from this crate can be
serialized and deserialized. During deserialization, the invariants are
validated (e.g., to prevent malformed winding tables).

Units and quantities can be deserialized from strings representing SI units via
the [dyn_quantity](https://crates.io/crates/dyn_quantity) crate. Similarly, it
is possible to serialize quantities as value-unit strings using the
[serialize_with_units](https://docs.rs/dyn_quantity/latest/dyn_quantity/quantity/serde_impl/fn.serialize_with_units.html)
function.

See the chapter [serialization and deserialization] of the [stem book]
for details.
  
# Acknowledgments

The technical drawings used in the documentation have been created using 
LibreCAD (<https://librecad.org/>).
