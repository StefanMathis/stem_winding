//! Provides the [`ToothCoilWinding`] as well as its builder structs.
//!
//! See the documentation of [`ToothCoilWinding`] for details.

use std::num::{NonZeroU16, NonZeroUsize};

use num::Integer;
use stem_types::{CoilLayout, Zone};
use stem_wire::{sff::SffWire, wire::Wire};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "stem_core")]
use stem_core::prelude::*;

#[cfg(feature = "stem_core")]
use crate::core_support::*;

use crate::{
    coils::{Coil, Coils, FullCoil},
    error::{Error, WindingTableConstructionError},
    winding::{Connection, Winding, base_winding_count_symmetric},
    winding_table::{WindingTable, WindingTableConstructor},
};

/**
A symmetric [`Winding`] where each [`Coil`] is wound around a single tooth and
has a [`throw`](crate::coils::CoilExt::throw) of 1.

This winding type is often used for motors with high production numbers such as
servo, pump or fan drives. Since the coils are non-overlapping, manufacturing
can be easily automated and high slot filling factors are achieved. Compared to
a [`DistributedWinding`](crate::winding::DistributedWinding), the other
mainstream winding type for radial flux motors, tooth coil winding usually have
a higher number of pole pairs. This is due to the fact that the highest
[`winding_factor`](Winding::winding_factor)s can be achieved when the coil throw
and the pole pitch are equal. Due to the uniform coil throw of 1, the slot pitch
therefore should be equal to the pole pitch to achieve the best possible
fundamental winding factor `k_w,p = 1`. However, in practice
this exact slot / pole combination is avoided because it would cause an
extremely high cogging torque. Real-world three-phase windings therefore utilise
combinations such as 12 slots and 8 poles, 12 slots and 10 pole or 9 slots and 8
poles.

The following image shows the coil layout for a double-layer 12 / 10 winding:
 */
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Coils of a 12 / 10 DL winding][coils_tooth_coil_winding]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "coils_tooth_coil_winding",
        "docs/img/coils_tooth_coil_winding.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**
_Image created with `/examples/winding_plots.rs`, function `tooth_coil_winding()`._

# Properties

With regards to its [`Winding`] implementation, a [`ToothCoilWinding`] has the
following properties:
- It always fulfills the symmetric conditions of the [`Winding::is_symmetric`]
method. This means that its [`base_winding_count`](Winding::base_winding_count)
is calculated in constant time using the [`base_winding_count_symmetric`]
function.
- The number of turns for all coils is identical. This means that its
[`series_turns_per_phase`](Winding::series_turns_per_phase) is identical for all
phases and is calculated in constant time.
- All coils use the same [`wire`](ToothCoilWinding::wire).
- It either has one or two [`layers`](Winding::layers); the coils in a slot are
arranged as defined by [`CoilLayout::Single`] or
[`CoilLayout::DoubleHorizontal`] respectively.
*/
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Winding table of a 12 / 10 DL winding][table_tooth_coil_winding]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "table_tooth_coil_winding",
        "docs/img/table_tooth_coil_winding.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**
- Its end winding turn length is approximated as a semicircle via the
[`end_winding_half_turn_length_semicircle`] function (only available if the
`stem_core` feature is enabled). This approximation can be overriden by
providing the value directly, see
[`ToothCoilWinding::set_mean_end_winding_half_turn_length`].
- Its end winding leakage inductance is calculated with the
[`end_winding_leakage_inductance_tooth_coil`] function (only available if the
`stem_core` feature is enabled).

The [`QuadrupleLayerToothCoilWinding`](crate::winding::QuadrupleLayerToothCoilWinding)
is a derivative of this winding type which has four layers per slot.

# Constructors

A [`ToothCoilWinding`] can be built from one of its builder structs
([`ToothCoilBuilder`] or [`ToothCoilMinimalBuilder`]) via its [`TryFrom`]
implementation or with the [`new`](ToothCoilWinding::new) method. Some of the
builder field values need to uphold certain invariants; for example the number
of winding layers must be 1 or 2. This is checked during the building process,
see the documentation of [`ToothCoilBuilder`] for details.

