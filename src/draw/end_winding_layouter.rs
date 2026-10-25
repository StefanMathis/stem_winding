//! This module contains the [`EndWindingLayouter`] which is used when using the
//! [`EndWindingStyle::Layered`](super::EndWindingStyle) variant for creating a
//! [`CoilDrawablesConfig`](super::CoilDrawablesConfig).

use std::num::NonZeroU16;

use stem_types::Zone;

use crate::coils::{Coil, CoilExt, FullCoil};
use crate::winding::{CoilsIterator, Winding};

/**
An iterator for defining the "end winding layer" of a coil for drawing.

To plot an end winding with horizontal coil connectors, it is necessary to
separate the winding heads of each coil into different layers so they don't
superimpose. To keep the visualization as clean as possible, it is a good idea
to minimize the number of "crossovers" over different coils. This means that the
layer of a coil with a short span should be closer to the core than that of a
coil with a longer span. Applying these rules to a 18-slot, 4-pole-pair
single-layer winding results in the following representation:

```text
End winding layer:
1)     ┌───────┐      ┌───────┐      ┌───────┐
0)   ┌─│──┐ ┌──│──┐ ┌─│──┐ ┌──│──┐ ┌─│──┐ ┌──│──┐
     1 2 -1 3 -2 -3 2 3 -2 1 -3 -1 3 1 -3 2 -1 -2
```

The short coils with a span of 2 are all in the zeroth layer, while the longer
coils with a span of 3 are in layer one.

This iterator returns all coils of a winding together with the respective layer
of its winding head, starting with the coils in layer 0 and adding as many
layers as needed.
 */
pub(super) struct EndWindingLayouter<'a> {
    winding: &'a dyn Winding,
    layer: u16,
    slot: u16,
    slots: NonZeroU16,
    layers: NonZeroU16,
    cyclic: bool,
    exhausted: bool,
    winding_head_layers: Vec<usize>,
    treated_zones: Vec<bool>,
}

impl<'a> EndWindingLayouter<'a> {
    pub(super) fn new(winding: &'a dyn Winding, cyclic: bool) -> Self {
        /*
        The start zone of the iterator is the left zone of the full coil with the shortest span.
        If the winding has no full coil, the start zone is slot 0, layer 0.
         */
        let s = if cyclic { Some(winding.slots()) } else { None };
        let start_zone =
            find_left_zone_of_shortest_coil(winding.coils_iter(), s).unwrap_or(Zone::new(0, 0));

        let num_zones = usize::from(winding.layers().get()) * usize::from(winding.slots().get());

        return Self {
            winding,
            layer: start_zone.layer.into(),
            slot: start_zone.slot.into(),
            exhausted: false,
            slots: winding.slots(),
            layers: winding.layers(),
            cyclic,
            winding_head_layers: vec![0; num_zones],
            treated_zones: vec![false; num_zones],
        };
    }

    pub(super) fn all_zones_treated(&self) -> bool {
        return self.treated_zones.iter().all(|elem| *elem);
    }

    pub(super) fn step(&mut self) {
        self.layer += 1;
        if self.layer == self.winding.layers().get() {
            // Set back to the zeroth layer and increase the slot number.
            self.layer = 0;
            self.slot += 1;

            // Set back to the zeroth slot
            if self.slot == self.winding.slots().get() {
                self.slot = 0;
            }
        }
    }

    fn cyclic_slots(&self) -> Option<NonZeroU16> {
        if self.cyclic { Some(self.slots) } else { None }
    }

    fn index(&self, slot: u16, layer: u16) -> usize {
        let indices = [usize::from(layer), usize::from(slot)];
        let dim_size = [
            usize::from(self.layers.get()),
            usize::from(self.slots.get()),
        ];
        cart_lin::cart_to_lin(&indices, &dim_size).unwrap_or(0)
    }
}

impl<'a> Iterator for EndWindingLayouter<'a> {
    type Item = CoilAndRank<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Check if the iterator is exhausted
        if self.exhausted {
            return None;
        }

