//! Provides [`WindingTable`], a tabular representation of a winding.
//!
//! A [`WindingTable`] represents the phase and polarity of each zone of a
//! winding and provides functionality for constructing, inspecting, and
//! manipulating winding tables.

use crate::error::WindingTableConstructionError;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub use stem_types::Zone;

use std::{marker::PhantomData, num::NonZeroU16};

mod constructors;
pub use constructors::WindingTableConstructor;

/**
A tabular representation of the phase and polarity of each winding [`Zone`].

Each [`Zone`] of a winding is either empty or occupied by a coil with a phase
and a polarity. The [`WindingTable`] represents all zones of a winding as a
table where one axis is the slot index (going from 0 to the total number of
slots minus one) and the other one is the layer index (likewise going from 0 to
the total number of layers minus one). If the zone is empty, the table entry is
0. If there is a coil, the value is `+phase` if the polarity is positive and
`-phase` if the polarity is negative (see also
[`ZoneAndPolarity`](crate::coils::ZoneAndPolarity)). For example, for the
winding shown below, the winding table would look like this:

```text
L \ S │   0   1   2   3   4   5
──────┼────────────────────────
  0   │  -3   2  -1   3  -2   1
  1   │   1  -3   2  -1   3  -2
```
The [`Display`](std::fmt::Display) implementation formats a `WindingTable` in
this tabular form.
 */
#[doc = ""]
#[cfg_attr(
    feature = "doc-images",
    doc = "![Winding table reference image][winding_table_reference_img]"
)]
#[cfg_attr(
    feature = "doc-images",
    embed_doc_image::embed_doc_image(
        "winding_table_reference_img",
        "docs/img/winding_table_reference_img.svg"
    )
)]
#[cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with
    `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
/**
A [`WindingTable`] can either be built incrementally (see
[`WindingTable::set`] and [`WindingTable::get_mut`]), from iterators (see
[`WindingTable::from_slot_major`] and [`WindingTable::from_layer_major`]), or
using a predefined [`WindingTableConstructor`] for symmetric multiphase
windings. Its elements can be accessed using a [`Zone`] index, either through
the getter methods or via the indexing syntax `table[Zone::new(0, 0)]`.

A [`WindingTable`] can be used in two ways:
1) As a compact representation of the winding design.
2) As an intermediate step from which the [`Coils`](crate::coils::Coils) of a
winding can be determined.

Many of the predefined symmetric [`Winding`](crate::winding::Winding)s in
this crate construct their [`Coils`](crate::coils::Coils) by first creating a
winding table and then constructing individual [`Coil`](crate::coils::Coil)
instances from it. Since, for example, a
[`DistributedWinding`](crate::winding::DistributedWinding) has fundamentally
different coil geometry from a
[`ToothCoilWinding`](crate::winding::ToothCoilWinding), the conversion from a
winding table to coils is specific to each winding variant.
 */
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WindingTable {
    layers: NonZeroU16,
    /// The data is stored slot-major: the entries for each slot are contiguous.
    ///
    /// The value for `(slot, layer)` is stored at
    /// `slot * layers + layer`.
    /// For example, with `slots = 6` and `layers = 2`, `data.len() == 12`:
    ///
    /// ```text
    /// slot 0: [layer 0, layer 1] -> data[0..2]
    /// slot 1: [layer 0, layer 1] -> data[2..4]
    /// ...
    /// slot 5: [layer 0, layer 1] -> data[10..12]
    /// ```
    data: Vec<i32>,
}

impl WindingTable {
    /// Creates a new [`WindingTable`] where all zones are set to `0`, i.e. do
    /// not contain a coil.
    ///
    /// The winding table can then be manually edited using methods such as
    /// [`WindingTable::get_mut`] and [`WindingTable::set`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    /// use stem_winding::prelude::*;
    ///
    /// let table = WindingTable::new(
    ///     NonZeroU16::new(6).expect("not zero"),
    ///     NonZeroU16::new(2).expect("not zero"),
    /// );
    ///
    /// assert_eq!(table.get(Zone::new(0, 0)), Some(&0));
    /// assert_eq!(table.get(Zone::new(5, 1)), Some(&0));
    /// ```
    pub fn new(slots: NonZeroU16, layers: NonZeroU16) -> Self {
        Self {
            layers,
            data: vec![0; usize::from(u16::from(slots)) * usize::from(u16::from(layers))],
        }
    }

    /// Returns a reference to the value stored at the given [`Zone`], or `None`
    /// if the zone is outside the bounds of the winding table.
    pub fn get(&self, zone: Zone) -> Option<&i32> {
        let linear_index = cart_lin::cart_to_lin(
            &[usize::from(zone.slot), usize::from(zone.layer)],
            &[
                usize::from(self.slots().get()),
                usize::from(self.layers().get()),
            ],
        )?;
        return self.data.get(linear_index);
    }

