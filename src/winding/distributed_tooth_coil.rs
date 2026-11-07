use std::num::{NonZeroU16, NonZeroUsize};

use compare_variables::compare_variables;

use dyn_clone::clone_box;
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
    winding::{Connection, Winding},
    winding_table::{WindingTable, WindingTableConstructor},
};

/**
This iterator returns all possible configurations for the number of coils in a coil group of an distributed tooth-coil winding.
```
use stem_winding::winding::CoilsPerCoilGroupIterator;
let mut iter = CoilsPerCoilGroupIterator::new(12, 3, 2, false);
assert_eq!(iter.next().unwrap(), 1);
assert_eq!(iter.next().unwrap(), 2);
assert!(iter.next().is_none());
```
 */
pub struct CoilsPerCoilGroupIterator {
    slots: u16,
    phases: u16,
    layers: u16,
    coils_per_coil_group: u16,
    max_pole_pairs: u16,
    multiplier: u16,
    iterator_is_exhausted: bool,
}

impl CoilsPerCoilGroupIterator {
    pub fn new(
        slots: NonZeroU16,
        phases: NonZeroU16,
        layers: NonZeroU16,
        double_zone_span: bool,
    ) -> Self {
        let slots = slots.get();
        let phases = phases.get();
        let layers = layers.get();

        // Check if the current number of turns per coil would lead to a valid winding
        let multiplier = if double_zone_span { 1 } else { 2 };

        let max_pole_pairs = multiplier * slots / phases * layers;
        Self {
            slots,
            phases,
            layers,
            coils_per_coil_group: 0,
            max_pole_pairs,
            multiplier,
            iterator_is_exhausted: false,
        }
    }
}

impl Iterator for CoilsPerCoilGroupIterator {
    type Item = u16;
    fn next(&mut self) -> Option<u16> {
        // Check if the iterator is exhausted
        if self.iterator_is_exhausted {
            return None;
        }

        // Exhaust the iterator
        if 2 * self.phases * self.coils_per_coil_group > self.layers * self.slots {
            self.iterator_is_exhausted = true;
            return None;
        }

        // Increase the iterator
        self.coils_per_coil_group = self.coils_per_coil_group + 1;

        for pole_pairs in 1..(self.max_pole_pairs + 1) {
            if self.layers * self.slots
                == self.multiplier * 2 * self.phases * self.coils_per_coil_group * pole_pairs
            {
                return Some(self.coils_per_coil_group);
            }
        }

        return self.next();
    }
}

/**
Data for a coil of a [`DistributedToothCoilWinding`].

This struct is used as part of the [`DistributedToothCoilBuilder`] to specify
certain properties of one of the concentric coils.
 */
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct CoilData {
    /// The [`Wire`] used by the coil.
    pub wire: Box<dyn Wire>,
    /// The number of [`turns`](crate::coils::CoilExt::turns) of the coil.
    pub turns: NonZeroUsize,
    /// An optional override for the length of an end-winding half-turn of the
    /// coil. If not provided, [`end_winding_half_turn_length_semicircle`] is
    /// used to calculate this value. See
    /// [`DistributedToothCoilWinding::end_winding_half_turn_length_override`]
    /// for details.
    pub end_winding_half_turn_length_override: Option<Length>,
}

/**
A short-pitched distributed winding with concentric non-overlapping coils.

This winding type is a derivative of the
[`DistributedWinding`](crate::winding::DistributedWinding) with a strong
pitching / large
[`coil_throw_reduction`](crate::winding::DistributedWinding::coil_throw_reduction).
Its distinctive feature are the concentric, non-overlapping coils and the
resulting short end windings, making this winding type particularly suited for
very short motors which require a sinusoidal air gap field and are therefore
unsuited for [`ToothCoilWinding`](crate::winding::ToothCoilWinding)s. A
practical example would be induction motors in fan applications, which need to
start directly on the grid. [\[1\]](#dtcw_1)

# Winding table and coil layout

The winding table of a distributed tooth coil winding is derived from an integer
slot winding. Hence, the [`hole_number`](Winding::hole_number)
`slots / (2 * pole_pairs * phases)` of a distributed tooth coil winding must
always be an integer. This is checked at construction time.

The derivation slightly differs for single- and double-layer windings, as
demonstrated in the following sections.

## Double-layer

Because it is a derivative, the zone plan of a distributed tooth-coil winding
can be created by short-pitching that of a
[`DistributedWinding`](crate::winding::DistributedWinding) [\[1\]](#dtcw_1).
Consider the following double-layered distributed winding with 12 slots and one
pole pair without any pitching.

*/
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Double-layer distributed winding without pitching][table_derive_dtc_0]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("table_derive_dtc_0", "docs/img/table_derive_dtc_0.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

By reducing the coil throw by four slots, one gets this heavily-pitched table:

*/
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Double-layer distributed winding with pitching][table_derive_dtc_1]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("table_derive_dtc_1", "docs/img/table_derive_dtc_1.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

Grouping neighboring zones of the same phase into the same layer gives the table
of the corresponding distributed tooth-coil winding.

*/
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Double-layer distributed tooth coil winding][table_double_layer_dtc_winding_0]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "table_double_layer_dtc_winding_0",
        "docs/img/table_double_layer_dtc_winding_0.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

