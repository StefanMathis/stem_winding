//! Provides the [`Winding`] trait and related functionality for describing
//! and analyzing electric machine windings.
//!
//! Besides the [`Winding`] trait, which is the centerpiece of this crate, this
//! module also includes tools for analyzing winding properties, such as
//! [`FecSpectralAnalysis`].
//!
//! The module reexports predefined winding implementations, including the
//! common [`DistributedWinding`], [`ToothCoilWinding`] and
//! [`SquirrelCageWinding`], as well as more specialized variants such as
//! [`QuadrupleLayerToothCoilWinding`] and [`DistributedToothCoilWinding`].
//! Associated helper types, such as builder structs, are reexported as well.

use dyn_clone::DynClone;
use num::{Complex, Integer, integer::gcd};
use std::{
    f64::consts::{PI, TAU},
    num::{NonZeroU16, NonZeroUsize},
};

use rayon::prelude::*;

#[cfg(feature = "stem_core")]
use stem_core::prelude::*;

#[cfg(feature = "stem_core")]
use crate::core_support::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "cairo")]
use crate::draw::{
    CoilDrawables, CoilDrawablesParameters, WindingZoneDrawables, ZoneDrawablesConfig,
};

use stem_types::{CoilLayout, Zone};
use stem_wire::{stem_material::si::Length, wire::Wire};

use crate::{
    coils::{Coil, CoilExt},
    winding_table::WindingTable,
};

pub mod coil_assembly;
pub mod distributed;
pub mod distributed_tooth_coil;
pub mod quadruple_layer_tooth_coil;
pub mod squirrel_cage;
pub mod tooth_coil;

mod support;

pub use coil_assembly::*;
pub use distributed::*;
pub use distributed_tooth_coil::*;
pub use quadruple_layer_tooth_coil::{
    QuadrupleLayerToothCoilBuilder, QuadrupleLayerToothCoilMinimalBuilder,
    QuadrupleLayerToothCoilWinding,
};
pub use squirrel_cage::*;
pub use support::*;
pub use tooth_coil::*;

/// A value (winding factor, resistance, inductance, ...) is considered zero if
/// its absolute value is below this threshold. This accounts for floating-point
/// rounding errors, which can cause theoretically zero winding factors to
/// evaluate to nonzero values. Such small values are negligible for the
/// purposes of this crate.
const ZERO_THRESHOLD: f64 = 1e-8;

/**
A trait for representing windings for AC multi-phase machines.

A winding consists of a collection of [`Coil`]s arranged within the slots of
an electrical machine. In addition to its coil arrangement, a winding defines
electrical properties such as its number of phases, pole pairs, parallel paths,
and connection type, as well as geometric properties such as its coil layout
and end-winding characteristics.

A [`Zone`] identifies a possible position of a coil side by its slot and layer
indices, while [`CoilLayout`] describes how these positions are arranged
within a slot. A [`Winding`] assigns coil sides to these positions and groups
them into [`Coil`]s.

Different winding types may use different methods to determine their coil
arrangement and winding properties, but all provide the common interface
required by electrical-machine calculations.

# Physical properties

On its own, a [`Winding`] is independent of the machine geometry and therefore
does not fully define physical properties such as the phase resistance. If the
`stem_core` feature is enabled, additional methods are available that take a
[`CoreRef`] as their second argument to calculate these properties.

Properties of the winding as a whole, such as the phase
[`resistance`](Winding::resistance), are provided directly by [`Winding`].
Properties of individual coils can be obtained through the
[`coil_properties_at`](Winding::coil_properties_at) API.
[`coil_properties_iter`](Winding::coil_properties_iter) creates an iterator
over the coils and their properties; this can e.g. be used to calculate winding
properties such as the total wire mass. See the documentation of
[`coil_properties_iter`](Winding::coil_properties_iter) for an example.

# Implementation notes

Besides the required methods, it is recommended to override the following
methods when appropriate. For examples, see the predefined winding types such
as [`DistributedWinding`].

- [`Winding::base_winding_count`] determines the number of base or repeating
  windings of `self`. The default implementation uses an O(n) algorithm, but
  some winding types can determine this in O(1) time.
- [`Winding::series_turns_per_phase`] calculates the number of turns of a phase
  by iterating over the coils and summing [`Coil::turns`], resulting in an O(n)
  operation. Some winding types can provide this value in O(1) time.
- [`Winding::end_winding_half_turn_length`] (only available with the
  `stem_core` feature enabled) approximates the end-winding geometry using
  simple geometric models. If a more accurate calculation method is available,
  override this method.
- [`Winding::end_winding_leakage_inductance`] (only available with the
  `stem_core` feature enabled) returns zero by default. Implementations should
  override this method if the winding has a non-negligible end-winding leakage
  inductance.
- [`Winding::is_symmetric`] (only available with the `stem_core` feature enabled)
  checks if the winding is symmetric across its phases. The default
  implementation relies on comparing a couple of physical properties such as
  the [`Winding::resistance`] or the [`Winding::main_inductance`]. If the
  implementor of [`Winding`] is known to be symmetric, this function should be
  overriden for performance reasons.
- [`Winding::make_cache`] (only available with the `stem_core` feature enabled)
  creates a [`SimulationCache`] for the given winding–core combination.
  The default implementation populates cache entries for symmetric windings
  using the default winding and core models. Implementations may override
  this method to customize which entries are populated.
 */
