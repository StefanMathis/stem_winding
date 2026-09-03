use magnetic_core::{CoreRef, IsCoreRef};
use stem_primitives::Overrides;
use uom::si::f64::*;

use crate::{Coil, CoilExt, Winding};

pub struct CoilPropertyIterator<'a> {
    pub coils: crate::CoilsIterator<'a>,
    pub winding: &'a dyn Winding,
    pub core: CoreRef<'a>,
    pub overrides: &'a Overrides,
}

impl<'a> Iterator for CoilPropertyIterator<'a> {
    type Item = CoilProperties<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.coils.next().map(|coil| CoilProperties {
            coil,
            winding: self.winding,
            core: self.core.clone(),
            overrides: self.overrides,
        })
    }
}

pub struct CoilProperties<'a> {
    pub coil: &'a Coil,
    pub winding: &'a dyn Winding,
    pub core: CoreRef<'a>,
    pub overrides: &'a Overrides,
}

impl<'a> CoilProperties<'a> {
    pub fn coil(&self) -> &'a Coil {
        return self.coil;
    }

    /**
    Return the end winding length of a half-turn. A half turn starts in the middle of the end winding
    on one side and ends in the middle of the end winding of the other side.

    If the space of a single character in the ASCII drawing below equals one mm, the return value of
    this function would be 7 mm (│ + ┌ + 4*─ + ┐).

    ```text
       ┌──────┐
    ┌──│      │──┐
    │  │ Core │  │ <-- Coil
    └──│      │──┘
       └──────┘
    ```
     */
    pub fn end_winding_half_turn_length(&self) -> Length {
        return self
            .winding
            .end_winding_half_turn_length(self.core.clone(), self.coil.first_zone(), self.overrides)
            .expect("must contain a coil");
    }

    pub fn end_winding_half_turn_volume(&self) -> Volume {
        return self
            .winding
            .end_winding_half_turn_volume(self.core.clone(), self.coil.first_zone(), self.overrides)
            .expect("must contain a coil");
    }

    pub fn end_winding_volume(&self) -> Volume {
        match self.coil() {
            Coil::Full(coil_full) => {
                return 2.0 * self.end_winding_half_turn_volume() * coil_full.turns() as f64;
            }
            Coil::Half(coil_half) => {
                return self.end_winding_half_turn_volume() * coil_half.turns() as f64;
            }
        }
    }

    pub fn volume(&self) -> Volume {
        let winding_area = self.core.zone_area();
        let first_zone = self.coil.first_zone();
        let cross_section = self
            .coil
            .wire()
            .cross_section(winding_area, self.coil.turns());
        let coil_length = self.core.axial_coil_length()
            + self
                .winding
                .axial_coil_overhang(self.core.clone(), first_zone)
                .expect("must contain a coil");
        let multiplier = match self.coil() {
            Coil::Full(_) => 2.0,
            Coil::Half(_) => 1.0,
        };
        return (cross_section * coil_length + self.end_winding_half_turn_volume())
            * multiplier
            * self.coil.turns() as f64;
    }

    pub fn mass(&self) -> Mass {
        let mass_density = self
            .coil
            .wire()
            .material_conductor()
            .mass_density()
            .get(&[]);
        return self.volume() * mass_density;
    }

    pub fn heat_capacity(&self) -> HeatCapacity {
        let specific_heat_capacity = self
            .coil
            .wire()
            .material_conductor()
            .heat_capacity()
            .get(&[]);
        return self.mass() * specific_heat_capacity;
    }
}