Obviously, the zones can be connected directly, resulting in concentric coils
where the innermost coil is a tooth-coil:

*/
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Coil geometry of a distributed tooth coil winding with 2 phases][coils_double_layer_dtc_winding_0]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "coils_double_layer_dtc_winding_0",
        "docs/img/coils_double_layer_dtc_winding_0.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

# Single-layer

It is also possible to derive a single-layer version by doubling the zone span
as shown in [\[2\]](#dtcw_2) (see also
[`DistributedDoubleZoneSpanBuilder`](crate::winding::DistributedDoubleZoneSpanBuilder)):

*/
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Double-layer distributed winding with a doubled zone span][table_derive_dtc_2]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image("table_derive_dtc_2", "docs/img/table_derive_dtc_2.svg")
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

By removing half of the coils, this winding table can be compressed into a
single layer distributed tooth-coil winding.

*/
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Single-layer distributed tooth coil winding][table_single_layer_dtc_winding_0]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "table_single_layer_dtc_winding_0",
        "docs/img/table_single_layer_dtc_winding_0.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

# Coil groups

The concentric coils belonging to the same phase are called a coil group. Taking
the previously shown 12-slot / 2-pole single-layer winding as an example, the
coil groups are as follows:

```text
L \ S │   0   1   2   3   4   5   6   7   8   9  10  11
──────┼─────────────────────────────────────────────────
  0   │   1   1  -1  -1   2   2  -2  -2   3   3  -3  -3
          ┃   ┗━━━┛   ┃   ┃   ┗━━━┛   ┃   ┃   ┗━━━┛   ┃
          ┗━━━━━━━━━━━┛   ┗━━━━━━━━━━━┛   ┗━━━━━━━━━━━┛
             Group A         Group B         Group C
```
The indexing convention for the coils inside a coil group is as follows: The
innermost coil has the index 0, the next one 1 and so on. In the case of group
A, the coil from slot 1 to 2 is the innermost one with index 0, the one from
slot 0 to 3 has the index 1. This indexing convention is used throughout this
module; for example in the methods [`DistributedToothCoilWinding::coil_data`] or
by the [`DistributedToothCoilBuilder::coil_data`] field.

To influence the winding factor, the number of turns per coil can be varied
within a coil group. Specifically for double-layer windings, this can be done
while keeping the total number of turns per slot and hence the slot filling
factor constant [\[1\]](#dtcw_1):

*/
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Double-layer distributed tooth coil winding with different turns per coil][table_double_layer_dtc_winding_1]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "table_double_layer_dtc_winding_1",
        "docs/img/table_double_layer_dtc_winding_1.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

For a single-layer winding, a change in wire size or slot filling factor between
coils is unavoidable when the number of turns is not kept identical:

*/
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Single-layer distributed tooth coil winding with different turns per coil][table_single_layer_dtc_winding_1]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "table_single_layer_dtc_winding_1",
        "docs/img/table_single_layer_dtc_winding_1.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**

In both cases, the winding stays symmetric.

# Properties