    /// Returns a mutable reference to the value stored at the given [`Zone`],
    /// or `None` if the zone is outside the bounds of the winding table.
    pub fn get_mut(&mut self, zone: Zone) -> Option<&mut i32> {
        let linear_index = cart_lin::cart_to_lin(
            &[usize::from(zone.slot), usize::from(zone.layer)],
            &[
                usize::from(self.slots().get()),
                usize::from(self.layers().get()),
            ],
        )?;
        return self.data.get_mut(linear_index);
    }

    /// Sets the value at the given [`Zone`] and returns the previous value.
    ///
    /// Returns `None` if the zone is outside the bounds of the winding table.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    /// use stem_winding::prelude::*;
    ///
    /// let mut table = WindingTable::new(
    ///     NonZeroU16::new(6).expect("not zero"),
    ///     NonZeroU16::new(1).expect("not zero"),
    /// );
    ///
    /// assert_eq!(table.set(Zone::new(2, 0), 1), Some(0));
    /// assert_eq!(table.set(Zone::new(2, 0), -2), Some(1));
    /// assert_eq!(table.get(Zone::new(2, 0)), Some(&-2));
    /// ```
    pub fn set(&mut self, zone: Zone, mut phase: i32) -> Option<i32> {
        let linear_index = cart_lin::cart_to_lin(
            &[usize::from(zone.slot), usize::from(zone.layer)],
            &[
                usize::from(self.slots().get()),
                usize::from(self.layers().get()),
            ],
        )?;
        std::mem::swap(&mut phase, &mut self.data[linear_index]);
        return Some(phase);
    }

    /// Returns a reference to the value stored at the given [`Zone`].
    ///
    /// The slot and layer indices are reduced modulo the number of slots and
    /// layers, respectively. Consequently, indices outside the bounds of the
    /// winding table wrap around.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    /// use stem_winding::prelude::*;
    ///
    /// let mut table = WindingTable::new(
    ///     NonZeroU16::new(6).expect("not zero"),
    ///     NonZeroU16::new(2).expect("not zero"),
    /// );
    /// table.set(Zone::new(0, 0), 1);
    ///
    /// assert_eq!(table.get_cyclic(Zone::new(6, 2)), &1);
    /// ```
    ///
    /// This method is useful for accessing zones of cyclic windings.
    pub fn get_cyclic(&self, zone: Zone) -> &i32 {
        let slot = zone.slot % self.slots();
        let layer = zone.layer % self.layers();
        return self
            .get(Zone { slot, layer })
            .expect("zone exists in winding table");
    }

    /// Returns a mutable reference to the value stored at the given [`Zone`].
    ///
    /// The slot and layer indices are reduced modulo the number of slots and
    /// layers, respectively. Consequently, indices outside the bounds of the
    /// winding table wrap around.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    /// use stem_winding::prelude::*;
    ///
    /// let mut table = WindingTable::new(
    ///     NonZeroU16::new(6).expect("not zero"),
    ///     NonZeroU16::new(2).expect("not zero"),
    /// );
    ///
    /// *table.get_cyclic_mut(Zone::new(6, 2)) = 1;
    /// assert_eq!(table.get(Zone::new(0, 0)), Some(&1));
    /// ```
    pub fn get_cyclic_mut(&mut self, zone: Zone) -> &mut i32 {
        let slot = zone.slot % self.slots();
        let layer = zone.layer % self.layers();
        return self
            .get_mut(Zone { slot, layer })
            .expect("zone exists in winding table");
    }

    /// Returns the number of slots in the winding table.
    pub fn slots(&self) -> NonZeroU16 {
        u16::try_from(self.data.len() / self.layers.get() as usize)
            .expect("resulting value can fit into an u16, this is checked at construction time")
            .try_into()
            .expect("not zero")
    }

    /// Returns the number of layers in the winding table.
    pub fn layers(&self) -> NonZeroU16 {
        return self.layers.into();
    }