# Serialization and deserialization

If the `serde` feature is enabled, this winding can be deserialized from a
serialized [`ToothCoilBuilder`] or [`ToothCoilMinimalBuilder`]. The serialized
representation of this winding corresponds to that of a [`ToothCoilBuilder`].

# Examples

The following example creates the 12 / 10 double-layer winding shown in the
image above and calculates some properties.

```
use approxim::assert_abs_diff_eq;

use stem_winding::prelude::*;
use stem_winding::winding::WindingHarmonic;

let builder = ToothCoilMinimalBuilder {
    slots: 12.try_into().expect("not zero"),
    pole_pairs: 5.try_into().expect("not zero"),
    phases: 3.try_into().expect("not zero"),
    layers: 2.try_into().expect("not zero"),
    winding_table_constructor: WindingTableConstructor::Tingley,
};

let winding = ToothCoilWinding::new(builder).expect("all invariants upheld");

assert_eq!(winding.base_winding_count().get(), 1);

assert_abs_diff_eq!(
    winding.winding_factor(1.try_into().expect("not zero"), SpatialOrder::Electrical(1)),
    0.933,
    epsilon = 1e-3
);

let mut iter = winding.harmonics();

// First subharmonic
assert_eq!(iter.next(),
    Some(
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(1),
            is_positive: true
        }));

// Fundamental harmonic
assert_eq!(iter.next(),
    Some(
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(5), // Equal to the first electrical order!
            is_positive: false
        }));

// First superharmonic
assert_eq!(iter.next(),
    Some(
        WindingHarmonic {
            spatial_order: SpatialOrder::Mechanical(7),
            is_positive: true
        }));
```
*/
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(into = "ToothCoilBuilder"))]
pub struct ToothCoilWinding {
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    phases: NonZeroU16,
    layers: NonZeroU16,
    turns_per_coil: NonZeroUsize,
    parallel_paths: NonZeroU16,
    connection: Connection,
    end_winding_leakage_coefficient: f64,
    wire: Box<dyn Wire>,
    #[cfg_attr(feature = "serde", serde(skip))]
    coils: Coils,
    winding_table_constructor: WindingTableConstructor,
    mean_end_winding_half_turn_length: Option<Length>,
}

impl ToothCoilWinding {
    /// Creates a new winding.
    ///
    /// This is a wrapper around `T::try_into`. The builder structs provided by
    /// the module implement [`TryInto<ToothCoilWinding>`], see their respective
    /// documentation for details on the failure modes. The struct documentation
    /// has an usage example.
    pub fn new<W>(builder: W) -> Result<Self, Error>
    where
        W: TryInto<ToothCoilWinding>,
        W::Error: Into<Error>,
    {
        builder.try_into().map_err(Into::into)
    }

    /// Returns the [`Wire`] used by all coils of `self`.
    ///
    /// As stated in the struct documentation, all coils of a
    /// [`ToothCoilWinding`] use the same wire by definition.
    pub fn wire(&self) -> &dyn Wire {
        &*self.wire
    }

    /// Returns the [`WindingTableConstructor`] used to create the
    /// [`WindingTable`] returned by the
    /// [`winding_table`](Winding::winding_table) method.
    pub fn winding_table_constructor(&self) -> &WindingTableConstructor {
        &self.winding_table_constructor
    }

    /// Returns the coefficient used for the
    /// [`end_winding_leakage_inductance`](Winding::end_winding_leakage_inductance)
    /// calculation.
    ///
    /// [`ToothCoilWinding`] uses the
    /// [`end_winding_leakage_inductance_tooth_coil`] function for implementing
    /// [`Winding::end_winding_leakage_inductance`]. This is the value passed
    /// into that function.
    pub fn end_winding_leakage_coefficient(&self) -> f64 {
        return self.end_winding_leakage_coefficient;
    }

