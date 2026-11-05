//! Provides the [`QuadrupleLayerToothCoilWinding`] - a variation of the
//! [`ToothCoilWinding`](crate::winding::ToothCoilWinding) - as well as its
//! builder structs.
//!
//! See the documentation of [`QuadrupleLayerToothCoilWinding`] for details.

use std::num::{NonZeroU16, NonZeroUsize};

use compare_variables::{Comparison, compare_variables};
use dyn_clone::clone_box;
use num::Integer;
use stem_types::{CoilLayout, Zone};
use stem_wire::{sff::SffWire, wire::Wire};

#[cfg(feature = "stem_core")]
use stem_core::prelude::*;

#[cfg(feature = "stem_core")]
use crate::core_support::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    coils::{Coil, Coils, FullCoil},
    error::{Error, WindingTableConstructionError},
    winding::{Connection, Winding, base_winding_count_symmetric, hole_number},
    winding_table::{WindingTable, WindingTableConstructor},
};

// Shorter aliases
const LL: u16 = stem_types::QUADRUPLE_LAYER_BOTTOM_LEFT;
const UL: u16 = stem_types::QUADRUPLE_LAYER_TOP_LEFT;
const UR: u16 = stem_types::QUADRUPLE_LAYER_TOP_RIGHT;
const LR: u16 = stem_types::QUADRUPLE_LAYER_BOTTOM_RIGHT;

/**
A symmetric tooth coil winding with four layers.

This winding is derived from a double-layer
[`ToothCoilWinding`](crate::winding::ToothCoilWinding) by splitting each layer
horizontally, resulting in four layers in total. This provides additional design
parameters - in particular, the possibility to shift the upper against the lower
layers - at the cost of higher manufacturing expenses. This shift in turn allows
using unequal number of turns per coil for individual coils while keeping the
total number of turns per slot constant.

As an example, consider a 9-slot / 8-pole configuration as showcased in
[\[1\]](#qltcw_1). When initially derived from the corresponding double-layer
[`ToothCoilWinding`](crate::winding::ToothCoilWinding), the winding table looks
like this:

*/
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Quadruple layer winding without shift][table_quadruple_layer_tooth_coil_winding_0]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "table_quadruple_layer_tooth_coil_winding_0",
        "docs/img/table_quadruple_layer_tooth_coil_winding_0.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

_This image, as well as the others in this documentation, has been created with
`/examples/winding_plots.rs`, function `quadruple_layer_tooth_coil_winding()`._

The sum of turns in layer 0 and layer 1 must be equal to the total number of
[`turns_per_slot_side`](QuadrupleLayerToothCoilWinding::turns_per_slot_side).
The same is true for layer 2 and 3. Therefore, the sum of all turns in a slot is
always equal to 2*`turns_per_slot_side`.

By defining a vector of shifted upper layer turns ([`turns_upper_layer_coils`](QuadrupleLayerToothCoilWinding::turns_upper_layer_coils)), the upper layer is
shifted against the lower layer by the vector length and the number of turns in
the upper layer are defined by the respective values in this vector.

# Example 1: 9-slot / 8-pole winding with a one-slot shift of the upper layer.

- [`turns_per_slot_side`](QuadrupleLayerToothCoilWinding::turns_per_slot_side): 10
- [`turns_upper_layer_coils`](QuadrupleLayerToothCoilWinding::turns_upper_layer_coils): [4]

This input gives the constructor the following instruction: Shift the upper
layer by one slot (since the length of
[`turns_upper_layer_coils`](QuadrupleLayerToothCoilWinding::turns_upper_layer_coils)
is 1) and set the number of turns in the shifted layer to 3. The number of turns
in the lower layer is therefore 6.

*/
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Quadruple layer winding with a one-slot shift][table_quadruple_layer_tooth_coil_winding_1]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "table_quadruple_layer_tooth_coil_winding_1",
        "docs/img/table_quadruple_layer_tooth_coil_winding_1.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

# Example 2: 9-slot / 8-pole winding with a two-slot shift of the upper layer.

- [`turns_per_slot_side`](QuadrupleLayerToothCoilWinding::turns_per_slot_side): 10
- [`turns_upper_layer_coils`](QuadrupleLayerToothCoilWinding::turns_upper_layer_coils): [4, 3]

This input gives the constructor the following instruction: Shift the upper
layer by two slots (since the length of
[`turns_upper_layer_coils`](QuadrupleLayerToothCoilWinding::turns_upper_layer_coils)
is 2) and set the number of turns to 4 in the first shifted coil and to 3 in the
second shifted coil.