Because a [`DistributedToothCoilWinding`] is derived from a
[`DistributedWinding`](crate::winding::DistributedWinding) but uses the coil
geometry of a [`ToothCoilWinding`](crate::winding::ToothCoilWinding), it
inherits its characteristics from both.
- It always fulfills the symmetric conditions of the [`Winding::is_symmetric`]
method. This means that its [`base_winding_count`](Winding::base_winding_count)
is calculated in constant time using the [`base_winding_count_symmetric`]
function.
- It either has one or two [`layers`](Winding::layers); the coils in a slot are
arranged as defined by [`CoilLayout::Single`] or
[`CoilLayout::DoubleHorizontal`] respectively.
- Its end winding turn lengths are approximated as semicircles via the
[`end_winding_half_turn_length_semicircle`] function (only available if the
`stem_core` feature is enabled). This approximation can be overridden by
providing the values for each coil of a group directly, see
[`DistributedToothCoilWinding::set_end_winding_half_turn_length_override`].
- Its end winding leakage inductance is calculated with the
[`end_winding_leakage_inductance_tooth_coil`] function (only available if the
`stem_core` feature is enabled).

# Constructors

For [`DistributedToothCoilWinding`], the builder structs
[`DistributedToothCoilBuilder`] and [`DistributedToothCoilMinimalBuilder`] are
available. The builder structs can be fallibly converted into a
[`DistributedToothCoilWinding`] with the
[`new`](DistributedToothCoilWinding::new) method or with the [`TryFrom`]
implementations, provided that the invariants for the builder field values are
upheld. This is checked during the building process, see the documentation of
the builder fields for details.

# Serialization and deserialization

If the `serde` feature is enabled, a [`DistributedToothCoilWinding`] can be
deserialized from the serialized representations of its builder structs. When
serializing it, it is converted into a [`DistributedToothCoilBuilder`], which is
then serialized with the standard [`Serialize`] macro.

# Literature

<a id="dtcw_1">\[1\]</a>
Huth, G. and Krotsch, J.: AC-Motoren mit verteilter Zahnspulenwicklung.
Elektrotechnik & Informationstechnik (2019) 136/2: p. 135-142.
<https://doi.org/10.1007/s00502-019-0717-9>

<a id="dtcw_2">\[2\]</a>
Huth, G. and Baun, M.: PM-Synchronmotoren mit verteilter Einschicht-Zahnspulenwicklung.
Elektrotechnik & Informationstechnik (2021) 138/2: p. 55-61.
<https://doi.org/10.1007/s00502-021-00869-2>

# Examples

```

```

 */
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(into = "DistributedToothCoilBuilder"))]
pub struct DistributedToothCoilWinding {
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    phases: NonZeroU16,
    layers: NonZeroU16,
    parallel_paths: NonZeroU16,
    connection: Connection,
    end_winding_leakage_coefficient: f64,
    coil_data: Vec<CoilData>,
    coils: Coils,
    base_winding_count: NonZeroU16,
    double_zone_span: bool,
}

impl DistributedToothCoilWinding {
    /// Creates a new winding.
    ///
    /// This is a wrapper around `T::try_into`. The builder structs provided by
    /// the module implement [`TryInto<DistributedToothCoilWinding>`], see
    /// their respective documentation for details on the failure modes. The
    /// struct documentation has an usage example.
    pub fn new<W>(builder: W) -> Result<Self, Error>
    where
        W: TryInto<DistributedToothCoilWinding>,
        W::Error: Into<Error>,
    {
        builder.try_into().map_err(Into::into)
    }

    /// Returns the number of coils per group of concentric coils.
    ///
    /// This value defines the upper bound for methods like
    /// [`DistributedToothCoilWinding::coil_data`] which access coil-specific
    /// information based on their index within the coil group.
    pub fn coils_per_coil_group(&self) -> NonZeroUsize {
        return self
            .coil_data
            .len()
            .try_into()
            .expect("constructor ensures there is at least one entry");
    }

    /// Whether the winding has a doubled zone span or not.
    ///
    /// TODO: Describe what this does during construction
    pub fn double_zone_span(&self) -> bool {
        self.double_zone_span
    }

