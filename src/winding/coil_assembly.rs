use std::{collections::HashMap, num::NonZeroU16};

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

/// Phase-specific data of a [`CoilAssembly`].
///
/// A [`CoilAssembly`] holds a vector of these structs which it uses for
/// determining phase-specific data. For example, when retrieving the number
/// of [`parallel_paths`](Winding::parallel_paths) for the nth phase,
/// [`CoilAssembly`] returns [`PhaseData::parallel_paths`] for the (n-1)th item
/// of the vector (phase 1 returns the value of vector[0], phase 2 that of
/// vector[1] and so on). See [`CoilAssembly::phase_data`] for details. It is
/// also possible to modify the phase data of an existing [`CoilAssembly`] using
/// [`CoilAssembly::phase_data_mut`].
///
/// The optional [`VarQuantity`] fields can be used to override the default
/// implementations of the [`Winding`] methods with the same name. Since a
/// [`CoilAssembly`] has no winding-specific information, this mechanic can be
/// used to let a [`CoilAssembly`] behave in the same way as the winding it has
/// been derived from. For an example, see
/// [`CoilAssembly::from_winding_and_core`].
///
/// [`PhaseData`] implements the [`Default`] trait:
/// [`PhaseData::parallel_paths`] is set to 1 and every other field to `None`.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PhaseData {
    /// Number of [`parallel_paths`](Winding::parallel_paths) of the phase.
    pub parallel_paths: NonZeroU16,
    /// An override for the [`Winding::end_winding_leakage_inductance`]. If this
    /// value is not given, the default implementation of the trait method is
    /// used instead.
    pub end_winding_leakage_inductance: Option<VarQuantity<Inductance>>,
    /// An override for the [`Winding::slot_leakage_inductance`]. If this value
    /// is not given, the default implementation of the trait method is used
    /// instead.
    pub slot_leakage_inductance: Option<VarQuantity<Inductance>>,
    /// An override for the [`Winding::resistance`]. If this value is not given,
    /// the default implementation of the trait method is used instead.
    pub resistance: Option<VarQuantity<ElectricalResistance>>,
}

impl Default for PhaseData {
    fn default() -> Self {
        Self {
            parallel_paths: NonZeroU16::MIN,
            resistance: Default::default(),
            slot_leakage_inductance: Default::default(),
            end_winding_leakage_inductance: Default::default(),
        }
    }
}

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
[`Winding::end_winding_half_turn_length`]. To overcome this limitation, it
offers the [`phase_data_mut`](CoilAssembly::phase_data_mut) and
[`end_winding_half_turn_length_overrides`](CoilAssembly::end_winding_half_turn_length_overrides)
methods where specific properties of the winding can be set manually. If a
non-provided property is requested, [`CoilAssembly`] falls back onto the default
trait method implementation of [`Winding`]. This can be used to have a
[`CoilAssembly`] behave in the same way as a winding with specialized
implementations. [`CoilAssembly::from_winding_and_core`] shows an example.

# Constructors

A [`CoilAssembly`] can be derived directly from another [`Winding`], as shown
above. Alternatively, it can be derived from a [`CoilAssemblyBuilder`] via
[`TryFrom`] / [`CoilAssembly::new`]. When using the builder, each coil is
inserted individually into the resulting assembly to ensure that it satisfies
the invariants documented for [`CoilAssembly::insert`]. If the `stem_core`
feature is enabled, the [`CoilAssembly::from_winding_and_core`] method provides
a convenience wrapper for populating the overrides directly from a winding and
a core during construction.

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
    CoilAssemblyBuilder {
        slots: 6.try_into().expect("not zero"),
        pole_pairs: 1.try_into().expect("not zero"),
        coil_layout: CoilLayout::DoubleVertical,
        coils: Default::default(), // No coils initially
        phase_data: vec![Default::default(); 3],
        connection: Connection::Star,
        end_winding_half_turn_length_overrides: Default::default(),
    }
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
#[cfg_attr(feature = "serde", serde(into = "CoilAssemblyBuilder"))]
pub struct CoilAssembly {
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    coil_layout: CoilLayout,
    coils: Coils,
    phase_data: Vec<PhaseData>,
    connection: Connection,
    end_winding_half_turn_length_overrides: HashMap<Zone, Length>,
}