*/
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Quadruple layer winding with a two-slot shift][table_quadruple_layer_tooth_coil_winding_2]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "table_quadruple_layer_tooth_coil_winding_2",
        "docs/img/table_quadruple_layer_tooth_coil_winding_2.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

The maximum length of
[`turns_upper_layer_coils`](QuadrupleLayerToothCoilWinding::turns_upper_layer_coils)
must be smaller than `z`, where `z` is the numerator of the number of slots per
pole and phase `q = z/n` (see [`Winding::hole_number`]). It is also possible to
have an empty vector as an input, meaning that no layer shift occurs.

As outlined in a variety of publications on this winding type
([\[1\]](#qltcw_1), [\[2\]](#qltcw_2) [\[3\]](#qltcw_3) or [\[4\]](#qltcw_4),
shifting the layers and adjusting the number of turns in the slots shared by
two phases can be used to suppress unwanted harmonics. See the example at the
end of this docstring for a practical demonstration.

# Properties

Because a [`QuadrupleLayerToothCoilWinding`] is derived from a
[`ToothCoilWinding`](crate::winding::ToothCoilWinding), it shares most of its
properties. In particular:
- It always fulfills the symmetric conditions of the [`Winding::is_symmetric`]
  method. This means that its [`base_winding_count`](Winding::base_winding_count)
  is calculated in constant time using the [`base_winding_count_symmetric`]
  function.
- The number of [`series_turns_per_phase`](Winding::series_turns_per_phase) for
  all phases is identical. As stated above, the number of turns for individual
  coils may differ, however the number of
  [`turns_per_slot_side`](QuadrupleLayerToothCoilWinding::turns_per_slot_side)
  is identical, as is therefore the number of turns per slot.
- All coils use the same [`wire`](ToothCoilWinding::wire).
- It uses the [`CoilLayout::Quadruple`].
- Its end winding turn length is approximated as a semicircle via the
  [`end_winding_half_turn_length_semicircle`] function (only available if the
  `stem_core` feature is enabled). This approximation can be overridden by
  providing the value directly, see
  [`QuadrupleLayerToothCoilWinding::set_end_winding_half_turn_length_override`].
- Its end winding leakage inductance is calculated with the
  [`end_winding_leakage_inductance_tooth_coil`] function (only available if the
  `stem_core` feature is enabled).

# Constructors

A [`QuadrupleLayerToothCoilWinding`] can be built from one of its builder
structs ([`QuadrupleLayerToothCoilBuilder`] or
[`QuadrupleLayerToothCoilMinimalBuilder`]) via its [`TryFrom`]
implementation or with the [`new`](QuadrupleLayerToothCoilWinding::new) method.
Some of the builder field values need to uphold certain invariants, see for
example the upper size limit of
[`turns_upper_layer_coils`](QuadrupleLayerToothCoilWinding::turns_upper_layer_coils)
outlined in the first section. This is checked during the building process,
see the documentation of [`QuadrupleLayerToothCoilBuilder`] for details.

# Serialization and deserialization

If the `serde` feature is enabled, this winding can be deserialized from a
serialized [`QuadrupleLayerToothCoilBuilder`] or
[`QuadrupleLayerToothCoilMinimalBuilder`]. The serialized representation of this
winding corresponds to that of a [`QuadrupleLayerToothCoilBuilder`].

# Literature

<a id="qltcw_1">\[1\]</a>
Alberti, L. and Bianchi, N.: Theory and design of fractional-slot multilayer
windings. 2011 IEEE Energy Conversion Congress and Exposition, DOI: 10.1109/ECCE.2011.6064188

<a id="qltcw_2">\[2\]</a>
Kim, H.-J., Kim, D.-J. and Hong, J.-P.: Characteristic analysis for concentrated
multi-layer winding machine with optimum turn ratio. IEEE Transactions on
Magnetics, Vol. 50, No. 2, February 2014.

<a id="qltcw_3">\[3\]</a>
Tessarolo, A.: Improved four-layer winding design for a 12-slot 10-pole
permanent magnet machine using unequal tooth coils. IECON 2016 - 42nd Annual
Conference of the IEEE Industrial Electronics Society

<a id="qltcw_4">\[4\]</a>
Wang, Y., Qu, R. and Li, J.: Multilayer Windings Effect on Interior PM Machines
for EV Applications. IEEE Transactions on industry applications, Vol. 51, No. 3,
May/June 2015.

# Examples


 */
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(into = "QuadrupleLayerToothCoilBuilder"))]
pub struct QuadrupleLayerToothCoilWinding {
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    phases: NonZeroU16,
    turns_per_slot_side: NonZeroUsize,
    turns_upper_layer_coils: Vec<NonZeroUsize>,
    parallel_paths: NonZeroU16,
    connection: Connection,
    end_winding_leakage_coefficient: f64,
    wire: Box<dyn Wire>,
    #[cfg_attr(feature = "serde", serde(skip))]
    coils: Coils,
    winding_table_constructor: WindingTableConstructor,
    end_winding_half_turn_length_override: Option<Length>,
}