    /// Returns the mean end-winding half-turn length, if one was provided.
    ///
    /// If this value is `Some`, [`ToothCoilWinding`] uses it for calculations
    /// such as [`Winding::resistance`] instead of calculating the length.
    /// Otherwise, the length is calculated on demand using the
    /// [`end_winding_half_turn_length_semicircle`] function. Providing a value
    /// is useful when the length is known from measurements or more precise
    /// models.
    pub fn mean_end_winding_half_turn_length(&self) -> Option<Length> {
        self.mean_end_winding_half_turn_length
    }

    /// Sets the mean end-winding half-turn length.
    ///
    /// The value must be non-negative. If a negative length is provided, the
    /// method returns an error and leaves the existing value unchanged. Passing
    /// `None` clears the value, so the length is calculated on demand. See
    /// [`ToothCoilWinding::mean_end_winding_half_turn_length`] for details on
    /// how the value is used.
    pub fn set_mean_end_winding_half_turn_length(
        &mut self,
        mean_end_winding_half_turn_length: Option<Length>,
    ) -> Result<(), compare_variables::Comparison<Length>> {
        if let Some(mean_end_winding_half_turn_length) = mean_end_winding_half_turn_length {
            let zero_length = Length::new::<meter>(0.0);
            compare_variables::compare_variables!(
                mean_end_winding_half_turn_length >= zero_length
            )?;
        }
        self.mean_end_winding_half_turn_length = mean_end_winding_half_turn_length;
        Ok(())
    }

    fn create_coils(
        &mut self,
        winding_table: &WindingTable,
        all_zones_must_be_used: bool,
    ) -> Result<(), Error> {
        for slot in 0..self.slots.get() {
            for layer in 0..self.layers.get() {
                let zone = Zone { slot, layer };
                if self.coils.occupied(zone) {
                    continue;
                }
                if let Some(coil) = self.create_single_coil(zone, winding_table) {
                    self.coils.insert(coil).map_err(Error::from)?;
                }
            }
        }

        // Check if all zones are occupied
        if all_zones_must_be_used {
            for (zone, _) in winding_table.iter_slot_major() {
                // Check if the zone is already occupied
                if !self.coils.occupied(zone) {
                    return Err(WindingTableConstructionError::EmptyZone(Some(zone)).into());
                }
            }
        }

        return Ok(());
    }

    fn create_single_coil(&self, zone: Zone, winding_table: &WindingTable) -> Option<Coil> {
        // Only create a coil if the phase is positive. Since each coil has a positive
        // and a negative phase, in the end all coils will be created anyway.
        // However, only considering positive phases makes the function much
        // simpler.
        let phase = *winding_table.get_cyclic(zone);
        if phase < 0 {
            return None;
        }
        let phase_abs = NonZeroU16::new(phase.try_into().expect("the number of phases cannot exceed the maximum value of an u16 (this is guaranteed by the winding constructor)")).expect("not zero");
        let slot = zone.slot;
        let layer = zone.layer;

        // Different treatment of single-and double-layer winding
        if self.layers().get() == 1 {
            /*
            Move "forward" through the slots of the zone plan and count the number of times that the (absolute) phase
            of the input slot is encountered. If the number is odd, the return conductor for the current zone is in "forward"
            direction as well. Otherwise, it is in the "backward direction" (with corresponding adjustment of the coil direction)
             */
            let mut counter: i32 = 0;
            for search_slot in 1..winding_table.slots().get() {
                let phase_search_slot =
                    winding_table.get_cyclic(Zone::new(search_slot + slot, layer));
                if phase_search_slot.abs() != phase.abs() {
                    let negative_zone = if counter.is_odd() {
                        Zone::new((slot + 1).rem_euclid(self.slots().get()), layer)
                    } else {
                        Zone::new(slot.checked_sub(1).unwrap_or(self.slots().get() - 1), layer)
                    };

                    return FullCoil::new(
                        Zone::new(slot, layer),
                        negative_zone,
                        counter.is_odd(),
                        self.turns_per_coil,
                        phase_abs,
                        dyn_clone::clone_box(&*self.wire),
                    )
                    .ok()
                    .map(|coil| Coil::Full(coil));
                }
                counter += 1;
            }
            panic!("zone plan does not contain a return conductor for the input conductor!")
        } else {
            let negative_zone = if layer == 0 {
                // Coil must start at the previous slot
                Zone::new(slot.checked_sub(1).unwrap_or(self.slots().get() - 1), 1)
            } else {
                // Coil must stop in the next slot
                Zone::new((slot + 1).rem_euclid(self.slots().get()), 0)
            };

            return FullCoil::new(
                Zone::new(slot, layer),
                negative_zone,
                layer == 1,
                self.turns_per_coil,
                phase_abs,
                dyn_clone::clone_box(&*self.wire),
            )
            .ok()
            .map(|coil| Coil::Full(coil));
        }
    }
}