impl CoilAssembly {
    /// Creates a new winding.
    ///
    /// This is a wrapper around `T::try_into`. The builder structs provided by
    /// the module implement [`TryInto<CoilAssembly>`], see their
    /// respective documentation for details on the failure modes. The
    /// struct documentation has an usage example.
    pub fn new<W>(builder: W) -> Result<Self, Error>
    where
        W: TryInto<CoilAssembly>,
        W::Error: Into<Error>,
    {
        builder.try_into().map_err(Into::into)
    }

    /// Creates a [`CoilAssembly`] from `winding` where the
    /// [`end_winding_half_turn_length_overrides`](CoilAssembly::end_winding_half_turn_length_overrides)
    /// and the [`phase_data`](CoilAssembly::phase_data) has been populated
    /// using `core`.
    ///
    /// This constructor first derives a [`CoilAssembly`] from the given
    /// `winding` via the [`From`] implementation. Afterwards, it populates
    /// the overrides using the methods of `winding` together with the `core`:
    /// - [`end_winding_half_turn_length_overrides`](CoilAssembly::end_winding_half_turn_length_overrides)
    ///   zone by zone from [`Winding::end_winding_half_turn_length`].
    /// - [`PhaseData::end_winding_leakage_inductance`] phase by phase from
    ///   [`Winding::end_winding_leakage_inductance`].
    /// - [`PhaseData::slot_leakage_inductance`] phase by phase from
    ///   [`Winding::slot_leakage_inductance`].
    /// - [`PhaseData::resistance`] phase by phase from [`Winding::resistance`].
    ///
    /// The other two arguments `spatial_order` and `effective_air_gap` are used
    /// as inputs to the trait method. The resulting coil assembly returns the
    /// same quantitative results as the original `winding` would for the given
    /// `core`, `spatial_order` and `effective_air_gap`. Hence, it can be used
    /// for quantitative analysis.
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
    ///     coil_throw_reduction: 1, // Short-pitching
    ///     zone_shift: 0,
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
    /// let spatial_order = SpatialOrder::Electrical(1);
    /// let effective_air_gap = Length::new::<millimeter>(0.6);
    ///
    /// let coil_assembly = CoilAssembly::from_winding_and_core(
    ///     &winding,
    ///     core.as_core_ref(),
    ///     spatial_order,
    ///     effective_air_gap
    /// );
    ///
    /// // Comparison of the total winding inductance
    /// assert_abs_diff_eq!(
    ///     winding.inductance(
    ///         core.as_core_ref(),
    ///         NonZeroU16::MIN,
    ///         spatial_order,
    ///         effective_air_gap
    ///     ).eval(&[]).get::<millihenry>(),
    ///     coil_assembly.inductance(
    ///         core.as_core_ref(),
    ///         NonZeroU16::MIN,
    ///         spatial_order,
    ///         effective_air_gap
    ///     ).eval(&[]).get::<millihenry>(),
    ///     epsilon = 1e-6,
    /// );
    /// ```
    #[cfg(feature = "stem_core")]
    pub fn from_winding_and_core<W: Winding + ?Sized>(
        winding: &W,
        core: CoreRef<'_>,
        spatial_order: SpatialOrder,
        effective_air_gap: Length,
    ) -> Self {
        let mut coil_assembly = CoilAssembly::from(winding);

        // Populate the overrides
        for slot in 0..winding.slots().get() {
            for layer in 0..winding.layers().get() {
                let zone = Zone::new(slot, layer);
                coil_assembly
                    .end_winding_half_turn_length_overrides
                    .insert(zone, winding.end_winding_half_turn_length(core, zone));
            }
        }

        for idx in 0..winding.phases().get() {
            let phase = NonZeroU16::new(idx + 1).expect("not zero");
            let phase_data = &mut coil_assembly.phase_data[usize::from(idx)];

            phase_data.end_winding_leakage_inductance =
                Some(winding.end_winding_leakage_inductance(core, phase, spatial_order));

            phase_data.slot_leakage_inductance =
                Some(winding.slot_leakage_inductance(core, phase, effective_air_gap));

            phase_data.resistance = Some(winding.resistance(core, phase));
        }
        coil_assembly
    }