impl QuadrupleLayerToothCoilWinding {
    pub fn new<W>(builder: W) -> Result<Self, Error>
    where
        W: TryInto<QuadrupleLayerToothCoilWinding>,
        W::Error: Into<Error>,
    {
        builder.try_into().map_err(Into::into)
    }

    pub fn upper_layer_shift(&self) -> usize {
        self.turns_upper_layer_coils.len()
    }

    pub fn turns_per_slot_side(&self) -> NonZeroUsize {
        self.turns_per_slot_side
    }

    pub fn turns_upper_layer_coils(&self) -> &[NonZeroUsize] {
        self.turns_upper_layer_coils.as_slice()
    }

    pub fn wire(&self) -> &dyn Wire {
        &*self.wire
    }

    pub fn winding_table_constructor(&self) -> &WindingTableConstructor {
        &self.winding_table_constructor
    }

    pub fn end_winding_leakage_coefficient(&self) -> f64 {
        self.end_winding_leakage_coefficient
    }

    pub fn end_winding_half_turn_length_override(&self) -> Option<Length> {
        self.end_winding_half_turn_length_override
    }

    pub fn set_end_winding_half_turn_length_override(
        &mut self,
        end_winding_half_turn_length_override: Option<Length>,
    ) -> Result<(), Comparison<Length>> {
        if let Some(end_winding_half_turn_length_override) = end_winding_half_turn_length_override {
            let zero_length = Length::new::<meter>(0.0);
            compare_variables!(end_winding_half_turn_length_override >= zero_length)?;
        }
        self.end_winding_half_turn_length_override = end_winding_half_turn_length_override;
        Ok(())
    }

    /**
    Calculate the number of turns in the given zone
     */
    fn calculated_turns_at(&self, zone: Zone, winding_table: &WindingTable) -> NonZeroUsize {
        /*
        Get the second layer which is on the same slot side as the zone
         */
        let second_layer_on_zone_side = match zone.layer {
            LL => UL,
            UL => LL,
            UR => LR,
            LR => UR,
            _ => panic!("a quadruple layer winding must have 4 layers"),
        };

        /*
        Check if both upper and lower layer have the same phase.
        If yes, this means that the entire side has the same phase and therefore `self.turns_per_slot_side()`
        can be splitted arbitrarily between the two coils.
        If no, the distance of this coil to the next "full" side with the same phase is calculated
        and the number of coils is determined from `self.turns_per_slot_side()` and `self.turns_per_coil()`.
        */
        if winding_table.get_cyclic(zone)
            == winding_table.get_cyclic(Zone::new(zone.slot, second_layer_on_zone_side))
        {
            // Arbitrary split of the number of turns per slot side
            // => Lower layer gets half of the number of turns per slot side (rounded up),
            // upper layer gets half of the number of turns per slot side (rounded down)
            if zone.layer == LL || zone.layer == LR {
                return num::Integer::div_ceil(&self.turns_per_slot_side.get(), &2)
                    .try_into()
                    .unwrap_or(NonZeroUsize::MIN);
            } else {
                return num::Integer::div_floor(&self.turns_per_slot_side.get(), &2)
                    .try_into()
                    .unwrap_or(NonZeroUsize::MIN);
            }
        } else {
            let upper_layer_shift = self.upper_layer_shift() as u16;

            // Check if we're in the upper or the lower layer. If we're in the lower layer,
            // we need to search "forwards" for the next full side. If we're in the lower
            // layer, we need to go back.
            let search_forward = zone.layer == LL || zone.layer == LR;

            for idx in 0..upper_layer_shift {
                let offset = idx + 1;

                // Calculate the current "search slot"
                let search_slot = if search_forward {
                    zone.slot + offset
                } else {
                    zone.slot
                        .checked_sub(offset)
                        .unwrap_or(zone.slot + self.slots().get() - offset)
                };

                /*
                Check if the search slot contains a full half of a single phase
                 */
                // If true, we've found the next full side and can therefore derive the correct
                // value from self.turns_per_coil().
                if winding_table.get_cyclic(Zone::new(search_slot, zone.layer))
                    == winding_table.get_cyclic(Zone::new(search_slot, second_layer_on_zone_side))
                {
                    // For the lower layer, the number of turns per coil is the total number of
                    // turns of the slot side minus the number of turns of the
                    // corresponding upper layer.
                    if search_forward {
                        let turns_in_upper_layer_coil = self.turns_upper_layer_coils()
                            [usize::from(upper_layer_shift - idx - 1)];
                        let turns =
                            self.turns_per_slot_side.get() - turns_in_upper_layer_coil.get();
                        return NonZeroUsize::new(turns).unwrap_or(NonZeroUsize::MIN);
                    } else {
                        let turns_in_upper_layer_coil =
                            self.turns_upper_layer_coils()[usize::from(idx)];
                        return turns_in_upper_layer_coil;
                    }
                }
            }
            unreachable!()
        }
    }

