use crate::error::WindingTableCreationError;
use num::traits::Euclid;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub use stem_coil_layout::Zone;

use std::{marker::PhantomData, num::NonZeroU16};

mod builders;
pub use builders::WindingTableMethod;

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
    pub fn new(slots: NonZeroU16, layers: NonZeroU16) -> Self {
        Self {
            layers,
            data: vec![0; usize::from(u16::from(slots)) * usize::from(u16::from(layers))],
        }
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

    pub fn slots(&self) -> u16 {
        (self.data.len() / self.layers.get() as usize)
            .try_into()
            .expect("resulting value can fit into an u16, this is checked at construction time")
    }

    pub fn layers(&self) -> u16 {
        return self.layers.into();
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
                let new_slot = (slot as i32 + shift).rem_euclid(slots as i32) as u16;
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
        let shift = shift.rem_euclid(slots as i32) as usize;

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

    pub fn get(&self, zone: Zone) -> Option<&i32> {
        let linear_index = cart_lin::cart_to_lin(
            &[usize::from(zone.slot), usize::from(zone.layer)],
            &[usize::from(self.slots()), usize::from(self.layers())],
        )?;
        return self.data.get(linear_index);
    }

    pub fn get_mut(&mut self, zone: Zone) -> Option<&mut i32> {
        let linear_index = cart_lin::cart_to_lin(
            &[usize::from(zone.slot), usize::from(zone.layer)],
            &[usize::from(self.slots()), usize::from(self.layers())],
        )?;
        return self.data.get_mut(linear_index);
    }

    // with a doc comment explicitly saying that indices are reduced modulo the
    // number of slots/layers.
    pub fn get_cyclic(&self, zone: Zone) -> &i32 {
        let slot = zone.slot % self.slots();
        let layer = zone.layer % self.layers();
        return self
            .get(Zone { slot, layer })
            .expect("zone exists in winding table");
    }

    // with a doc comment explicitly saying that indices are reduced modulo the
    // number of slots/layers.
    pub fn get_cyclic_mut(&mut self, zone: Zone) -> &mut i32 {
        let slot = zone.slot % self.slots();
        let layer = zone.layer % self.layers();
        return self
            .get_mut(Zone { slot, layer })
            .expect("zone exists in winding table");
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

    pub(crate) fn check(self, phases: NonZeroU16) -> Result<Self, WindingTableCreationError> {
        for phase in 1..(u16::from(phases) as i32 + 1) {
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
        write!(
            f,
            "Zone plan with {} slots and {} layers",
            self.slots(),
            self.layers()
        )?;

        todo!();

        // https://github.com/dimforge/nalgebra/blob/3320ecca21dc08f7a93c9595f6b257f05ba21273/src/base/matrix.rs#L1917
        // #[cfg(feature = "std")]
        // fn val_width<T: Scalar + $trait>(val: &T, f: &mut fmt::Formatter<'_>)
        // -> usize {     match f.precision() {
        //         Some(precision) => format!($fmt_str_with_precision, val,
        // precision)             .chars()
        //             .count(),
        //         None => format!($fmt_str_without_precision,
        // val).chars().count(),     }
        // }

        // #[cfg(not(feature = "std"))]
        // fn val_width<T: Scalar + $trait>(_: &T, _: &mut fmt::Formatter<'_>)
        // -> usize {     4
        // }

        // let (nrows, ncols) = self.shape();

        // if nrows == 0 || ncols == 0 {
        //     return write!(f, "[ ]");
        // }

        // let mut max_length = 0;

        // for i in 0..nrows {
        //     for j in 0..ncols {
        //         max_length = crate::max(max_length, val_width(&self[(i, j)],
        // f));     }
        // }

        // let max_length_with_space = max_length + 1;

        // writeln!(f)?;
        // writeln!(
        //     f,
        //     "  ┌ {:>width$} ┐",
        //     "",
        //     width = max_length_with_space * ncols - 1
        // )?;

        // for i in 0..nrows {
        //     write!(f, "  │")?;
        //     for j in 0..ncols {
        //         let number_length = val_width(&self[(i, j)], f) + 1;
        //         let pad = max_length_with_space - number_length;
        //         write!(f, " {:>thepad$}", "", thepad = pad)?;
        //         match f.precision() {
        //             Some(precision) => {
        //                 write!(f, $fmt_str_with_precision, (*self)[(i, j)],
        // precision)?             }
        //             None => write!(f, $fmt_str_without_precision, (*self)[(i,
        // j)])?,         }
        //     }
        //     writeln!(f, " │")?;
        // }

        // writeln!(
        //     f,
        //     "  └ {:>width$} ┘",
        //     "",
        //     width = max_length_with_space * ncols - 1
        // )?;
        // writeln!(f)
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

// =============================================================================
// Iterators

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
        let len = usize::from(self.slots * self.layers);
        return (len, Some(len));
    }
}

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
        let len = usize::from(self.slots * self.layers);
        return (len, Some(len));
    }
}

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
        let len = usize::from(self.slots * self.layers);
        return (len, Some(len));
    }
}

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
        let len = usize::from(self.slots * self.layers);
        return (len, Some(len));
    }
}

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