    /// Returns a reference to the overrides for the
    /// [`Winding::end_winding_half_turn_length`].
    ///
    /// The [`Winding::end_winding_half_turn_length`] implementation of
    /// [`CoilAssembly`] returns the value from this map for a particular
    /// [`Zone`], if one is available and it is not negative. Otherwise, it
    /// falls back to the default calculation routines described in the
    /// trait method documentation.
    pub fn end_winding_half_turn_length_overrides(&self) -> &HashMap<Zone, Length> {
        &self.end_winding_half_turn_length_overrides
    }

    /// Returns a mutable reference to the overrides for the
    /// [`Winding::end_winding_half_turn_length`].
    ///
    /// This method can be used to add, change or remove overrides from the
    /// [`CoilAssembly::end_winding_half_turn_length_overrides`] map. Negative
    /// values are treated as non-existant when reading values from this map
    /// within [`CoilAssembly::end_winding_half_turn_length`].
    pub fn end_winding_half_turn_length_overrides_mut(&mut self) -> &mut HashMap<Zone, Length> {
        &mut self.end_winding_half_turn_length_overrides
    }

    /// Returns references to the [`PhaseData`] for all phases of `self`.
    ///
    /// The [`PhaseData`] vector specifies certain properties such as the
    /// [`parallel_paths`](Winding::parallel_paths) for each phase of the
    /// assembly. Additionally, it provides the opportunity to override certain
    /// physical properties such as the phase resistance. This can be useful to
    /// create a [`CoilAssembly`] which imitates the behaviour of the winding it
    /// has been derived from, see [`CoilAssembly::from_winding_and_core`] for
    /// an example (only available with the `stem_core` feature enables).
    ///
    /// The zeroth entry of the slice holds the data for the first phase, the
    /// first entry that of the second phase and so on. The length of this slice
    /// therefore determines the number of [`CoilAssembly::phases`].
    ///
    /// # Examples
    ///
    /// In this example, a resistance override has been provided for two phases
    /// during construction. The third value is calculated.
    ///
    /// ```
    /// use stem_winding::prelude::*;
    ///
    /// let phase_data = vec![
    ///     PhaseData {
    ///         resistance: Some(ElectricalResistance::new::<ohm>(1.0).into()),
    ///         ..Default::default()
    ///     },
    ///     PhaseData {
    ///         resistance: Some(ElectricalResistance::new::<ohm>(2.0).into()),
    ///         ..Default::default()
    ///     },
    ///     PhaseData::default()
    /// ];
    /// assert_eq!(phase_data.len(), 3);
    ///
    /// let mut winding = CoilAssembly::new(
    ///     CoilAssemblyBuilder {
    ///         slots: 6.try_into().expect("not zero"),
    ///         pole_pairs: 1.try_into().expect("not zero"),
    ///         coil_layout: CoilLayout::Single,
    ///         coils: Default::default(),
    ///         phase_data,
    ///         connection: Connection::Star,
    ///         end_winding_half_turn_length_overrides: Default::default(),
    ///     }
    /// ).expect("valid");
    /// assert_eq!(winding.phases().get(), 3);
    ///
    /// let core = RotCore::from_winding(&winding);
    ///
    /// // Values from the overrides
    /// assert_eq!(
    ///     winding.resistance(core.as_core_ref(), 1.try_into().expect("not zero")).eval(&[]).get::<ohm>(),
    ///     1.0
    /// );
    /// assert_eq!(
    ///     winding.resistance(core.as_core_ref(), 2.try_into().expect("not zero")).eval(&[]).get::<ohm>(),
    ///     2.0
    /// );
    ///
    /// // Since we never inserted a coil in the assembly, the calculated resistance is simply 0.
    /// assert_eq!(
    ///     winding.resistance(core.as_core_ref(), 3.try_into().expect("not zero")).eval(&[]).get::<ohm>(),
    ///     0.0
    /// );
    /// ```
    pub fn phase_data(&self) -> &[PhaseData] {
        &self.phase_data
    }