    /**
    Returns a [`SlotMajorIter`] over the zones and value references of the
    table.

    This iterator returns `(Zone, &i32)` tuples in slot-major order: all layers
    of one slot are visited before moving to the next slot. In other words, the
    layer index of the [`Zone`] changes fastest, while the slot index changes
    slowest.

    ```
    use std::num::NonZeroU16;
    use stem_winding::prelude::*;

    let mut table = WindingTable::new(
        NonZeroU16::new(6).expect("not zero"),
        NonZeroU16::new(2).expect("not zero"),
    );
    table.set(Zone::new(0, 0), 1);
    table.set(Zone::new(0, 1), 2);
    table.set(Zone::new(1, 0), 3);
    table.set(Zone::new(1, 1), 4);

    let mut iter = table.iter_slot_major();
    assert_eq!(iter.next(), Some((Zone::new(0, 0), &1)));
    assert_eq!(iter.next(), Some((Zone::new(0, 1), &2)));
    assert_eq!(iter.next(), Some((Zone::new(1, 0), &3)));
    assert_eq!(iter.next(), Some((Zone::new(1, 1), &4)));
    ```
    */
    pub fn iter_slot_major(&self) -> SlotMajorIter<'_> {
        SlotMajorIter::new(self)
    }

    /**
    Returns a [`SlotMajorIterMut`] over the zones and mutable value references
    of the table.

    This iterator returns `(Zone, &mut i32)` tuples in slot-major order: all
    layers of one slot are visited before moving to the next slot. In other
    words, the layer index of the [`Zone`] changes fastest, while the slot index
    changes slowest.

    ```
    use std::num::NonZeroU16;
    use stem_winding::prelude::*;

    let mut table = WindingTable::new(
        NonZeroU16::new(6).expect("not zero"),
        NonZeroU16::new(2).expect("not zero"),
    );
    table.set(Zone::new(0, 0), 1);
    table.set(Zone::new(0, 1), 2);
    table.set(Zone::new(1, 0), 3);
    table.set(Zone::new(1, 1), 4);

    let mut iter = table.iter_slot_major_mut();
    assert_eq!(iter.next(), Some((Zone::new(0, 0), &mut 1)));
    assert_eq!(iter.next(), Some((Zone::new(0, 1), &mut 2)));
    assert_eq!(iter.next(), Some((Zone::new(1, 0), &mut 3)));
    assert_eq!(iter.next(), Some((Zone::new(1, 1), &mut 4)));
    ```
    */
    pub fn iter_slot_major_mut(&mut self) -> SlotMajorIterMut<'_> {
        SlotMajorIterMut::new(self)
    }

    /**
    Returns a [`LayerMajorIter`] over the zones and value references of the
    table.

    This iterator returns `(Zone, &i32)` tuples in layer-major order: all slots
    of one layer are visited before moving to the next layer. In other words,
    the slot index of the [`Zone`] changes fastest, while the layer index changes
    slowest.

    ```
    use std::num::NonZeroU16;
    use stem_winding::prelude::*;

    let mut table = WindingTable::new(
        NonZeroU16::new(6).expect("not zero"),
        NonZeroU16::new(2).expect("not zero"),
    );
    table.set(Zone::new(0, 0), 1);
    table.set(Zone::new(0, 1), 2);
    table.set(Zone::new(1, 0), 3);
    table.set(Zone::new(1, 1), 4);

    let mut iter = table.iter_layer_major();
    assert_eq!(iter.next(), Some((Zone::new(0, 0), &1)));
    assert_eq!(iter.next(), Some((Zone::new(1, 0), &3)));
    assert_eq!(iter.next(), Some((Zone::new(2, 0), &0)));
    assert_eq!(iter.next(), Some((Zone::new(3, 0), &0)));
    assert_eq!(iter.next(), Some((Zone::new(4, 0), &0)));
    assert_eq!(iter.next(), Some((Zone::new(5, 0), &0)));
    assert_eq!(iter.next(), Some((Zone::new(0, 1), &2)));
    assert_eq!(iter.next(), Some((Zone::new(1, 1), &4)));
    ```
    */
    pub fn iter_layer_major(&self) -> LayerMajorIter<'_> {
        LayerMajorIter::new(self)
    }

    /**
    Returns a [`LayerMajorIter`] over the zones and mutable value references of
    the table.

    This iterator returns `(Zone, &mut i32)` tuples in layer-major order: all
    slots of one layer are visited before moving to the next layer. In other
    words, the slot index of the [`Zone`] changes fastest, while the layer index
    changes slowest.

    ```
    use std::num::NonZeroU16;
    use stem_winding::prelude::*;

    let mut table = WindingTable::new(
        NonZeroU16::new(6).expect("not zero"),
        NonZeroU16::new(2).expect("not zero"),
    );
    table.set(Zone::new(0, 0), 1);
    table.set(Zone::new(0, 1), 2);
    table.set(Zone::new(1, 0), 3);
    table.set(Zone::new(1, 1), 4);

    let mut iter = table.iter_layer_major_mut();
    assert_eq!(iter.next(), Some((Zone::new(0, 0), &mut 1)));
    assert_eq!(iter.next(), Some((Zone::new(1, 0), &mut 3)));
    assert_eq!(iter.next(), Some((Zone::new(2, 0), &mut 0)));
    assert_eq!(iter.next(), Some((Zone::new(3, 0), &mut 0)));
    assert_eq!(iter.next(), Some((Zone::new(4, 0), &mut 0)));
    assert_eq!(iter.next(), Some((Zone::new(5, 0), &mut 0)));
    assert_eq!(iter.next(), Some((Zone::new(0, 1), &mut 2)));
    assert_eq!(iter.next(), Some((Zone::new(1, 1), &mut 4)));
    ```
    */
    pub fn iter_layer_major_mut(&mut self) -> LayerMajorIterMut<'_> {
        LayerMajorIterMut::new(self)
    }

    /// Constructs a [`WindingTable`] from data in slot-major order.
    ///
    /// The first `layers` elements of `iterator` are assigned to slot 0, the
    /// next `layers` elements to slot 1, and so on. The layer index
    /// therefore changes fastest, while the slot index changes slowest.
    ///
    /// If `iterator` yields fewer than `slots * layers` elements, the remaining
    /// entries are initialized to 0. If it yields more, the excess elements are
    /// ignored.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    /// use stem_winding::prelude::*;
    ///
    /// let table = WindingTable::from_slot_major(
    ///     [1, 2, 3, 4].into_iter(),
    ///     NonZeroU16::new(6).unwrap(),
    ///     NonZeroU16::new(2).unwrap(),
    /// );
    ///
    /// assert_eq!(table[Zone::new(0, 0)], 1);
    /// assert_eq!(table[Zone::new(0, 1)], 2);
    /// assert_eq!(table[Zone::new(1, 0)], 3);
    /// assert_eq!(table[Zone::new(1, 1)], 4);
    /// ```
    pub fn from_slot_major<I: Iterator<Item = i32>>(
        iterator: I,
        slots: NonZeroU16,
        layers: NonZeroU16,
    ) -> Self {
        let mut this = Self::new(slots, layers);
        this.iter_slot_major_mut()
            .zip(iterator)
            .for_each(|((_, phase_this), phase_iter)| {
                *phase_this = phase_iter;
            });
        return this;
    }

    /// Constructs a [`WindingTable`] from data in layer-major order.
    ///
    /// The first `slots` elements of `iterator` are assigned to layer 0, the
    /// next `slots` elements to layer 1, and so on. The slot index
    /// therefore changes fastest, while the layer index changes slowest.
    ///
    /// If `iterator` yields fewer than `slots * layers` elements, the remaining
    /// entries are initialized to 0. If it yields more, the excess elements are
    /// ignored.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU16;
    /// use stem_winding::prelude::*;
    ///
    /// let table = WindingTable::from_layer_major(
    ///     [1, 2, 3, 4].into_iter(),
    ///     NonZeroU16::new(6).unwrap(),
    ///     NonZeroU16::new(2).unwrap(),
    /// );
    ///
    /// assert_eq!(table[Zone::new(0, 0)], 1);
    /// assert_eq!(table[Zone::new(1, 0)], 2);
    /// assert_eq!(table[Zone::new(2, 0)], 3);
    /// assert_eq!(table[Zone::new(3, 0)], 4);
    /// ```
    pub fn from_layer_major<I: Iterator<Item = i32>>(
        iterator: I,
        slots: NonZeroU16,
        layers: NonZeroU16,
    ) -> Self {
        let mut this = Self::new(slots, layers);
        this.iter_layer_major_mut()
            .zip(iterator)
            .for_each(|((_, phase_this), phase_iter)| {
                *phase_this = phase_iter;
            });
        return this;
    }

    /**
    Performs a zone shift by `shift` slots on a double-layer winding table.

    A double-layer winding can be obtained from a single-layer winding by
    splitting each slot into two layers. A zone shift increases the effective
    zone span by shifting each positive-polarity zone in the lower layer and
    each negative-polarity zone in the upper layer by `shift` slots.

    This transformation can be used to suppress selected winding harmonics. For
    some harmonic orders, the winding factor becomes zero after the zone shift.

    The shift is cyclic: positive values shift toward increasing slot indices,
    while negative values shift toward decreasing slot indices.

    This method has no effect if `self` does not have exactly two layers.

    See [\[1\]](#zone_shift_1), sections 1.2.2.1b and 1.2.3.4c.

    # Literature
    <a id="zone_shift_1">\[1\]</a>
    Müller, G., Vogt, K. and Ponick, B.: Berechnung elektrischer Maschinen,
    6th edition, Wiley-VCH, 2008

    # Examples

    ```
    use std::num::NonZeroU16;
    use stem_winding::prelude::*;

    // Manually build an ordinary integer-slot winding with one pole pair
    let mut table = WindingTable::new(
        NonZeroU16::new(6).expect("not zero"),
        NonZeroU16::new(2).expect("not zero"),
    );
    table.set(Zone::new(0, 0), 1);
    table.set(Zone::new(0, 1), 1);
    table.set(Zone::new(1, 0), -3);
    table.set(Zone::new(1, 1), -3);
    table.set(Zone::new(2, 0), 2);
    table.set(Zone::new(2, 1), 2);
    table.set(Zone::new(3, 0), -1);
    table.set(Zone::new(3, 1), -1);
    table.set(Zone::new(4, 0), 3);
    table.set(Zone::new(4, 1), 3);
    table.set(Zone::new(5, 0), -2);
    table.set(Zone::new(5, 1), -2);

    // A shift of 1 is sufficient to double the zone span because each coil group
    // contains only one coil.
    table.shift_zones(1);
    assert_eq!(table[Zone::new(0, 0)], 1);
    assert_eq!(table[Zone::new(1, 0)], 1);
    assert_eq!(table[Zone::new(2, 0)], 2);
    assert_eq!(table[Zone::new(3, 0)], 2);
    assert_eq!(table[Zone::new(4, 0)], 3);
    assert_eq!(table[Zone::new(5, 0)], 3);
    assert_eq!(table[Zone::new(0, 1)], -2);
    assert_eq!(table[Zone::new(1, 1)], -3);
    assert_eq!(table[Zone::new(2, 1)], -3);
    assert_eq!(table[Zone::new(3, 1)], -1);
    assert_eq!(table[Zone::new(4, 1)], -1);
    assert_eq!(table[Zone::new(5, 1)], -2);
    ```
    */
    pub fn shift_zones(&mut self, shift: i32) {
        if self.layers().get() != 2 {
            return;
        }

        // An internal "reference copy" of the zone plan is created.
        let winding_table_ref = self.clone();
        let slots = self.slots().get();

        for layer in 0..self.layers().get() {
            for slot in 0..slots {
                let new_slot = (i32::from(u16::from(slot)) + shift)
                    .rem_euclid(i32::from(u16::from(slots))) as u16;
                let phase = winding_table_ref[Zone::new(slot, layer)];

                // Expand positive zones in the lower layer
                if layer == 0 && phase > 0 {
                    self[Zone::new(new_slot, layer)] = phase;
                }

                // Expand negative zones in the upper layer
                if layer == 1 && phase < 0 {
                    self[Zone::new(new_slot, layer)] = phase;
                }
            }
        }
    }

    /// Shifts all layers of the table by `shift` slots.
    ///
    /// See [`WindingTable::shift_layer`] for details.
    pub fn shift_layers(&mut self, shift: i32) {
        for layer in 0..self.layers().get() {
            self.shift_layer(shift, layer)
        }
    }

    /// Shifts the specified `layer` of `self` by `shift` slots.
    ///
    /// Positive values shift toward increasing slot indices, while negative
    /// values shift toward decreasing slot indices. The shift is cyclic, so
    /// values shifted beyond either end of the slot range wrap around to
    /// the opposite end.
    ///
    /// Examples:
    ///
    /// ```
    /// use std::num::NonZeroU16;
    /// use stem_winding::prelude::*;
    ///
    /// let mut table = WindingTable::from_layer_major(
    ///     [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12].into_iter(),
    ///     NonZeroU16::new(6).unwrap(),
    ///     NonZeroU16::new(2).unwrap(),
    /// );
    ///
    /// // First layer
    /// assert_eq!(table[Zone::new(0, 0)], 1);
    /// assert_eq!(table[Zone::new(1, 0)], 2);
    /// assert_eq!(table[Zone::new(5, 0)], 6);
    /// // Second layer
    /// assert_eq!(table[Zone::new(0, 1)], 7);
    /// assert_eq!(table[Zone::new(1, 1)], 8);
    ///
    /// table.shift_layer(-1, 0);
    ///
    /// // First layer
    /// assert_eq!(table[Zone::new(0, 0)], 2);
    /// assert_eq!(table[Zone::new(1, 0)], 3);
    /// assert_eq!(table[Zone::new(5, 0)], 1);
    ///
    /// // Second layer
    /// assert_eq!(table[Zone::new(0, 1)], 7);
    /// assert_eq!(table[Zone::new(1, 1)], 8);
    /// ```
    pub fn shift_layer(&mut self, shift: i32, layer: u16) {
        // Swap with the Triple reversal algorithm: https://en.wikipedia.org/wiki/Block_swap_algorithms

        let slots = self.slots().get();
        let shift = shift.rem_euclid(i32::from(u16::from(slots))) as usize;

        if shift == 0 {
            return;
        }

        let layer = layer as usize;

        // Reverse [0, slots - shift)
        self.reverse_layer_range(layer, 0, slots as usize - shift);

        // Reverse [slots - shift, slots)
        self.reverse_layer_range(layer, slots as usize - shift, slots as usize);

        // Reverse the whole layer
        self.reverse_layer_range(layer, 0, slots as usize);
    }

    fn reverse_layer_range(&mut self, layer: usize, start: usize, end: usize) {
        let layers = usize::from(self.layers().get());

        let mut left = start;
        let mut right = end;

        while left < right.saturating_sub(1) {
            right -= 1;

            let lhs = left * layers + layer;
            let rhs = right * layers + layer;

            self.data.swap(lhs, rhs);

            left += 1;
        }
    }

    /// Checks if the table fulfills the following invariants:
    ///
    /// - For each phase, the number of zones with positive polarity is equal to
    /// that of the zones with negative polarity
    /// - There are no empty zones (value of 0) in the table.
    fn check(self, phases: NonZeroU16) -> Result<Self, WindingTableConstructionError> {
        for phase in 1..(i32::from(u16::from(phases)) + 1) {
            let mut counter = 0;
            for (zone, zone_phase) in self.iter_slot_major() {
                if phase == *zone_phase {
                    counter += 1;
                } else if -phase == *zone_phase {
                    counter -= 1;
                } else if *zone_phase == 0 {
                    return Err(WindingTableConstructionError::EmptyZone(Some(zone)));
                }
            }

            // Check if there is the same number of positive and negative zones for a phase
            if counter != 0 {
                return Err(WindingTableConstructionError::InequalPositiveNegativeZones(
                    phase as u16,
                ));
            }
        }
        return Ok(self);
    }
}