impl Default for ToothCoilWinding {
    fn default() -> Self {
        ToothCoilBuilder {
            slots: NonZeroU16::new(6).expect("not zero"),
            pole_pairs: NonZeroU16::MIN,
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::MIN,
            turns_per_coil: NonZeroUsize::MIN,
            parallel_paths: NonZeroU16::MIN,
            connection: Connection::Star,
            end_winding_leakage_coefficient: 0.0,
            wire: Box::new(SffWire::default()),
            winding_table_constructor: WindingTableConstructor::Tingley,
            mean_end_winding_half_turn_length: None,
        }
        .try_into()
        .expect("valid inputs")
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Winding for ToothCoilWinding {
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
        self.layers
    }

    fn base_winding_count(&self) -> NonZeroU16 {
        base_winding_count_symmetric(
            self.slots(),
            self.pole_pairs(),
            self.phases(),
            self.layers(),
        )
    }

    fn coil_layout(&self) -> CoilLayout {
        if self.layers().get() == 1 {
            return CoilLayout::Single;
        } else {
            return CoilLayout::DoubleHorizontal;
        }
    }

    fn series_turns_per_phase(&self, phase: NonZeroU16) -> num::rational::Ratio<usize> {
        return num::rational::Ratio::new_raw(
            (usize::from(self.layers().get() * self.slots().get())) * self.turns_per_coil.get()
                / usize::from(2 * self.phases().get() * self.parallel_paths(phase).get()),
            1,
        );
    }

    fn parallel_paths(&self, _phase: NonZeroU16) -> NonZeroU16 {
        self.parallel_paths
    }

    fn connection(&self) -> Connection {
        return self.connection;
    }

    fn coil_at(&self, zone: Zone) -> Option<&Coil> {
        return self.coils.get(zone);
    }

    fn num_coils(&self) -> usize {
        return (self.slots().get() * self.layers().get() / 2).into();
    }

    fn as_dyn(&self) -> &dyn Winding {
        self
    }

    #[cfg(feature = "stem_core")]
    fn is_symmetric(&self, _core: CoreRef<'_>) -> bool {
        return true;
    }

    #[cfg(feature = "stem_core")]
    fn end_winding_leakage_inductance(
        &self,
        core: CoreRef<'_>,
        _phase: NonZeroU16,
        spatial_order: SpatialOrder,
        conditions: &[DynQuantity<f64>],
    ) -> Inductance {
        let _ = spatial_order;
        let _ = conditions;
        end_winding_leakage_inductance_tooth_coil(self, core, self.end_winding_leakage_coefficient)
    }

    #[cfg(feature = "stem_core")]
    fn end_winding_half_turn_length(&self, core: CoreRef<'_>, zone: Zone) -> Length {
        if let Some(l) = self.mean_end_winding_half_turn_length() {
            return l;
        }
        end_winding_half_turn_length_semicircle(self, core, zone)
    }
}

/**
A builder struct for a [`ToothCoilWinding`].

This builder can be used to create a [`ToothCoilWinding`] via its
[`new`](ToothCoilWinding::new) method or via its [`TryFrom`] implementation.
Creating the winding fails if the conditions stated on the documentation of the
individual fields are not fulfilled (for example,
[`ToothCoilBuilder::end_winding_leakage_coefficient`] must not be negative).
 */
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ToothCoilBuilder {
    /// Number of slots. Corresponds to [`Winding::slots`] in the final winding.
    pub slots: NonZeroU16,
    /// Number of pole pairs. Corresponds to [`Winding::pole_pairs`] in the
    /// final winding.
    pub pole_pairs: NonZeroU16,
    /// Number of phases. Corresponds to [`Winding::phases`] in the final
    /// winding.
    pub phases: NonZeroU16,
    /// Number of layers. Corresponds to [`Winding::layers`] in the final
    /// winding.
    pub layers: NonZeroU16,
    /// Winding table constructor used to determine the winding table of the
    /// final winding via [`WindingTable::from_constructor`]. The `span`
    /// argument to that function is always set to 1 (tooth-coil winding
    /// property), all other arguments are taken from the fields of this struct.
    /// If the construction of the winding table fails, the winding cannot
    /// be created.
    pub winding_table_constructor: WindingTableConstructor,
    /// Number of turns for each coil. Corresponds to
    /// [`CoilExt::turns`](crate::coils::CoilExt::turns).
    pub turns_per_coil: NonZeroUsize,
    /// Number of parallel paths. This value must be a divisor of the maximum
    /// number of parallel paths returned by
    /// [`max_parallel_paths_symmetric`](super::max_parallel_paths_symmetric).
    pub parallel_paths: NonZeroU16,
    /// Electrical connection of the individual phases.
    pub connection: Connection,
    /// Coefficient used to calculate
    /// [`Winding::end_winding_leakage_inductance`], see
    /// [`ToothCoilWinding::end_winding_leakage_coefficient`]. Must not be
    /// negative (`end_winding_leakage_coefficient >= 0`).
    pub end_winding_leakage_coefficient: f64,
    /// The wire used for all coils of the winding.
    pub wire: Box<dyn Wire>,
    /// An override for the end winding half-turn length, see
    /// [`ToothCoilWinding::mean_end_winding_half_turn_length`]. If provided,
    /// this value must not be negative (`mean_end_winding_half_turn_length >= 0
    /// m`).
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "deserialize_opt_quantity")
    )]
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_opt_quantity"))]
    pub mean_end_winding_half_turn_length: Option<Length>,
}