    /// Returns mutable references to the [`PhaseData`] for all phases of
    /// `self`.
    ///
    /// This mutable slice can be used to change the [`PhaseData`] of individual
    /// phases. For example, it can be used to create a winding with asymmetric
    /// phase resistances as shown in the example below. For more details, see
    /// [`CoilAssembly::phase_data`].
    ///
    /// # Examples
    ///
    /// ```
    /// use stem_winding::prelude::*;
    ///
    /// let mut winding = CoilAssembly::new(
    ///     CoilAssemblyBuilder {
    ///         slots: 6.try_into().expect("not zero"),
    ///         pole_pairs: 1.try_into().expect("not zero"),
    ///         coil_layout: CoilLayout::Single,
    ///         coils: Default::default(),
    ///         phase_data: vec![Default::default(); 3],
    ///         connection: Connection::Star,
    ///         end_winding_half_turn_length_overrides: Default::default(),
    ///     }
    /// ).expect("valid");
    ///
    /// // Set the resistance of phase 1 and 2 to 1 Ohm and that of phase 3 to 1.5 Ohm
    /// winding.phase_data_mut()[0].resistance = Some(ElectricalResistance::new::<ohm>(1.0).into());
    /// winding.phase_data_mut()[1].resistance = Some(ElectricalResistance::new::<ohm>(1.0).into());
    /// winding.phase_data_mut()[2].resistance = Some(ElectricalResistance::new::<ohm>(1.5).into());
    ///
    /// let core = RotCore::from_winding(&winding);
    /// assert_eq!(
    ///     winding.resistance(core.as_core_ref(), 1.try_into().expect("not zero")).eval(&[]).get::<ohm>(),
    ///     1.0
    /// );
    /// assert_eq!(
    ///     winding.resistance(core.as_core_ref(), 2.try_into().expect("not zero")).eval(&[]).get::<ohm>(),
    ///     1.0
    /// );
    /// assert_eq!(
    ///     winding.resistance(core.as_core_ref(), 3.try_into().expect("not zero")).eval(&[]).get::<ohm>(),
    ///     1.5
    /// );
    /// ```
    pub fn phase_data_mut(&mut self) -> &mut [PhaseData] {
        &mut self.phase_data
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
        NonZeroU16::new(self.phase_data.len() as u16).unwrap_or(NonZeroU16::MIN)
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
        self.phase_data
            .get(usize::from(phase.get() - 1))
            .map(|pd| pd.parallel_paths)
            .unwrap_or(NonZeroU16::MIN)
    }

    fn connection(&self) -> Connection {
        self.connection
    }

    fn as_dyn(&self) -> &dyn Winding {
        self
    }