    pub fn coil_group_turns<'a>(&'a self) -> impl Iterator<Item = NonZeroUsize> + 'a {
        self.coil_data.iter().map(|d| d.turns)
    }

    pub fn end_winding_leakage_coefficient(&self) -> f64 {
        self.end_winding_leakage_coefficient
    }

    /// Returns the [`CoilData`] for the nth coil of a coil group with index 0
    /// starting at the innermost coil.
    ///
    /// If n is out of bounds (i.e. larger than or equal to
    /// [`DistributedToothCoilWinding::coils_per_coil_group`]), this method
    /// returns `None` instead.
    pub fn coil_data(&self, n: usize) -> Option<&CoilData> {
        self.coil_data.get(n)
    }

    pub fn end_winding_half_turn_length_override(&self, n: usize) -> Option<Length> {
        self.coil_data
            .get(n)
            .map(|d| d.end_winding_half_turn_length_override)
            .flatten()
    }

    pub fn set_end_winding_half_turn_length_override(
        &mut self,
        end_winding_half_turn_length_override: Option<Length>,
        coil: usize,
    ) -> Result<(), compare_variables::Comparison<Length>> {
        if let Some(end_winding_half_turn_length_override) = end_winding_half_turn_length_override {
            let zero_length = Length::new::<meter>(0.0);
            compare_variables!(end_winding_half_turn_length_override >= zero_length)?;
        }
        if let Some(d) = self.coil_data.get_mut(coil) {
            d.end_winding_half_turn_length_override = end_winding_half_turn_length_override;
        }
        Ok(())
    }

    /**
    Create a turn distribution for a double-layer distributed winding which keeps the number of turns per slot constant.

    # Example
    ```
    use stem_winding::prelude::*;

    // Three coils per coil group, 10 turns per slot (5 per coil). The inner coil gets 5 - 2 = 3 turns, the middle coil gets 5 turns, the outer coil gets 5 + 2 = 7 turns.
    let turns = DistributedToothCoilWinding::double_layer_turn_distribution(3, 10, 2).unwrap();
    assert_eq!(turns[0], 3);
    assert_eq!(turns[1], 5);
    assert_eq!(turns[2], 7);

    // Four coils per coil group, 10 turns per slot. The inner coils get 3 turns, the outer coils get 7 turns.
    let turns = DistributedToothCoilWinding::double_layer_turn_distribution(4, 10, 2).unwrap();
    assert_eq!(turns[0], 3);
    assert_eq!(turns[1], 3);
    assert_eq!(turns[2], 7);
    assert_eq!(turns[3], 7);

    // Function errors for the following cases: turns per slot is zero or odd, absolute value of turn_difference equal to or greater than turns_per_slot
    assert!(DistributedToothCoilWinding::double_layer_turn_distribution(3, 0, 3).is_err());
    assert!(DistributedToothCoilWinding::double_layer_turn_distribution(3, 9, 3).is_err());
    assert!(DistributedToothCoilWinding::double_layer_turn_distribution(3, 10, 5).is_err());
    ```
     */
    pub fn double_layer_turn_distribution(
        coils_per_coil_group: u16,
        turns_per_slot: NonZeroUsize,
        turn_difference: i32,
    ) -> Result<Vec<NonZeroUsize>, Error> {
        let turns_per_slot = turns_per_slot.get();
        compare_variables!(0 < coils_per_coil_group)?;
        compare_variables!(0 < turns_per_slot)?;

        let twice_turn_difference = 2 * turn_difference.abs() as usize;
        compare_variables!(turns_per_slot > twice_turn_difference)?;

        if turns_per_slot.is_odd() {
            return Err(Error::OddNumberOfTurnsPerSlot);
        }

        // Create the turns per coil vector from coil_group_turns and turn_difference as
        // described in [Hut19], p. 137.
        let mut coil_group_turns: Vec<NonZeroUsize> = vec![
            NonZeroUsize::new(turns_per_slot / 2)
                .unwrap_or(NonZeroUsize::MIN);
            coils_per_coil_group as usize
        ];
        for hole in 0..coils_per_coil_group {
            // If q_star is odd, the center value of the vector does not change
            if 2 * hole == coils_per_coil_group - 1 {
                continue;
            // Inner coils: Reduce the number of turns per coil
            } else if 2 * hole < coils_per_coil_group - 1 {
                let turns =
                    NonZeroUsize::new((turns_per_slot as i32 / 2 - turn_difference) as usize)
                        .unwrap_or(NonZeroUsize::MIN);
                coil_group_turns[hole as usize] = turns;
            // Outer coils: Increase the number of turns per coil
            } else {
                let turns =
                    NonZeroUsize::new((turns_per_slot as i32 / 2 + turn_difference) as usize)
                        .unwrap_or(NonZeroUsize::MIN);
                coil_group_turns[hole as usize] = turns;
            }
        }
        return Ok(coil_group_turns);
    }

    fn calculated_turns_at(&self, zone: Zone, _winding_table: &WindingTable) -> NonZeroUsize {
        // The number of turns is encoded in the vector self.coil_group_turns as
        // follows: First entry is the innermost coil, second entry is the
        // second-innermost coil and so on.
        let cpcg = self.coils_per_coil_group().get() as u16;
        let offset = if zone.layer == 0 { 0 } else { cpcg };
        let normalized_slot = (zone.slot + offset) % (2 * cpcg);
        let idx = if normalized_slot < cpcg {
            (cpcg - 1 - normalized_slot) as usize
        } else {
            (normalized_slot % (2 * cpcg) - cpcg) as usize
        };
        self.coil_data[idx].turns
    }

    fn create_coils(&mut self, winding_table: &WindingTable) -> Result<(), Error> {
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

        return Ok(());
    }

    fn create_single_coil(&self, zone: Zone, winding_table: &WindingTable) -> Option<Coil> {
        let phase = *winding_table.get_cyclic(zone);
        let slot = zone.slot;
        let layer = zone.layer;

        let mut return_slot: Option<u16> = None;
        let mut positive_slot_direction: Option<bool> = None;
        let mut index: Option<u16> = None;

        let cpcg = self.coils_per_coil_group().get() as u16;

        // Search through the neighboring slots, but stay in the same layer
        for k in 1..(cpcg + 1) {
            let search_slot = (k + slot).rem_euclid(self.slots().get());

            if self.coils.occupied(Zone::new(search_slot, layer)) {
                break;
            }

            // Detect direction change in search_slot: Use the slot in position q-k+1
            if *winding_table.get_cyclic(Zone::new(search_slot, layer)) == -phase {
                return_slot = Some((slot + 2 * k - 1).rem_euclid(self.slots().get()));
                positive_slot_direction = Some(true);
                index = Some(k - 1);
                break;
            }

            // Detect phase change in search_slot: Use the slot in position k
            if winding_table
                .get_cyclic(Zone::new(search_slot, layer))
                .abs()
                != phase.abs()
            {
                return_slot = Some(
                    (slot as i32 - 2 * i32::from(cpcg) + 2 * k as i32 - 1)
                        .rem_euclid(self.slots().get() as i32) as u16,
                );
                positive_slot_direction = Some(false);
                index = Some(k - 1);
                break;
            }
        }

        match return_slot {
            Some(return_slot) => {
                let positive_slot_direction = positive_slot_direction.unwrap();
                let index = index.unwrap();
                let phase_abs = NonZeroU16::new(phase.abs() as u16)
                    .expect("phase cannot be zero for this winding type");
                let wire = clone_box(&*self.coil_data[index as usize].wire);
                if phase > 0 {
                    return FullCoil::new(
                        Zone::new(slot, layer),
                        Zone::new(return_slot, layer),
                        positive_slot_direction,
                        self.calculated_turns_at(Zone::new(slot, layer), winding_table),
                        phase_abs,
                        wire,
                    )
                    .ok()
                    .map(|coil| Coil::Full(coil));
                } else {
                    return FullCoil::new(
                        Zone::new(return_slot, layer),
                        Zone::new(slot, layer),
                        !positive_slot_direction,
                        self.calculated_turns_at(Zone::new(slot, layer), winding_table),
                        phase_abs,
                        wire,
                    )
                    .ok()
                    .map(|coil| Coil::Full(coil));
                }
            }
            None => return None,
        }
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Winding for DistributedToothCoilWinding {
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
        self.base_winding_count
    }

    fn coil_layout(&self) -> CoilLayout {
        if self.layers().get() == 1 {
            return CoilLayout::Single;
        } else {
            return CoilLayout::DoubleVertical;
        }
    }

    /// Returns the hole number representation q = g + z/n where g, z and n are
    /// integers in order [g, z, n].
    fn hole_number(&self) -> num::rational::Ratio<u16> {
        return num::rational::Ratio::new(2 * self.coils_per_coil_group().get() as u16, 1);
    }

    fn connection(&self) -> Connection {
        self.connection
    }

    /// The number of parallel paths of this winding type is always twice the
    /// number of basic windings.
    fn parallel_paths(&self, _phase: NonZeroU16) -> NonZeroU16 {
        self.parallel_paths
    }

    fn coil_at(&self, zone: Zone) -> Option<&Coil> {
        return self.coils.get(zone);
    }

    fn as_dyn(&self) -> &dyn Winding {
        self
    }

    #[cfg(feature = "stem_core")]
    fn is_symmetric(&self, _core: CoreRef<'_>) -> bool {
        true
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
pub struct DistributedToothCoilBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub layers: NonZeroU16,
    pub parallel_paths: NonZeroU16,
    pub connection: Connection,
    pub end_winding_leakage_coefficient: f64,
    pub coil_data: Vec<CoilData>,
    pub double_zone_span: bool,
}

impl From<DistributedToothCoilWinding> for DistributedToothCoilBuilder {
    fn from(value: DistributedToothCoilWinding) -> Self {
        Self {
            slots: value.slots,
            pole_pairs: value.pole_pairs,
            phases: value.phases,
            layers: value.layers,
            parallel_paths: value.parallel_paths,
            connection: value.connection,
            end_winding_leakage_coefficient: value.end_winding_leakage_coefficient,
            coil_data: value.coil_data,
            double_zone_span: value.double_zone_span,
        }
    }
}

impl TryFrom<DistributedToothCoilBuilder> for DistributedToothCoilWinding {
    type Error = Error;

    fn try_from(builder: DistributedToothCoilBuilder) -> Result<Self, Self::Error> {
        let coils_per_group = builder.coil_data.len() as u16;
        compare_variables!(coils_per_group > 0)?;

        let zero_length = Length::new::<meter>(0.0);
        for end_winding_half_turn_length_override in builder
            .coil_data
            .iter()
            .filter_map(|d| d.end_winding_half_turn_length_override)
        {
            compare_variables::compare_variables!(
                end_winding_half_turn_length_override >= zero_length
            )?;
        }

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

        // Try creating the zone plan for the basic pole pair number (which is
        // calculated by winding_holes)
        let slots_winding_table = if builder.double_zone_span {
            2 * coils_per_group * builder.phases.get() / builder.layers.get()
        } else {
            4 * coils_per_group * builder.phases.get() / builder.layers.get()
        };

        let base_winding_count = builder.slots.get() / slots_winding_table;
        compare_variables!(base_winding_count > 0)?;
        if builder.pole_pairs.get() % base_winding_count != 0 {
            return Err(Error::InvalidPolePairNumber);
        }

        let mut winding_table = distributed_tooth_coil_assembly(
            NonZeroU16::new(slots_winding_table).expect("not zero"),
            NonZeroU16::MIN,
            builder.phases,
            builder.layers,
            builder.coil_data.len(),
            builder.double_zone_span,
        )?;

        let num_zones = (builder.slots.get() * builder.layers.get()).into();
        let num_coils = num_zones / 2;
        let mut winding = DistributedToothCoilWinding {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            layers: builder.layers,
            parallel_paths: builder.parallel_paths,
            connection: builder.connection,
            end_winding_leakage_coefficient: builder.end_winding_leakage_coefficient,
            coil_data: builder.coil_data,
            coils: Coils::with_capacity(num_zones, num_coils),
            base_winding_count: NonZeroU16::new(base_winding_count).expect("not zero"),
            double_zone_span: builder.double_zone_span,
        };

        // Create the coils from the zone plan
        winding.create_coils(&winding_table)?;

        // Invert the zone plan if the first harmonic order is negative.
        // For an integer slot winding, the first harmonic order equals the
        // number of pole pairs. Because distributed tooth-coil windings operate on a
        // super harmonic of the integer winding they are based on, the zone
        // plan direction must be reversed if the pole pair order is negative.
        if let Some(is_positive) =
            winding.is_harmonic(SpatialOrder::Mechanical(winding.pole_pairs().get().into()))
        {
            if !is_positive {
                for layer in 0..winding_table.layers().get() {
                    let mut left = 0;
                    let mut right = winding_table.slots().get();

                    while left < right.saturating_sub(1) {
                        right -= 1;

                        let tmp = winding_table[Zone::new(left, layer)];
                        winding_table[Zone::new(left, layer)] =
                            winding_table[Zone::new(right, layer)];
                        winding_table[Zone::new(right, layer)] = tmp;

                        left += 1;
                    }
                }
                winding_table.shift_layers(2 * coils_per_group as i32);
            }
        } else {
            return Err(Error::InvalidPolePairNumber);
        }

        // Recreate the coils from the adjusted zone plan
        winding.create_coils(&winding_table)?;

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
pub struct DistributedToothCoilMinimalBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub layers: NonZeroU16,
    pub turns: Vec<NonZeroUsize>,
    pub double_zone_span: bool,
}

impl TryFrom<DistributedToothCoilMinimalBuilder> for DistributedToothCoilWinding {
    type Error = Error;

    fn try_from(builder: DistributedToothCoilMinimalBuilder) -> Result<Self, Self::Error> {
        let mut coil_data: Vec<CoilData> = Vec::with_capacity(builder.turns.len());
        for turns in builder.turns {
            coil_data.push(CoilData {
                wire: Box::new(SffWire::default()),
                turns,
                end_winding_half_turn_length_override: None,
            });
        }
        DistributedToothCoilBuilder {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            layers: builder.layers,
            parallel_paths: NonZeroU16::MIN,
            connection: Connection::Star,
            end_winding_leakage_coefficient: 0.0,
            coil_data,
            double_zone_span: builder.double_zone_span,
        }
        .try_into()
    }
}

/// Returns a zone plan for a single-layer distributed tooth-coil winding.
/// The algorithm is taken from [\[2\]](#dtcw_1).
fn distributed_tooth_coil_assembly(
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    phases: NonZeroU16,
    layers: NonZeroU16,
    coils_per_coil_group: usize,
    double_zone_span: bool,
) -> Result<WindingTable, Error> {
    let layers_times_slot = usize::from(layers.get() * slots.get());
    if double_zone_span {
        let poles_times_phases_times_coils_per_coil_group =
            usize::from(2 * phases.get() * pole_pairs.get()) * coils_per_coil_group;
        compare_variables!(layers_times_slot == poles_times_phases_times_coils_per_coil_group)?;
    } else {
        let two_times_poles_times_phases_times_coils_per_coil_group =
            usize::from(4 * phases.get() * pole_pairs.get()) * coils_per_coil_group;
        compare_variables!(
            layers_times_slot == two_times_poles_times_phases_times_coils_per_coil_group
        )?;
    }

    let span: i32 = if layers.get() == 1 {
        coils_per_coil_group as i32 * (phases.get() as i32 + 1)
    } else {
        coils_per_coil_group as i32
    };

    // Create a double-layer zone plan with 2*m zones and a single pole pair
    let mut winding_table = WindingTable::from_constructor(
        &WindingTableConstructor::DistributionTable,
        slots,
        NonZeroU16::new(2).expect("not zero"),
        pole_pairs,
        phases,
        span,
    )?;

    let winding_table = if layers.get() == 1 {
        // Winding with m zones as discussed in TODO [Hut19], section 2.1
        if double_zone_span {
            // Convert to m zones
            winding_table.shift_zones(coils_per_coil_group as i32);
        }

        // Derived the distributed tooth-coil winding from the integer-slot winding as
        // shown in TODO [Hut21], fig. 1
        let mut wt = WindingTable::new(slots, NonZeroU16::MIN);
        for slot in 0..slots.get() {
            if usize::from(slot).rem_euclid(2 * coils_per_coil_group) >= coils_per_coil_group {
                wt[Zone::new(slot, 0)] = -winding_table[Zone::new(slot, 0)]
            } else {
                wt[Zone::new(slot, 0)] = winding_table[Zone::new(slot, 0)]
            }
        }
        wt
    } else {
        // Move the return conductors of the distributed tooth-coil winding in the
        // same layer as the original conductors of each phase. This means exchanging
        // upper and lower layers for every second winding zone defined by the number
        // of winding holes q.
        for slot in 0..slots.get() {
            if usize::from(slot).rem_euclid(2 * coils_per_coil_group) >= coils_per_coil_group {
                let ul = winding_table[Zone::new(slot, 0)];
                let ll = winding_table[Zone::new(slot, 1)];
                winding_table[Zone::new(slot, 0)] = ll;
                winding_table[Zone::new(slot, 1)] = ul;
            }
        }

        if double_zone_span {
            // Convert to m zones by inverting every phase in the second layer
            for slot in 0..slots.get() {
                winding_table[Zone::new(slot, 1)] = -winding_table[Zone::new(slot, 1)];
            }
        }
        winding_table
    };
    return Ok(winding_table);
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for DistributedToothCoilWinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(deserialize_untagged_verbose_error::DeserializeUntaggedVerboseError)]
        enum DistributedToothCoilEnum {
            DistributedToothCoilBuilder(DistributedToothCoilBuilder),
            DistributedToothCoilMinimalBuilder(DistributedToothCoilMinimalBuilder),
        }
        let w = DistributedToothCoilEnum::deserialize(deserializer)?;
        match w {
            DistributedToothCoilEnum::DistributedToothCoilBuilder(w) => {
                w.try_into().map_err(serde::de::Error::custom)
            }
            DistributedToothCoilEnum::DistributedToothCoilMinimalBuilder(w) => {
                w.try_into().map_err(serde::de::Error::custom)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_layer_distributed_tooth_coil_assembly() {
        {
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(12).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                2,
                true,
            )
            .unwrap();

            let expected_result = WindingTable::from_slot_major(
                [1, 1, -1, -1, 2, 2, -2, -2, 3, 3, -3, -3].into_iter(),
                NonZeroU16::new(12).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
            );
            assert_eq!(winding_table, expected_result);
        }

        {
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(18).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                3,
                true,
            )
            .unwrap();

            let expected_result = WindingTable::from_slot_major(
                [
                    1, 1, 1, -1, -1, -1, 2, 2, 2, -2, -2, -2, 3, 3, 3, -3, -3, -3,
                ]
                .into_iter(),
                NonZeroU16::new(18).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
            );
            assert_eq!(winding_table, expected_result);
        }

        {
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(24).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                2,
                false,
            )
            .unwrap();

            let expected_result = WindingTable::from_slot_major(
                [
                    1, 1, -1, -1, -3, -3, 3, 3, 2, 2, -2, -2, -1, -1, 1, 1, 3, 3, -3, -3, -2, -2,
                    2, 2,
                ]
                .into_iter(),
                NonZeroU16::new(24).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
            );
            assert_eq!(winding_table, expected_result);
        }

        {
            // This should fail because the number of slots and winding holes does not
            // match.
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(12).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                2,
                false,
            );
            assert!(winding_table.is_err());
        }

        {
            // This should fail because the number of slots is not applicable
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(13).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                2,
                true,
            );
            assert!(winding_table.is_err());
        }

        {
            // This should fail because the number of pole pairs is not applicable
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(12).expect("not zero"),
                NonZeroU16::new(2).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                2,
                true,
            );
            assert!(winding_table.is_err());
        }
    }

    #[test]
    fn test_double_layer_distributed_tooth_coil_assembly() {
        {
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(12).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(2).expect("not zero"),
                2,
                false,
            )
            .unwrap();

            let expected_result = WindingTable::from_layer_major(
                [
                    1, 1, -1, -1, 2, 2, -2, -2, 3, 3, -3, -3, 2, 2, -3, -3, 3, 3, -1, -1, 1, 1, -2,
                    -2,
                ]
                .into_iter(),
                NonZeroU16::new(12).expect("not zero"),
                NonZeroU16::new(2).expect("not zero"),
            );
            assert_eq!(winding_table, expected_result);
        }

        {
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(18).expect("not zero"),
                NonZeroU16::new(1).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(2).expect("not zero"),
                3,
                false,
            )
            .unwrap();

            let expected_result = WindingTable::from_layer_major(
                [
                    1, 1, 1, -1, -1, -1, 2, 2, 2, -2, -2, -2, 3, 3, 3, -3, -3, -3, 2, 2, 2, -3, -3,
                    -3, 3, 3, 3, -1, -1, -1, 1, 1, 1, -2, -2, -2,
                ]
                .into_iter(),
                NonZeroU16::new(18).expect("not zero"),
                NonZeroU16::new(2).expect("not zero"),
            );
            assert_eq!(winding_table, expected_result);
        }

        {
            // This should fail because the number of pole pairs is not applicable
            let winding_table = distributed_tooth_coil_assembly(
                NonZeroU16::new(12).expect("not zero"),
                NonZeroU16::new(4).expect("not zero"),
                NonZeroU16::new(3).expect("not zero"),
                NonZeroU16::new(2).expect("not zero"),
                3,
                true,
            );
            assert!(winding_table.is_err());
        }
    }
}
