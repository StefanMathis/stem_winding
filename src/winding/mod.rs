use dyn_clone::DynClone;
use num::{Complex, Integer, integer::gcd};
#[cfg(feature = "stem_core")]
use std::collections::HashMap;
use std::{
    any::Any,
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

use stem_coil_layout::{CoilLayout, Zone};
use stem_wire::{stem_material::si::Length, wire::Wire};

use crate::{
    coils::{Coil, CoilExt},
    iterators::*,
    winding_table::WindingTable,
};

pub mod coil_assembly;
pub mod distributed;
pub mod distributed_tooth_coil;
pub mod quadruple_layer_tooth_coil;
pub mod squirrel_cage;
pub mod tooth_coil;

pub use coil_assembly::*;
pub use distributed::*;
pub use distributed_tooth_coil::*;
pub use quadruple_layer_tooth_coil::{
    QuadrupleLayerToothCoilBuilder, QuadrupleLayerToothCoilMinimalBuilder,
    QuadrupleLayerToothCoilWinding,
};
pub use squirrel_cage::*;
pub use tooth_coil::*;

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
 */
#[cfg_attr(feature = "serde", typetag::serde)]
pub trait Winding: Sync + Send + Any + DynClone + std::fmt::Debug + 'static {
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
    /// Winding with a large pole pair and slot number are often realized by
    /// repeating a "base winding" multiple times. For example, the following
    /// [`WindingTable`] for a 12-slot, 2-pole-pair, 3-phase winding consists of
    /// two repetitions of a 6-slot, 1-pole-pair, 3-phase winding:
    ///
    /// ```text
    /// L \ S │   0   1   2   3   4   5   6   7   8   9  10  11
    /// ──────┼────────────────────────────────────────────────
    ///   0   │   1  -3   2  -1   3  -2   1  -3   2  -1   3  -2
    /// ``
    ///
    /// This method returns the number of repetitions of such a "base winding"
    /// in `self`. The [`WindingTable`] of the base winding can then be
    /// obtained from the full winding by considering only the first `slots
    /// / base_winding_count` columns of the table. This property is used by
    /// [`Winding::winding_table`] when the `full` argument is set to
    /// `false`.
    ///
    /// As stated in the [trait documentation](Winding), it is recommended to
    /// override this method if possible, as the default implementation is O(n)
    /// with respect to the number of zones because it uses the
    /// [`repeating_pattern_count`] algorithm.
    fn base_winding_count<'a>(&'a self) -> NonZeroU16 {
        // Wrapper structure which makes the winding indexable by an usize (slot-major)
        struct IndexWrapper<'a, W: ?Sized>(&'a W);

        impl<'a, W: Winding + ?Sized> RandomAccess for IndexWrapper<'a, W> {
            type Item = i32;

            fn get(&self, index: usize) -> Self::Item {
                let layers = usize::from(self.0.layers().get());

                let slot = index / layers;
                let layer = index % layers;

                self.0.phase_at(Zone {
                    slot: slot as u16,
                    layer: layer as u16,
                })
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
        end_winding_half_turn_length: Option<Length>,
    ) -> Inductance {
        let _ = core;
        let _ = phase;
        let _ = end_winding_half_turn_length;
        return Default::default();
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

    # Example

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
    \[1\](lowest_torque_ripple_order_1), section 9.4.2b)

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

    // TODO
    /// Returns the winding grade (first grade or second grade) according to
    /// [Phy08]. If the denominator of q is odd, then the winding is of
    /// first grade, otherwise of second grade. Integer windings are always
    /// of first grade, since the denominator is always 1.
    fn winding_grade(&self) -> usize {
        let n = self.hole_number().denom().clone();
        return usize::from(2 - n % 2);
    }

    // TODO
    /// Returns the number of wound coils per phase (equals winding_holes in
    /// case of single-layer winding).
    fn coils_per_phase(&self) -> u16 {
        return self.layers().get() * self.slots().get() / (2 * self.phases().get());
    }

    // TODO
    /**
    Returns the number of coil groups per phase. This value is equal to the maximum possible number of parallel paths and can be calculated
    as described in [Seq50], p. 37: First, the number of coils per phase in a basic winding is calculated. Then, it is checked whether this
    number is even or odd. If it is even, the number of coil groups equals twice the number of basic windings (= the base_winding_count). If it is odd,
    the number of coil groups equals the number of basic windings.

    At least one coil group per phase is always possible
    */
    fn coil_groups_per_phase(&self) -> NonZeroU16 {
        let t = self.base_winding_count();
        let number_of_coils_in_basic_winding =
            self.slots().get() * self.layers().get() / (t.get() * 2 * self.phases().get());
        if number_of_coils_in_basic_winding % 2 == 0 {
            // Antiparallel coil groups are possible
            return NonZeroU16::new(2 * t.get()).expect("t is not zero");
        } else {
            // Only parallel coil groups are possible
            return t;
        }
    }

    // TODO
    /**
    Return the number of coils in a coil group
     */
    fn coils_per_coil_group(&self) -> u16 {
        return self.coils_per_phase() / self.coil_groups_per_phase();
    }

    // TODO
    /// Returns an iterator for all possible numbers of parallel paths, starting
    /// from 1.
    fn possible_parallel_paths(&self) -> crate::iterators::ParallelPathIterator {
        return crate::iterators::ParallelPathIterator::new(self.coil_groups_per_phase());
    }

    // TODO
    /// Returns the winding factor for the given phase and harmonic order.
    /// Note that the phase counting starts with 1 as it usually does in
    /// scientific literature regarding electrical machines. If the phase is
    /// set to 0 or to a number larger than the number of phases, this functions
    /// returns zero. The winding factor is calculated with the voltage
    /// phasor method as described in e.g. [Pyr08].
    fn winding_factor(&self, phase: NonZeroU16, order: f64) -> f64 {
        if phase > self.phases() && phase.get() == 0 {
            return 0.0;
        }

        // The winding factor is formally calculated as k_w =
        // sin(ν*π/2)/Z*Σ_ρ=1^Z(cos(α_ρ)) The angle α_ρ can be derivatived from
        // the phasor star. To find the beams of a phase and their respective
        // sign, the zone plan is used.

        // It is sufficient to calculate the phasor star for the basic winding
        let slots_basic = self.slots().get() / self.base_winding_count().get();
        let layers = self.layers();
        let alpha_u = self.phasor_angle();

        let (phasor_sum_geo, phasor_sum_abs) = (0..slots_basic)
            .into_par_iter()
            .map(|slot| {
                let mut phasor_sum_geo = Complex::new(0.0f64, 0.0);
                let mut phasor_sum_abs = 0;

                let slot_angle = slot as f64 * alpha_u * order;

                for layer in 0..layers.get() {
                    // Check if the current slot and layer is assigned to the phase
                    let current_phase = self.phase_at(Zone::new(slot, layer));
                    if current_phase.abs() == i32::from(u16::from(phase)) {
                        // Get the angle of the ρ-th zone. The reference is the first zone
                        // of phase 1, which always equals the first beam of the phasor star
                        // (alpha_rho=0 = 0°) If the zone is
                        // negative, the beam direction needs to be inverted (sign-function)
                        let dir_angle = if current_phase < 0 { PI } else { 0.0 };
                        let phasor_length = self.turns_at(Zone::new(slot, layer));
                        phasor_sum_geo = phasor_sum_geo
                            + (phasor_length as f64)
                                * (Complex::new(0.0, slot_angle + dir_angle)).exp();
                        phasor_sum_abs = phasor_sum_abs + phasor_length;
                    }
                }

                (phasor_sum_geo, phasor_sum_abs)
            })
            .reduce(
                || (Complex::new(0.0, 0.0), 0),
                |a, b| (a.0 + b.0, a.1 + b.1),
            );

        // Sum up the phasors and get the absolute value, which equals the resulting
        // phasor length. Calculate winding factor as quotient of the absolute sum of
        // all phasors and the geometrical sum of all phasors.
        return phasor_sum_geo.norm_sqr().sqrt() / (phasor_sum_abs as f64);
    }

    // TODO
    /// Returns the angle between two neighbouring phases.
    fn phase_angle_difference(&self) -> f64 {
        return TAU / self.phases().get() as f64;
    }

    // TODO
    /// Calculate the vertices of the Görges polygon for the winding `obj`
    /// according to [MVP08], p. 97ff. as a vector of complex coordinates.
    fn goerges_polygon(&self, full: bool) -> Vec<Complex<f64>> {
        let slots = if full {
            self.slots()
        } else {
            NonZeroU16::new(self.slots().get() / self.base_winding_count().get()).expect("not zero")
        };

        let mut verts = vec![Complex::new(0.0, 0.0); usize::from(slots.get())];

        // Angle between two phases
        let delta_phase_angle = self.phase_angle_difference();

        for slot in 0..slots.get() {
            for layer in 0..self.layers().get() {
                // The new polygon edge starts at the end of the previous polygon edge
                if layer == 0 && slot > 0 {
                    verts[slot as usize] = verts[slot as usize - 1].clone()
                }

                // Add the magnetic voltage to the current polygon node
                let phase = self.phase_at(Zone::new(slot, layer));
                if phase == 0 {
                    continue;
                } else {
                    let turns = self.turns_at(Zone::new(slot, layer));
                    let phase_angle = ((phase as f64).abs() - 1.0) * delta_phase_angle;

                    verts[slot as usize] = verts[slot as usize]
                        + (turns as f64 * num::signum(phase) as f64)
                            * (Complex::new(0.0, phase_angle)).exp();
                }
            }
        }

        // Calculate the focal point and center the polygon
        let sum = verts.as_slice().iter().sum::<Complex<f64>>() as Complex<f64>;
        let focal_point = sum / verts.len() as f64;
        for ii in 0..verts.len() {
            verts[ii] = verts[ii] - focal_point;
        }

        return verts;
    }

    // TODO
    /// Check if the winding factors of all phases are identical.
    /// A default implementation exists.
    fn equal_winding_factors(&self) -> bool {
        let ref_winding_factor = self.winding_factor(NonZeroU16::MIN, 1.0);
        for phase in 2..(self.phases().get() + 1) {
            let winding_factor =
                self.winding_factor(NonZeroU16::new(phase).expect("is not zero"), 1.0);
            if approxim::abs_diff_ne!(winding_factor, ref_winding_factor, epsilon = 1e-15) {
                return false;
            }
        }
        return true;
    }

    // TODO
    /// Calculate the air gap leakage factor from the Görges diagram ([MVP08],
    /// p. 97 ff.) A default implementation exists.
    fn air_gap_leakage_factor(&self) -> f64 {
        let verts = self.goerges_polygon(false);
        let slots = self.slots().get() / self.base_winding_count();
        let phase = NonZeroU16::MIN;
        let k_w = self.winding_factor(phase, 1.0);

        // Calculate the "Trägheitsradius" (inertia radius) squared according to , eq.
        // (1.2.85)
        let mut rg2: f64 = verts.iter().map(|v| v.norm_sqr()).sum();

        // For the magnetic slot voltage, the following holds true:
        // Θ_n = obj.layers*Θ_sp,
        // which means that the inertia radius has to be corrected accordingly here
        rg2 = rg2 / ((slots * self.layers().get().pow(2)) as f64);

        // Calculate the "Trägheitsradius der Hauptwelle" squared according to [MVP08],
        // eq. (1.2.86)

        // Current is arbitrarily set to 1 A (as it is in
        // self.goerges_polygon()) Correction by the number of winding
        // layers is necessary due to the same reason as for rg2
        let series_turns_per_phase = *self.series_turns_per_phase(phase).numer() as f64
            / *self.series_turns_per_phase(phase).denom() as f64;
        let rp2 = (self.phases().get() as f64 * series_turns_per_phase * k_w
            / (PI * (self.pole_pairs().get() * self.layers().get()) as f64))
            .powi(2);

        // [MVP08], eq. (1.2.87)
        return rg2 / rp2 - 1.0;
    }

    // TODO
    /**
    Calculates the field excitation curve (FEC) of `self` for the given phase currents into the provided buffer.
    The buffer length must be equal to the number of winding slots and can therefore be created by `vec![0.0; winding.slots() as usize]`.
    The current slice length must be equal to the number of phases.

    # Example
    ```
    use winding::{DistributedWinding, Winding, WindingTableConstructor};

    let winding = DistributedWinding::new_minimal(12, 1, 3, 1, 0, 0, WindingTableConstructor::Tingley).unwrap();
    let mut buffer = vec![0.0; winding.slots() as usize];
    let currents = [1.0, -0.5, -0.5];

    winding.field_excitation_curve(&mut buffer, currents.as_slice()).unwrap();

    assert_eq!(buffer.as_slice(), &[0.0, 1.0, 1.5, 2.0, 1.5, 1.0, 0.0, -1.0, -1.5, -2.0, -1.5, -1.0]);
    ```
     */
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

    // TODO
    /**
    Returns a `HarmonicOrdersIterator` which gives the harmonic orders of the field excitation curve created by the winding.
    For further details, see the documentation on `HarmonicOrdersIterator`.

    The returned iterator has an infinite length, because the number of harmonics is infinite as well.
    Therefore, this iterator should not be used directly in e.g. a loop. Instead, a subset of the iterator
    can be used with the `take()` method (see example).

    ```
    use winding::{DistributedWinding, Winding};
    use num::rational::Ratio;

    let winding = DistributedWinding::default(); // This is a 6/2 single-layer integer-slot winding
    let ho_iter = winding.harmonic_orders();

    let orders: Vec<Ratio<i32>> = winding.harmonic_orders().take(5).collect();
    assert_eq!(orders[0], Ratio::new(1, 1));
    assert_eq!(orders[1], Ratio::new(-5, 1));
    assert_eq!(orders[2], Ratio::new(7, 1));
    assert_eq!(orders[3], Ratio::new(-11, 1));
    assert_eq!(orders[4], Ratio::new(13, 1));
    ```
    A default implementation exists.
    */
    fn harmonic_orders(&self) -> HarmonicOrdersIterator<'_> {
        return HarmonicOrdersIterator::new(self.as_dyn());
    }

    // TODO
    /**
     *
    Return an iterator over the normalized air gap flux density / induction |B_v / B_p| and the
    associated harmonic order (calculated by [`harmonic_orders`](Winding::harmonic_orders)).
    for each harmonic order returned by [`harmonic_orders`](Winding::harmonic_orders)
    The formula is based on [Hut18a], page 25.x
     */
    fn harmonic_inductions(&self) -> NormalizedInductionIterator<'_> {
        return NormalizedInductionIterator::new(self.as_dyn());
    }

    // TODO
    /// Returns an iterator over all coils of the winding.
    fn coils_iter(&self) -> CoilsIterator<'_> {
        return CoilsIterator::new(self.as_dyn());
    }

    // TODO
    /// Assert the symmetry of the winding. If true, the following is also true:
    /// * The winding factor is identical for all phases (this holds true for
    ///   all orders individually).
    /// * The phase resistance of all phases is identical
    /// * The number of turns per phase is identical for all phases.
    /// * All wires have the same material
    #[cfg(feature = "stem_core")]
    fn is_symmetric(
        &self,
        core: CoreRef<'_>,
        end_winding_half_turn_lengths: &HashMap<Zone, Length>,
    ) -> bool {
        use uom::si::electrical_resistance::ohm;

        // Condition 1: Comparison of winding factors
        if !self.equal_winding_factors() {
            return false;
        }

        // Condition 2: Calculate the phase resistance for some assumed conditions
        let resistance_1 = self
            .resistance(core, NonZeroU16::MIN, &[], end_winding_half_turn_lengths)
            .get::<ohm>();
        for phase in 2..(self.phases().get() + 1) {
            if approxim::abs_diff_ne!(
                self.resistance(
                    core,
                    NonZeroU16::new(phase).unwrap_or(NonZeroU16::MIN),
                    &[],
                    end_winding_half_turn_lengths
                )
                .get::<ohm>(),
                resistance_1,
                epsilon = 1e-10
            ) {
                return false;
            }
        }

        // Condition 3: Add the number of turns of phase 1 and compare it to the number
        // of turns of all phases
        let turns_phase_1 = self.series_turns_per_phase(NonZeroU16::MIN);
        for phase in 2..(self.phases().get() + 1) {
            if self.series_turns_per_phase(NonZeroU16::new(phase).unwrap_or(NonZeroU16::MIN))
                != turns_phase_1
            {
                return false;
            }
        }

        return true;
    }

    // fn curvature_factor(&self, core: CoreRef<'_>)

    //     /// Return the air gap leakage inductance ("doppeltverkettete Streuung")
    // as /// defined in e.g. [MVP08] or [Bin12].
    // fn air_gap_leakage_inductance(&self, phase: u16) -> Inductance {
    //     let winding = match self.winding() {
    //         Some(winding) => winding,
    //         None => return Inductance::new::<henry>(0.0),
    //     };

    //     let main_inductance = self.main_inductance(phase);
    //     let leakage_factor = self
    //         .overrides()
    //         .air_gap_leakage_factor
    //         .unwrap_or_else(|| winding.air_gap_leakage_factor());
    //     return main_inductance * leakage_factor;
    // }

    // /// Return the total air gap inductance, which equals the sum of
    // /// `self.air_gap_leakage_inductance(effective_air_gap)` and
    // /// `self.main_inductance(effective_air_gap)`.
    // fn air_gap_inductance(&self, phase: u16) -> Inductance {
    //     let winding = match self.winding() {
    //         Some(winding) => winding,
    //         None => return Inductance::new::<henry>(0.0),
    //     };

    //     let leakage_factor = self
    //         .overrides()
    //         .air_gap_leakage_factor
    //         .unwrap_or_else(|| winding.air_gap_leakage_factor());

    //     return self.main_inductance(phase) * (1.0 + leakage_factor);
    // }

    // /// Return the total leakage inductance. The temperature and frequency
    // /// parameters are used to take AC effects like current displacement into
    // /// account. If only the DC behaviour is of interest, those values can
    // /// be set to zero.
    // fn leakage_inductance(&self, phase: u16, conditions: &[InfluencingQuantity])
    // -> Inductance {     return self.air_gap_leakage_inductance(phase)
    //         + self.end_winding_leakage_inductance(phase)
    //         + self.slot_leakage_inductance(phase, conditions);
    // }

    // /// Return the total inductance.
    // fn inductance(&self, phase: u16, conditions: &[InfluencingQuantity]) ->
    // Inductance {     return self.air_gap_inductance(phase)
    //         + self.end_winding_leakage_inductance(phase)
    //         + self.slot_leakage_inductance(phase, conditions);
    // }

    // /// Return the electrical filling factor for the given slot.
    // fn slot_filling_factor_electrical_at(&self, slot: u16) -> Option<f64> {
    //     let winding = self.winding()?;

    //     if slot >= winding.slots() {
    //         return None;
    //     }

    //     let layers = winding.layers() as f64;
    //     let zone_area = self.zone_area() / layers;

    //     let range = 0..winding.layers();
    //     let sum_sff: f64 = range
    //         .into_iter()
    //         .map(|layer| {
    //             let turns = winding.turns_at(Zone::new(slot, layer));
    //             return winding
    //                 .wire_at(Zone::new(slot, layer))
    //                 .map(|wire| wire.slot_fill_factor_conductor(zone_area,
    // turns))                 .unwrap_or(0.0);
    //         })
    //         .sum();

    //     return Some(sum_sff / layers);
    // }

    // /// Return the electrical filling factor for the given slot.
    // fn slot_filling_factor_mechanical_at(&self, slot: u16) -> Option<f64> {
    //     let winding = self.winding()?;

    //     if slot >= winding.slots() {
    //         return None;
    //     }

    //     let layers = winding.layers() as f64;
    //     let zone_area = self.zone_area() / layers;

    //     let range = 0..winding.layers();
    //     let sum_sff: f64 = range
    //         .into_iter()
    //         .map(|layer| {
    //             let turns = winding.turns_at(Zone::new(slot, layer));
    //             return winding
    //                 .wire_at(Zone::new(slot, layer))
    //                 .map(|wire| wire.slot_fill_factor_overall(zone_area, turns))
    //                 .unwrap_or(0.0);
    //         })
    //         .sum();

    //     return Some(sum_sff / layers);
    // }

    //     /**
    // Calculate the coupling factor k between the air gap flux B and the flux
    // linkage psi: psi = B * k

    // If the component has no winding, this factor is 0 (since no coupling occurs)
    // See eq. (5.34) and (5.35) from [Mat19].
    //  */
    // fn convert_air_gap_flux_to_winding_linkage(&self) -> Area {
    //         let series_turns_per_phase = winding.series_turns_per_phase(1);
    //         let tpf_float = *series_turns_per_phase.numer() as f64 /
    // *series_turns_per_phase.denom() as f64;         let ag_area = match
    // self.core() {             CoreRef::Lin(c) => c.air_gap_area(),
    //             CoreRef::Rot(c) => c.air_gap_area(),
    //         };
    //         return ag_area * tpf_float * self.winding_factor(1, 1.0)
    //             / (std::f64::consts::PI * winding.pole_pairs() as f64);
    // }

    // TODO
    /**
    Calculate the slot leakage inductance for a symmetric winding
     */
    #[cfg(feature = "stem_core")]
    fn slot_leakage_inductance(
        &self,
        core: CoreRef<'_>,
        phase: NonZeroU16,
        effective_air_gap: Length,
        _conditions: &[DynQuantity<f64>],
        overrides: &Overrides,
    ) -> Inductance {
        if let Some(slot_leakage_inductance) = overrides.slot_leakage_inductance {
            return slot_leakage_inductance;
        }

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

        // Calculate the normalized current of all phases
        let normalized_current: Vec<f64> = multiphase_system(
            Time::new::<second>(0.0),
            Frequency::new::<hertz>(0.0),
            0.0,
            self.phases(),
        )
        .collect();

        // Precalculate the leakage coefficient matrix
        let lambda_slot = slot.leakage_coefficient_matrix(&self.coil_layout());

        // Precalculate the product of axial length and vacuum permeability
        let single_turn_inductance =
            (*VACUUM_PERMEABILITY * core.axial_coil_length()).get::<si::inductance::henry>();

        // Calculate the total slot inductance for the selected phase
        let basic_winding_slot_inductance: f64 = slots
            .into_par_iter()
            .map(|slot_idx| {
                // Calculate the self-inductance of a single slot according to [MVP08], section
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
                                return 0.0;
                            }

                            if let Some(excitation_coil) =
                                self.coil_at(Zone::new(slot_idx, excitation_layer))
                            {
                                // Calculate the coupling direction between the linked coil and
                                // the excitation coil
                                let coupling = 1
                                    * self.phase_at(Zone::new(slot_idx, linked_layer)).signum()
                                    * self
                                        .phase_at(Zone::new(slot_idx, excitation_layer))
                                        .signum();

                                // Get the normalized excitation current and modify its
                                // direction according to the coupling calculated above
                                let idx = excitation_coil.phase().get() as usize - 1;
                                let excitation_current = normalized_current[idx] * coupling as f64;

                                // Calculate the inductance
                                return single_turn_inductance
                                    * linked_coil.turns().get() as f64
                                    * excitation_coil.turns().get() as f64
                                    * excitation_current
                                    * (lambda_slot
                                        [(linked_layer as usize, excitation_layer as usize)]
                                        + lambda_opening_and_tooth_tip);
                            } else {
                                return 0.0;
                            }
                        } else {
                            return 0.0;
                        }
                    })
                    .sum::<f64>();
            })
            .sum();

        // Scale with the winding base_winding_count and the number of parallel paths
        return Inductance::new::<si::inductance::henry>(basic_winding_slot_inductance)
            * self.base_winding_count().get() as f64
            / self.parallel_paths(phase).get() as f64;
    }

    //     /// Calculate the main inductance of the component.
    // fn main_inductance(&self, phase: u16) -> Inductance;

    // /**
    // Convert the given current in A to the electric loading in A/m, if the
    // component has a winding. If the component has no winding, the electric
    // loading defaults to zero. The conversion is based on eq. (9.1.23c) from
    // [MVP08].

    // In [MVP08], table 9.1.4, the following typical values for the electric
    // loading are given:

    // # DC machines
    // * Indirect air cooling: 20 ... 80 A/mm

    // # Synchronous machines
    // * Indirect air cooling: 30 ... 120 A/mm
    // * Indirect hydrogen cooling: 90 ... 150 A/mm
    // * Direct hydrogen cooling: 120 ... 200 A/mm
    // * Direct water cooling: 160 ... 300 A/mm

    // # Induction machines:
    // * Indirect air cooling: 20 ... 120 A/mm
    //  */
    // fn convert_current_to_electric_loading(&self) -> ReciprocalLength;

    //     /**
    // Convert the given current in A to the electric loading in A/m, if the
    // component has a winding. If the component has no winding, the electric
    // loading defaults to zero. The conversion is based on eq. (9.1.23c) from
    // [MVP08].

    // In [MVP08], table 9.1.4, the following typical values for the electric
    // loading are given:

    // # DC machines
    // * Indirect air cooling: 20 ... 80 A/mm

    // # Synchronous machines
    // * Indirect air cooling: 30 ... 120 A/mm
    // * Indirect hydrogen cooling: 90 ... 150 A/mm
    // * Direct hydrogen cooling: 120 ... 200 A/mm
    // * Direct water cooling: 160 ... 300 A/mm

    // # Induction machines:
    // * Indirect air cooling: 20 ... 120 A/mm
    //  */
    // fn convert_current_to_electric_loading(&self) -> ReciprocalLength {
    //     match self.winding() {
    //         Some(wdg) => {
    //             let series_turns_per_phase = wdg.series_turns_per_phase(1);
    //             let tpf_float = *series_turns_per_phase.numer() as f64 /
    // *series_turns_per_phase.denom() as f64;             return tpf_float *
    // wdg.phases() as f64 / (PI * self.core_rot().air_gap_radius());         }
    //         None => return ReciprocalLength::new::<reciprocal_meter>(0.0),
    //     }
    // }

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
    /// ```
    /// use std::collections::HashMap;
    ///
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
    /// let end_winding_half_turn_lengths = HashMap::new();
    ///
    /// // Calculate the total volume of all coils
    /// let volume: Volume = winding
    ///     .coil_properties_iter(core, end_winding_half_turn_lengths)
    ///     .map(|cp|cp.volume()).sum();
    ///
    /// assert_abs_diff_eq!(volume.get::<cubic_millimeter>(), 0.10108, epsilon = 0.0001);
    /// ```
    #[cfg(feature = "stem_core")]
    fn coil_properties_iter<'a>(
        &'a self,
        core: CoreRef<'a>,
        end_winding_half_turn_lengths: &'a HashMap<Zone, Length>,
    ) -> crate::core_support::CoilPropertyIterator<'a> {
        return crate::core_support::CoilPropertyIterator {
            coils: self.coils_iter(),
            winding: self.as_dyn(),
            core,
            end_winding_half_turn_lengths,
        };
    }

    /// Returns a [`CoilProperties`] context for the [`Coil`] containing the
    /// specified [`Zone`]. The returned `CoilProperties` combines the coil with
    /// this winding, the specified [`CoreRef`], and the supplied end-winding
    /// half-turn length overrides. If `end_winding_half_turn_lengths` does not
    /// contain an override for a zone of the coil, the end winding length is
    /// calculated on demand with [`Winding::end_winding_half_turn_length`].
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
        end_winding_half_turn_lengths: &'a HashMap<Zone, Length>,
    ) -> Option<crate::core_support::CoilProperties<'a>> {
        let coil = self.coil_at(zone)?;
        return Some(crate::core_support::CoilProperties::new(
            coil,
            self.as_dyn(),
            core,
            end_winding_half_turn_lengths,
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
    /// Here, axial_coil_length and axial_coil_overhang are obtained from `core`
    /// via [`CoreExt::axial_coil_length`] and [`CoreExt::axial_coil_overhang`],
    /// respectively. The end_winding_half_turn_length can be supplied
    /// explicitly for each zone through the `end_winding_half_turn_lengths`
    /// map. If no override is provided for a zone, it is calculated using
    /// [`Winding::end_winding_half_turn_length`]. If no overrides are desired,
    /// an empty map can be provided with [`Default::default`].
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
    ///             &Default::default(),
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
    ///             &Default::default(),
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
        end_winding_half_turn_lengths: &HashMap<Zone, Length>,
    ) -> ElectricalResistance {
        // Calculate the phase resistance by calculating the phase resistance of each
        // individual coil and then summing them up
        let mut resistance = ElectricalResistance::new::<ohm>(0.0);

        for coil in self.coils_iter() {
            if coil.phase() == phase {
                for zone in coil.zones() {
                    if let Some(zone_contour) = core.winding_zone_at(&self.coil_layout(), zone) {
                        let zone_area = Area::new::<square_meter>(zone_contour.area());

                        let end_winding_half_turn_length = end_winding_half_turn_lengths
                            .get(&zone)
                            .cloned()
                            .unwrap_or_else(|| self.end_winding_half_turn_length(core, zone));
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
    ///     ).expect("valid inputs"))
    /// }
    /// .try_into()
    /// .unwrap();
    /// let core = RotCore::from_winding(&winding);
    ///
    /// let decomposition = winding.resistance_decomposition(
    ///     core.as_core_ref(),
    ///     &Default::default()
    /// ).expect("symmetric winding");
    ///
    /// // Phase resistance at 20 °C.
    /// let conditions = [DynQuantity::from_str("20 °C").expect("parseable")];
    /// let r20 = winding.resistance(
    ///     CoreRef::Rot(&core),
    ///     1.try_into().unwrap(),
    ///     &conditions,
    ///     &Default::default()
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
    ///     &Default::default()
    /// ).get::<ohm>();
    /// assert_abs_diff_eq!(r120, 0.2816357, epsilon = 0.0001);
    /// assert_abs_diff_eq!(r120, decomposition.resistance(&conditions).get::<ohm>(), epsilon = 0.0001);
    /// ```
    #[cfg(feature = "stem_core")]
    fn resistance_decomposition(
        &self,
        core: CoreRef<'_>,
        end_winding_half_turn_lengths: &HashMap<Zone, Length>,
    ) -> Option<ResistanceDecomposition> {
        if !self.is_symmetric(core, end_winding_half_turn_lengths) {
            return None;
        }

        let material = self.coils_iter().next()?.wire().material_arc().clone();

        let resistance = self.resistance(core, NonZeroU16::MIN, &[], end_winding_half_turn_lengths);
        let electrical_resistivity = material.electrical_resistivity().get(&[]);
        return Some(ResistanceDecomposition {
            resistance_constant: resistance / electrical_resistivity,
            material,
        });
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

/// Electrical connection of the phases of a [`Winding`].
///
/// The connection determines how the winding phase voltages and currents
/// relate to the line voltages and currents of a symmetric multi-phase power
/// supply.
///
/// [`Connection::Star`] connects one terminal of each phase to a common
/// neutral point. [`Connection::Delta`] connects the phases in a closed
/// loop, with the line terminals connected at the phase junctions.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum Connection {
    /// Star (Y) connection with a common neutral point.
    Star,
    /// Delta (Δ) connection.
    Delta,
}

impl Connection {
    /// Returns the complex conversion factor from line voltage to phase voltage
    /// for a symmetric multi-phase power supply.
    ///
    /// The returned value `k` relates the line voltage `U_line` to the winding
    /// phase voltage `U_phase` as
    ///
    /// `U_phase = k * U_line`.
    ///
    /// For a [`Connection::Star`] connection, the phase voltage is the voltage
    /// between a phase terminal and the neutral point. For a
    /// [`Connection::Delta`] connection, the phase voltage is equal to the line
    /// voltage.
    ///
    /// The conversion assumes a symmetric power supply with `phases` equally
    /// spaced phase voltages.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let phases = NonZeroU16::new(3).unwrap();
    ///
    /// let factor = Connection::Star.line_to_phase_voltage(phases);
    /// assert_abs_diff_eq!(0.57735, factor.norm(), epsilon = 0.0001);
    /// assert_abs_diff_eq!(0.5, factor.re, epsilon = 0.0001);
    /// assert_abs_diff_eq!(0.288675, factor.im, epsilon = 0.0001);
    ///
    /// let factor = Connection::Delta.line_to_phase_voltage(phases);
    /// assert_abs_diff_eq!(1.0, factor.norm(), epsilon = 0.0001);
    /// assert_abs_diff_eq!(1.0, factor.re, epsilon = 0.0001);
    /// assert_abs_diff_eq!(0.0, factor.im, epsilon = 0.0001);
    /// ```
    pub fn line_to_phase_voltage(&self, phases: NonZeroU16) -> num::Complex<f64> {
        // Assumption of a symmetric grid
        let a = Complex::new(0.0, TAU / (phases.get() as f64)).exp();
        match self {
            Connection::Star => return Complex::new(1.0, 0.0) / (Complex::new(1.0, 0.0) - a),
            Connection::Delta => return Complex::new(1.0, 0.0),
        }
    }

    /// Returns the ratio of the phase-voltage magnitude to the line-voltage
    /// magnitude for a symmetric multi-phase power supply.
    ///
    /// This is the magnitude of [`Connection::line_to_phase_voltage`]:
    ///
    /// `|U_phase| / |U_line|`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let phases = NonZeroU16::new(3).unwrap();
    ///
    /// assert_abs_diff_eq!(0.57735, Connection::Star.ratio_line_to_phase_voltage(phases), epsilon = 0.0001);
    /// assert_abs_diff_eq!(1.0, Connection::Delta.ratio_line_to_phase_voltage(phases), epsilon = 0.0001);
    /// ```
    pub fn ratio_line_to_phase_voltage(&self, phases: NonZeroU16) -> f64 {
        self.line_to_phase_voltage(phases).norm()
    }

    /// Returns the phase-angle difference from line voltage to phase voltage
    /// for a symmetric multi-phase power supply.
    ///
    /// The returned angle is the phase-voltage angle relative to the
    /// line-voltage angle, in radians.
    ///
    /// This is the argument of [`Connection::line_to_phase_voltage`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let phases = NonZeroU16::new(3).unwrap();
    ///
    /// assert_abs_diff_eq!(0.52359, Connection::Star.angle_line_to_phase_voltage(phases), epsilon = 0.0001);
    /// assert_abs_diff_eq!(0.0, Connection::Delta.angle_line_to_phase_voltage(phases), epsilon = 0.0001);
    /// ```
    pub fn angle_line_to_phase_voltage(&self, phases: NonZeroU16) -> f64 {
        self.line_to_phase_voltage(phases).arg()
    }

    /// Returns the complex conversion factor from line current to phase current
    /// for a symmetric multi-phase power supply.
    ///
    /// The returned value `k` relates the line current `I_line` to the winding
    /// phase current `I_phase` as
    ///
    /// `I_phase = k * I_line`.
    ///
    /// For a [`Connection::Star`] connection, the phase current is equal to the
    /// line current. For a [`Connection::Delta`] connection, the phase current
    /// is the current through a winding phase between two line terminals.
    ///
    /// The conversion assumes a symmetric power supply with `phases` equally
    /// spaced phase currents.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let phases = NonZeroU16::new(3).unwrap();
    ///
    /// let factor = Connection::Star.line_to_phase_current(phases);
    /// assert_abs_diff_eq!(1.0, factor.norm(), epsilon = 0.0001);
    /// assert_abs_diff_eq!(1.0, factor.re, epsilon = 0.0001);
    /// assert_abs_diff_eq!(0.0, factor.im, epsilon = 0.0001);
    ///
    /// let factor = Connection::Delta.line_to_phase_current(phases);
    /// assert_abs_diff_eq!(0.57735, factor.norm(), epsilon = 0.0001);
    /// assert_abs_diff_eq!(0.5, factor.re, epsilon = 0.0001);
    /// assert_abs_diff_eq!(-0.288675, factor.im, epsilon = 0.0001);
    /// ```
    pub fn line_to_phase_current(&self, phases: NonZeroU16) -> num::Complex<f64> {
        // Assumption of a symmetric grid
        let a = Complex::new(0.0, -TAU / (phases.get() as f64)).exp();
        match self {
            Connection::Star => Complex::new(1.0, 0.0),
            Connection::Delta => Complex::new(1.0, 0.0) / (Complex::new(1.0, 0.0) - a),
        }
    }

    /// Returns the ratio of the phase-current magnitude to the line-current
    /// magnitude for a symmetric multi-phase power supply.
    ///
    /// This is the magnitude of [`Connection::line_to_phase_current`]:
    ///
    /// `|I_phase| / |I_line|`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let phases = NonZeroU16::new(3).unwrap();
    ///
    /// assert_abs_diff_eq!(1.0, Connection::Star.ratio_line_to_phase_current(phases), epsilon = 0.0001);
    /// assert_abs_diff_eq!(0.57735, Connection::Delta.ratio_line_to_phase_current(phases), epsilon = 0.0001);
    /// ```
    pub fn ratio_line_to_phase_current(&self, phases: NonZeroU16) -> f64 {
        self.line_to_phase_current(phases).norm()
    }

    /// Returns the phase-angle difference from line current to phase current
    /// for a symmetric multi-phase power supply.
    ///
    /// The returned angle is the phase-current angle relative to the
    /// line-current angle, in radians.
    ///
    /// This is the argument of [`Connection::line_to_phase_current`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    ///
    /// use approxim::assert_abs_diff_eq;
    /// use stem_winding::prelude::*;
    ///
    /// let phases = NonZeroU16::new(3).unwrap();
    ///
    /// assert_abs_diff_eq!(0.0, Connection::Star.angle_line_to_phase_current(phases), epsilon = 0.0001);
    /// assert_abs_diff_eq!(-0.52359, Connection::Delta.angle_line_to_phase_current(phases), epsilon = 0.0001);
    /// ```
    pub fn angle_line_to_phase_current(&self, phases: NonZeroU16) -> f64 {
        self.line_to_phase_current(phases).arg()
    }
}

/// Returns the hole number `q` of a winding as a reduced fraction.
///
/// The hole number is the number of slots per pole and phase:
///
/// `q = slots / (2 * pole_pairs * phases)`.
///
/// It can be represented as
///
/// `q = g + z / n`,
///
/// where `g` is the integer part and `z` and `n` are integers with
/// `0 <= z < n`. The returned [`Ratio`] is reduced, so its numerator and
/// denominator directly provide `z` and `n` for the fractional part.
///
/// ```
/// use winding::hole_number;
///
/// // A 12/10 three-phase winding has a hole number of 2/5, which means that g = 0, z = 2, n = 5
/// let ratio = hole_number(12, 5, 3);
/// let g = ratio.trunc().numer().clone();
/// let z = ratio.fract().numer().clone();
/// let n = ratio.denom().clone();
/// assert_eq!(g, 0);
/// assert_eq!(z, 2);
/// assert_eq!(n, 5);
///
/// // A 36/10 three-phase winding has a hole number of 6/5, which means that g = 1, z = 1, n = 5
/// let ratio = hole_number(36, 5, 3);
/// let g = ratio.trunc().numer().clone();
/// let z = ratio.fract().numer().clone();
/// let n = ratio.denom().clone();
/// assert_eq!(g, 1);
/// assert_eq!(z, 1);
/// assert_eq!(n, 5);
///
/// // A 36/4 three-phase winding has a hole number of 3, which means that g = 3, z = 0, n = 1
/// let ratio = hole_number(36, 2, 3);
/// let g = ratio.trunc().numer().clone();
/// let z = ratio.fract().numer().clone();
/// let n = ratio.denom().clone();
/// assert_eq!(g, 3);
/// assert_eq!(z, 0);
/// assert_eq!(n, 1);
/// ```
pub fn hole_number(
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    phases: NonZeroU16,
) -> num::rational::Ratio<u16> {
    // Get greatest common divisor of numerator and denominator
    let val = num::integer::gcd(slots.get(), 2 * pole_pairs.get() * phases.get());

    // Calculate smallest integer numerator and denominator
    let n = (2 * pole_pairs.get() * phases.get()) / val;
    let z = slots.get() / val;
    return num::rational::Ratio::new_raw(z, n);
}

// TODO
/// The curvature factor accounts for the curvature of the air gap field due to
/// the stator roundness (see [Hut04])
pub fn curvature_factor(
    pole_pairs: NonZeroU16,
    air_gap_radius: Length,
    air_gap_width: Length,
    order: f64,
    is_outer: bool,
) -> f64 {
    let other_radius = air_gap_radius
        + if is_outer {
            -air_gap_width
        } else {
            air_gap_width
        };
    let v_times_p = (order * pole_pairs.get() as f64).abs();
    let a = f64::from(air_gap_radius / other_radius).powf(2.0 * v_times_p);
    return f64::from(air_gap_width * v_times_p / air_gap_radius * (a + 1.0) / (a - 1.0));
}

/// Provides random access to the values of a collection.
///
/// Unlike [`std::ops::Index`], `RandomAccess` does not require the value to be
/// stored in the collection or to have an addressable location. Implementations
/// may calculate the value on demand.
///
/// The index is expected to be valid for the collection, and implementations
/// should provide constant-time access.
///
/// A blanket implementation is provided for all types implementing
/// [`std::ops::Index<usize>`] whose [`std::ops::Index::Output`] is [`Copy`].
/// This allows ordinary indexable collections to be used as `RandomAccess`
/// collections without any additional implementation.
///
/// # Examples
///
/// A collection may calculate its values on demand rather than storing them.
/// This would not work with the `Index` trait, because that trait requires
/// returning a reference to a stored item, which simply won't exist.
///
/// ```
/// use stem_winding::winding::RandomAccess;
///
/// struct Calculated;
/// impl Calculated {
///     fn value_at(&self, index: usize) -> i32 {
///         index as i32 * 2
///     }
/// }
///
/// impl RandomAccess for Calculated {
///     type Item = i32;
///
///     fn get(&self, index: usize) -> Self::Item {
///         self.value_at(index)
///     }
/// }
///
/// assert_eq!(Calculated {}.get(2), 4);
/// ```
///
/// [`Copy`]: std::marker::Copy
pub trait RandomAccess<I = usize> {
    type Item;

    /// Returns the value at `index`.
    ///
    /// The value may be retrieved from stored data or calculated on demand.
    /// Implementations should provide constant-time access.
    ///
    /// The behavior for an out-of-bounds `index` is implementation-defined.
    /// Implementations should document whether such an index panics, returns a
    /// sentinel value, or is otherwise handled.
    fn get(&self, index: I) -> Self::Item;
}

impl<C> RandomAccess for C
where
    C: std::ops::Index<usize>,
    C::Output: Copy,
{
    type Item = C::Output;

    fn get(&self, index: usize) -> Self::Item {
        self[index]
    }
}

// TODO
pub fn repeating_pattern_count<C>(collection: &C, collection_len: NonZeroUsize) -> NonZeroUsize
where
    C: RandomAccess,
    C::Item: PartialEq,
{
    struct PatternLength {
        len: NonZeroUsize,
        divisor: usize,
    }

    impl PatternLength {
        fn new(len: NonZeroUsize) -> Self {
            Self {
                len,
                divisor: len.get(),
            }
        }
    }

    impl Iterator for PatternLength {
        type Item = usize;

        fn next(&mut self) -> Option<Self::Item> {
            while self.divisor > 0 {
                if self.len.get() % self.divisor == 0 {
                    let divisor = self.divisor;
                    self.divisor -= 1;
                    return Some(self.len.get() / divisor);
                }
                self.divisor -= 1;
            }
            return None;
        }
    }

    let mut c1 = 0;
    let mut c2 = 1;
    let mut pattern_len_iter = PatternLength::new(collection_len);

    // Candidate for the pattern length. We start with the smallest possible pattern
    // length, which is always 1.
    let mut pattern_len_cand = match pattern_len_iter.next() {
        Some(l) => l,
        None => unreachable!(
            "collection_len cannot be zero, hence the iterator will always return at least one item"
        ),
    };
    while c2 < collection_len.get() {
        if collection.get(c1) == collection.get(c2) {
            c1 = (c1 + 1) % pattern_len_cand;
            c2 += 1;
        } else {
            // If the new pattern length is not larger than c2, we must exclude it.
            // because we already skipped the possibility to validate that pattern
            // (since c2 already went too far). Furthermore, this pattern length
            // isn't valid anyway because we just encountered a case invalidating
            // a pattern length of at least c2, hence pattern_len_cand <= c2 cannot
            // be a valid pattern.
            loop {
                pattern_len_cand = match pattern_len_iter.next() {
                    Some(l) => l,
                    None => unreachable!("the last iterator item is always collection_len"),
                };

                if pattern_len_cand >= c2 {
                    break;
                }
            }

            // We can jump ahead to the next multiple of the pattern_len_cand
            c2 = ((c2 + pattern_len_cand - 1) / pattern_len_cand) * pattern_len_cand;

            // We start checking the new pattern length from the beginning again
            c1 = 0;
        }
    }

    return NonZeroUsize::new(c2 / pattern_len_cand).unwrap_or(NonZeroUsize::MIN);
}

// TODO
/// Calculate the number of basic windings with the formulae from [Pyr08],
/// section 2.11 (p. 102 ff)
///
/// TODO: every winding contains at least one base winding
///
/// What is a base winding: Repeating winding. This formula assumes symmetric
/// repetition of coils (like DistributedWinding or ToothCoilWinding) Will
/// return wrong values for arbitrary winding such as e.g. a CoilAssembly,
/// consider using repeating_pattern_count via the
/// [`Winding::base_winding_count`] wrapper instead, which can deal with
/// arbitrary windings. Default impl of [`Winding::base_winding_count`] wraps
/// repeating_pattern_count, which can deal with arbitrary windings. So this
/// method is just an optimization for particular windings.
///
/// ```
/// use winding::base_winding_count;
///
/// // 12/4 single-layer integer winding
/// assert_eq!(2, base_winding_count(12, 2, 3, 1));
///
/// // 12/10 double-layer tooth-coil winding
/// assert_eq!(1, base_winding_count(12, 5, 3, 2));
///
/// // 12/8 double-layer tooth-coil winding
/// assert_eq!(4, base_winding_count(12, 4, 3, 2));
///
/// // 36/8 single-layer fractional slot winding
/// assert_eq!(2, base_winding_count(36, 4, 3, 1));
/// ```
pub fn base_winding_count_repeating_coil_groups(
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    phases: NonZeroU16,
    layers: NonZeroU16,
) -> NonZeroU16 {
    // Inner of basic windings
    let t = NonZeroU16::new(gcd(slots.get(), pole_pairs.get()))
        .expect("cannot be zero, because gcd inputs are not zero");
    let r = hole_number(slots, pole_pairs, phases);
    let n = r.denom().clone();

    // Check if the winding is an integer slot winding
    if n == 1 {
        return t;

    // Check whether the fractional slot winding is of first or of second grade
    } else {
        // First grade fractional slot winding
        if n.is_odd() {
            return t;
        } else {
            // Single-layer winding
            if layers.get() == 1 {
                if t.get().is_odd() {
                    return t;
                } else {
                    return NonZeroU16::new(t.get() / 2).unwrap_or(NonZeroU16::MIN);
                }

            // Multi-layer winding
            } else {
                return t;
            }
        }
    }
}

/// Returns the electrical angle between the phasors of two neighboring
/// slots.
///
/// The angle is calculated as
///
/// `phasor_angle = 2π * pole_pairs / slots`.
///
/// This relation is given by Pyrhönen, J., Jokinen, T., Hrabovcová, V.:
/// Design of Rotating Electrical Machines, 1st edition, John Wiley &
/// Sons, 2008, eq. (2.66).
pub fn phasor_angle(slots: NonZeroU16, pole_pairs: NonZeroU16) -> f64 {
    return TAU * (pole_pairs.get() as f64) / (slots.get() as f64);
}