    #[cfg(feature = "stem_core")]
    fn end_winding_half_turn_length(&self, core: CoreRef<'_>, zone: Zone) -> Length {
        use crate::core_support::*;

        // Do we have a valid override?
        if let Some(length) = self
            .end_winding_half_turn_length_overrides
            .get(&zone)
            .cloned()
        {
            if length >= Length::new::<meter>(0.0) {
                return length;
            }
        }

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

    #[cfg(feature = "stem_core")]
    fn end_winding_leakage_inductance(
        &self,
        core: CoreRef<'_>,
        phase: NonZeroU16,
        spatial_order: SpatialOrder,
    ) -> VarQuantity<Inductance> {
        let _ = core;
        let _ = spatial_order;

        if let Some(inductance) = self
            .phase_data()
            .get(usize::from(phase.get()) - 1)
            .map(|data| data.end_winding_leakage_inductance.clone())
            .flatten()
        {
            return inductance;
        }

        return Default::default();
    }

    #[cfg(feature = "stem_core")]
    fn slot_leakage_inductance(
        &self,
        core: CoreRef<'_>,
        phase: NonZeroU16,
        effective_air_gap: Length,
    ) -> VarQuantity<Inductance> {
        if let Some(inductance) = self
            .phase_data()
            .get(usize::from(phase.get()) - 1)
            .map(|data| data.slot_leakage_inductance.clone())
            .flatten()
        {
            return inductance;
        }

        super::slot_leakage_inductance(self, core, phase, effective_air_gap)
    }

    #[cfg(feature = "stem_core")]
    fn resistance(
        &self,
        core: CoreRef<'_>,
        phase: NonZeroU16,
    ) -> VarQuantity<ElectricalResistance> {
        if let Some(resistance) = self
            .phase_data()
            .get(usize::from(phase.get()) - 1)
            .map(|data| data.resistance.clone())
            .flatten()
        {
            return resistance;
        }

        super::resistance(self, core, phase)
    }
}

impl<W: Winding + ?Sized> From<&W> for CoilAssembly {
    fn from(winding: &W) -> Self {
        // Coils
        let mut coils = Coils::with_capacity(winding.num_coils(), winding.num_coils());
        for coil in winding.coils_iter() {
            coils
                .insert(coil.clone())
                .expect("two coils occupy the same zone. This is a bug.")
        }

        let mut phase_data = Vec::with_capacity(winding.phases().get().into());
        for phase in 1..(winding.phases().get() + 1) {
            phase_data.push(PhaseData {
                parallel_paths: winding.parallel_paths(phase.try_into().expect("not zero")),
                ..Default::default()
            })
        }

        return CoilAssembly {
            slots: winding.slots(),
            pole_pairs: winding.pole_pairs(),
            coils,
            coil_layout: winding.coil_layout(),
            connection: winding.connection(),
            phase_data,
            end_winding_half_turn_length_overrides: Default::default(),
        };
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CoilAssemblyBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub coil_layout: CoilLayout,
    pub coils: Coils,
    pub phase_data: Vec<PhaseData>,
    #[cfg_attr(feature = "serde", serde(default = "connection_default"))]
    pub connection: Connection,
    pub end_winding_half_turn_length_overrides: HashMap<Zone, Length>,
}

#[cfg(feature = "serde")]
fn connection_default() -> Connection {
    Connection::Star
}

impl From<CoilAssembly> for CoilAssemblyBuilder {
    fn from(value: CoilAssembly) -> Self {
        Self {
            slots: value.slots,
            pole_pairs: value.pole_pairs,
            coil_layout: value.coil_layout,
            coils: value.coils,
            phase_data: value.phase_data,
            connection: value.connection,
            end_winding_half_turn_length_overrides: value.end_winding_half_turn_length_overrides,
        }
    }
}

impl TryFrom<CoilAssemblyBuilder> for CoilAssembly {
    type Error = Error;

    fn try_from(builder: CoilAssemblyBuilder) -> Result<Self, Self::Error> {
        let phase_data_len = builder.phase_data.len();
        compare_variables::compare_variables!(phase_data_len > 0)?;

        let mut winding = CoilAssembly {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phase_data: builder.phase_data,
            coils: Default::default(),
            coil_layout: builder.coil_layout,
            connection: builder.connection,
            end_winding_half_turn_length_overrides: builder.end_winding_half_turn_length_overrides,
        };

        for coil in builder.coils.into_iter() {
            winding.insert(coil)?;
        }

        // Verify the overrides
        let zero_length = Length::new::<meter>(0.0);
        for length in winding
            .end_winding_half_turn_length_overrides()
            .values()
            .cloned()
        {
            compare_variables::compare_variables!(length >= zero_length)?;
        }

        return Ok(winding);
    }
}
