//! Provides [`WindingTable`], a tabular representation of a winding.
//!
//! A [`WindingTable`] represents the phase and polarity of each zone of a
//! winding and provides functionality for constructing, inspecting, and
//! manipulating winding tables.

use crate::error::WindingTableCreationError;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub use stem_coil_layout::Zone;

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
layer \ slot  0   1   2   3   4   5
      0      -3   2  -1   3  -2   1
      1       1  -3   2  -1   3  -2
```
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
            &[usize::from(self.slots()), usize::from(self.layers())],
        )?;
        return self.data.get(linear_index);
    }

    /// Returns a mutable reference to the value stored at the given [`Zone`],
    /// or `None` if the zone is outside the bounds of the winding table.
    pub fn get_mut(&mut self, zone: Zone) -> Option<&mut i32> {
        let linear_index = cart_lin::cart_to_lin(
            &[usize::from(zone.slot), usize::from(zone.layer)],
            &[usize::from(self.slots()), usize::from(self.layers())],
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
            &[usize::from(self.slots()), usize::from(self.layers())],
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
    pub fn slots(&self) -> u16 {
        (self.data.len() / self.layers.get() as usize)
            .try_into()
            .expect("resulting value can fit into an u16, this is checked at construction time")
    }

    /// Returns the number of layers in the winding table.
    pub fn layers(&self) -> u16 {
        return self.layers.into();
    }

    // Slot-major
    //     slot 0: layer 0, layer 1, ...
    //     slot 1: layer 0, layer 1, ...
    //     ...
    pub fn iter_slots(&self) -> SlotMajorIter<'_> {
        SlotMajorIter::new(self)
    }

    // Slot-major
    //     slot 0: layer 0, layer 1, ...
    //     slot 1: layer 0, layer 1, ...
    //     ...
    pub fn iter_slots_mut(&mut self) -> SlotMajorIterMut<'_> {
        SlotMajorIterMut::new(self)
    }

    // Layer-major
    // layer 0: slot 0, slot 1, ...
    //layer 1: slot 0, slot 1, ...
    //...
    pub fn iter_layers(&self) -> LayerMajorIter<'_> {
        LayerMajorIter::new(self)
    }

    // Layer-major
    // layer 0: slot 0, slot 1, ...
    //layer 1: slot 0, slot 1, ...
    //...
    pub fn iter_layers_mut(&mut self) -> LayerMajorIterMut<'_> {
        LayerMajorIterMut::new(self)
    }

    /// Uses [`WindingTable::iter_slots_mut`] to put the data from `iterator`
    /// into the zones of `self`
    pub fn from_slot_major<I: Iterator<Item = i32>>(
        iterator: I,
        slots: NonZeroU16,
        layers: NonZeroU16,
    ) -> Self {
        let mut this = Self::new(slots, layers);
        this.iter_slots_mut()
            .zip(iterator)
            .for_each(|((_, phase_this), phase_iter)| {
                *phase_this = phase_iter;
            });
        return this;
    }

    /// Uses [`WindingTable::iter_layers_mut`] to put the data from `iterator`
    /// into the zones of `self`
    pub fn from_layer_major<I: Iterator<Item = i32>>(
        iterator: I,
        slots: NonZeroU16,
        layers: NonZeroU16,
    ) -> Self {
        let mut this = Self::new(slots, layers);
        this.iter_layers_mut()
            .zip(iterator)
            .for_each(|((_, phase_this), phase_iter)| {
                *phase_this = phase_iter;
            });
        return this;
    }

    /// Perform a zone shift on the input zone plan. As described in [MVP08]
    /// p. 42f., this is done by expanding all positive phase zones in the lower
    /// layer and by expanding all negative phase zones in the upper layer
    /// by qΔ.
    pub fn shift_zones(&mut self, shift: i32) -> () {
        // An internal "reference copy" of the zone plan is created.
        let winding_table_ref = self.clone();
        let slots = self.slots();

        for layer in 0..self.layers() {
            for slot in 0..slots {
                let new_slot = (i32::from(u16::from(slot)) + shift)
                    .rem_euclid(i32::from(u16::from(slots))) as u16;
                let phase = winding_table_ref[Zone::new(slot, layer)];

                // Expand all positive slots in the upper layer
                if layer == 0 && phase > 0 {
                    self[Zone::new(new_slot, layer)] = phase;
                }

                // Expand all negative slots in the lower layer
                if layer == 1 && phase < 0 {
                    self[Zone::new(new_slot, layer)] = phase;
                }
            }
        }
    }

    /*
    The coil arrangement is very sensitive to the starting location. As an example, let's consider a
    18 slot / 4 pole pairs single layer winding with the following zone plan:
    ```ignore
    1 2 -1 3 -2 -3 2 3 -2 1 -3 -1 3 1 -3 2 -1 -2
    ```
    Starting the coil builder algorithm at the first slot results in the following coil configuration for phase 2:
    ```ignore
    ──┐       ┌────┐    ┌────────────────┐     ┌
    1 2 -1 3 -2 -3 2 3 -2 1 -3 -1 3 1 -3 2 -1 -2
    ──┘       └────┘    └────────────────┘     └
    ```
    The very long third coil is solely a result of the starting location. IF we instead start at the third positive zone,
    the coil configuration looks like this:
      ```ignore
      ┌───────┐    ┌────┐                ┌─────┐
    1 2 -1 3 -2 -3 2 3 -2 1 -3 -1 3 1 -3 2 -1 -2
      └───────┘    └────┘                └─────┘
    ```
    The overall end winding length of this configuration is much shorter. To find the optimal configuration with a
    minimum overall end winding length, a heuristic approach is used: We search for the hypothetical coil
    with the longest span and start the coil creater algorithm at the positive zone of this coil.
     */
    pub fn start_at_largest_possible_span(&self, phase: i32) -> u16 {
        let mut longest_span = 0;
        let mut span_start_slot = 0;
        let mut positive_slot: u16 = 0;
        let mut last_phase: i32 = 0;

        /*
        In order to find coils which go from the end to the start of the zone plan,
        the algorithm scans the zone plan two times.
         */
        for slot in 0..(2 * self.slots()) {
            /*
            We only need to check layer 0, since in case of a double-layer winding layer 1
            is merely a shifted and mirrored version of layer 0.
             */
            let current_phase = self.get_cyclic(Zone::new(slot, 0)).clone();
            if current_phase.abs() == phase {
                if current_phase != last_phase {
                    let current_span = slot - span_start_slot;
                    if current_span > longest_span {
                        longest_span = current_span;
                        if current_phase > 0 {
                            positive_slot = slot
                        } else {
                            positive_slot = span_start_slot;
                        }
                    }
                    last_phase = current_phase;
                }
                span_start_slot = slot;
            }
        }
        return positive_slot;
    }

    /// Shift all layers of the given zone plan by `shift` slots.
    pub(crate) fn shift_layers(&mut self, shift: i32) -> () {
        for layer in 0..self.layers() {
            self.shift_layer(shift, layer)
        }
    }

    /// Shift the `layer` of the given zone plan by `shift` slots.
    pub(crate) fn shift_layer(&mut self, shift: i32, layer: u16) {
        // Swap with the Triple reversal algorithm: https://en.wikipedia.org/wiki/Block_swap_algorithms

        let slots = self.slots();
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

    pub(crate) fn reverse_layer_range(&mut self, layer: usize, start: usize, end: usize) {
        let layers = usize::from(self.layers());

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

    /// Inverts all coils in the given `layer`.
    pub(crate) fn invert_coils_in_layer(&mut self, layer: u16) {
        self.iter_layers_mut().for_each(|(zone, phase)| {
            if layer == zone.layer {
                *phase = -*phase;
            }
        });
    }

    pub(crate) fn check(self, phases: NonZeroU16) -> Result<Self, WindingTableCreationError> {
        for phase in 1..(i32::from(u16::from(phases)) + 1) {
            let mut counter = 0;
            for (zone, zone_phase) in self.iter_slots() {
                if phase == *zone_phase {
                    counter += 1;
                } else if -phase == *zone_phase {
                    counter -= 1;
                } else if *zone_phase == 0 {
                    return Err(WindingTableCreationError::EmptyZone(Some(zone)));
                }
            }

            // Check if there is the same number of positive and negative zones for a phase
            if counter != 0 {
                return Err(WindingTableCreationError::InequalPositiveNegativeZones(
                    phase as u16,
                ));
            }
        }
        return Ok(self);
    }
}

impl std::fmt::Display for WindingTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let slots = self.slots();
        let layers = self.layers();

        let slot_width = usize::from(slots - 1).to_string().len();
        let value_width = self
            .iter_layers()
            .map(|(_, value)| value.to_string().len())
            .max()
            .unwrap_or(1);
        let column_width = slot_width.max(value_width);

        let layer_width = usize::from(layers - 1).to_string().len().max("layer".len());

        // Header.
        write!(
            f,
            "{:>layer_width$} \\ slot",
            "layer",
            layer_width = layer_width
        )?;

        for slot in 0..slots {
            write!(f, " {:>column_width$}", slot, column_width = column_width)?;
        }
        writeln!(f)?;

        // Separator.
        let line_width = layer_width + " \\ slot".len() + usize::from(slots) * (column_width + 1);

        writeln!(f, "{}", "─".repeat(line_width))?;

        // Data.
        let mut iter = self.iter_layers();

        for layer in 0..layers {
            write!(f, "{layer:>layer_width$}", layer_width = layer_width + 2)?;
            write!(f, "{:>width$}", "", width = " slot".len())?;

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

#[derive(Clone)]
pub struct SlotMajorIter<'a> {
    winding_table: &'a WindingTable,
    idx: u16,
    slots: u16,
    layers: u16,
}

impl<'a> SlotMajorIter<'a> {
    pub fn new(winding_table: &'a WindingTable) -> Self {
        let slots = winding_table.slots();
        let layers = winding_table.layers();
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

pub struct SlotMajorIterMut<'a> {
    idx: u16,
    slots: u16,
    layers: u16,
    ptr: *mut i32,
    _marker: PhantomData<&'a mut i32>,
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

impl<'a> SlotMajorIterMut<'a> {
    pub fn new(winding_table: &'a mut WindingTable) -> Self {
        let slots = winding_table.slots();
        let layers = winding_table.layers();
        Self {
            ptr: winding_table.data.as_mut_ptr(),
            idx: 0,
            slots,
            layers,
            _marker: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct LayerMajorIter<'a> {
    winding_table: &'a WindingTable,
    idx: u16,
    slots: u16,
    layers: u16,
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

impl<'a> LayerMajorIter<'a> {
    pub fn new(winding_table: &'a WindingTable) -> Self {
        let slots = winding_table.slots();
        let layers = winding_table.layers();
        Self {
            winding_table,
            idx: 0,
            slots,
            layers,
        }
    }
}

pub struct LayerMajorIterMut<'a> {
    idx: u16,
    slots: u16,
    layers: u16,
    ptr: *mut i32,
    _marker: PhantomData<&'a mut i32>,
}

impl<'a> LayerMajorIterMut<'a> {
    pub fn new(winding_table: &'a mut WindingTable) -> Self {
        let slots = winding_table.slots();
        let layers = winding_table.layers();
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

        for layer in 0..usize::from(rev_table.layers()) {
            rev_table.reverse_layer_range(layer, 0, rev_table.slots().into());
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

        for layer in 0..usize::from(rev_table.layers()) {
            rev_table.reverse_layer_range(layer, 0, rev_table.slots().into());
        }

        assert_eq!(table, rev_table);
    }

    #[test]
    fn test_invert_coils_in_layer() {
        let mut winding_table = WindingTable::from_layer_major(
            [
                1, 1, -3, -3, 2, 2, -1, -1, 3, 3, -2, -2, 1, 1, -3, -3, 2, 2, -1, -1, 3, 3, -2, -2,
            ]
            .into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        winding_table.invert_coils_in_layer(1);

        let expected_result = WindingTable::from_layer_major(
            [
                1, 1, -3, -3, 2, 2, -1, -1, 3, 3, -2, -2, -1, -1, 3, 3, -2, -2, 1, 1, -3, -3, 2, 2,
            ]
            .into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);

        winding_table.invert_coils_in_layer(0);

        let expected_result = WindingTable::from_layer_major(
            [
                -1, -1, 3, 3, -2, -2, 1, 1, -3, -3, 2, 2, -1, -1, 3, 3, -2, -2, 1, 1, -3, -3, 2, 2,
            ]
            .into_iter(),
            NonZeroU16::new(12).expect("not zero"),
            NonZeroU16::new(2).expect("not zero"),
        );
        assert_eq!(winding_table, expected_result);
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
