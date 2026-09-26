use std::num::NonZeroU16;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use stem_coil_layout::{CoilLayout, Zone};

#[cfg(feature = "stem_core")]
use stem_core::prelude::*;

#[cfg(feature = "stem_core")]
use crate::core_support::*;

use crate::{
    coils::{Coil, CoilExt, Coils},
    error::Error,
    winding::{Connection, Winding},
};

/**
This winding type defines a winding as a collection of coils connected to each other,
giving fine-grained control of all aspects (e.g. defining a specific number of
turns for each coil) at the cost of increased complexity. All other winding types
can be interpreted as simplified versions of this winding type.

TODO: end_winding_leakage_inductance and end_winding_half_turn_length cannot
really be calculated for arbitrary coil assembly, so we use best guess approaches here.
When the CoilAssembly was derived from a specialized winding, store the values
from the specialized winding in an [`Overrides`] and use this when calculating
data such as e.g. the total winding resistance!
*/
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "CoilAssemblyBuilder"))]
pub struct CoilAssembly {
    slots: NonZeroU16,
    pole_pairs: NonZeroU16,
    phases: NonZeroU16,
    coil_layout: CoilLayout,
    coils: Coils,
    end_winding_leakage_coefficient: f64,
    parallel_paths: NonZeroU16,
    connection: Connection,
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
struct CoilAssemblyBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub coil_layout: CoilLayout,
    pub coils: Coils,
    #[cfg_attr(feature = "serde", serde(default))]
    pub end_winding_leakage_coefficient: f64,
    #[cfg_attr(feature = "serde", serde(default = "parallel_paths_default"))]
    pub parallel_paths: NonZeroU16,
    #[cfg_attr(feature = "serde", serde(default = "connection_default"))]
    pub connection: Connection,
}

#[cfg(feature = "serde")]
fn parallel_paths_default() -> NonZeroU16 {
    NonZeroU16::MIN
}
#[cfg(feature = "serde")]
fn connection_default() -> Connection {
    Connection::Star
}

impl TryFrom<CoilAssemblyBuilder> for CoilAssembly {
    type Error = Error;

    fn try_from(builder: CoilAssemblyBuilder) -> Result<Self, Self::Error> {
        let winding = CoilAssembly {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            coils: builder.coils,
            coil_layout: builder.coil_layout,
            end_winding_leakage_coefficient: builder.end_winding_leakage_coefficient,
            parallel_paths: builder.parallel_paths,
            connection: builder.connection,
        };

        // Check all coils of the winding
        for coil in winding.coils.iter_coils() {
            winding.coil_is_valid(coil)?;
        }

        return Ok(winding);
    }
}

impl CoilAssembly {
    pub fn new(
        slots: NonZeroU16,
        pole_pairs: NonZeroU16,
        phases: NonZeroU16,
        coil_layout: CoilLayout, // Defines layers
        coils: Coils,
        end_winding_leakage_coefficient: f64,
        parallel_paths: NonZeroU16,
        connection: Connection,
    ) -> Result<Self, Error> {
        let winding = CoilAssembly {
            slots,
            pole_pairs,
            phases,
            coils,
            coil_layout,
            end_winding_leakage_coefficient,
            parallel_paths,
            connection,
        };

        // Check all coils of the winding
        for coil in winding.coils.iter_coils() {
            winding.coil_is_valid(coil)?;
        }

        return Ok(winding);
    }

    pub fn new_minimal(
        slots: NonZeroU16,
        pole_pairs: NonZeroU16,
        phases: NonZeroU16,
        coil_layout: CoilLayout,
        coils: Coils,
    ) -> Result<Self, Error> {
        return Self::new(
            slots,
            pole_pairs,
            phases,
            coil_layout,
            coils,
            0.0,
            NonZeroU16::MIN,
            Connection::Star,
        );
    }

    // Try to insert a coil
    pub fn insert<C: Into<Coil>>(&mut self, coil: C) -> Result<(), Error> {
        let coil: Coil = coil.into();
        self.coil_is_valid(&coil)?;
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

    /**
    Mutably access a coil
     */
    pub fn coil_at_mut(&mut self, zone: Zone) -> Option<&mut Coil> {
        self.coils.get_mut(zone)
    }

    /// Check if the given coil collection corresponds to the defined number of
    /// phases, slots and layers
    fn coil_is_valid(&self, coil: &Coil) -> Result<(), Error> {
        let coil_phase = coil.phase().get();
        let winding_phases = self.phases().get();
        let winding_slots = self.slots().get();
        let winding_layers = self.layers().get();
        compare_variables::compare_variables!(coil_phase <= winding_phases)?;

        for zone in coil.zones() {
            compare_variables::compare_variables!(zone.slot < winding_slots)?;
            compare_variables::compare_variables!(zone.layer < winding_layers)?;
        }
        return Ok(());
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Winding for CoilAssembly {
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
        self.coil_layout().layers()
    }

    fn coil_at(&self, zone: Zone) -> Option<&Coil> {
        return self.coils.get(zone);
    }

    fn coil_layout(&self) -> CoilLayout {
        self.coil_layout
    }

    fn parallel_paths(&self) -> NonZeroU16 {
        self.parallel_paths
    }

    fn connection(&self) -> Connection {
        self.connection
    }

    fn end_winding_leakage_coefficient(&self) -> f64 {
        self.end_winding_leakage_coefficient
    }

    fn as_dyn(&self) -> &dyn Winding {
        self
    }

    #[cfg(feature = "stem_core")]
    fn end_winding_leakage_inductance(
        &self,
        _core: CoreRef<'_>,
        _phase: NonZeroU16,
        _end_winding_half_turn_length: Option<Length>,
    ) -> Inductance {
        // We cannot analytically calculate the end winding leakage inductance
        // for an arbitrary coil setup, so we just return zero.
        return Inductance::new::<si::inductance::henry>(0.0);
    }

    #[cfg(feature = "stem_core")]
    fn end_winding_half_turn_length(&self, core: CoreRef<'_>, zone: Zone) -> Length {
        let coil = match self.coil_at(zone) {
            Some(c) => c,
            None => return Length::new::<meter>(0.0),
        };

        // If the zones of the coil are in neighboring slots, use the
        // end_winding_half_turn_length_semicircle approximation, otherwise use
        // end_winding_half_turn_length_circular_arc /
        // end_winding_half_turn_length_straight. This is obviously a rough
        // approximation and will not necessarily match with the calculation
        // method of a specialized winding type such as a ToothCoilWinding
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
}

impl<W: Winding + ?Sized> From<&W> for CoilAssembly {
    fn from(winding: &W) -> Self {
        // Build the hashmap
        let mut coils = Coils::with_capacity(winding.number_coils(), winding.number_coils());
        for coil in winding.coils() {
            coils
                .insert(coil.clone())
                .expect("two coils occupy the same zone. This is a bug.")
        }

        return CoilAssembly {
            slots: winding.slots(),
            pole_pairs: winding.pole_pairs(),
            phases: winding.phases(),
            coils,
            coil_layout: winding.coil_layout(),
            end_winding_leakage_coefficient: winding.end_winding_leakage_coefficient(),
            parallel_paths: winding.parallel_paths(),
            connection: winding.connection(),
        };
    }
}
