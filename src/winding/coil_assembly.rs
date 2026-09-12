use std::num::NonZeroU16;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use stem_coil_layout::{CoilLayout, Zone};

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
        for coil in winding.coils.0.values() {
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
        for coil in winding.coils.0.values() {
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
        let zones: Vec<Zone> = coil.zones().collect();
        return self.coils.0.insert_many(zones, coil).map_err(Error::from);
    }

    /// Try to remove the coil occupying the given zone
    pub fn remove(&mut self, zone: Zone) -> Option<Coil> {
        return self.coils.0.remove(&zone);
    }

    /// Remove all coils from the coil assembly
    pub fn clear_coils(&mut self) {
        self.coils.0.clear();
    }

    pub fn set_pole_pairs(&mut self, pole_pairs: NonZeroU16) {
        self.pole_pairs = pole_pairs;
    }

    /**
    Mutably access a coil
     */
    pub fn coil_at_mut(&mut self, zone: Zone) -> Option<&mut Coil> {
        self.coils.0.get_mut(&zone)
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

    fn base_winding_count(&self) -> NonZeroU16 {
        NonZeroU16::MIN
    }

    fn coil_at(&self, zone: Zone) -> Option<&Coil> {
        return self.coils.0.get(&zone);
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
        phase: u16,
        core: CoreRef<'_>,
        overrides: &stem_primitives::Overrides,
    ) -> Inductance {
        if let Some(end_winding_leakage_inductance) = overrides.end_winding_leakage_inductance {
            return end_winding_leakage_inductance;
        }

        // Calculate the mean end winding and overhang length
        let number_coils = self.number_coils();
        let mean_ew_length = self
            .coils()
            .map(|coil| {
                let zone = coil.first_zone();
                self.end_winding_half_turn_length(core, zone, overrides)
                    .expect("must contain a coil")
                    + self
                        .axial_coil_overhang(core, zone)
                        .expect("must contain a coil")
            })
            .sum::<Length>()
            / number_coils as f64;

        // This calculation assumes a symmetric winding -> Only calculate for phase 1
        return 2.0
            * self.end_winding_leakage_coefficient()
            * *material::VACUUM_PERMEABILITY
            * self.turns_per_phase(phase).to_integer().pow(2) as f64
            * mean_ew_length
            / self.pole_pairs() as f64;
    }

    #[cfg(feature = "stem_core")]
    fn end_winding_half_turn_length(
        &self,
        core: CoreRef<'_>,
        zone: Zone,
        overrides: &stem_primitives::Overrides,
    ) -> Option<Length> {
        let coil = self.coil_at(zone)?;

        if let Some(end_winding_half_turn_length) = overrides.end_winding_half_turn_length {
            return Some(end_winding_half_turn_length);
        }
        if let Some(value) = coil.end_length() {
            return Some(value);
        }

        // This is an approximation of the end winding calculation based on heuristic
        // values from [Mat19]. The pitch ratio is calculated individually for
        // each coil
        let pitch_ratio = coil.span(self.slots()) as f64 / self.pole_pitch() as f64; // W/tau_p

        return Some(
            std::f64::consts::PI / (4.0 * self.pole_pairs() as f64)
                * core.mean_slot_distance()
                * self.slots() as f64
                * pitch_ratio,
        );
    }

    #[cfg(feature = "stem_core")]
    fn axial_coil_overhang(&self, core: CoreRef<'_>, zone: Zone) -> Option<Length> {
        let coil = self.coil_at(zone)?;
        if let Some(value) = coil.axial_overhang() {
            return Some(value);
        }
        return Some(core.axial_coil_overhang());
    }
}

impl<W: Winding + ?Sized> From<&W> for CoilAssembly {
    fn from(winding: &W) -> Self {
        // Build the hashmap
        let mut coils = Coils::with_capacity(winding.number_coils());
        for coil in winding.coils() {
            let zones: Vec<Zone> = coil.zones().collect();
            coils
                .0
                .insert_many(zones, coil.clone())
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
