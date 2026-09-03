use slot::CoilLayout;

use crate::{Coil, CoilExt, Coils, Connection, Winding, Zone};


use uom::si::f64::*;


use magnetic_core::{CoreRef, IsCoreRef};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/**
This winding type defines a winding as a collection of coils connected to each other,
giving fine-grained control of all aspects (e.g. defining a specific number of
turns for each coil) at the cost of increased complexity. All other winding types
can be interpreted as simplified versions of this winding type.
*/
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CoilAssembly {
    slots: u16,      // Inner of slots
    pole_pairs: u16, // Inner of pole pairs
    phases: u16,     // Inner of phases
    layers: u16,     // Inner of layers
    coils: Coils,
    coil_layout: CoilLayout,
    #[cfg_attr(feature = "serde", serde(default))]
    end_winding_leakage_coefficient: f64,
    #[cfg_attr(feature = "serde", serde(default = "parallel_paths_default"))]
    parallel_paths: u16,
    #[cfg_attr(feature = "serde", serde(default = "connection_default"))]
    connection: Connection,
}

impl CoilAssembly {
    pub fn new(
        slots: u16,
        pole_pairs: u16,
        phases: u16,
        layers: u16,
        coils: Coils,
        coil_layout: CoilLayout,
        end_winding_leakage_coefficient: f64,
        parallel_paths: u16,
        connection: Connection,
    ) -> stem_primitives::Result<Self> {
        // Check if the coil layout is suitable for the number of layers
        if coil_layout.layers() != layers {
            let cl_layers = coil_layout.layers();
            return Err(stem_primitives::ErrorType::Other(
                format!("The number of layers ({layers} does not fit the selected coil layout, whose number of layers equals {cl_layers}."),
            )
            .into());
        }

        let winding = CoilAssembly {
            slots,
            pole_pairs,
            phases,
            layers,
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
        slots: u16,
        pole_pairs: u16,
        phases: u16,
        layers: u16,
        coils: Coils,
        coil_layout: CoilLayout,
    ) -> stem_primitives::Result<Self> {
        return Self::new(
            slots,
            pole_pairs,
            phases,
            layers,
            coils,
            coil_layout,
            0.0,
            1,
            Connection::Star,
        );
    }

    // Try to insert a coil
    pub fn insert<C: Into<Coil>>(&mut self, coil: C) -> stem_primitives::Result<()> {
        pub fn insert_priv(ca: &mut CoilAssembly, coil: Coil) -> stem_primitives::Result<()> {
            ca.coil_is_valid(&coil)?;
            let zones: Vec<Zone> = coil.zones().collect();
            return ca
                .coils
                .0
                .insert_many(zones, coil)
                .map_err(|err| stem_primitives::ErrorType::Other(err.to_string()).into());
        }
        return insert_priv(self, coil.into());
    }

    /// Try to remove the coil occupying the given zone
    pub fn remove(&mut self, zone: Zone) -> Option<Coil> {
        return self.coils.0.remove(&zone);
    }

    /// Remove all coils from the coil assembly
    pub fn clear_coils(&mut self) {
        self.coils.0.clear();
    }

    pub fn set_pole_pairs(&mut self, pole_pairs: u16) {
        self.pole_pairs = pole_pairs;
    }

    /**
    Mutably access a coil
     */
    pub fn coil_at_mut(&mut self, zone: Zone) -> stem_primitives::Result<&mut Coil> {
        match self.coils.0.get_mut(&zone) {
            Some(coil) => return Ok(coil),
            None => {
                return Err(stem_primitives::ErrorType::EmptyZone {
                    slot: zone.slot,
                    layer: zone.layer,
                }
                .into());
            }
        }
    }

    /// Check if the given coil collection corresponds to the defined number of
    /// phases, slots and layers
    fn coil_is_valid(&self, coil: &Coil) -> stem_primitives::Result<()> {
        if coil.phase() > self.phases() {
            return Err(stem_primitives::ErrorType::Other(format!(
                "the winding has {} phases, but the coil has the phase {}",
                self.phases(),
                coil.phase()
            ))
            .into());
        }
        for zone in coil.zones() {
            if zone.slot >= self.slots() {
                return Err(stem_primitives::ErrorType::Other(format!(
                    "the winding has {} slots, but the coil occupies slot {}",
                    self.slots(),
                    zone.slot
                ))
                .into());
            }
            if zone.layer >= self.layers() {
                return Err(stem_primitives::ErrorType::Other(format!(
                    "the winding has {} layers, but the coil occupies layer {}",
                    self.layers(),
                    zone.layer
                ))
                .into());
            }
        }
        return Ok(());
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Winding for CoilAssembly {
    fn phases(&self) -> u16 {
        return self.phases;
    }

    fn slots(&self) -> u16 {
        return self.slots;
    }

    fn pole_pairs(&self) -> u16 {
        return self.pole_pairs;
    }

    fn layers(&self) -> u16 {
        return self.layers;
    }

    fn periodicity(&self) -> u16 {
        return 1;
    }

    fn coil_at(&self, zone: Zone) -> Option<&Coil> {
        return self.coils.0.get(&zone);
    }

    fn coil_layout(&self) -> CoilLayout {
        return self.coil_layout;
    }

    fn parallel_paths(&self) -> u16 {
        return self.parallel_paths;
    }

    fn connection(&self) -> Connection {
        return self.connection;
    }

    fn end_winding_leakage_coefficient(&self) -> f64 {
        return self.end_winding_leakage_coefficient;
    }

    fn as_dyn(&self) -> &dyn Winding {
        self
    }

    
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
            layers: winding.layers(),
            coils,
            coil_layout: winding.coil_layout(),
            end_winding_leakage_coefficient: winding.end_winding_leakage_coefficient(),
            parallel_paths: winding.parallel_paths(),
            connection: winding.connection(),
        };
    }
}

#[cfg(feature = "serde")]
fn parallel_paths_default() -> u16 {
    1
}
#[cfg(feature = "serde")]
fn connection_default() -> Connection {
    Connection::Star
}