impl From<ToothCoilWinding> for ToothCoilBuilder {
    fn from(winding: ToothCoilWinding) -> Self {
        ToothCoilBuilder {
            slots: winding.slots,
            pole_pairs: winding.pole_pairs,
            phases: winding.phases,
            layers: winding.layers,
            winding_table_constructor: winding.winding_table_constructor,
            turns_per_coil: winding.turns_per_coil,
            parallel_paths: winding.parallel_paths,
            connection: winding.connection,
            end_winding_leakage_coefficient: winding.end_winding_leakage_coefficient,
            wire: winding.wire,
            mean_end_winding_half_turn_length: winding.mean_end_winding_half_turn_length,
        }
    }
}

impl TryFrom<ToothCoilBuilder> for ToothCoilWinding {
    type Error = Error;

    fn try_from(builder: ToothCoilBuilder) -> Result<Self, Self::Error> {
        let layers = builder.layers.get();
        compare_variables::compare_variables!(layers < 3)?;
        compare_variables::compare_variables!(0.0 <= builder.end_winding_leakage_coefficient)?;

        // Check if the number of parallel paths is valid
        let max_par_paths = super::max_parallel_paths_symmetric(
            builder.slots,
            builder.pole_pairs,
            builder.phases,
            builder.layers,
        )
        .get();
        if max_par_paths % builder.parallel_paths.get() != 0 {
            return Err(Error::InvalidNumberParallelPaths);
        }

        // Calculate the basic winding parameters
        let t = base_winding_count_symmetric(
            builder.slots,
            builder.pole_pairs,
            builder.phases,
            builder.layers,
        )
        .get();
        let slots_basic = builder.slots.get() / t;
        let pole_pairs_basic = builder.pole_pairs.get() / t;

        // Create the zone plan by method
        let winding_table = WindingTable::from_constructor(
            &builder.winding_table_constructor,
            NonZeroU16::new(slots_basic).expect("not zero"),
            builder.layers,
            NonZeroU16::new(pole_pairs_basic).expect("not zero"),
            builder.phases,
            1, // Equals 1 by definition
        )?;

        let num_zones = (builder.slots.get() * builder.layers.get()).into();
        let num_coils = num_zones / 2;
        let mut winding = ToothCoilWinding {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            layers: builder.layers,
            turns_per_coil: builder.turns_per_coil,
            parallel_paths: builder.parallel_paths,
            connection: builder.connection,
            end_winding_leakage_coefficient: builder.end_winding_leakage_coefficient,
            wire: builder.wire,
            coils: Coils::with_capacity(num_zones, num_coils),
            winding_table_constructor: builder.winding_table_constructor,
            mean_end_winding_half_turn_length: None,
        };
        winding.set_mean_end_winding_half_turn_length(builder.mean_end_winding_half_turn_length)?;
        winding.create_coils(&winding_table, true)?;

        if winding.equal_winding_factors() {
            return Ok(winding);
        } else {
            return Err(WindingTableConstructionError::NotSymmetric.into());
        }
    }
}