impl<W: crate::winding::Winding> From<&W> for WindingTable {
    fn from(winding: &W) -> Self {
        winding.winding_table(false)
    }
}

impl std::fmt::Display for WindingTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let slots = self.slots().get();
        let layers = self.layers().get();

        let slot_width = usize::from(slots - 1).to_string().len();
        let value_width = self
            .iter_layer_major()
            .map(|(_, value)| value.to_string().len())
            .max()
            .unwrap_or(1);
        let column_width = slot_width.max(value_width);

        let layer_width = 1;

        // Header.
        write!(f, "{:>layer_width$} \\ S │", "L", layer_width = layer_width)?;

        for slot in 0..slots {
            write!(f, " {:>column_width$}", slot, column_width = column_width)?;
        }
        writeln!(f)?;

        // Separator.
        let line_width = usize::from(slots) * (column_width + 1);

        writeln!(f, "──────┼{}", "─".repeat(line_width))?;

        // Data.
        let mut iter = self.iter_layer_major();

        for layer in 0..layers {
            write!(
                f,
                "{layer:>layer_width$}   │",
                layer_width = layer_width + 2
            )?;

            for _ in 0..slots {
                let (_, value) = iter.next().expect("iterator length is known");
                write!(f, " {:>column_width$}", value, column_width = column_width)?;
            }

            if layer + 1 < layers {
                writeln!(f)?;
            }
        }

        Ok(())
    }
}