        let mut shortest_coil: Option<&FullCoil> = None;
        let layers = self.winding.layers().get();
        let cyclic_slots = self.cyclic_slots();
        loop {
            // Zone hasn't been treated yet
            if !self.treated_zones[self.index(self.slot, self.layer)] {
                let zone = Zone::new(self.slot, self.layer);
                if let Some(new_candidate) = self.winding.coil_at(zone) {
                    match new_candidate {
                        Coil::Full(full_coil) => {
                            /*
                            Only evaluate the candidate if its end winding continues to the right.
                            This is the case if the first zone is positive and the coil is clockwise
                            OR if the first zone is negative and the coil is counter-clockwise.
                             */

                            if let Some(shortest_coil_candidate) = shortest_coil {
                                /*
                                Did we find the second conductor of the shortest coil?.
                                 */
                                if std::ptr::eq(full_coil, shortest_coil_candidate) {
                                    let [left_zone, right_zone] = left_and_right_zone(full_coil);

                                    // Determine the winding head layer of this coil
                                    let mut winding_head_layer = 0;
                                    for slot in full_coil.covered_slots(cyclic_slots) {
                                        // In the left and right zone, the layer range is limited
                                        let layers_range = if left_zone.slot == slot {
                                            left_zone.layer..layers
                                        } else if right_zone.slot == slot {
                                            0..right_zone.layer
                                        } else {
                                            0..layers
                                        };

                                        for layer in layers_range {
                                            let stored_whl =
                                                self.winding_head_layers[self.index(slot, layer)];
                                            if stored_whl > winding_head_layer {
                                                winding_head_layer = stored_whl;
                                            }
                                        }
                                    }

                                    // Now set the new minimum layer for all the zones covered by
                                    // this coil
                                    for slot in full_coil.covered_slots(cyclic_slots) {
                                        // In the left and right zone, the layer range is limited
                                        let layers_range = if left_zone.slot == slot {
                                            left_zone.layer..layers
                                        } else if right_zone.slot == slot {
                                            0..right_zone.layer
                                        } else {
                                            0..layers
                                        };

                                        for layer in layers_range {
                                            let i = self.index(slot, layer);
                                            self.winding_head_layers[i] = winding_head_layer + 1;
                                        }
                                    }

                                    let result = CoilAndRank {
                                        coil: new_candidate,
                                        winding_head_layer,
                                    };

                                    // Remove the zones of the found coil from the evaluation
                                    let pos_zone = full_coil.positive_zone();
                                    let i = self.index(pos_zone.slot, pos_zone.layer);
                                    self.treated_zones[i] = true;

                                    let neg_zone = full_coil.negative_zone();
                                    let i = self.index(neg_zone.slot, neg_zone.layer);
                                    self.treated_zones[i] = true;

                                    // Check for exhaustion
                                    self.exhausted = self.all_zones_treated();

                                    self.step();
                                    return Some(result);
                                } else {
                                    /*
                                    If a candidate for the shortest coil in this end winding layer has already been identified,
                                    compare the span of the candidate to that of the current coil
                                    If true, new_candidate becomes the new shortest_coil.
                                    */
                                    if zone == left_and_right_zone(&full_coil)[0]
                                        && shortest_coil_candidate.throw(cyclic_slots)
                                            > new_candidate.throw(cyclic_slots)
                                    {
                                        shortest_coil = Some(full_coil);
                                    }
                                }
                            } else {
                                // Currently no shortest coil candidate has been selected => The new
                                // candidate becomes the shortest coil candidate.
                                if zone == left_and_right_zone(&full_coil)[0] {
                                    shortest_coil = Some(full_coil);
                                }
                            }
                        }
                        Coil::Half(coil_half) => {
                            // Half-coils are returned immediately
                            let result = CoilAndRank {
                                coil: new_candidate,
                                winding_head_layer: 0,
                            };

                            // Remove the zone of the found coil from the evaluation
                            let coil_zone = coil_half.any_zone();
                            let i = self.index(coil_zone.slot, coil_zone.layer);
                            self.treated_zones[i] = true;

                            // Check for exhaustion
                            self.exhausted = self.all_zones_treated();
                            self.step();
                            return Some(result);
                        }
                    }
                } else {
                    // Zone is empty
                    let i = self.index(self.slot, self.layer);
                    self.treated_zones[i] = true;
                }
            }
            self.step();
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct CoilAndRank<'a> {
    pub(super) coil: &'a Coil,
    pub(super) winding_head_layer: usize,
}

/**
Find the coil with the shortest span.
 */
fn find_left_zone_of_shortest_coil(
    coils: CoilsIterator<'_>,
    slots: Option<NonZeroU16>,
) -> Option<Zone> {
    return coils
        .filter_map(|coil| {
            // Only check full coils
            match coil {
                Coil::Full(full_coil) => Some(full_coil),
                Coil::Half(_) => None,
            }
        })
        .reduce(|shortest, current| {
            if shortest.throw(slots) > current.throw(slots) {
                current
            } else {
                shortest
            }
        })
        .map(|coil| left_and_right_zone(coil)[0]);
}

/**
Get the zone on the "left" side of the coil
 */
fn left_and_right_zone(coil: &FullCoil) -> [Zone; 2] {
    if coil.positive_slot_direction() {
        [coil.positive_zone(), coil.negative_zone()]
    } else {
        [coil.negative_zone(), coil.positive_zone()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn test_find_shortest_coil_left_zone() {
        let mut winding: DistributedWinding = DistributedMinimalBuilder {
            slots: 18.try_into().expect("not zero"),
            pole_pairs: 1.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            coil_throw_reduction: 0,
            zone_shift: 0,
            winding_table_constructor: WindingTableConstructor::Tingley,
        }
        .try_into()
        .unwrap();
        winding.set_concentric_coils(true);
        let zone =
            find_left_zone_of_shortest_coil(winding.coils_iter(), Some(winding.slots())).unwrap();
        assert_eq!(zone, Zone::new(11, 0));
    }

    #[test]
    fn test_single_layer_18_4() {
        let winding: DistributedWinding = DistributedMinimalBuilder {
            slots: 18.try_into().expect("not zero"),
            pole_pairs: 4.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_throw_reduction: 0,
            zone_shift: 0,
            winding_table_constructor: WindingTableConstructor::CoilSide,
        }
        .try_into()
        .unwrap();

        let mut layouter = EndWindingLayouter::new(&winding, true);
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 0);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(0, 0), Zone::new(2, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 0);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(3, 0), Zone::new(5, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 0);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(6, 0), Zone::new(8, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 0);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(9, 0), Zone::new(11, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 0);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(12, 0), Zone::new(14, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 0);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(15, 0), Zone::new(17, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 1);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(1, 0), Zone::new(4, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 1);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(7, 0), Zone::new(10, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 1);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(13, 0), Zone::new(16, 0)]
            );
        }
        assert!(layouter.next().is_none());
    }

    #[test]
    fn test_single_layer_18_1_dl() {
        let mut winding: DistributedWinding = DistributedMinimalBuilder {
            slots: 18.try_into().expect("not zero"),
            pole_pairs: 1.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            coil_throw_reduction: 0,
            zone_shift: 0,
            winding_table_constructor: WindingTableConstructor::Tingley,
        }
        .try_into()
        .unwrap();
        winding.set_concentric_coils(true);

        let layouter = EndWindingLayouter::new(&winding, true);
        let coils_and_rank: Vec<_> = layouter.collect();
        assert_eq!(coils_and_rank.len(), 18);
    }

    #[test]
    fn test_single_layer_36_2() {
        let mut winding: DistributedWinding = DistributedMinimalBuilder {
            slots: 36.try_into().expect("not zero"),
            pole_pairs: 2.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 2.try_into().expect("not zero"),
            coil_throw_reduction: 0,
            zone_shift: 0,
            winding_table_constructor: WindingTableConstructor::Tingley,
        }
        .try_into()
        .unwrap();
        winding.set_concentric_coils(true);

        let layouter = EndWindingLayouter::new(&winding, true);
        let coils_and_rank: Vec<_> = layouter.collect();
        assert_eq!(coils_and_rank.len(), 36);
    }

    #[test]
    fn test_single_layer_12_1_concentric() {
        let mut winding: DistributedWinding = DistributedMinimalBuilder {
            slots: 12.try_into().expect("not zero"),
            pole_pairs: 1.try_into().expect("not zero"),
            phases: 3.try_into().expect("not zero"),
            layers: 1.try_into().expect("not zero"),
            coil_throw_reduction: 0,
            zone_shift: 0,
            winding_table_constructor: WindingTableConstructor::Tingley,
        }
        .try_into()
        .unwrap();
        winding.set_concentric_coils(true);

        let mut layouter = EndWindingLayouter::new(&winding, true);

        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 0);
            assert_eq!(item.coil.phase().get(), 1);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(1, 0), Zone::new(6, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 1);
            assert_eq!(item.coil.phase().get(), 3);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(9, 0), Zone::new(2, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 2);
            assert_eq!(item.coil.phase().get(), 2);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(5, 0), Zone::new(10, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 3);
            assert_eq!(item.coil.phase().get(), 1);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(0, 0), Zone::new(7, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 4);
            assert_eq!(item.coil.phase().get(), 3);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(8, 0), Zone::new(3, 0)]
            );
        }
        {
            let item = layouter.next().unwrap();
            assert_eq!(item.winding_head_layer, 5);
            assert_eq!(item.coil.phase().get(), 2);
            assert_eq!(
                item.coil.zones().collect::<Vec<_>>(),
                vec![Zone::new(4, 0), Zone::new(11, 0)]
            );
        }

        assert!(layouter.next().is_none());
    }
}