#[cfg_attr(feature = "serde", typetag::serde)]
pub trait Winding: Sync + Send + std::any::Any + DynClone + std::fmt::Debug + 'static {
    /// Returns the number of phases in the winding.
    fn phases(&self) -> NonZeroU16;

    /// Returns the number of slots in the winding.
    fn slots(&self) -> NonZeroU16;

    /// Returns the number of pole pairs in the winding.
    fn pole_pairs(&self) -> NonZeroU16;

    /// Returns the number of parallel paths of the given phase.
    ///
    /// Parallel paths are electrically connected to the same terminals and are
    /// assumed to be electrically balanced. In particular, the paths contain
    /// the same number of turns and have the same electrical impedance.
    ///
    /// Refer to standard electrical machine literature for more information,
    /// for example:
    /// - Binder, A.: Elektrische Maschinen und Antriebe, 1st edition, Springer
    ///   Heidelberg, 2012
    /// - Müller, G., Vogt, K. and Ponick, B.: Berechnung elektrischer
    ///   Maschinen, 6th edition, Wiley-VCH, 2008
    /// - Pyrhönen, J., Jokinen, T., Hrabovcová, V.: Design of rotating
    ///   electrical machines, 1st edition, John Wiley & Sons, 2008
    fn parallel_paths(&self, phase: NonZeroU16) -> NonZeroU16;

    /// Returns the electrical connection of the winding.
    fn connection(&self) -> Connection;

    /// Returns the layout of the winding coils within a slot.
    fn coil_layout(&self) -> CoilLayout;

    /// Returns the coil occupying the given [`Zone`], if any.
    fn coil_at(&self, zone: Zone) -> Option<&Coil>;

    /// Returns `self` as a trait object.
    fn as_dyn(&self) -> &dyn Winding;

    // =========================================================================
    // These functions are likely to be overriden.

    /// Returns the number of base windings contained in the winding.
    ///
    /// Windings with a large number of pole pairs and slots are often realized
    /// by repeating a "base winding" multiple times. A repetition requires
    /// both the phase assignments and the numbers of turns to repeat together.
    /// For example, the following [`WindingTable`] describes two
    /// repetitions of a 6-slot, 1-pole-pair, 3-phase winding, assuming
    /// identical coil turns in corresponding slots:
    ///
    /// ```text
    /// L \ S │   0   1   2   3   4   5   6   7   8   9  10  11
    /// ──────┼────────────────────────────────────────────────
    ///   0   │   1  -3   2  -1   3  -2   1  -3   2  -1   3  -2
    /// ``
    /// This method returns the number of repetitions of the base winding in
    /// `self`. The [`WindingTable`] of the base winding can then be obtained by
    /// considering only the first `slots / base_winding_count` columns of the
    /// full winding table. This property is used by [`Winding::winding_table`]
    /// when the `full` argument is set to `false`.
    ///
    /// As stated in the [trait documentation](Winding), it is recommended to
    /// override this method if possible, as the default implementation is O(n)
    /// with respect to the number of zones because it uses the
    /// [`repeating_pattern_count`] algorithm.
    fn base_winding_count<'a>(&'a self) -> NonZeroU16 {
        // Wrapper structure which makes the winding indexable by an usize (slot-major)
        struct IndexWrapper<'a, W: ?Sized>(&'a W);

        impl<'a, W: Winding + ?Sized> RandomAccess for IndexWrapper<'a, W> {
            type Item = (usize, i32);

            fn get(&self, index: usize) -> Self::Item {
                let layers = usize::from(self.0.layers().get());

                let slot = index / layers;
                let layer = index % layers;

                let zone = Zone {
                    slot: slot as u16,
                    layer: layer as u16,
                };

                (self.0.turns_at(zone), self.0.phase_at(zone))
            }
        }

        let num_zones = usize::from(self.layers().get()) * usize::from(self.slots().get());
        let value = repeating_pattern_count(
            &IndexWrapper(self),
            NonZeroUsize::new(num_zones).expect("cannot be zero"),
        )
        .get();
        // We don't need to worry about the as-cast, because values can at most be
        // "num_zones".
        return NonZeroU16::new(value as u16).unwrap_or(NonZeroU16::MIN);
    }

    /// Returns the number of series turns per phase.
    ///
    /// The series turns per phase are an important design parameter because
    /// they determine the induced voltage of the machine. The number of
    /// turns must be chosen appropriately for the intended terminal
    /// voltage: the relationship between the induced voltage and the
    /// terminal voltage determines the winding current and therefore
    /// strongly affects the operating point of the machine. An unsuitable
    /// number of turns can result in excessive current and overheating or,
    /// conversely, insufficient torque.
    ///
    /// The series turns per phase are an important design parameter because
    /// they determine the induced voltage and therefore the terminal
    /// voltage. The desired voltage level can be achieved by choosing the
    /// appropriate number of turns for the coils. The amplitude of the
    /// fundamental component of the induced phase voltage `û` is
    ///
    /// `û = omega * w * kw1 * Phi_h`,
    ///
    /// where `omega` is the angular frequency, `w` is the number of series
    /// turns per phase, `kw1` is the fundamental-wave winding factor, and
    /// `Phi_h` is the fundamental flux per pole. Alternatively, the RMS induced
    /// phase voltage `U` is often used:
    ///
    /// `U = omega / sqrt(2) * w * kw1 * Phi_h`.
    ///
    /// The default implementation loops through all coil sides of the winding
    /// and sums the turns belonging to the specified `phase`. This is an
    /// O(n) operation, and it is recommended to override it with an O(1)
    /// algorithm if the winding structure allows it, as is the case for
    /// [`DistributedWinding`].
    fn series_turns_per_phase(&self, phase: NonZeroU16) -> num::rational::Ratio<usize> {
        // Sum of the turns in all coil sides belonging to that phase
        let mut turns = 0;

        for coil in self.coils_iter() {
            if coil.phase() == phase {
                turns = turns
                    + match coil {
                        Coil::Full(_) => 2 * usize::from(coil.turns()),
                        Coil::Half(_) => usize::from(coil.turns()),
                    };
            }
        }

        // Take into consideration the number of parallel paths
        let parallel_paths = usize::from(self.parallel_paths(phase).get());
        let gcd_turns_pp = gcd(turns, parallel_paths);
        let path_turns = turns / gcd_turns_pp;
        if path_turns % 2 == 0 {
            return num::rational::Ratio::new(path_turns / 2, parallel_paths / gcd_turns_pp);
        } else {
            return num::rational::Ratio::new(path_turns, 2 * parallel_paths / gcd_turns_pp);
        }
    }

    /// Returns the [`series_turns_per_phase`](Winding::series_turns_per_phase)
    /// as a floating-point number.
    fn series_turns_per_phase_float(&self, phase: NonZeroU16) -> f64 {
        let turns = self.series_turns_per_phase(phase);
        return *turns.numer() as f64 / *turns.denom() as f64;
    }

    /**
    Returns the length of one half turn of the end winding associated with the
    specified [`Zone`].

    The end winding connects the two coil sides of a coil outside the active
    core region. The following diagram illustrates the quantity returned by this
    method. If the space occupied by one character in the diagram corresponds to
    one millimeter, the half-turn shown below has a length of 7 mm:
    `━ + ━ + ┏ + ┃ + ┗ + ━ + ━`

    ```text
       ╔══════╗
    ┏━━║      ║━━┓
    ┃  ║ Core ║  ┃ <-- Coil
    ┗━━║      ║━━┛
       ╚══════╝
    ```

    The half turn is associated with the [`Zone`] from which the coil side
    originates. For example, consider a full coil occupying two zones:
    ```text
           ┏━━━┓   ┏━━━┓
           │   │   │           │
    coil   ▲   ▼   ▲           ▼
           │   │   │           │
           ┗━━━┛           ┗━━━┛
            (a)     (b)     (c)
    ```
    Here, the end winding associated with the first zone is the portion leaving
    the coil side at that zone, while the end winding associated with the second
    zone is the portion leaving the coil side at that zone:

    (a) The full coil occupies [`Zone { slot: 0, layer: 0 }`] and
    [`Zone { slot: 1, layer: 0 }`].

    (b) The end winding is associated with [`Zone { slot: 0, layer: 0 }`].

    (c) The end winding is associated with [`Zone { slot: 1, layer: 0 }`].

    The default implementation provides a simple geometric approximation:

    - For coil sides in neighboring slots, the winding is treated as a
    tooth-coil winding and [`end_winding_half_turn_length_semicircle`] is used.
    - Otherwise, for a [`LinCore`], [`end_winding_half_turn_length_straight`]
    is used.
    - For a [`RotCore`], [`end_winding_half_turn_length_circular_arc`] is
    used.

    These approximations are intended for general-purpose calculations. Winding
    types with more detailed geometric information should override this method
    with a more accurate calculation.
     */
    #[cfg(feature = "stem_core")]
    fn end_winding_half_turn_length(&self, core: CoreRef<'_>, zone: Zone) -> Length {
        let coil = match self.coil_at(zone) {
            Some(c) => c,
            None => return Length::new::<meter>(0.0),
        };

        let slots = core.rot().map(|_| self.slots());
        if coil.throw(slots) <= 1 {
            end_winding_half_turn_length_semicircle(self, core, zone)
        } else {
            match core {
                CoreRef::Lin(lin_core) => {
                    end_winding_half_turn_length_straight(self, lin_core, zone)
                }
                CoreRef::Rot(rot_core) => {
                    end_winding_half_turn_length_circular_arc(self, rot_core, zone)
                }
            }
        }
    }

    /// Returns the end winding leakage inductance for the specified phase.
    ///
    /// End-winding leakage inductance describes the inductance associated with
    /// the leakage flux produced by the end-winding portions of the phase.
    /// This flux does not contribute to the useful air-gap flux but affects
    /// the electrical behavior of the winding, for example its voltage drop
    /// and current.
    ///
    /// Unlike [`Winding::end_winding_half_turn_length`], for which a generic
    /// geometric approximation can be provided, the end-winding leakage
    /// inductance cannot in general be determined from the winding geometry
    /// alone [\[1\]](#end_winding_leakage_inductance_1), section 3.7.2. Its
    /// value depends on the detailed geometry and arrangement of the end
    /// windings as well as on surrounding ferromagnetic parts such as e.g.
    /// the machine housing.
    ///
    /// The default implementation therefore returns zero. Winding types for
    /// which an analytical approximation is available should override this
    /// method. Such calculations often require the end-winding turn length;
    /// this can either be obtained from
    /// [`Winding::end_winding_half_turn_length`] or supplied explicitly
    /// through `end_winding_half_turn_length`. The latter is useful when
    /// the actual turn length is known from a measurement which should be used
    /// instead of a geometric approximation.
    ///
    /// # Literature
    /// <a id="end_winding_leakage_inductance_1">\[1\]</a>
    /// Müller, G., Vogt, K. and Ponick, B.: Berechnung elektrischer Maschinen,
    /// 6th edition, Wiley-VCH, 2008
    #[cfg(feature = "stem_core")]
    fn end_winding_leakage_inductance(
        &self,
        core: CoreRef<'_>,
        phase: NonZeroU16,
        spatial_order: SpatialOrder,
        conditions: &[DynQuantity<f64>],
    ) -> Inductance {
        let _ = core;
        let _ = phase;
        let _ = spatial_order;
        let _ = conditions;
        return Default::default();
    }

    /// Checks whether the winding exhibits phase symmetry under a set of
    /// specified evaluation conditions.
    ///
    /// The following properties are compared across all phases:
    ///
    /// - The fundamental winding factor.
    /// - The electrical resistance.
    /// - The main inductance, air-gap leakage inductance, end-winding leakage
    ///   inductance and slot leakage inductance.
    /// - The number of series turns per phase.
    ///
    /// The winding factor is evaluated at the fundamental electrical spatial
    /// order. The inductances are evaluated with an effective air gap of 1 mm.
    /// All properties are evaluated without environmental conditions, such as
    /// temperature or excitation frequency.
    ///
    /// If all these properties are equal across phases, the winding is
    /// considered symmetric.
    ///
    /// This check is only an indication of symmetry under the stated evaluation
    /// conditions. In particular, it cannot guarantee symmetry for other
    /// spatial orders, effective air gaps or environmental conditions, nor
    /// can it account for arbitrary behavior introduced by overridden
    /// methods.
    ///
    /// This method can be relatively expensive because it evaluates the listed
    /// properties for every phase. Implementations known to be symmetric should
    /// override this method to avoid these calculations. An example from this
    /// crate would be [`DistributedWinding::is_symmetric`].
    #[cfg(feature = "stem_core")]
    fn is_symmetric(&self, core: CoreRef<'_>) -> bool {
        use uom::si::electrical_resistance::ohm;

        // Condition 1: Comparison of winding factors
        if !self.equal_winding_factors() {
            return false;
        }

        // All other conditions
        let conditions = &[];
        let air_gap = Length::new::<millimeter>(1.0);
        let fundamental = SpatialOrder::Electrical(1);

        // Condition 2: Calculate the phase resistance for some assumed conditions
        let resistance_ref = self
            .resistance(core, NonZeroU16::MIN, conditions)
            .get::<ohm>();
        let main_inductance_ref = self
            .main_inductance(core, NonZeroU16::MIN, fundamental, air_gap, conditions)
            .get::<henry>();
        let air_gap_leakage_inductance_ref = self
            .air_gap_leakage_inductance(core, NonZeroU16::MIN, fundamental, air_gap, conditions)
            .get::<henry>();
        let end_winding_leakage_inductance_ref = self
            .end_winding_leakage_inductance(core, NonZeroU16::MIN, fundamental, conditions)
            .get::<henry>();
        let slot_leakage_inductance_ref = self
            .slot_leakage_inductance(core, NonZeroU16::MIN, air_gap, conditions)
            .get::<henry>();
        let turns_phase_ref = self.series_turns_per_phase(NonZeroU16::MIN);

        for ph in 2..(self.phases().get() + 1) {
            let phase = NonZeroU16::new(ph).expect("not zero");

            let resistance = self.resistance(core, phase, conditions).get::<ohm>();
            if approxim::abs_diff_ne!(resistance, resistance_ref, epsilon = ZERO_THRESHOLD) {
                return false;
            }

            let main_inductance = self
                .main_inductance(core, phase, fundamental, air_gap, conditions)
                .get::<henry>();
            if approxim::abs_diff_ne!(
                main_inductance,
                main_inductance_ref,
                epsilon = ZERO_THRESHOLD
            ) {
                return false;
            }

            let air_gap_leakage_inductance = self
                .air_gap_leakage_inductance(core, phase, fundamental, air_gap, conditions)
                .get::<henry>();
            if approxim::abs_diff_ne!(
                air_gap_leakage_inductance,
                air_gap_leakage_inductance_ref,
                epsilon = ZERO_THRESHOLD
            ) {
                return false;
            }

            let end_winding_leakage_inductance = self
                .end_winding_leakage_inductance(core, phase, fundamental, conditions)
                .get::<henry>();
            if approxim::abs_diff_ne!(
                end_winding_leakage_inductance,
                end_winding_leakage_inductance_ref,
                epsilon = ZERO_THRESHOLD
            ) {
                return false;
            }

            let slot_leakage_inductance = self
                .slot_leakage_inductance(core, phase, air_gap, conditions)
                .get::<henry>();
            if approxim::abs_diff_ne!(
                slot_leakage_inductance,
                slot_leakage_inductance_ref,
                epsilon = ZERO_THRESHOLD
            ) {
                return false;
            }

            if self.series_turns_per_phase(phase) != turns_phase_ref {
                return false;
            }
        }
        true
    }

    /// Creates a [`SimulationCache`] for this winding and the given core.
    ///
    /// The default implementation populates cache entries that can be
    /// determined for [symmetric](Winding::is_symmetric) windings and
    /// returns an empty cache for other windings. Implementations may
    /// override this method to customize which entries are populated.
    ///
    /// The default implementation assumes that the inductances calculated by
    /// the default implementations of the corresponding [`Winding`] methods
    /// are independent of environmental conditions, as described in the
    /// [`SimulationCache`] documentation. Implementations for which this
    /// assumption does not hold must override this method to ensure that the
    /// cache contains only valid values.
    ///
    /// A populated entry is authoritative: downstream code uses its value
    /// instead of recalculating the corresponding quantity. Entries left as
    /// `None` are calculated on demand, as described in the documentation of
    /// each field.
    ///
    /// Implementations are responsible for ensuring that populated entries are
    /// valid for the winding–core combination and the simulation conditions in
    /// which they will be used.
    #[cfg(feature = "stem_core")]
    fn make_cache(&self, core: CoreRef<'_>, effective_air_gap: Length) -> SimulationCache {
        if self.is_symmetric(core) {
            let phase = NonZeroU16::MIN;
            let fundamental = SpatialOrder::Electrical(1);

            SimulationCache {
                resistance_decomposition: self.resistance_decomposition(core),
                fundamental_winding_factor: Some(self.winding_factor(phase, fundamental)),
                fundamental_effective_winding_factor: Some(self.effective_winding_factor(
                    core,
                    phase,
                    fundamental,
                    effective_air_gap,
                )),
                main_inductance: Some(self.main_inductance(
                    core,
                    phase,
                    fundamental,
                    effective_air_gap,
                    &[],
                )),
                air_gap_leakage_inductance: Some(self.air_gap_leakage_inductance(
                    core,
                    phase,
                    fundamental,
                    effective_air_gap,
                    &[],
                )),
                slot_leakage_inductance: Some(self.slot_leakage_inductance(
                    core,
                    phase,
                    effective_air_gap,
                    &[],
                )),
                end_winding_leakage_inductance: Some(self.end_winding_leakage_inductance(
                    core,
                    phase,
                    fundamental,
                    &[],
                )),
                fundamental_slot_opening_factor: Some(core.slot_opening_factor(fundamental)),
            }
        } else {
            SimulationCache::default()
        }
    }

    // =========================================================================
    // These functions usually don't need to be overriden.

    /// Return the number of turns at the given [`Zone`].
    ///
    /// If there is a coil at the specified index, this function returns the
    /// [`turns`](CoilExt::turns) of that coil. If there is no coil, this method
    /// returns 0.
    fn turns_at(&self, zone: Zone) -> usize {
        self.coil_at(zone).map(|c| c.turns().get()).unwrap_or(0)
    }

    /// Returns the number of winding layers within each slot.
    ///
    /// The number of layers is determined by the winding's [CoilLayout]. This
    /// method is equivalent to calling [CoilLayout::layers] on
    /// [Winding::coil_layout].
    fn layers(&self) -> NonZeroU16 {
        self.coil_layout().layers()
    }

    /// Returns the pole pitch in slots.
    ///
    /// The pole pitch is the number of slots corresponding to one pole, i.e.
    /// the angular distance between two adjacent poles of opposite
    /// polarity. It is calculated as
    ///
    /// `pole_pitch = slots / (2 * pole_pairs)`.
    ///
    /// The pole pitch may be fractional for windings with a fractional-slot
    /// configuration.
    fn pole_pitch(&self) -> f64 {
        self.slots().get() as f64 / (2.0 * self.pole_pairs().get() as f64)
    }

    /// Returns the [`Wire`] of the [`Coil`] containing the coil side at the
    /// given [`Zone`].
    ///
    /// If the specified [`Zone`] is empty, this method returns `None`.
    fn wire_at(&self, zone: Zone) -> Option<&dyn Wire> {
        self.coil_at(zone).map(CoilExt::wire)
    }

    /// Returns the signed phase of the coil side at the given [`Zone`].
    ///
    /// The sign indicates the polarity of the coil side: a positive value
    /// denotes a positive coil-side polarity, while a negative value
    /// denotes a negative coil-side polarity. The absolute value identifies
    /// the phase.
    ///
    /// If the specified [`Zone`] is empty, this method returns `0`. The signed
    /// phase follows the same convention as [`WindingTable`].
    fn phase_at(&self, zone: Zone) -> i32 {
        let coil = match self.coil_at(zone) {
            Some(c) => c,
            None => return 0,
        };
        let is_positive = match coil {
            Coil::Full(coil) => zone == coil.positive_zone(),
            Coil::Half(coil) => coil.is_positive(),
        };
        let phase = i32::from(coil.phase().get());
        if is_positive {
            return phase;
        } else {
            return -phase;
        }
    }

    // Returns the number of [`Coil`]s in the winding.
    fn num_coils(&self) -> usize {
        self.coils_iter().count()
    }

    /**
    Returns the [`WindingTable`] of the winding.

    The [`WindingTable`] shows the phase and polarity of the coil sides at the
    individual winding zones. It is populated from `self` by iterating over the
    slots and layers and calling [`Winding::phase_at`] for each [`Zone`].

    Some windings are composed of multiple "base windings"; see
    [`Winding::base_winding_count`] for details. If `full` is `false`, only the
    winding table of the base winding is returned, covering the first
    `slots / base_winding_count` slots. If `full` is `true`, the winding table
    contains all slots of the winding.

    [`WindingTable`] also implements [`From<&Winding>`], which is equivalent to
    calling this method with `full` set to `true`.

    # Examples

    ```
    use std::num::NonZeroU16;
    use stem_winding::prelude::*;

    let winding: DistributedWinding = DistributedMinimalBuilder {
        slots: 12.try_into().expect("not zero"),
        pole_pairs: 2.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 1.try_into().expect("not zero"),
        coil_span_reduction: 0,
        zone_span_variation: 0,
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();

    // Base winding table
    let wt = winding.winding_table(false);
    assert_eq!(wt[Zone::new(0, 0)], 1);
    assert_eq!(wt[Zone::new(1, 0)], -3);
    assert_eq!(wt[Zone::new(2, 0)], 2);
    assert_eq!(wt[Zone::new(3, 0)], -1);
    assert_eq!(wt[Zone::new(4, 0)], 3);
    assert_eq!(wt[Zone::new(5, 0)], -2);

    // Full winding table
    let wt = winding.winding_table(true);
    assert_eq!(wt[Zone::new(0, 0)], 1);
    assert_eq!(wt[Zone::new(1, 0)], -3);
    assert_eq!(wt[Zone::new(2, 0)], 2);
    assert_eq!(wt[Zone::new(3, 0)], -1);
    assert_eq!(wt[Zone::new(4, 0)], 3);
    assert_eq!(wt[Zone::new(5, 0)], -2);
    assert_eq!(wt[Zone::new(6, 0)], 1);
    assert_eq!(wt[Zone::new(7, 0)], -3);
    assert_eq!(wt[Zone::new(8, 0)], 2);
    assert_eq!(wt[Zone::new(9, 0)], -1);
    assert_eq!(wt[Zone::new(10, 0)], 3);
    assert_eq!(wt[Zone::new(11, 0)], -2);
    ```
     */
    fn winding_table(&self, full: bool) -> WindingTable {
        let slots = if full {
            self.slots()
        } else {
            NonZeroU16::new(self.slots().get() / self.base_winding_count().get()).expect("not zero")
        };
        let layers = self.layers();

        let mut winding_table = WindingTable::new(slots, layers);

        for slot in 0..slots.get() {
            for layer in 0..layers.get() {
                let zone = Zone::new(slot, layer);
                winding_table[zone] = self.phase_at(zone);
            }
        }

        return winding_table;
    }

    /// Returns the electrical angle between the phasors of two neighboring
    /// slots.
    ///
    /// The angle is calculated as
    ///
    /// `phasor_angle = 2π * pole_pairs / slots`.
    ///
    /// This relation is given by Pyrhönen, J., Jokinen, T., Hrabovcová, V.:
    /// *Design of Rotating Electrical Machines*, 1st edition, John Wiley &
    /// Sons, 2008, eq. (2.66).
    fn phasor_angle(&self) -> f64 {
        phasor_angle(self.slots(), self.pole_pairs())
    }

    /// Returns the electrical angle associated with a phase.
    ///
    /// The default implementation assumes that the phases are evenly spaced
    /// over one electrical period, with phase 1 at angle 0.
    fn phase_angle(&self, phase: NonZeroU16) -> f64 {
        TAU / f64::from(self.phases().get()) * (f64::from(phase.get()) - 1.0)
    }

    /// Returns the hole number of the winding.
    ///
    /// This is equivalent to calling [`hole_number`] with the number of slots,
    /// pole pairs, and phases of this winding.
    fn hole_number(&self) -> num::rational::Ratio<u16> {
        hole_number(self.slots(), self.pole_pairs(), self.phases())
    }

    /// Returns the [`hole_number`](Winding::hole_number) as a floating-point
    /// number.
    fn hole_number_float(&self) -> f64 {
        let hole_number = self.hole_number();
        return f64::from(*hole_number.numer()) / f64::from(*hole_number.denom());
    }

    /// Returns the total number of turns of the specified `slot`.
    fn turns_in_slot(&self, slot: u16) -> usize {
        let mut turns: usize = 0;
        for layer in 0..self.layers().get() {
            turns += self.turns_at(Zone::new(slot, layer));
        }
        return turns;
    }

    /// Returns the lowest order of the radial force resulting from the winding
    /// topology.
    ///
    /// The radial-force order describes the spatial variation of the radial
    /// force acting on the stator [\[1\]](#lowest_radial_force_order_1),
    /// section 3.5.3. An order of 1 corresponds to a constant radial force
    /// acting in one direction, resulting in a dynamic load on the
    /// bearings. An order of 2 produces an ovalization of the stator, an
    /// order of 3 produces a three-lobed deformation, and so on. All
    /// multiples of the lowest radial-force order occur as well.
    ///
    /// The default implementation is valid only for
    /// [symmetric](Winding::is_symmetric) windings.
    ///
    /// # Literature
    /// <a id="lowest_radial_force_order_1">\[1\]</a>
    /// Poltschak, F.: *Untersuchungen zu permanentmagneterregten
    /// Synchronmaschinen hoher Leistungsdichte*, 1st edition, Trauner Druck,
    /// 2012
    fn lowest_radial_force_order(&self) -> usize {
        return num::integer::gcd(
            2 * self.pole_pairs().get(),
            self.slots().get() / self.phases().get(),
        ) as usize;
    }

    /**
    Returns the lowest order of the torque ripple caused by the winding
    currents.

    The torque ripple order describes the number of complete torque-ripple
    waves occurring during one full mechanical revolution of the rotor. For
    example, an order of 24 means that the torque completes 24 sinusoidal
    cycles per rotor revolution; equivalently, the positive peak of the
    sinusoidal torque component occurs 24 times per revolution.
    \[1\](#lowest_torque_ripple_order_1), section 9.4.2b)

    Higher-order torque-ripple components may occur as well. Their orders
    are integer multiples of the lowest order.

    The default implementation is valid only for symmetric windings.

    # Literature
    <a id="lowest_torque_ripple_order_1">\[1\]</a>
    Binder, A.: Elektrische Maschinen und Antriebe, 1st edition, Springer
    Heidelberg, 2012

    # Examples

    ```
    use std::num::NonZeroU16;
    use stem_winding::prelude::*;

    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 12.try_into().expect("not zero"),
        pole_pairs: 5.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 1.try_into().expect("not zero"),
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();

    // 30 = number of poles × number of phases = 10 × 3
    assert_eq!(winding.lowest_torque_ripple_order(), 30);
    ```
    */
    fn lowest_torque_ripple_order(&self) -> usize {
        // In [1], the lowest-order pulsation frequency is stated as
        // 2 * phases * stator_frequency for a single pole pair. To get the
        // order over all pole pairs, we need to multiply by self.pole_pairs.
        2 * (self.pole_pairs().get() * self.phases().get()) as usize
    }

    /**
    Returns the lowest order of the cogging torque caused by reluctance effects
    between the rotor and the stator.

    The cogging-torque order has the same meaning as the
    [`lowest_torque_ripple_order`](Winding::lowest_torque_ripple_order): it is
    the number of complete cogging-torque cycles occurring during one full
    mechanical revolution of the rotor. Higher-order components may occur as
    well.

    Cogging torque occurs only in topologies where the magnetic reluctance
    varies with the rotor position, such as slotted machines. It does not occur
    in topologies without such reluctance variation, such as ideal air-gap
    windings.

    The lowest cogging-torque order is the least common multiple of the number
    of slots and the number of poles [\[1\]](#lowest_cogging_torque_order_1),
    section 9.4.2a, [\[2\]](#lowest_cogging_torque_order_2), eq. (14).

    The default implementation is valid only for symmetric windings.

    # Literature
    <a id="lowest_cogging_torque_order_1">\[1\]</a>
    Binder, A.: *Elektrische Maschinen und Antriebe*, 1st edition, Springer
    Heidelberg, 2012

    <a id="lowest_cogging_torque_order_2">\[2\]</a>
    Huth, G.: *Nutrastung von permanenterregten AC-Servomotoren mit gestaffelter
    Rototanordnung*, Electrical Engineering 78, p. 391–397, Springer-Verlag, 1995

    # Examples

    ```
    use std::num::NonZeroU16;
    use stem_winding::prelude::*;

    let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
        slots: 12.try_into().expect("not zero"),
        pole_pairs: 5.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 1.try_into().expect("not zero"),
        winding_table_constructor: WindingTableConstructor::Tingley,
    }
    .try_into()
    .unwrap();

    // Least common multiple of poles and slots: lcm(12, 10) = 60
    assert_eq!(winding.lowest_cogging_torque_order(), 60);
    ```
    */
    fn lowest_cogging_torque_order(&self) -> usize {
        return num::integer::lcm(2 * self.pole_pairs().get(), self.slots().get()) as usize;
    }

    /// Returns the winding factor of the specified `spatial_order` for the
    /// `phase`.
    ///
    /// Since the winding is spatially distributed along the air gap surface,
    /// the magnetic air gap flux penetrating the winding does not intersect all
    /// coils of a phase simultaneously, but with an individual phase shift for
    /// each coil. The phase shifts cause the individual coil contributions to
    /// partially cancel, reducing the resulting magnitude compared to an
    /// idealized winding in which all coils are concentrated at the same
    /// point. The ratio of the two is called the "winding factor".
    /// [\[1\]](#winding_factor_1), section 2.4.
    ///
    /// Mathematically, it can be expressed as the ratio between the geometric
    /// sum of the voltage phasors and the sum of the phasor lengths. It is
    /// particularly elegant to calculate the geometric sum in complex
    /// coordinates:
    ///
    /// `k_v = |Σ^(N-1)_s=0 n_s * e^(j * s * α_u * v)| / Σ^(N-1)_s=0 n_s,abs`,
    ///
    /// where `N` is the number of [`slots`](Winding::slots), `s` is the slot
    /// index from `0` to `N-1`, `n_s` is the signed number of turns in slot `s`
    /// belonging to the `phase`, summed over all layers, and `n_s,abs` is the
    /// total number of turns in that slot belonging to the `phase`,
    /// irrespective of polarity. `α_u` is the
    /// [`phasor_angle`](Winding::phasor_angle), and `v` is the electrical
    /// spatial order obtained from the `spatial_order` argument.
    ///
    /// The winding factor accounts only for the spatial distribution of the
    /// coils. It does not account for geometric effects such as the
    /// distribution of flux across the slot
    /// opening([CoreExt::slot_opening_factor]) or air-gap curvature
    /// ([CoreExt::air_gap_curvature_factor]). These effects are included in
    /// [Winding::effective_winding_factor].
    ///
    /// # Literature
    /// <a id="winding_factor_1">\[1\]</a>
    /// Pyrhönen, J., Jokinen, T., Hrabovcová, V.: Design of rotating electrical
    /// machines, 1st edition, John Wiley & Sons, 2008
    ///
    /// # Examples
    ///
    /// ```
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
    ///     slots: 12.try_into().expect("not zero"),
    ///     pole_pairs: 5.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 2.try_into().expect("not zero"),
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    /// }
    /// .try_into()
    /// .unwrap();
    ///
    /// // Fundamental for all three phases
    /// for phase in 1..4 {
    ///     assert_abs_diff_eq!(
    ///         0.933,
    ///         winding.winding_factor(phase.try_into().unwrap(), SpatialOrder::Electrical(1)),
    ///         epsilon = 1e-3
    ///     );
    /// }
    ///
    /// // A 12/10 double layer tooth coil winding produces a subharmonic:
    /// // Mechanical order 1 corresponds to electrical order 1/5 for this machine.
    /// for phase in 1..4 {
    ///     assert_abs_diff_eq!(
    ///         0.067,
    ///         winding.winding_factor(phase.try_into().unwrap(), SpatialOrder::Mechanical(1)),
    ///         epsilon = 1e-3
    ///     );
    /// }
    /// ```
    fn winding_factor(&self, phase: NonZeroU16, spatial_order: SpatialOrder) -> f64 {
        // It is sufficient to calculate the phasor star for the basic winding.
        // However, this also means that the spatial and likewise the phasor
        // angle must adjusted accordingly!
        let base_winding_count = self.base_winding_count().get();
        let slots_basic = self.slots().get() / base_winding_count;

        let elec_order = match spatial_order {
            SpatialOrder::Mechanical(v) => {
                if v % u32::from(base_winding_count) != 0 {
                    // W.r.t. the basic winding, no fractional order can exist!
                    return 0.0;
                }
                f64::from(v) / f64::from(self.pole_pairs().get() * base_winding_count)
            }
            SpatialOrder::Electrical(v) => f64::from(v) / f64::from(base_winding_count),
        };
        let layers = self.layers().get();
        let alpha_u = self.phasor_angle() * f64::from(base_winding_count);

        let (phasor_sum_geo, phasor_sum_abs) = (0..slots_basic)
            .into_par_iter()
            .map(|slot| {
                let mut phasor_sum_abs = 0;

                let mut signed_turns_in_layer: f64 = 0.0;
                for layer in 0..layers {
                    let current_phase = self.phase_at(Zone::new(slot, layer));
                    if current_phase.abs() == i32::from(phase.get()) {
                        let turns = self.turns_at(Zone::new(slot, layer));
                        phasor_sum_abs += turns;
                        signed_turns_in_layer += f64::from(current_phase.signum()) * turns as f64;
                    }
                }
                let phasor_sum_geo = signed_turns_in_layer
                    * Complex::new(0.0, slot as f64 * alpha_u * elec_order).exp();

                (phasor_sum_geo, phasor_sum_abs)
            })
            .reduce(
                || (Complex::new(0.0, 0.0), 0),
                |a, b| (a.0 + b.0, a.1 + b.1),
            );

        // Sum up the phasors and get the absolute value, which equals the resulting
        // phasor length. Calculate winding factor as quotient of the absolute sum of
        // all phasors and the geometrical sum of all phasors.
        return phasor_sum_geo.norm() / (phasor_sum_abs as f64);
    }

    /// Checks whether all phases have the same winding factor at the
    /// fundamental electrical spatial order.
    ///
    /// The default implementation compares the fundamental winding factors of
    /// all phases using an absolute tolerance of `1e-12`.
    fn equal_winding_factors(&self) -> bool {
        let ref_winding_factor = self.winding_factor(NonZeroU16::MIN, SpatialOrder::Electrical(1));
        for phase in 2..(self.phases().get() + 1) {
            let winding_factor = self.winding_factor(
                NonZeroU16::new(phase).expect("is not zero"),
                SpatialOrder::Electrical(1),
            );
            if approxim::abs_diff_ne!(winding_factor, ref_winding_factor, epsilon = ZERO_THRESHOLD)
            {
                return false;
            }
        }
        return true;
    }

    /// Returns the vertices of the Görges polygon.
    ///
    /// The Görges polygon represents the space-time voltage polygon of the
    /// idealized air-gap field in a complex plane. Each vertex corresponds to a
    /// slot of the winding. The polygon can be used to determine the
    /// [`air_gap_leakage_factor`](Winding::air_gap_leakage_factor), as
    /// described in [\[1\]](#goerges_polygon_1), section 1.2.4.
    ///
    /// Connecting the vertices in their order of appearance (slot order) in the
    /// complex plane yields the Görges diagram.
    ///
    /// If `full` is `false`, only the vertices of the underlying base winding
    /// are returned, since the Görges diagram is cyclic over the base
    /// windings. See [`Winding::base_winding_count`] for details.
    ///
    /// # Literature
    /// <a id="goerges_polygon_1">\[1\]</a>
    /// Müller, G., Vogt, K. and Ponick, B.: Berechnung elektrischer Maschinen,
    /// 6th edition, Wiley-VCH, 2008
    fn goerges_polygon(&self, full: bool) -> Vec<Complex<f64>> {
        let slots = if full {
            self.slots().get()
        } else {
            self.slots().get() / self.base_winding_count().get()
        };
        let layers = self.layers().get();

        let mut vertices = Vec::with_capacity(usize::from(slots));

        // Sum of all magnetic voltages; needed for the focal point of the diagram
        let mut sum = Complex::new(0.0, 0.0);

        for slot in 0..slots {
            let v_prev = vertices.last().cloned().unwrap_or(Complex::new(0.0, 0.0));

            // The new polygon edge starts at the end of the previous polygon edge
            let v_curr = v_prev
                + (0..layers)
                    .map(|layer| {
                        let zone = Zone::new(slot, layer);

                        // Add the magnetic voltage to the current polygon node
                        let phase = self.phase_at(zone);
                        let turns = self.turns_at(zone);
                        let phase_angle = if let Ok(ph) = NonZeroU16::try_from(phase.abs() as u16) {
                            self.phase_angle(ph)
                        } else {
                            0.0
                        };

                        (turns as f64 * f64::from(phase.signum()))
                            * (Complex::new(0.0, phase_angle)).exp()
                    })
                    .sum::<Complex<f64>>();
            vertices.push(v_curr);
            sum += v_curr;
        }

        // Calculate the focal point and center the polygon
        let focal_point = sum / vertices.len() as f64;
        vertices.iter_mut().for_each(|v| {
            *v = *v - focal_point;
        });

        return vertices;
    }

    /// Returns the air gap leakage factor for the specifid `phase`.
    ///
    /// This calculation is implemented using the
    /// [`goerges_polygon`](Winding::goerges_polygon) as described in
    /// [\[1\]](#air_gap_leakage_factor_1), section 1.2.4.
    ///
    /// For the vertices `v_s` of the Görges polygon and the number of
    /// [`slots`](Winding::slots) `N`, the so-called "inertia radius" of the
    /// Görges polygon is calculated:
    ///
    /// `R_g = sqrt(1/N * Σ^(N-1)_s=0 |v_s|²)`
    /// [\[1\]](#air_gap_leakage_factor_1) eq. (1.2.85)
    ///
    /// Next, the "inertia radius" of the fundamental harmonic is determined:
    ///
    /// `R_p = m * w * k_wp / (π * p)` [\[1\]](#air_gap_leakage_factor_1) eq.
    /// (1.2.86)
    ///
    /// where `m` is the number of [`phases`](Winding::phases), `w` is the
    /// [`series_turns_per_phase`](Winding::series_turns_per_phase), `k_wp` is
    /// the [`winding_factor`](Winding::winding_factor) of the fundamental
    /// electric spatial order and `p` is the number of
    /// [`pole_pairs`](Winding::pole_pairs).
    ///
    /// The air gap leakage factor is the ratio between the squares of those
    /// radii minus one:
    ///
    /// `σ_o = (R_g / R_p)² - 1`. [\[1\]](#air_gap_leakage_factor_1) eq.
    /// (1.2.87)
    ///
    /// Multiplying this value with the
    /// [`main_inductance`](Winding::main_inductance) gives the
    /// [`air_gap_leakage_inductance`](Winding::air_gap_leakage_inductance).
    ///
    /// # Literature
    /// <a id="air_gap_leakage_factor_1">\[1\]</a>
    /// Müller, G., Vogt, K. and Ponick, B.: Berechnung elektrischer Maschinen,
    /// 6th edition, Wiley-VCH, 2008
    ///
    /// # Examples
    ///
    /// This example compares the air gap leakage factor of a tooth-coil
    /// winding, an integer-slot distributed winding and a cage winding.
    /// Usually, tooth-coil windings have a higher value than integer-slot
    /// distributed windings which in turn have a higher value than cage
    /// windings.
    ///
    /// ```
    /// use std::num::NonZeroU16;
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// // Tooth-coil winding
    /// let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
    ///     slots: 12.try_into().expect("not zero"),
    ///     pole_pairs: 5.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 2.try_into().expect("not zero"),
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    /// }
    /// .try_into()
    /// .unwrap();
    /// approxim::assert_abs_diff_eq!(
    ///     0.96835,
    ///     winding.air_gap_leakage_factor(NonZeroU16::MIN),
    ///     epsilon = 0.0001
    /// );
    ///
    /// // Distributed winding
    /// let winding: DistributedWinding = DistributedMinimalBuilder {
    ///     slots: 12.try_into().expect("not zero"),
    ///     pole_pairs: 2.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 1.try_into().expect("not zero"),
    ///     coil_span_reduction: 0,
    ///     zone_span_variation: 0,
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    /// }
    /// .try_into()
    /// .unwrap();
    /// approxim::assert_abs_diff_eq!(
    ///     0.096622,
    ///     winding.air_gap_leakage_factor(NonZeroU16::MIN),
    ///     epsilon = 0.0001
    /// );
    ///
    /// // Cage winding
    /// let winding: SquirrelCageWinding = SquirrelCageMinimalBuilder {
    ///     slots: 14.try_into().expect("not zero"),
    ///     pole_pairs: NonZeroU16::MIN,
    /// }.try_into().unwrap();
    /// approxim::assert_abs_diff_eq!(
    ///     0.01696,
    ///     winding.air_gap_leakage_factor(NonZeroU16::MIN),
    ///     epsilon = 0.0001
    /// );
    /// ```
    fn air_gap_leakage_factor(&self, phase: NonZeroU16) -> f64 {
        let verts = self.goerges_polygon(false);
        let slots = self.slots().get() / self.base_winding_count();
        let k_w = self.winding_factor(phase, SpatialOrder::Electrical(1));

        // Calculate the "Trägheitsradius" (inertia radius) squared according to
        // [1], eq. (1.2.85)
        let mut rg2: f64 = verts.iter().map(|v| v.norm_sqr()).sum();

        // For the magnetic slot voltage, the following holds true:
        // Θ_n = obj.layers*Θ_sp,
        // which means that the inertia radius has to be corrected accordingly here
        rg2 = rg2 / ((slots * self.layers().get().pow(2)) as f64);

        // Calculate the "Trägheitsradius der Hauptwelle" squared according to [1],
        // eq. (1.2.86). Current is arbitrarily set to 1 A (as it is in
        // self.goerges_polygon()). Correction by the number of winding
        // layers is necessary due to the same reason as for rg2.
        let rp2 = (self.phases().get() as f64 * self.series_turns_per_phase_float(phase) * k_w
            / (PI * (self.pole_pairs().get() * self.layers().get()) as f64))
            .powi(2);

        // [1], eq. (1.2.87)
        return rg2 / rp2 - 1.0;
    }

    /// Calculates the field excitation curve (FEC) of this winding for the
    /// given phase currents and writes it into the provided buffer.
    ///
    /// The field excitation curve represents the piecewise-constant spatial
    /// distribution of the magnetic voltage around the air gap, sampled at
    /// the winding slots [\[1\]](#fec_1), section 2.5. Its spatial harmonics
    /// can be obtained by Fourier analysis; this functionality is realized
    /// in the [`FecSpectralAnalysis`] struct. If one is just interested in
    /// the spatial orders of the nonzero harmonics, use [`Winding::harmonics`].
    ///
    /// The buffer length must equal the number of winding slots, and the
    /// current slice length must equal the number of phases. If either length
    /// is incorrect, the method returns an error without modifying the buffer.
    #[doc = ""]
    #[cfg_attr(
        feature = "doc-images",
        doc = "![Example of a field excitation curve][field_excitation_curve]"
    )]
    #[cfg_attr(
        feature = "doc-images",
        embed_doc_image::embed_doc_image(
            "field_excitation_curve",
            "docs/img/field_excitation_curve.svg"
        )
    )]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
        `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    ///
    /// # Literature
    /// <a id="fec_1">\[1\]</a>
    /// Binder, A.: Elektrische Maschinen und Antriebe, 1st edition, Springer
    /// Heidelberg, 2012
    ///
    /// # Examples
    ///
    /// ```
    /// use stem_winding::prelude::*;
    ///
    /// let winding: DistributedWinding = DistributedMinimalBuilder {
    ///     slots: 18.try_into().expect("not zero"),
    ///     pole_pairs: 1.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 1.try_into().expect("not zero"),
    ///     coil_span_reduction: 0,
    ///     zone_span_variation: 0,
    ///     winding_table_constructor: WindingTableConstructor::CoilSide,
    /// }
    /// .try_into()
    /// .unwrap();
    ///
    /// let mut buffer = vec![0.0; usize::from(winding.slots().get())];
    /// let three_phase_currents = [1.0, -0.5, -0.5];
    /// let expected_curve = &[
    ///         -0.5, 0.5, 1.5, 2.0, 2.5, 3.0, 2.5, 2.0, 1.5, 0.5, -0.5, -1.5,
    ///         -2.0, -2.5, -3.0, -2.5,-2.0, -1.5
    ///     ];
    ///
    /// assert!(winding.field_excitation_curve(&mut buffer, three_phase_currents.as_slice()).is_ok());
    /// assert_eq!(buffer.as_slice(), expected_curve);
    ///
    /// // Providing currents of the wrong length results in an error. The curve buffer is left unchanged.
    /// let five_phase_currents = [1.0, 0.0, 0.0, -0.5, -0.5];
    /// assert!(winding.field_excitation_curve(&mut buffer, five_phase_currents.as_slice()).is_err());
    /// assert_eq!(buffer.as_slice(), expected_curve);
    ///
    /// // If the field excitation curve buffer itself has the wrong length, an error is returned as well.
    /// buffer.resize(12, 0.0);
    /// assert!(winding.field_excitation_curve(&mut buffer, three_phase_currents.as_slice()).is_err());
    /// ```
    fn field_excitation_curve(
        &self,
        buffer: &mut [f64],
        currents: &[f64],
    ) -> Result<(), compare_variables::Comparison<usize>> {
        // Assert that the input slices have the right length
        let buffer_len = buffer.len();
        let slots = self.slots().get() as usize;
        compare_variables::compare_variables!(slots == buffer_len)?;

        let currents_len = currents.len();
        let phases = self.phases().get() as usize;
        compare_variables::compare_variables!(phases == currents_len)?;

        let mut ampere_turns_slot = 0.0;
        for (slot, buffer) in (0..self.slots().get()).zip(buffer.iter_mut()) {
            *buffer = ampere_turns_slot;
            for layer in 0..self.layers().get() {
                if let Some(coil) = self.coil_at(Zone::new(slot, layer)) {
                    // Get the direction of the coil
                    let is_positive = match coil {
                        Coil::Full(coil) => coil.positive_zone() == Zone { slot, layer },
                        Coil::Half(coil) => coil.is_positive(),
                    };

                    let current = currents
                        .get(usize::from(coil.phase().get()) - 1)
                        .expect("The current vector should have a value for each winding phase.");
                    let ampere_turns = usize::from(coil.turns()) as f64 * current;
                    if is_positive {
                        *buffer = *buffer + ampere_turns;
                    } else {
                        *buffer = *buffer - ampere_turns;
                    }
                }
            }

            // Store the previous ampere turns value as starting point for the next
            // iteration
            ampere_turns_slot = *buffer;
        }

        /*
        The FEC is now an undefined integral with the value of the constant C
        depending on the starting slot. C is equal to the a0 coefficient (constant)
        of the Fourier series of the FEC. Therefore, a Fourier analysis of the
        FEC is performed to identify C. Afterwards, C is set to 0 by substracting
        a0 (the offset) from the FEC. The face integral of the FEC over all slots
        then equals 0, meaning that no unipolar flux exists (pure 2D-flux in
        non-axial orientation).
        To avoid performing the FFT, the mean value is calculated and used for the
        correction
         */
        let sum: f64 = buffer.iter().sum();
        let mean: f64 = sum / buffer.len() as f64;
        buffer.iter_mut().for_each(|val| *val = *val - mean);

        return Ok(());
    }

    /**
    Returns an iterator over the [`WindingHarmonic`]s of a winding.

    The iterator increments the mechanical spatial order and checks whether
    the corresponding harmonic exists using [`Winding::is_harmonic`]. If it
    does, the iterator yields a [`WindingHarmonic`] containing the spatial
    order and the direction of rotation. Otherwise, it proceeds to the next
    spatial order.

    In other words, this iterator yields the spatial harmonics of the field
    excitation curve whose components do not cancel. It is therefore infinite.

    # Examples

    The following example shows a fractional-slot winding whose working harmonic
    is the fourth mechanical spatial harmonic. The iterator also yields
    lower-order subharmonics and higher-order harmonics.

    ```
    use stem_winding::prelude::*;
    use stem_winding::winding::WindingHarmonic;

    let winding: DistributedWinding = DistributedMinimalBuilder {
        slots: 18.try_into().expect("not zero"),
        pole_pairs: 4.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 1.try_into().expect("not zero"),
        coil_span_reduction: 0,
        zone_span_variation: 0,
        winding_table_constructor: WindingTableConstructor::CoilSide,
    }
    .try_into()
    .unwrap();

    let mut iter = winding.harmonics();

    // First subharmonic
    assert_eq!(iter.next(),
        Some(
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(1),
                is_positive: true
            }));

    // Second subharmonic
    assert_eq!(iter.next(),
        Some(
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(2),
                is_positive: false
            }));

    // Working harmonic
    assert_eq!(iter.next(),
        Some(
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(4),
                is_positive: true
            }));

    // First superharmonic after the working harmonic
    assert_eq!(iter.next(),
        Some(
            WindingHarmonic {
                spatial_order: SpatialOrder::Mechanical(5),
                is_positive: false
            }));
    ```
     */
    fn harmonics(&self) -> WindingHarmonicsIterator<'_> {
        return WindingHarmonicsIterator::new(self.as_dyn());
    }

    /**
    Checks whether the field excitation curve of the winding contains a
    nonzero component of the given `spatial_order`. If so, returns the
    direction of the resulting wave relative to the current system:
    `Some(true)` if co-rotating, `Some(false)` if counter-rotating, or
    `None` if no such component exists.

    The general mathematical treatment of the field excitation curve is based
    on its Fourier decomposition into its individual orders.
    In [\[1\]](#is_harmonic_1), the wave equation for the `i`th phase and a
    spatial order `v'_mech` is given as:

    `V_i(γ', t) = V_amp * cos(v'_mech*(γ' - 2π * i / m)) * cos(ω * t - 2π * i / m)` [\[1\]](#is_harmonic_1), eq. 3.2-24 to 3.2-26

    The amplitude `V_amp` of the wave is:

    `V_amp = n_i * i_amp / (2 * p) * 4 / (v'_mech * π) * k_w` [\[1\]](#is_harmonic_1), eq. 3.2.20

    with:
    * `γ'`: Spatial coordinate scaled by [`base_winding_count`](Winding::base_winding_count)
    * `v'_mech`: Mechanical spatial order divided by the [`base_winding_count`](Winding::base_winding_count)
    * `m`: Number of [`Winding::phases`]
    * `ω`: Angular frequency of the supply current
    * `n_i`: [`series_turns_per_phase`](Winding::series_turns_per_phase)
    * `i_amp`: Amplitude of the feeding current
    * `p`: Number of [`Winding::pole_pairs`]
    * `k_w`: [`winding_factor`](Winding::winding_factor) for `v'_mech` and phase `i`.

    `V_i(γ', t)` describes a stationary, pulsating wave. As shown in
    [\[1\]](#is_harmonic_1), section 2.5 it can be written as the sum of a
    co- and a counter-rotating wave relative to the current system:

    ```text
    V_i(γ', t) = V_amp / 2 * (
            cos(v'_mech*γ' - v'_mech * 2π * i / m + ω * t - 2π * i / m)
          + cos(v'_mech*γ' - v'_mech * 2π * i / m - ω * t + 2π * i / m))
    ```
    [\[1\]](#is_harmonic_1), eq. 3.3-12 to 3.3-13

    When summing the contributions of all phases, the co- and counter-rotating
    components may cancel. If both do cancel, the winding does not have a
    component of the given `spatial_order` in its field excitation curve and
    this function returns `None`.

    If the co-rotating part cancels, the resulting wave for the `spatial_order`
    counter-rotates relative to the current system and this method
    returns `Some(false)`. If the counter-rotating part cancels, likewise the
    resulting wave co-rotates relative to the current system and this method
    returns `Some(true)`.

    # Literature
    <a id="is_harmonic_1">\[1\]</a>
    Binder, A.: Elektrische Maschinen und Antriebe, 1st edition, Springer
    Heidelberg, 2012

    # Examples

    The following example shows a fractional-slot winding where the working
    harmonic is not the first one, but the fourth one:

    ```
    use stem_winding::prelude::*;

    let winding: DistributedWinding = DistributedMinimalBuilder {
        slots: 18.try_into().expect("not zero"),
        pole_pairs: 4.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 1.try_into().expect("not zero"),
        coil_span_reduction: 0,
        zone_span_variation: 0,
        winding_table_constructor: WindingTableConstructor::CoilSide,
    }
    .try_into()
    .unwrap();

    // First spatial harmonic is co-rotating.
    assert_eq!(winding.is_harmonic(SpatialOrder::Mechanical(1)), Some(true));

    // Second spatial harmonic is counter-rotating
    assert_eq!(winding.is_harmonic(SpatialOrder::Mechanical(2)), Some(false));

    // Third harmonic is absent. This is typical for symmetric three-phase
    // machines, as all spatial orders divisible by the number of phases result
    // in cancellation of both the co-rotating and the counter-rotating wave.
    assert_eq!(winding.is_harmonic(SpatialOrder::Mechanical(3)), None);

    // Fourth harmonic is co-rotating. Since this spatial order is the same as
    // the number of pole pairs, the continuous torque will be generated by this
    // harmonic.
    assert_eq!(winding.is_harmonic(SpatialOrder::Mechanical(4)), Some(true));

    // Fifth spatial harmonic is counter-rotating.
    assert_eq!(winding.is_harmonic(SpatialOrder::Mechanical(5)), Some(false));
    ```
     */
    fn is_harmonic(&self, spatial_order: SpatialOrder) -> Option<bool> {
        let mech_order = spatial_order.to_mechanical(self.pole_pairs());
        let winding_factor_all_phases = if self.equal_winding_factors() {
            Some(self.winding_factor(NonZeroU16::MIN, spatial_order))
        } else {
            None
        };
        is_harmonic(
            self,
            winding_factor_all_phases,
            self.phases().get(),
            self.base_winding_count().get(),
            mech_order,
        )
    }

    /**
    Returns a relative measure of the magnetic induction amplitude
    associated with a spatial order.

    The returned value is proportional to the amplitude of the corresponding
    air-gap magnetic induction component, normalized to a current of 1 A and one
    series turn per phase. It is intended only for comparing different
    spatial orders of the same winding; values must not be compared
    across different windings to infer their relative magnetic induction
    amplitudes.

    The value is calculated as

    `B_norm = k_w * p / v_mech`,

    where `k_w` is the winding factor for the given spatial order, `p` is
    the number of pole pairs, and `v_mech` is the mechanical spatial
    order. If the spatial order is not present in the field excitation
    curve, the method returns zero.

    This quantity does not represent the magnetic induction amplitude, which
    also depends on parameters such as the air-gap width or the reluctance of
    the magnetic core itself. To get an idealized magnetic induction amplitude
    `B`, multiply the returned value with:

    `B = B_norm * m / 2 * μ0 / δ * w * I` [\[1\]](#is_harmonic_1), eq. 2.5-17 and p. 83

    where `m` is the [`Winding phases`] number, `μ0` is the vacuum permeability,
    `w` is the [`series_turns_per_phase`](Winding::series_turns_per_phase),
    `δ` is the magnetically effective air gap width and `I` is the RMS current.

    This formula assumes infinitely permeable core material, a uniform, smooth
    air gap without slotting and a [symmetric](Winding::is_symmetric) winding.

    # Literature
    <a id="is_harmonic_1">\[1\]</a>
    Binder, A.: Elektrische Maschinen und Antriebe, 1st edition, Springer
    Heidelberg, 2012

    # Examples

    This example uses the same winding as [`Winding::is_harmonic`].
    The subharmonics have a noticeable impact on the air-gap field, while
    the first superharmonic has a comparatively small amplitude. The fourth
    spatial harmonic is the working harmonic. When designing a machine with
    this winding, it is therefore worth considering measures to reduce the
    subharmonic contributions.

    ```
    use std::num::NonZeroU16;

    use approxim::assert_abs_diff_eq;
    use stem_winding::prelude::*;

    let winding: DistributedWinding = DistributedMinimalBuilder {
        slots: 18.try_into().expect("not zero"),
        pole_pairs: 4.try_into().expect("not zero"),
        phases: 3.try_into().expect("not zero"),
        layers: 1.try_into().expect("not zero"),
        coil_span_reduction: 0,
        zone_span_variation: 0,
        winding_table_constructor: WindingTableConstructor::CoilSide,
    }
    .try_into()
    .unwrap();

    let phase_1 = NonZeroU16::MIN;

    // First subharmonic
    assert_abs_diff_eq!(
        winding.relative_induction_amplitude(phase_1, SpatialOrder::Mechanical(1)),
        0.666666,
        epsilon = 1e-6
    );

    // Second subharmonic
    assert_abs_diff_eq!(
        winding.relative_induction_amplitude(phase_1, SpatialOrder::Mechanical(2)),
        0.279699,
        epsilon = 1e-6
    );

    // Absent harmonic
    assert_abs_diff_eq!(
        winding.relative_induction_amplitude(phase_1, SpatialOrder::Mechanical(3)),
        0.0,
        epsilon = 1e-6
    );

    // Working harmonic
    assert_abs_diff_eq!(
        winding.relative_induction_amplitude(phase_1, SpatialOrder::Mechanical(4)),
        0.945213,
        epsilon = 1e-6
    );

    // First superharmonic
    assert_abs_diff_eq!(
        winding.relative_induction_amplitude(phase_1, SpatialOrder::Mechanical(5)),
        0.133333,
        epsilon = 1e-6
    );
    ```
    */
    fn relative_induction_amplitude(&self, phase: NonZeroU16, spatial_order: SpatialOrder) -> f64 {
        let k_w = self.winding_factor(phase, spatial_order);

        let winding_factor_all_phases = if self.equal_winding_factors() {
            Some(k_w)
        } else {
            None
        };
        let mech_order = spatial_order.to_mechanical(self.pole_pairs());
        let is_harmonic = is_harmonic(
            self,
            winding_factor_all_phases,
            self.phases().get(),
            self.base_winding_count().get(),
            mech_order,
        );

        if is_harmonic.is_some() {
            let p = self.pole_pairs();
            return k_w * f64::from(p.get()) / f64::from(spatial_order.to_mechanical(p));
        } else {
            return 0.0;
        }
    }

    /// Returns an iterator over all [`Coil`]s of the winding.
    ///
    /// Each coil is returned exactly once, regardless of how many winding zones
    /// it occupies.
    ///
    /// # Examples
    ///
    /// This example uses both [`FullCoil`]s and [`HalfCoil`]s.
    ///
    /// ```
    /// use std::num::{NonZeroUsize, NonZeroU16};
    /// use stem_winding::prelude::*;
    ///
    /// let mut winding = CoilAssembly::new_minimal(
    ///     6.try_into().expect("not zero"),
    ///     1.try_into().expect("not zero"),
    ///     3.try_into().expect("not zero"),
    ///     CoilLayout::Single,
    ///     Coils::new(),
    /// )
    /// .unwrap();
    ///
    /// // Add a full coil and a half coil
    /// winding
    ///     .insert(FullCoil::new(
    ///         Zone::new(0, 0),
    ///         Zone::new(1, 0),
    ///         true,
    ///         NonZeroUsize::MIN,
    ///         NonZeroU16::MIN,
    ///         Box::new(SffWire::default()),
    ///     )
    ///     .expect("zones identical")).unwrap();
    /// winding
    ///     .insert(HalfCoil::new(
    ///         Zone::new(2, 0),
    ///         true,
    ///         1.try_into().expect("not zero"),
    ///         2.try_into().expect("not zero"),
    ///         Box::new(SffWire::default()),
    ///     )).unwrap();
    ///
    /// // The total number of coils is 2.
    /// assert_eq!(winding.coils_iter().count(), 2);
    /// ```
    fn coils_iter(&self) -> CoilsIterator<'_> {
        return CoilsIterator::new(self.as_dyn());
    }

    /// Returns the main inductance of the specified phase for the given spatial
    /// order.
    ///
    /// The calculation assumes that:
    /// - The air gap is smooth.
    /// - The stator and rotor cores have infinite permeability, so the
    ///   reluctance of the air gap dominates.
    ///
    /// Under this assumption, the main inductance for the `spatial_order` can
    /// be calculated via [\[1\]](#main_inductance_1), eq. (4.3-5):
    ///
    /// `L_hν = μ0 * N_s² * k_w,ν² * l_ag * l_ax * m / (δ * ν² * p² * π²)`
    ///
    /// where `μ0` is the [`VACUUM_PERMEABILITY`], `N_s` is the
    /// [`series_turns_per_phase`](Winding::series_turns_per_phase), `k_w,ν` is
    /// the [`effective_winding_factor`](Winding::effective_winding_factor) for
    /// the spatial electrical order `ν`, `l_ag` is the air-gap circumference
    /// for a rotary core or the width for a linear core
    /// ([`CoreExt::air_gap_length`]), `l_ax` is the [`CoreExt::axial_length`],
    /// `δ` is the `effective_air_gap` and `p` is the number of
    /// [`pole_pairs`](Winding::pole_pairs).
    ///
    /// This inductance is used when calculating the
    /// [`air_gap_leakage_inductance`]. The assumptions above also make this
    /// model suitable for a first approximation of induction machines. For
    /// synchronous machines, the d-axis and q-axis inductances generally
    /// differ, an effect that this model does not capture. This inaccuracy
    /// can be accepted when calculating the [`air_gap_leakage_inductance`]
    /// of synchronous machines because their larger air gap generally
    /// results in a smaller leakage inductance than in induction machines.
    ///
    /// # Literature
    ///
    /// <a id="main_inductance_1">[\[1\]]</a>
    /// Binder, A.: Elektrische Maschinen und Antriebe, 1st edition, Springer
    /// Heidelberg, 2012
    fn main_inductance(
        &self,
        core: CoreRef<'_>,
        phase: NonZeroU16,
        spatial_order: SpatialOrder,
        effective_air_gap: Length,
        conditions: &[DynQuantity<f64>],
    ) -> Inductance {
        // Conditions are here to allow their usage when overriding this method.
        let _ = conditions;

        let k_w = self.effective_winding_factor(core, phase, spatial_order, effective_air_gap);

        let p = f64::from(self.pole_pairs().get());

        let electrical_spatial_order: f64 = match spatial_order {
            SpatialOrder::Mechanical(mech_o) => f64::from(mech_o) / p,
            SpatialOrder::Electrical(el_o) => el_o.into(),
        };

        f64::from(self.phases().get())
            * *VACUUM_PERMEABILITY
            * core.air_gap_length()
            * core.axial_length()
            * k_w.powi(2)
            * self.series_turns_per_phase_float(phase).powi(2)
            / (effective_air_gap * (PI * p * electrical_spatial_order).powi(2))
    }

    /// Returns the air-gap leakage inductance.
    ///
    /// This is the product of [`Winding::main_inductance`] and
    /// [`Winding::air_gap_leakage_factor`]. It quantifies the self-inductance
    /// associated with higher-order magnetic field harmonics resulting from the
    /// [`field_excitation_curve`](Winding::field_excitation_curve). See
    /// [\[1\]](#air_gap_leakage_inductance_1), section 3.7.3, for details.
    ///
    /// # Literature
    ///
    /// <a id="air_gap_leakage_inductance_1">[\[1\]]</a>
    /// Müller, G., Vogt, K. and Ponick, B.: *Berechnung elektrischer
    /// Maschinen*, 6th edition, Wiley-VCH, 2008
    fn air_gap_leakage_inductance(
        &self,
        core: CoreRef<'_>,
        phase: NonZeroU16,
        spatial_order: SpatialOrder,
        effective_air_gap: Length,
        conditions: &[DynQuantity<f64>],
    ) -> Inductance {
        let main_inductance =
            self.main_inductance(core, phase, spatial_order, effective_air_gap, conditions);
        let leakage_factor = self.air_gap_leakage_factor(phase);
        return main_inductance * leakage_factor;
    }

    /// Calculates the slot leakage inductance of the specified `phase` for a
    /// winding supplied by a symmetric sinusoidal multiphase system.
    ///
    /// The slot leakage inductance arises from leakage flux produced by
    /// currents flowing through conductors in the slots of the core. The
    /// time variation of this flux induces a voltage in the winding. A
    /// similar effect occurs for conductors located in the air gap, but its
    /// contribution is usually negligible. The slot leakage inductance is
    /// zero if [`CoreExt::slot`] returns `None`.
    ///
    /// The induced voltage is described by
    ///
    /// `u(t) = L di(t) / dt`
    ///
    /// where `L` is the slot leakage inductance. The following explanation
    /// of its calculation is based on [\[1\]](#slot_leakage_inductance_1),
    /// section 3.5.2.
    ///
    /// The calculation proceeds on a per-slot basis. For each slot, the
    /// algorithm iterates over its layers and considers each layer containing
    /// a coil belonging to `phase`. For each such layer ("linked layer"), it
    /// evaluates the coupling with every layer ("excitation layer") in the same
    /// slot using the [`Slot::leakage_coefficient_matrix`]. The
    /// matrix provides a dimensionless leakage coefficient for each
    /// combination of linked and excitation layers. The coefficients from
    /// [`Slot::leakage_coefficient_opening`] and
    /// [`Slot::leakage_coefficient_tooth_tip`] are added to obtain the total
    /// leakage coefficient, which is then multiplied by tthe sign of the
    /// product of the signed phase assignments of the two layers.
    ///
    /// The contribution is then scaled by the normalized current in the
    /// excitation layer, as calculated by [`multiphase_value`], with the
    /// offset chosen such that the current of `phase` is 1 at `t = 0`.
    /// The result is multiplied by the sign of the product of the phases
    /// of the two layers: a positive product increases the inductance,
    /// whereas a negative product decreases it.
    ///
    /// The contributions from all layer combinations and slots are summed
    /// to obtain the slot leakage inductance of the specified phase.
    ///
    /// The `conditions` parameter may affect the inductance. The
    /// `effective_air_gap` parameter is used by
    /// [`Slot::leakage_coefficient_tooth_tip`]; see its documentation for
    /// details.
    ///
    /// # Literature
    ///
    /// <a id="slot_leakage_inductance_1">[\[1\]]</a>
    /// Müller, G., Vogt, K. and Ponick, B.: *Berechnung elektrischer
    /// Maschinen*, 6th edition, Wiley-VCH, 2008
    ///
    /// # Examples
    ///
    /// The following example compares the result of this algorithm with the
    /// analytical formula [\[1\]](#slot_leakage_inductance_1), eq. (3.7.15),
    /// for a symmetric, short-pitched distributed winding. This
    /// demonstrates that eq. (3.7.15) is a special case of the
    /// general-purpose algorithm.
    ///
    /// ```
    /// use std::num::NonZeroU16;
    /// use std::sync::Arc;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// // Use a rectangular slot to compare the numerical result with the analytical formula.
    /// let opening_height = Length::new::<millimeter>(2.0);
    /// let opening_width = Length::new::<millimeter>(3.0);
    /// let width = Length::new::<millimeter>(3.0);
    /// let height = Length::new::<millimeter>(20.0);
    /// let slot = RectangularSlot::new(width, opening_width, height, opening_height, true).expect("valid inputs");
    /// let core: RotCore = RotCoreBuilder {
    ///     air_gap_radius: Length::new::<millimeter>(55.0),
    ///     yoke_radius: Length::new::<millimeter>(85.0),
    ///     axial_length: Length::new::<millimeter>(165.0),
    ///     axial_coil_overhang: Length::new::<millimeter>(0.0),
    ///     iron_fill_factor: 0.95,
    ///     material: Arc::new(Material::default()),
    ///     pole_pairs: 2.try_into().expect("not zero"),
    ///     skew_angle: 0.0,
    ///     air_gap: Box::new(SlottedAirGap::new(
    ///         36.try_into().expect("not zero"),
    ///         true,
    ///         CarterFactorModel::Bin12,
    ///         Box::new(slot),
    ///     )),
    ///     flux_barrier: None,
    /// }
    /// .try_into()
    /// .expect("valid magnetic core");
    ///
    /// let winding: DistributedWinding = DistributedBuilder {
    ///     slots: 36.try_into().expect("not zero"),
    ///     pole_pairs: 2.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 2.try_into().expect("not zero"),
    ///     coil_span_reduction: 1, // Short-pitching
    ///     zone_span_variation: 0,
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    ///     turns_per_coil: 10.try_into().expect("not zero"),
    ///     parallel_paths: 1.try_into().expect("not zero"),
    ///     connection: Connection::Star,
    ///     end_winding_leakage_coefficient: 0.25,
    ///     wire: Box::new(SffWire::default()),
    ///     concentric_coils: false,
    ///     mean_end_winding_half_turn_length: None,
    /// }
    /// .try_into()
    /// .expect("valid winding core");
    ///
    /// // Assumed air gap (needed for the tooth-tip leakage)
    /// let air_gap = Length::new::<millimeter>(1.0);
    ///
    /// // Slot leakage coefficient from [1], eq. (3.7.15)
    /// let span = winding.pole_pitch() - winding.coil_span_reduction() as f64;
    ///
    /// // Analytical slot leakage coefficient for the special case of a rectangular slot.
    /// let lambda_l = f64::from((height - opening_height) / (3.0 * width));
    /// let lambda_o = f64::from(opening_height / opening_width) + core.slot().unwrap().leakage_coefficient_tooth_tip(air_gap);
    /// let lambda = (1.0 - 9.0 / 16.0 * (1.0 - span as f64 / winding.pole_pitch())) * lambda_l
    ///     + (1.0 - 3.0 / 4.0 * (1.0 - span as f64 / winding.pole_pitch())) * lambda_o;
    ///
    /// // Slot leakage inductance from [1], eq. (3.7.15)
    /// let analytical_inductance = 2.0
    ///     * *VACUUM_PERMEABILITY
    ///     * core.axial_coil_length()
    ///     * winding.series_turns_per_phase_float(NonZeroU16::MIN).powi(2)
    ///     / (winding.pole_pairs().get() as f64 * winding.hole_number_float())
    ///     * lambda;
    ///
    /// assert_abs_diff_eq!(analytical_inductance.get::<henry>(), 0.0024743, epsilon = 1e-7);
    /// for ph in 1..4 {
    ///     let phase = ph.try_into().unwrap();
    ///     let calculated_inductance = winding.slot_leakage_inductance(
    ///         CoreRef::Rot(&core),
    ///         phase,
    ///         air_gap,
    ///         &[],
    ///     );
    ///
    ///     assert_abs_diff_eq!(
    ///         analytical_inductance.get::<henry>(),
    ///         calculated_inductance.get::<henry>(),
    ///         epsilon = 1e-7
    ///     );
    /// }
    /// ```
    #[cfg(feature = "stem_core")]
    fn slot_leakage_inductance(
        &self,
        core: CoreRef<'_>,
        phase: NonZeroU16,
        effective_air_gap: Length,
        conditions: &[DynQuantity<f64>],
    ) -> Inductance {
        // Argument for usage in overrides, for the default implementation it isn't
        // needed.
        let _ = conditions;

        let slot = match core.slot() {
            Some(s) => s,
            None => return Inductance::new::<si::inductance::henry>(0.0),
        };

        // Opening and tooth tip inductance are independent of the layer configuration
        let lambda_opening_and_tooth_tip = slot.leakage_coefficient_opening()
            + slot.leakage_coefficient_tooth_tip(effective_air_gap);

        // Calculate the total slot leakage inductance by iterating over all slots of a
        // basic winding and calculating the sum of the slot flux leakage inductance.
        let number_basic_slots = self.slots().get() / self.base_winding_count();
        let slots = 0..number_basic_slots;
        let number_layers = self.layers().get();
        let number_layers_squared = number_layers.pow(2);

        // Set the phase offset so that the current of `phase` is always 1.
        let phase_offset = TAU / f64::from(self.phases().get()) * f64::from(phase.get() - 1);

        // Precalculate the leakage coefficient matrix
        let lambda_slot = slot.leakage_coefficient_matrix(&self.coil_layout());

        // Precalculate the product of axial length and vacuum permeability
        let single_turn_inductance = *VACUUM_PERMEABILITY * core.axial_coil_length();

        // Calculate the total slot inductance for the selected phase
        let basic_winding_slot_inductance: Inductance = slots
            .into_par_iter()
            .map(|slot_idx| {
                // Calculate the self-inductance of a single slot according to [1], section
                // 3.5.2.1 (p. 311)
                return (0..number_layers_squared)
                    .into_iter()
                    .map(|lin_idx| {
                        let [linked_layer, excitation_layer] = cart_lin::lin_to_cart_unchecked(
                            lin_idx.into(),
                            &[number_layers.into(), number_layers.into()],
                        );
                        let linked_layer = u16::try_from(linked_layer)
                            .expect("the input values to lin_to_cart must be in the u16 range");
                        let excitation_layer = u16::try_from(excitation_layer)
                            .expect("the input values to lin_to_cart must be in the u16 range");

                        if let Some(linked_coil) = self.coil_at(Zone::new(slot_idx, linked_layer)) {
                            // Only consider the selected phase
                            if linked_coil.phase() != phase {
                                return Inductance::default();
                            }

                            if let Some(excitation_coil) =
                                self.coil_at(Zone::new(slot_idx, excitation_layer))
                            {
                                // Calculate the coupling direction between the linked coil and
                                // the excitation coil
                                let coupling =
                                    self.phase_at(Zone::new(slot_idx, linked_layer)).signum()
                                        * self
                                            .phase_at(Zone::new(slot_idx, excitation_layer))
                                            .signum();

                                // Get the normalized excitation current and modify its
                                // direction according to the coupling calculated above
                                let excitation_current = multiphase_value(
                                    Time::new::<second>(0.0),
                                    Frequency::new::<hertz>(0.0),
                                    phase_offset,
                                    excitation_coil.phase(),
                                    self.phases(),
                                );

                                // Calculate the inductance
                                return single_turn_inductance
                                    * linked_coil.turns().get() as f64
                                    * excitation_coil.turns().get() as f64
                                    * excitation_current
                                    * f64::from(coupling)
                                    * (lambda_slot
                                        [(linked_layer as usize, excitation_layer as usize)]
                                        + lambda_opening_and_tooth_tip);
                            } else {
                                return Inductance::default();
                            }
                        } else {
                            return Inductance::default();
                        }
                    })
                    .sum::<Inductance>();
            })
            .sum();

        // Scale with the winding base_winding_count and the number of parallel paths
        return basic_winding_slot_inductance * self.base_winding_count().get() as f64
            / self.parallel_paths(phase).get() as f64;
    }

    /// Returns the total air-gap inductance.
    ///
    /// This is the sum of [`Winding::main_inductance`] and
    /// [`Winding::air_gap_leakage_inductance`].
    fn air_gap_inductance(
        &self,
        core: CoreRef<'_>,
        phase: NonZeroU16,
        spatial_order: SpatialOrder,
        effective_air_gap: Length,
        conditions: &[DynQuantity<f64>],
    ) -> Inductance {
        self.main_inductance(core, phase, spatial_order, effective_air_gap, conditions)
            + self.air_gap_leakage_inductance(
                core,
                phase,
                spatial_order,
                effective_air_gap,
                conditions,
            )
    }

    /// Returns the total phase leakage inductance for the given spatial order.
    ///
    /// This is the sum of the [`Winding::air_gap_leakage_inductance`],
    /// [`Winding::end_winding_leakage_inductance`] and
    /// [`Winding::slot_leakage_inductance`]. See their respective documentation
    /// for details on how each contribution is calculated.
    fn leakage_inductance(
        &self,
        core: CoreRef<'_>,
        phase: NonZeroU16,
        spatial_order: SpatialOrder,
        effective_air_gap: Length,
        conditions: &[DynQuantity<f64>],
    ) -> Inductance {
        self.air_gap_leakage_inductance(core, phase, spatial_order, effective_air_gap, conditions)
            + self.end_winding_leakage_inductance(core, phase, spatial_order, conditions)
            + self.slot_leakage_inductance(core, phase, effective_air_gap, conditions)
    }

    /// Returns the total phase inductance for the given spatial order.
    ///
    /// This is the sum of the [`Winding::air_gap_inductance`],
    /// [`Winding::end_winding_leakage_inductance`] and
    /// [`Winding::slot_leakage_inductance`]. See their respective documentation
    /// for details on how each contribution is calculated.
    fn inductance(
        &self,
        core: CoreRef<'_>,
        phase: NonZeroU16,
        spatial_order: SpatialOrder,
        effective_air_gap: Length,
        conditions: &[DynQuantity<f64>],
    ) -> Inductance {
        self.air_gap_inductance(core, phase, spatial_order, effective_air_gap, conditions)
            + self.end_winding_leakage_inductance(core, phase, spatial_order, conditions)
            + self.slot_leakage_inductance(core, phase, effective_air_gap, conditions)
    }

    /// Returns an iterator over all coils of the winding and their physical
    /// properties.
    ///
    /// This method uses [`Winding::coils_iter`] to iterate over all coils of
    /// the winding. Each coil is wrapped in a
    /// [`CoilProperties`](crate::core_support::CoilProperties) which is then
    /// returned as an iterator item.
    ///
    /// See [`Winding::coil_properties_at`] for details.
    ///
    /// # Examples
    ///
    /// Calculate the mass of a ll coils of the winding.
    ///
    /// ```
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let winding: ToothCoilWinding = ToothCoilMinimalBuilder {
    ///     slots: 12.try_into().expect("not zero"),
    ///     pole_pairs: 5.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 1.try_into().expect("not zero"),
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    /// }
    /// .try_into()
    /// .unwrap();
    ///
    /// let core = RotCore::from_winding(&winding);
    ///
    /// // Calculate the total mass of all coils
    /// let mass: Mass = winding
    ///     .coil_properties_iter(core.as_core_ref())
    ///     .map(|cp| cp.volume() * cp.material().mass_density().get(&[])).sum();
    ///
    /// assert_abs_diff_eq!(mass.get::<kilogram>(), 0.02142, epsilon = 0.0001);
    /// ```
    #[cfg(feature = "stem_core")]
    fn coil_properties_iter<'a>(
        &'a self,
        core: CoreRef<'a>,
    ) -> crate::core_support::CoilPropertyIterator<'a> {
        return crate::core_support::CoilPropertyIterator {
            coils: self.coils_iter(),
            winding: self.as_dyn(),
            core,
        };
    }

    /// Returns a [`CoilProperties`] context for the [`Coil`] containing the
    /// specified [`Zone`]. The returned `CoilProperties` combines the coil with
    /// this winding, the specified [`CoreRef`], and the supplied end-winding
    /// half-turn length overrides.
    ///
    /// These values are used by the physical-property calculations provided by
    /// [`CoilProperties`].
    ///
    /// Returns `None` if the specified zone does not contain a coil.
    #[cfg(feature = "stem_core")]
    fn coil_properties_at<'a>(
        &'a self,
        core: CoreRef<'a>,
        zone: Zone,
    ) -> Option<crate::core_support::CoilProperties<'a>> {
        let coil = self.coil_at(zone)?;
        return Some(crate::core_support::CoilProperties::new(
            coil,
            self.as_dyn(),
            core,
            None,
        ));
    }

    /// Returns the phase resistance of the winding.
    ///
    /// The phase resistance is calculated as
    ///
    /// `R = 1 / a² * Σ R_coil,i`,
    ///
    /// where `a` is the number of [parallel paths](Winding::parallel_paths) of
    /// the phase and `R_coil,i` is the
    /// [resistance of an individual coil](CoilProperties::resistance), with `i`
    /// indexing all coils belonging to that phase. This calculation assumes
    /// that the parallel paths are electrically balanced.
    ///
    /// The resistance of each coil is calculated by [`CoilExt::resistance`],
    /// which requires the zone area and the turn length. For each [`Zone`]
    /// occupied by the coil, the zone area is obtained from core via
    /// [`CoreExt::winding_zone_at`]. The turn length associated with the zone
    /// is calculated as
    ///
    /// `turn_length = axial_coil_length + axial_coil_overhang +
    /// end_winding_half_turn_length`.
    ///
    /// Here, `axial_coil_length` and `axial_coil_overhang` are obtained from
    /// `core` via [`CoreExt::axial_coil_length`] and
    /// [`CoreExt::axial_coil_overhang`], respectively. The
    /// `end_winding_half_turn_length` value comes from
    /// [`Winding::end_winding_half_turn_length`].
    ///
    /// The specified environmental conditions are forwarded to
    /// [`CoilExt::resistance`].
    ///
    /// # Examples
    ///
    /// The following example calculates the phase resistance in the prototype
    /// machine developed during the author's PhD [\[1\]](#resistance_1).
    /// The geometry, winding, and conductor data are constructed explicitly,
    /// and the result is compared with the measured resistance of 1.0176 Ω
    /// at 22 °C.
    ///
    /// <a id="resistance_1">\[1\]</a>
    /// Mathis, S.: Permanentmagneterregte Line-Start-Antriebe in Ferrittechnik,
    /// PhD thesis, Shaker, 2019
    #[doc = ""]
    #[cfg_attr(
        feature = "doc-images",
        doc = "![Stator with winding table of the PhD motor][phd_stator]"
    )]
    #[cfg_attr(
        feature = "doc-images",
        embed_doc_image::embed_doc_image("phd_stator", "docs/img/phd_stator.svg")
    )]
    #[cfg_attr(
        not(feature = "doc-images"),
        doc = "**Doc images not enabled**. Compile docs with
        `cargo doc --features 'doc-images'` and Rust version >= 1.54."
    )]
    /// ```
    /// use std::str::FromStr;
    /// use std::sync::Arc;
    /// use std::f64::consts::PI;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    /// use stem_winding::var_quantity::unary::FirstOrderTaylor;
    ///
    /// // Define a material with an electrical resistivity which changes linearly
    /// // with the temperature.
    /// let copper = {
    ///     let mut material = Material::default();
    ///     material.electrical_resistivity = VarQuantity::new(
    ///         FirstOrderTaylor::new(
    ///             DynQuantity::from_str("1 / 56 m/MS").expect("parseable"),
    ///             DynQuantity::from_str("0.393 % / K").expect("parseable"),
    ///             DynQuantity::from_str("20.0 °C").expect("parseable"),
    ///         )
    ///         .expect("units match")
    ///     ).expect("units match");
    ///     Arc::new(material)
    /// };
    ///
    /// // Create a stranded wire composed of two wire types. wire_1 occurs twice
    /// // and wire_2 occurs three times in the strand.
    /// let wire_1 = RoundWire::new(
    ///     copper.clone(),
    ///     Length::new::<millimeter>(0.67),
    ///     Length::new::<millimeter>(0.0),
    ///     Length::new::<millimeter>(0.0),
    /// )
    /// .expect("valid_geometry");
    /// let wire_2 = RoundWire::new(
    ///     copper.clone(),
    ///     Length::new::<millimeter>(0.71),
    ///     Length::new::<millimeter>(0.0),
    ///     Length::new::<millimeter>(0.0),
    /// )
    /// .expect("valid_geometry");
    /// let wire = StrandedWire::new(vec![
    ///     WireGroup::new(Box::new(wire_1), 2.try_into().expect("not zero")),
    ///     WireGroup::new(Box::new(wire_2), 3.try_into().expect("not zero")),
    /// ]).expect("not empty");
    ///
    /// // Define the winding using the full constructor
    /// let winding: DistributedWinding = DistributedBuilder {
    ///     slots: 36.try_into().expect("not zero"),
    ///     pole_pairs: 2.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 1.try_into().expect("not zero"),
    ///     coil_span_reduction: 0,
    ///     zone_span_variation: 0,
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    ///     turns_per_coil: 31.try_into().expect("not zero"),
    ///     parallel_paths: 1.try_into().expect("not zero"),
    ///     connection: Connection::Star,
    ///     end_winding_leakage_coefficient: 0.25,
    ///     wire: Box::new(wire),
    ///     concentric_coils: false,
    ///     mean_end_winding_half_turn_length: None,
    /// }
    /// .try_into()
    /// .unwrap();
    ///
    /// // Create the core of the machine
    /// let slot: SemiTrapezoidSlot = SemiTrapezoidWithoutSlopesBuilder {
    ///     bottom_width: Length::new::<millimeter>(9.2),
    ///     opening_width: Length::new::<millimeter>(2.0),
    ///     height: Length::new::<millimeter>(17.75),
    ///     opening_height: Length::new::<millimeter>(2.0),
    ///     slot_angle: PI / 18.0,
    ///     bottom_radius: Length::new::<millimeter>(2.0),
    ///     top_radius: Length::new::<millimeter>(2.0),
    ///     opening_radius: Length::new::<millimeter>(0.5),
    ///     consider_tooth_tip_leakage: false,
    ///     }.try_into().expect("valid slot geometry");
    /// let core: RotCore = RotCoreBuilder {
    ///     air_gap_radius: Length::new::<millimeter>(55.0),
    ///     yoke_radius: Length::new::<millimeter>(85.0),
    ///     axial_length: Length::new::<millimeter>(165.0),
    ///     axial_coil_overhang: Length::new::<millimeter>(0.0),
    ///     iron_fill_factor: 0.95,
    ///     material: Arc::new(Material::default()),
    ///     pole_pairs: 2.try_into().expect("not zero"),
    ///     skew_angle: 0.0,
    ///     air_gap: Box::new(SlottedAirGap::new(
    ///         36.try_into().expect("not zero"),
    ///         true,
    ///         CarterFactorModel::Bin12,
    ///         Box::new(slot),
    ///     )),
    ///     flux_barrier: None,
    /// }.try_into().expect("valid magnetic core");
    ///
    /// // The measured phase resistance of this machine was 1.0176 Ω at 22 °C. The
    /// // calculated value at 22 °C is 1.05673 Ω, corresponding to a deviation of
    /// // 4 %. This deviation may stem from the following assumptions:
    /// //
    /// // 1) Material data: The electrical resistivity of the copper or the
    /// // temperature relationship might be slightly off.
    /// //
    /// // 2) End winding length: Here, we use a calculated value, which is an
    /// // approximation of an idealized end winding geometry.
    /// let conditions = [DynQuantity::from_str("22 °C").expect("parseable")];
    /// assert_abs_diff_eq!(
    ///     winding
    ///         .resistance(
    ///             CoreRef::Rot(&core),
    ///             1.try_into().unwrap(), // First phase
    ///             &conditions,
    ///         )
    ///     .get::<ohm>(),
    ///     1.05673,
    ///     epsilon = 0.0001
    /// );
    ///
    /// // Calculate the phase resistance at 120 °C.
    /// assert_abs_diff_eq!(
    ///     winding
    ///         .resistance(
    ///             CoreRef::Rot(&core),
    ///             1.try_into().unwrap(), // First phase
    ///             &[DynQuantity::from_str("120 °C").expect("parseable")],
    ///         )
    ///     .get::<ohm>(),
    ///     1.46055,
    ///     epsilon = 0.0001
    /// );
    /// ```
    #[cfg(feature = "stem_core")]
    fn resistance(
        &self,
        core: CoreRef<'_>,
        phase: NonZeroU16,
        conditions: &[DynQuantity<f64>],
    ) -> ElectricalResistance {
        // Calculate the phase resistance by calculating the phase resistance of each
        // individual coil and then summing them up
        let mut resistance = ElectricalResistance::new::<ohm>(0.0);

        for coil in self.coils_iter() {
            if coil.phase() == phase {
                for zone in coil.zones() {
                    if let Some(zone_contour) = core.winding_zone_at(&self.coil_layout(), zone) {
                        let zone_area = Area::new::<square_meter>(zone_contour.area());

                        let end_winding_half_turn_length =
                            self.end_winding_half_turn_length(core, zone);
                        let length = core.axial_coil_length()
                            + core.axial_coil_overhang()
                            + end_winding_half_turn_length;
                        resistance += coil.resistance(zone_area, length, conditions);
                    }
                }
            }
        }
        let parallel_paths = f64::from(self.parallel_paths(phase).get());
        return resistance / parallel_paths.powi(2);
    }

    /// Returns the [`ResistanceDecomposition`] of the phase
    /// [`resistance`](Winding::resistance).
    ///
    /// The phase resistance is decomposed as
    ///
    /// `R = k_R * rho`,
    ///
    /// where `R` is the phase resistance, `k_R` is a resistance constant, and
    /// `rho` is the electrical resistivity of the winding material.
    ///
    /// The resistance constant depends only on the winding and core geometry,
    /// including the end-winding geometry specified by
    /// `end_winding_half_turn_lengths`, and has the unit reciprocal length
    /// (`1 / Length`). Separating it from the electrical resistivity allows it
    /// to be calculated once and reused, for example when evaluating the
    /// resistance of the same winding at different temperatures.
    ///
    /// This method returns `None` if the winding is not symmetric with respect
    /// to the specified `core` and `end_winding_half_turn_lengths`, or if
    /// it contains no coils. The returned material is that of the first
    /// coil returned by [`Winding::coils_iter`], so all coils must have the
    /// same electrical resistivity.
    ///
    /// # Examples
    ///
    /// This example just serves to demonstrate the workflow, hence we use a
    /// fictional core and winding here to simplify the setup.
    ///
    /// ```
    /// use std::str::FromStr;
    /// use std::sync::Arc;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    /// use stem_winding::var_quantity::unary::FirstOrderTaylor;
    ///
    /// // Define a material with an electrical resistivity which changes linearly
    /// // with the temperature.
    /// let copper = {
    ///     let mut material = Material::default();
    ///     material.electrical_resistivity = VarQuantity::new(
    ///         FirstOrderTaylor::new(
    ///             DynQuantity::from_str("1 / 56 m/MS").expect("parseable"),
    ///             DynQuantity::from_str("0.393 % / K").expect("parseable"),
    ///             DynQuantity::from_str("20.0 °C").expect("parseable"),
    ///         )
    ///         .expect("units match")
    ///     ).expect("units match");
    ///     Arc::new(material)
    /// };
    ///
    /// let winding: ToothCoilWinding = ToothCoilBuilder {
    ///     slots: 12.try_into().expect("not zero"),
    ///     pole_pairs: 5.try_into().expect("not zero"),
    ///     phases: 3.try_into().expect("not zero"),
    ///     layers: 1.try_into().expect("not zero"),
    ///     winding_table_constructor: WindingTableConstructor::Tingley,
    ///     turns_per_coil: 30.try_into().expect("not zero"),
    ///     parallel_paths: 1.try_into().expect("not zero"),
    ///     connection: Connection::Star,
    ///     end_winding_leakage_coefficient: 0.0,
    ///     wire: Box::new(SffWire::new(
    ///         copper,
    ///         0.5, // slot_fill_factor_conductor
    ///         0.6, // slot_fill_factor_overall
    ///     ).expect("valid inputs")),
    ///     mean_end_winding_half_turn_length: None,
    /// }
    /// .try_into()
    /// .unwrap();
    /// let core = RotCore::from_winding(&winding);
    ///
    /// let decomposition = winding.resistance_decomposition(
    ///     core.as_core_ref(),
    /// ).expect("symmetric winding");
    ///
    /// // Phase resistance at 20 °C.
    /// let conditions = [DynQuantity::from_str("20 °C").expect("parseable")];
    /// let r20 = winding.resistance(
    ///     CoreRef::Rot(&core),
    ///     1.try_into().unwrap(),
    ///     &conditions,
    /// ).get::<ohm>();
    /// assert_abs_diff_eq!(r20, 0.202179, epsilon = 0.0001);
    /// assert_abs_diff_eq!(r20, decomposition.resistance(&conditions).get::<ohm>(), epsilon = 0.0001);
    ///
    /// // Phase resistance at 120 °C.
    /// let conditions = [DynQuantity::from_str("120 °C").expect("parseable")];
    /// let r120 = winding.resistance(
    ///     CoreRef::Rot(&core),
    ///     1.try_into().unwrap(),
    ///     &conditions,
    /// ).get::<ohm>();
    /// assert_abs_diff_eq!(r120, 0.2816357, epsilon = 0.0001);
    /// assert_abs_diff_eq!(r120, decomposition.resistance(&conditions).get::<ohm>(), epsilon = 0.0001);
    /// ```
    #[cfg(feature = "stem_core")]
    fn resistance_decomposition(&self, core: CoreRef<'_>) -> Option<ResistanceDecomposition> {
        if !self.is_symmetric(core) {
            return None;
        }

        let material = self.coils_iter().next()?.wire().material_arc().clone();

        let resistance = self.resistance(core, NonZeroU16::MIN, &[]);
        let electrical_resistivity = material.electrical_resistivity().get(&[]);
        return Some(ResistanceDecomposition {
            resistance_constant: resistance / electrical_resistivity,
            material,
        });
    }

    /// Returns the effective winding factor.
    ///
    /// In contrast to the idealized
    /// [`winding_factor`](Winding::winding_factor), which considers the
    /// winding distribution without accounting for the machine geometry,
    /// the effective winding factor also accounts for the effects of
    /// air-gap curvature and slot openings.
    ///
    /// It is the product of the ideal winding factor,
    /// [`CoreExt::air_gap_curvature_factor`], and
    /// [`CoreExt::slot_opening_factor`].
    ///
    /// The effective winding factor can be used to determine the amplitude of a
    /// spatial harmonic of the magnetic field, taking these geometric effects
    /// into account.
    #[cfg(feature = "stem_core")]
    fn effective_winding_factor(
        &self,
        core: CoreRef<'_>,
        phase: NonZeroU16,
        spatial_order: SpatialOrder,
        effective_air_gap: Length,
    ) -> f64 {
        self.winding_factor(phase, spatial_order)
            * core.air_gap_curvature_factor(effective_air_gap, spatial_order)
            * core.slot_opening_factor(spatial_order)
    }

    /// Returns the flux linkage per air-gap flux density for the given phase
    /// and spatial order.
    ///
    /// The returned area `Cν` relates the air-gap flux density harmonic `Bν` to
    /// the resulting flux linkage `Ψν`:
    ///
    /// `Ψν = Cν * Bν`.
    ///
    /// The time variation of the flux linkage induces a voltage:
    ///
    /// `uν = dΨν / dt`
    ///
    /// The factor `Cν` accounts for the coil geometry by transforming the
    /// actual winding to an idealized single-turn coil spanning an entire
    /// pole pair. Reference [\[1\]](#flux_linkage_per_air_gap_flux_density_1),
    /// equations (5.34) and (5.35), derives this factor for rotary cores.
    /// Generalized to both rotary and linear machines, it is calculated as
    ///
    /// `Cν = A * N_s * k_w,ν / (π * p)`
    ///
    /// where `A` is the air gap area of the core ([`CoreExt::air_gap_area`]),
    /// `N_s` is the
    /// [`series_turns_per_phase`](Winding::series_turns_per_phase), `k_w,ν`
    /// is the [`effective_winding_factor`](Winding::effective_winding_factor)
    /// for the spatial order `ν` and `p` is the number of
    /// [`pole_pairs`](Winding::pole_pairs).
    ///
    /// # Literature
    /// <a id="flux_linkage_per_air_gap_flux_density_1">\[1\]</a>
    /// Mathis, S.: Permanentmagneterregte Line-Start-Antriebe in Ferrittechnik,
    /// PhD thesis, Shaker, 2019, URL:
    /// <https://kluedo.ub.rptu.de/frontdoor/index/index/docId/8192>
    fn flux_linkage_per_air_gap_flux_density(
        &self,
        core: CoreRef<'_>,
        phase: NonZeroU16,
        spatial_order: SpatialOrder,
        effective_air_gap: Length,
    ) -> Area {
        return core.air_gap_area()
            * self.series_turns_per_phase_float(phase)
            * self.effective_winding_factor(core, phase, spatial_order, effective_air_gap)
            / (PI * f64::from(self.pole_pairs().get()));
    }

    // TODO
    #[cfg(all(feature = "cairo", feature = "stem_core"))]
    fn zone_drawables<'a>(
        &'a self,
        core: CoreRef<'_>,
        zone_config: &'a ZoneDrawablesConfig,
    ) -> WindingZoneDrawables<'a> {
        WindingZoneDrawables::new(self.as_dyn(), core, zone_config)
    }

    // TODO
    #[cfg(all(feature = "cairo", feature = "stem_core"))]
    fn coil_drawables<'a>(&'a self, parameters: &'a CoilDrawablesParameters) -> CoilDrawables<'a> {
        CoilDrawables::new(self.as_dyn(), parameters)
    }
}

dyn_clone::clone_trait_object!(Winding);