impl std::ops::Index<Zone> for WindingTable {
    type Output = i32;

    fn index(&self, index: Zone) -> &Self::Output {
        return self.get(index).expect("index out of bounds");
    }
}

impl std::ops::IndexMut<Zone> for WindingTable {
    fn index_mut(&mut self, index: Zone) -> &mut Self::Output {
        return self.get_mut(index).expect("index out of bounds");
    }
}

/// A slot-major iterator over the zones and references to the values of a
/// [`WindingTable`].
///
/// See [`WindingTable::iter_slot_major`] for details and examples.
#[derive(Clone)]
pub struct SlotMajorIter<'a> {
    winding_table: &'a WindingTable,
    idx: u16,
    slots: u16,
    layers: u16,
}

impl<'a> SlotMajorIter<'a> {
    /// Creates a new iterator over the given [`WindingTable`].
    ///
    /// See [`WindingTable::iter_slot_major`] for examples.
    pub fn new(winding_table: &'a WindingTable) -> Self {
        let slots = winding_table.slots().get();
        let layers = winding_table.layers().get();
        Self {
            winding_table,
            idx: 0,
            slots,
            layers,
        }
    }
}

impl<'a> Iterator for SlotMajorIter<'a> {
    type Item = (Zone, &'a i32);

    fn next(&mut self) -> Option<Self::Item> {
        let [slot, layer] =
            cart_lin::lin_to_cart::<2>(self.idx.into(), &[self.slots.into(), self.layers.into()])?;
        let zone = Zone {
            slot: slot as u16,
            layer: layer as u16,
        };
        self.idx += 1;
        let phase = self.winding_table.get(zone)?;
        return Some((zone, phase));
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = usize::from(self.slots * self.layers) - usize::from(self.idx);
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for SlotMajorIter<'a> {}

/// A slot-major iterator over the zones and mutable references to the values
/// of a [`WindingTable`].
///
/// See [`WindingTable::iter_slot_major_mut`] for details and examples.
pub struct SlotMajorIterMut<'a> {
    idx: u16,
    slots: u16,
    layers: u16,
    ptr: *mut i32,
    _marker: PhantomData<&'a mut i32>,
}

impl<'a> SlotMajorIterMut<'a> {
    /// Creates a new iterator over the given [`WindingTable`].
    ///
    /// See [`WindingTable::iter_slot_major_mut`] for examples.
    pub fn new(winding_table: &'a mut WindingTable) -> Self {
        let slots = winding_table.slots().get();
        let layers = winding_table.layers().get();
        Self {
            ptr: winding_table.data.as_mut_ptr(),
            idx: 0,
            slots,
            layers,
            _marker: PhantomData,
        }
    }
}

impl<'a> Iterator for SlotMajorIterMut<'a> {
    type Item = (Zone, &'a mut i32);

    fn next(&mut self) -> Option<Self::Item> {
        let [slot, layer] =
            cart_lin::lin_to_cart::<2>(self.idx.into(), &[self.slots.into(), self.layers.into()])?;

        // Layer-major logical order, slot-major physical storage.
        let physical_index = slot * usize::from(self.layers) + layer;

        self.idx += 1;

        // SAFETY: cart_lin::lin_to_cart performs the bounds check for us.
        let phase = unsafe { &mut *self.ptr.add(physical_index.into()) };
        Some((
            Zone {
                slot: slot as u16,
                layer: layer as u16,
            },
            phase,
        ))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = usize::from(self.slots * self.layers) - usize::from(self.idx);
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for SlotMajorIterMut<'a> {}

/// A layer-major iterator over the zones and references to the values of a
/// [`WindingTable`].
///
/// See [`WindingTable::iter_layer_major`] for details and examples.
#[derive(Clone)]
pub struct LayerMajorIter<'a> {
    winding_table: &'a WindingTable,
    idx: u16,
    slots: u16,
    layers: u16,
}

impl<'a> LayerMajorIter<'a> {
    /// Creates a new iterator over the given [`WindingTable`].
    ///
    /// See [`WindingTable::iter_layer_major`] for examples.
    pub fn new(winding_table: &'a WindingTable) -> Self {
        let slots = winding_table.slots().get();
        let layers = winding_table.layers().get();
        Self {
            winding_table,
            idx: 0,
            slots,
            layers,
        }
    }
}

impl<'a> Iterator for LayerMajorIter<'a> {
    type Item = (Zone, &'a i32);

    fn next(&mut self) -> Option<Self::Item> {
        let [layer, slot] =
            cart_lin::lin_to_cart::<2>(self.idx.into(), &[self.layers.into(), self.slots.into()])?;
        let zone = Zone {
            slot: slot as u16,
            layer: layer as u16,
        };
        self.idx += 1;
        let phase = self.winding_table.get(zone)?;
        return Some((zone, phase));
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = usize::from(self.slots * self.layers) - usize::from(self.idx);
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for LayerMajorIter<'a> {}

/// A layer-major iterator over the zones and mutable references to the values
/// of a [`WindingTable`].
///
/// See [`WindingTable::iter_layer_major_mut`] for details and examples.
pub struct LayerMajorIterMut<'a> {
    idx: u16,
    slots: u16,
    layers: u16,
    ptr: *mut i32,
    _marker: PhantomData<&'a mut i32>,
}

impl<'a> LayerMajorIterMut<'a> {
    /// Creates a new iterator over the given [`WindingTable`].
    ///
    /// See [`WindingTable::iter_layer_major_mut`] for examples.
    pub fn new(winding_table: &'a mut WindingTable) -> Self {
        let slots = winding_table.slots().get();
        let layers = winding_table.layers().get();
        Self {
            ptr: winding_table.data.as_mut_ptr(),
            idx: 0,
            slots,
            layers,
            _marker: PhantomData,
        }
    }
}

impl<'a> Iterator for LayerMajorIterMut<'a> {
    type Item = (Zone, &'a mut i32);

    fn next(&mut self) -> Option<Self::Item> {
        let [layer, slot] =
            cart_lin::lin_to_cart::<2>(self.idx.into(), &[self.layers.into(), self.slots.into()])?;

        // Layer-major logical order, slot-major physical storage.
        let physical_index = slot * usize::from(self.layers) + layer;

        self.idx += 1;

        // SAFETY: cart_lin::lin_to_cart performs the bounds check for us.
        let phase = unsafe { &mut *self.ptr.add(physical_index.into()) };
        Some((
            Zone {
                slot: slot as u16,
                layer: layer as u16,
            },
            phase,
        ))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = usize::from(self.slots * self.layers) - usize::from(self.idx);
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for LayerMajorIterMut<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_table() -> WindingTable {
        let mut table = WindingTable::new(
            NonZeroU16::new(6).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        table[Zone { slot: 0, layer: 0 }] = 1;
        table[Zone { slot: 0, layer: 1 }] = -2;
        table[Zone { slot: 1, layer: 0 }] = -3;
        table[Zone { slot: 1, layer: 1 }] = 1;
        table[Zone { slot: 2, layer: 0 }] = 2;
        table[Zone { slot: 2, layer: 1 }] = -3;
        table[Zone { slot: 3, layer: 0 }] = -1;
        table[Zone { slot: 3, layer: 1 }] = 2;
        table[Zone { slot: 4, layer: 0 }] = 3;
        table[Zone { slot: 4, layer: 1 }] = -1;
        table[Zone { slot: 5, layer: 0 }] = -2;
        table[Zone { slot: 5, layer: 1 }] = 3;
        return table;
    }

    #[test]
    fn test_shift_layer() {
        let mut table = create_table();

        assert_eq!(table[Zone { slot: 0, layer: 1 }], -2);

        table.shift_layer(-1, 1);

        assert_eq!(table[Zone { slot: 0, layer: 0 }], 1);
        assert_eq!(table[Zone { slot: 1, layer: 0 }], -3);
        assert_eq!(table[Zone { slot: 2, layer: 0 }], 2);
        assert_eq!(table[Zone { slot: 3, layer: 0 }], -1);
        assert_eq!(table[Zone { slot: 4, layer: 0 }], 3);
        assert_eq!(table[Zone { slot: 5, layer: 0 }], -2);

        assert_eq!(table[Zone { slot: 0, layer: 1 }], 1);
        assert_eq!(table[Zone { slot: 1, layer: 1 }], -3);
        assert_eq!(table[Zone { slot: 2, layer: 1 }], 2);
        assert_eq!(table[Zone { slot: 3, layer: 1 }], -1);
        assert_eq!(table[Zone { slot: 4, layer: 1 }], 3);
        assert_eq!(table[Zone { slot: 5, layer: 1 }], -2);

        table.shift_layer(1, 1);

        assert_eq!(table[Zone { slot: 0, layer: 1 }], -2);
        assert_eq!(table[Zone { slot: 1, layer: 1 }], 1);
        assert_eq!(table[Zone { slot: 2, layer: 1 }], -3);
        assert_eq!(table[Zone { slot: 3, layer: 1 }], 2);
        assert_eq!(table[Zone { slot: 4, layer: 1 }], -1);
        assert_eq!(table[Zone { slot: 5, layer: 1 }], 3);

        table.shift_layer(6, 1);

        assert_eq!(table[Zone { slot: 0, layer: 1 }], -2);
        assert_eq!(table[Zone { slot: 1, layer: 1 }], 1);
        assert_eq!(table[Zone { slot: 2, layer: 1 }], -3);
        assert_eq!(table[Zone { slot: 3, layer: 1 }], 2);
        assert_eq!(table[Zone { slot: 4, layer: 1 }], -1);
        assert_eq!(table[Zone { slot: 5, layer: 1 }], 3);

        table.shift_layer(5, 1);

        assert_eq!(table[Zone { slot: 0, layer: 0 }], 1);
        assert_eq!(table[Zone { slot: 1, layer: 0 }], -3);
        assert_eq!(table[Zone { slot: 2, layer: 0 }], 2);
        assert_eq!(table[Zone { slot: 3, layer: 0 }], -1);
        assert_eq!(table[Zone { slot: 4, layer: 0 }], 3);
        assert_eq!(table[Zone { slot: 5, layer: 0 }], -2);
    }

    #[test]
    fn test_reverse() {
        let table = create_table();
        let mut rev_table = table.clone();

        for layer in 0..usize::from(rev_table.layers().get()) {
            rev_table.reverse_layer_range(layer, 0, rev_table.slots().get().into());
        }

        assert_eq!(rev_table[Zone { slot: 5, layer: 0 }], 1);
        assert_eq!(rev_table[Zone { slot: 4, layer: 0 }], -3);
        assert_eq!(rev_table[Zone { slot: 3, layer: 0 }], 2);
        assert_eq!(rev_table[Zone { slot: 2, layer: 0 }], -1);
        assert_eq!(rev_table[Zone { slot: 1, layer: 0 }], 3);
        assert_eq!(rev_table[Zone { slot: 0, layer: 0 }], -2);

        assert_eq!(rev_table[Zone { slot: 5, layer: 1 }], -2);
        assert_eq!(rev_table[Zone { slot: 4, layer: 1 }], 1);
        assert_eq!(rev_table[Zone { slot: 3, layer: 1 }], -3);
        assert_eq!(rev_table[Zone { slot: 2, layer: 1 }], 2);
        assert_eq!(rev_table[Zone { slot: 1, layer: 1 }], -1);
        assert_eq!(rev_table[Zone { slot: 0, layer: 1 }], 3);

        for layer in 0..usize::from(rev_table.layers().get()) {
            rev_table.reverse_layer_range(layer, 0, rev_table.slots().get().into());
        }

        assert_eq!(table, rev_table);
    }

    #[test]
    fn test_shift_second_layer() {
        let mut winding_table = WindingTable::from_slot_major(
            [
                1, 1, -3, -3, 2, 2, -1, -1, 3, 3, -2, -2, 1, 1, -3, -3, 2, 2, -1, -1, 3, 3, -2, -2,
            ]
            .into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );

        // Shift to the right
        winding_table.shift_layer(3, 1);
        let expected_result = WindingTable::from_slot_major(
            [
                1, -1, -3, 3, 2, -2, -1, 1, 3, -3, -2, 2, 1, -1, -3, 3, 2, -2, -1, 1, 3, -3, -2, 2,
            ]
            .into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);

        // Shift to the left
        winding_table.shift_layer(-4, 1);
        let expected_result = WindingTable::from_slot_major(
            [
                1, -3, -3, 2, 2, -1, -1, 3, 3, -2, -2, 1, 1, -3, -3, 2, 2, -1, -1, 3, 3, -2, -2, 1,
            ]
            .into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);

        // No-op
        winding_table.shift_layer(0, 1);
        assert_eq!(winding_table, expected_result);
    }
}