/**
A minimal builder struct for a [`ToothCoilWinding`].

This struct is useful if one is interested in a qualitative analysis for the
specified topology (e.g. determining the winding factor or the harmonics).
Compared to the fully-featured [`ToothCoilBuilder`], values such as the
[`ToothCoilBuilder::end_winding_leakage_coefficient`] are set to sensible
defaults. For quantitative calculations (e.g. the [`Winding::resistance`]), use
[`ToothCoilBuilder`] instead.
 */
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ToothCoilMinimalBuilder {
    /// Number of slots. Corresponds to [`Winding::slots`] in the final winding.
    pub slots: NonZeroU16,
    /// Number of pole pairs. Corresponds to [`Winding::pole_pairs`] in the
    /// final winding.
    pub pole_pairs: NonZeroU16,
    /// Number of phases. Corresponds to [`Winding::phases`] in the final
    /// winding.
    pub phases: NonZeroU16,
    /// Number of layers. Corresponds to [`Winding::layers`] in the final
    /// winding.
    pub layers: NonZeroU16,
    /// Winding table constructor used to determine the winding table of the
    /// final winding via [`WindingTable::from_constructor`]. The `span`
    /// argument to that function is always set to 1 (tooth-coil winding
    /// property), all other arguments are taken from the fields of this struct.
    /// If the construction of the winding table fails, the winding cannot
    /// be created.
    pub winding_table_constructor: WindingTableConstructor,
}

impl From<ToothCoilWinding> for ToothCoilMinimalBuilder {
    fn from(winding: ToothCoilWinding) -> Self {
        ToothCoilMinimalBuilder {
            slots: winding.slots,
            pole_pairs: winding.pole_pairs,
            phases: winding.phases,
            layers: winding.layers,
            winding_table_constructor: winding.winding_table_constructor,
        }
    }
}

impl TryFrom<ToothCoilMinimalBuilder> for ToothCoilWinding {
    type Error = Error;

    fn try_from(builder: ToothCoilMinimalBuilder) -> Result<Self, Self::Error> {
        ToothCoilBuilder {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            layers: builder.layers,
            turns_per_coil: NonZeroUsize::MIN,
            parallel_paths: NonZeroU16::MIN,
            connection: Connection::Star,
            end_winding_leakage_coefficient: 0.0,
            wire: Box::new(SffWire::default()),
            winding_table_constructor: builder.winding_table_constructor,
            mean_end_winding_half_turn_length: None,
        }
        .try_into()
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for ToothCoilWinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(deserialize_untagged_verbose_error::DeserializeUntaggedVerboseError)]
        enum ToothCoilEnum {
            ToothCoilBuilder(ToothCoilBuilder),
            ToothCoilMinimalBuilder(ToothCoilMinimalBuilder),
        }
        let w = ToothCoilEnum::deserialize(deserializer)?;
        match w {
            ToothCoilEnum::ToothCoilBuilder(w) => w.try_into().map_err(serde::de::Error::custom),
            ToothCoilEnum::ToothCoilMinimalBuilder(w) => {
                w.try_into().map_err(serde::de::Error::custom)
            }
        }
    }
}