    fn create_coils(
        &mut self,
        winding_table: &WindingTable,
        all_zones_must_be_used: bool,
    ) -> Result<(), Error> {
        for slot in 0..self.slots.get() {
            for layer in 0..self.layers().get() {
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
        let phase_abs: u16 = phase.try_into().expect("the number of phases cannot exceed the maximum value of an u16 (this is guaranteed by the winding constructor)");
        let slot = zone.slot;
        let layer = zone.layer;

        // Search in the opposite layer => See the documentation of `CoilLayout`
        let search_layer = match zone.layer {
            LL => LR,
            UL => UR,
            UR => UL,
            LR => LL,
            _ => panic!("a quadruple layer winding must have 4 layers"),
        };

        let (negative_zone, positive_slot_direction) = if layer == LL || layer == UL {
            // Coil must start at the previous slot
            (
                Zone::new(
                    slot.checked_sub(1).unwrap_or(self.slots().get() - 1),
                    search_layer,
                ),
                false,
            )
        } else {
            // Coil must stop in the next slot
            (
                Zone::new((slot + 1).rem_euclid(self.slots().get()), search_layer),
                true,
            )
        };

        let turns = self.calculated_turns_at(Zone::new(slot, layer), winding_table);

        return FullCoil::new(
            Zone::new(slot, layer),
            negative_zone,
            positive_slot_direction,
            turns,
            phase_abs.try_into().unwrap_or(NonZeroU16::MIN),
            clone_box(&*self.wire),
        )
        .ok()
        .map(|coil| Coil::Full(coil));
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Winding for QuadrupleLayerToothCoilWinding {
    fn phases(&self) -> NonZeroU16 {
        self.phases
    }

    fn slots(&self) -> NonZeroU16 {
        self.slots
    }

    fn pole_pairs(&self) -> NonZeroU16 {
        self.pole_pairs
    }

    /// The number of winding layers is always 4 for a quadruple-layer winding
    fn layers(&self) -> NonZeroU16 {
        return NonZeroU16::new(4).expect("not zero");
    }

    fn base_winding_count(&self) -> NonZeroU16 {
        base_winding_count_symmetric(
            self.slots(),
            self.pole_pairs(),
            self.phases(),
            NonZeroU16::new(2).expect("not zero"),
        )
    }

    fn coil_layout(&self) -> CoilLayout {
        CoilLayout::Quadruple
    }

    fn parallel_paths(&self, _phase: NonZeroU16) -> NonZeroU16 {
        self.parallel_paths
    }

    fn series_turns_per_phase(&self, phase: NonZeroU16) -> num::rational::Ratio<usize> {
        return num::rational::Ratio::new(
            usize::from(self.layers().get() * self.slots().get()) * self.turns_per_slot_side.get()
                / usize::from(2 * self.phases().get() * self.parallel_paths(phase).get()),
            1,
        );
    }

    fn connection(&self) -> Connection {
        self.connection
    }

    fn coil_at(&self, zone: Zone) -> Option<&Coil> {
        self.coils.get(zone)
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
    ) -> VarQuantity<Inductance> {
        let _ = spatial_order;
        end_winding_leakage_inductance_tooth_coil(self, core, self.end_winding_leakage_coefficient)
            .into()
    }

    #[cfg(feature = "stem_core")]
    fn end_winding_half_turn_length(&self, core: CoreRef<'_>, zone: Zone) -> Length {
        end_winding_half_turn_length_semicircle(self, core, zone)
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct QuadrupleLayerToothCoilBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub turns_per_slot_side: NonZeroUsize,
    pub turns_upper_layer_coils: Vec<NonZeroUsize>,
    pub parallel_paths: NonZeroU16,
    pub connection: Connection,
    pub end_winding_leakage_coefficient: f64,
    pub wire: Box<dyn Wire>,
    pub winding_table_constructor: WindingTableConstructor,
    #[cfg_attr(feature = "serde", serde(default))]
    pub end_winding_half_turn_length_override: Option<Length>,
}

impl From<QuadrupleLayerToothCoilWinding> for QuadrupleLayerToothCoilBuilder {
    fn from(value: QuadrupleLayerToothCoilWinding) -> Self {
        Self {
            slots: value.slots,
            pole_pairs: value.pole_pairs,
            phases: value.phases,
            turns_per_slot_side: value.turns_per_slot_side,
            turns_upper_layer_coils: value.turns_upper_layer_coils,
            parallel_paths: value.parallel_paths,
            connection: value.connection,
            end_winding_leakage_coefficient: value.end_winding_leakage_coefficient,
            wire: value.wire,
            winding_table_constructor: value.winding_table_constructor,
            end_winding_half_turn_length_override: value.end_winding_half_turn_length_override,
        }
    }
}

impl TryFrom<QuadrupleLayerToothCoilBuilder> for QuadrupleLayerToothCoilWinding {
    type Error = Error;

    fn try_from(builder: QuadrupleLayerToothCoilBuilder) -> Result<Self, Self::Error> {
        compare_variables!(0.0 <= builder.end_winding_leakage_coefficient)?;

        // Check if the number of parallel paths is valid
        let max_par_paths = super::max_parallel_paths_symmetric(
            builder.slots,
            builder.pole_pairs,
            builder.phases,
            NonZeroU16::new(4).expect("not zero"),
        )
        .get();
        if max_par_paths % builder.parallel_paths.get() != 0 {
            return Err(Error::InvalidNumberParallelPaths);
        }

        let upper_layer_shift = builder.turns_upper_layer_coils.len();

        // Check if the upper layer shift is smaller than the numerator of the number of
        // slots per pole and phase.
        let hole_number_val: usize =
            (*hole_number(builder.slots, builder.pole_pairs, builder.phases).numer()).into();
        if upper_layer_shift >= hole_number_val {
            compare_variables!(upper_layer_shift < hole_number_val as hole_number)?;
        }

        for turns_upper_layer_coil in builder.turns_upper_layer_coils.iter() {
            let turns_upper_layer_coil = turns_upper_layer_coil.get();
            let turns_per_slot_side = builder.turns_per_slot_side.get();
            compare_variables!(turns_upper_layer_coil < turns_per_slot_side)?;
        }

        // Calculate the basic winding parameters
        let t = base_winding_count_symmetric(
            builder.slots,
            builder.pole_pairs,
            builder.phases,
            NonZeroU16::new(2).expect("not zero"),
        );
        let slots_basic = builder.slots.get() / t;
        let pole_pairs_basic = builder.pole_pairs.get() / t;

        // Create the zone plan by method
        let winding_table_dl = WindingTable::from_constructor(
            &builder.winding_table_constructor,
            NonZeroU16::new(slots_basic).unwrap_or(NonZeroU16::MIN),
            NonZeroU16::new(2).expect("not zero"),
            NonZeroU16::new(pole_pairs_basic).unwrap_or(NonZeroU16::MIN),
            builder.phases,
            1,
        )?;

        // Add a new third and fourth layer to the zone plan. The third layer is above
        // the first layer (slot side) in the slot, while the fourth layer is above the
        // second layer (slot side)
        let mut winding_table =
            WindingTable::new(builder.slots, NonZeroU16::new(4).expect("not zero"));
        for slot in 0..winding_table_dl.slots().get() {
            for layer in 0..4 {
                winding_table[Zone::new(slot.into(), layer.into())] =
                    winding_table_dl[Zone::new(slot, layer / 2)];
            }
        }

        // Shift the upper layers according to upper_layer_shift. If upper_layer_shift
        // is odd, the layers also need to be inverted, because otherwise the
        // 1st layer would be cancelled by the third layer (same for 2nd and 4th).
        winding_table.shift_layer(upper_layer_shift as i32, UL);
        winding_table.shift_layer(upper_layer_shift as i32, UR);

        if upper_layer_shift.is_odd() {
            winding_table
                .iter_layer_major_mut()
                .for_each(|(zone, phase)| {
                    if UL == zone.layer || UR == zone.layer {
                        *phase = -*phase;
                    }
                });
        }

        let mut winding = QuadrupleLayerToothCoilWinding {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            turns_per_slot_side: builder.turns_per_slot_side,
            turns_upper_layer_coils: builder.turns_upper_layer_coils,
            parallel_paths: builder.parallel_paths,
            connection: builder.connection,
            end_winding_leakage_coefficient: builder.end_winding_leakage_coefficient,
            wire: builder.wire,
            winding_table_constructor: builder.winding_table_constructor,
            coils: Coils::with_capacity(
                usize::from(builder.slots.get()) * 4,
                usize::from(builder.slots.get()) * 2,
            ),
            end_winding_half_turn_length_override: None,
        };
        winding.set_end_winding_half_turn_length_override(
            builder.end_winding_half_turn_length_override,
        )?;

        /*
        Assert that all coils of a coil group are located next to each other.
         */
        let coils_per_coil_group = winding.layers().get() * winding.slots().get()
            / (2 * winding.phases().get())
            / max_par_paths
            / 2;
        let mut phase_counter = coils_per_coil_group;
        let mut previous_phase = 0;
        let mut first_coil_group = true;
        for slot in 0..slots_basic {
            // Select the current phase of the first layer, if none is selected
            let current_phase = *winding_table_dl.get_cyclic(Zone::new(slot, 0));
            if phase_counter == coils_per_coil_group {
                phase_counter = 1;
            } else {
                if current_phase == -previous_phase {
                    // Current coil belongs to the same coil group
                    phase_counter += 1;
                } else {
                    // Current coil does not belong to the same coil group. If we are in the first
                    // coil group, the loop continues. Otherwise, this is an
                    // indicator of a failed zone plan and the winding
                    // creation is aborted
                    if first_coil_group {
                        first_coil_group = false;
                        phase_counter = 1;
                    } else {
                        return Err(Error::CoilsOfCoilGroupNotNextToEachOther);
                    }
                }
            }
            previous_phase = current_phase;
        }

        winding.create_coils(&winding_table, true)?;

        if winding.equal_winding_factors() {
            return Ok(winding);
        } else {
            return Err(WindingTableConstructionError::NotSymmetric.into());
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct QuadrupleLayerToothCoilMinimalBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub turns_per_slot_side: NonZeroUsize,
    pub turns_upper_layer_coils: Vec<NonZeroUsize>,
    pub winding_table_constructor: WindingTableConstructor,
}

impl TryFrom<QuadrupleLayerToothCoilMinimalBuilder> for QuadrupleLayerToothCoilWinding {
    type Error = Error;

    fn try_from(builder: QuadrupleLayerToothCoilMinimalBuilder) -> Result<Self, Self::Error> {
        QuadrupleLayerToothCoilBuilder {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            turns_per_slot_side: builder.turns_per_slot_side,
            turns_upper_layer_coils: builder.turns_upper_layer_coils,
            parallel_paths: NonZeroU16::MIN,
            connection: Connection::Star,
            end_winding_leakage_coefficient: 0.0,
            wire: Box::new(SffWire::default()),
            winding_table_constructor: builder.winding_table_constructor,
            end_winding_half_turn_length_override: None,
        }
        .try_into()
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for QuadrupleLayerToothCoilWinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(deserialize_untagged_verbose_error::DeserializeUntaggedVerboseError)]
        enum QuadrupleLayerToothCoilEnum {
            QuadrupleLayerToothCoilBuilder(QuadrupleLayerToothCoilBuilder),
            QuadrupleLayerToothCoilMinimalBuilder(QuadrupleLayerToothCoilMinimalBuilder),
        }
        let w = QuadrupleLayerToothCoilEnum::deserialize(deserializer)?;
        match w {
            QuadrupleLayerToothCoilEnum::QuadrupleLayerToothCoilBuilder(w) => {
                w.try_into().map_err(serde::de::Error::custom)
            }
            QuadrupleLayerToothCoilEnum::QuadrupleLayerToothCoilMinimalBuilder(w) => {
                w.try_into().map_err(serde::de::Error::custom)
            }
        }
    }
}
