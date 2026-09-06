use std::num::{NonZeroU16, NonZeroUsize};

use num::Integer;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use stem_coil_layout::{CoilLayout, Zone};
use stem_wire::{round::RoundWire, wire::Wire};

use crate::{
    coils::{Coil, CoilFull, Coils},
    error::{Error, WindingTableCreationError},
    winding::{Connection, Winding, periodicity},
    winding_table::{WindingTable, WindingTableMethod},
};

#[derive(Debug, Clone)]
pub struct ToothCoilWinding {
    slots: NonZeroU16,            // Inner of slots
    pole_pairs: NonZeroU16,       // Inner of pole pairs
    phases: NonZeroU16,           // Inner of phases
    layers: NonZeroU16,           // Inner of layers
    turns_per_coil: NonZeroUsize, // Inner of turns per coil
    parallel_paths: NonZeroU16,   // Inner of parallel paths
    connection: Connection,       /* Connection type, e.g. star (Y), delta (D) or
                                   * combinations like YY (double star) or DY
                                   * (star-delta) */
    end_winding_leakage_coefficient: f64, // End winding flux leakage coefficient
    wire: Box<dyn Wire>,                  // Wire motor
    coils: Coils,
    winding_table_method: WindingTableMethod,
}

impl ToothCoilWinding {
    pub fn wire(&self) -> &dyn Wire {
        &*self.wire
    }

    pub fn winding_table_method(&self) -> &WindingTableMethod {
        &self.winding_table_method
    }

    fn create_coils(
        &mut self,
        winding_table: &WindingTable,
        all_zones_must_be_used: bool,
    ) -> Result<(), Error> {
        for slot in 0..self.slots.get() {
            for layer in 0..self.layers.get() {
                let zone = Zone { slot, layer };

                // Check if the zone is already occupied
                if self.coils.0.contains_key(&zone) {
                    continue;
                }

                // Insert the coil
                if let Some(coil) = self.create_single_coil(zone, winding_table) {
                    let zones: Vec<Zone> = coil.zones().collect();
                    self.coils.0.insert_many(zones, coil).map_err(Error::from)?;
                }
            }
        }

        // Check if all zones are occupied
        if all_zones_must_be_used {
            for (zone, _) in winding_table.iter_slots() {
                // Check if the zone is already occupied
                if !self.coils.0.contains_key(&zone) {
                    return Err(WindingTableCreationError::EmptyZone(Some(zone)).into());
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
        let phase_abs = NonZeroU16::new(phase.try_into().expect("the number of phases cannot exceed the maximum value of an u16 (this is guaranteed by the winding constructor)")).expect("not zero");
        let slot = zone.slot;
        let layer = zone.layer;

        // Different treatment of single-and double-layer winding
        if self.layers().get() == 1 {
            /*
            Move "forward" through the slots of the zone plan and count the number of times that the (absolute) phase
            of the input slot is encountered. If the number is odd, the return conductor for the current zone is in "forward"
            direction as well. Otherwise, it is in the "backward direction" (with corresponding adjustment of the coil direction)
             */
            let mut counter: i32 = 0;
            for search_slot in 1..winding_table.slots() {
                let phase_search_slot =
                    winding_table.get_cyclic(Zone::new(search_slot + slot, layer));
                if phase_search_slot.abs() != phase.abs() {
                    let negative_zone = if counter.is_odd() {
                        Zone::new((slot + 1).rem_euclid(self.slots().get()), layer)
                    } else {
                        Zone::new(slot.checked_sub(1).unwrap_or(self.slots().get() - 1), layer)
                    };

                    return CoilFull::new(
                        Zone::new(slot, layer),
                        negative_zone,
                        true,
                        counter.is_odd(),
                        self.turns_per_coil,
                        phase_abs,
                        dyn_clone::clone_box(&*self.wire),
                    )
                    .ok()
                    .map(|coil| Coil::Full(coil));
                }
                counter += 1;
            }
            panic!("zone plan does not contain a return conductor for the input conductor!")
        } else {
            let negative_zone = if layer == 0 {
                // Coil must start at the previous slot
                Zone::new(slot.checked_sub(1).unwrap_or(self.slots().get() - 1), 1)
            } else {
                // Coil must stop in the next slot
                Zone::new((slot + 1).rem_euclid(self.slots().get()), 0)
            };

            return CoilFull::new(
                Zone::new(slot, layer),
                negative_zone,
                true,
                layer == 1,
                self.turns_per_coil,
                phase_abs,
                dyn_clone::clone_box(&*self.wire),
            )
            .ok()
            .map(|coil| Coil::Full(coil));
        }
    }
}

impl Default for ToothCoilWinding {
    fn default() -> Self {
        ToothCoilBuilder {
            slots: NonZeroU16::new(6).expect("not zero"),
            pole_pairs: NonZeroU16::MIN,
            phases: NonZeroU16::new(3).expect("not zero"),
            layers: NonZeroU16::MIN,
            turns_per_coil: NonZeroUsize::MIN,
            parallel_paths: NonZeroU16::MIN,
            connection: Connection::Star,
            end_winding_leakage_coefficient: 0.0,
            wire: Box::new(RoundWire::default()),
            winding_table_method: WindingTableMethod::Tingley,
        }
        .try_into()
        .expect("valid inputs")
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Winding for ToothCoilWinding {
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

    fn periodicity(&self) -> NonZeroU16 {
        periodicity(
            self.slots(),
            self.pole_pairs(),
            self.phases(),
            self.layers(),
        )
    }

    fn coil_layout(&self) -> CoilLayout {
        if self.layers().get() == 1 {
            return CoilLayout::Single;
        } else {
            return CoilLayout::DoubleHorizontal;
        }
    }

    fn turns_per_phase(&self, _phase: NonZeroU16) -> num::rational::Ratio<usize> {
        return num::rational::Ratio::new_raw(
            (usize::from(self.layers().get() * self.slots().get())) * self.turns_per_coil.get()
                / usize::from(2 * self.phases().get() * self.parallel_paths().get()),
            1,
        );
    }

    fn parallel_paths(&self) -> NonZeroU16 {
        self.parallel_paths
    }

    fn connection(&self) -> Connection {
        return self.connection;
    }

    fn end_winding_leakage_coefficient(&self) -> f64 {
        return self.end_winding_leakage_coefficient;
    }

    fn coil_at(&self, zone: Zone) -> Option<&Coil> {
        return self.coils.0.get(&zone);
    }

    /**
    Returns the number of coil groups per phase. This value is equal to the maximum possible number of parallel paths and can be calculated
    as described in [Seq50], p. 37: First, the number of coils per phase in a basic winding is calculated. Then, it is checked whether this
    number is even or odd. If it is even, the number of coil groups equals twice the number of basic windings (= the periodicity). If it is odd,
    the number of coil groups equals the number of basic windings.
    */
    fn coil_groups_per_phase(&self) -> NonZeroU16 {
        let t = self.periodicity();
        let number_of_coils_in_basic_winding =
            self.slots().get() * self.layers().get() / (t.get() * 2 * self.phases().get());
        if number_of_coils_in_basic_winding % 2 == 0 {
            // Antiparallel coil groups are possible
            return NonZeroU16::new(2 * t.get()).expect("cannot be zero, since t is NonZeroU16");
        } else {
            // Only parallel coil groups are possible
            return t;
        }
    }

    fn number_coils(&self) -> usize {
        return (self.slots().get() * self.layers().get() / 2).into();
    }

    fn as_dyn(&self) -> &dyn Winding {
        self
    }

    #[cfg(feature = "stem_core")]
    fn is_symmetric(&self, _core: CoreRef<'_>, _overrides: &Overrides) -> bool {
        return true;
    }

    #[cfg(feature = "stem_core")]
    fn resistance_components(
        &self,
        core: CoreRef<'_>,
        overrides: &Overrides,
    ) -> Option<crate::ResistanceComponents> {
        let material = self.wire().material_conductor().clone();
        if let Some(resistance_constant) = overrides.resistance_constant {
            return Some(crate::ResistanceComponents {
                resistance_constant,
                material,
            });
        }

        let resistance = self.resistance(1, core, &[], overrides);
        let electrical_resistivity = material.electrical_resistivity().get(&[]);
        return Some(crate::ResistanceComponents {
            resistance_constant: resistance / electrical_resistivity,
            material,
        });
    }

    #[cfg(feature = "stem_core")]
    fn resistance(
        &self,
        phase: u16,
        core: CoreRef<'_>,
        conditions: &[InfluencingQuantity],
        overrides: &Overrides,
    ) -> ElectricalResistance {
        let electrical_resistivity = self
            .wire()
            .material_conductor()
            .electrical_resistivity()
            .get(conditions);

        if let Some(resitance_constant) = overrides.resistance_constant {
            return resitance_constant * electrical_resistivity;
        }

        let mean_coil_turn_length = 2.0
            * (core.axial_coil_length()
                + core.axial_coil_overhang()
                + self
                    .end_winding_half_turn_length(core, Zone::new(0, 0), overrides)
                    .expect("must contain a coil"));

        return self.wire().resistance(
            core.zone_area() / self.turns_in_slot(0) as f64,
            mean_coil_turn_length,
            conditions,
        ) * self.turns_per_phase(phase).to_integer() as f64
            / self.parallel_paths() as f64;
    }

    #[cfg(feature = "stem_core")]
    fn end_winding_leakage_inductance(
        &self,
        _phase: u16,
        core: CoreRef<'_>,
        overrides: &Overrides,
    ) -> Inductance {
        if let Some(end_winding_leakage_inductance) = overrides.end_winding_leakage_inductance {
            return end_winding_leakage_inductance;
        }

        return *material::VACUUM_PERMEABILITY
            * self.end_winding_leakage_coefficient()
            * self.slots() as f64
            / (self.layers() as f64 * self.phases() as f64)
            * self.turns_in_slot(0).pow(2) as f64
            / self.parallel_paths().pow(2) as f64
            * (self
                .end_winding_half_turn_length(core, Zone::new(0, 0), overrides)
                .expect("must contain a coil")
                + core.axial_coil_overhang());
    }

    #[cfg(feature = "stem_core")]
    fn end_winding_half_turn_length(
        &self,
        core: CoreRef<'_>,
        _zone: Zone,
        overrides: &Overrides,
    ) -> Option<Length> {
        use std::f64::consts::{FRAC_PI_2, PI};

        if let Some(end_winding_half_turn_length) = overrides.end_winding_half_turn_length {
            return Some(end_winding_half_turn_length);
        }

        let length = if self.layers() == 1 {
            FRAC_PI_2 * core.mean_slot_distance()
        } else {
            // Different calculation patterns depending on the "slottedness" of the core
            if core.slotted() {
                PI / 4.0 * (core.tooth_width() + core.mean_slot_distance())
            } else {
                PI / 4.0 * core.mean_slot_distance()
            }
        };
        return Some(length);
    }
}

// =================================================================================
// Builders

pub struct ToothCoilBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub layers: NonZeroU16,
    pub turns_per_coil: NonZeroUsize,
    pub parallel_paths: NonZeroU16,
    pub connection: Connection,
    pub end_winding_leakage_coefficient: f64,
    pub wire: Box<dyn Wire>,
    pub winding_table_method: WindingTableMethod,
}

impl TryFrom<ToothCoilBuilder> for ToothCoilWinding {
    type Error = Error;

    fn try_from(builder: ToothCoilBuilder) -> Result<Self, Self::Error> {
        let layers = builder.layers.get();
        compare_variables::compare_variables!(layers < 3)?;
        compare_variables::compare_variables!(0.0 <= builder.end_winding_leakage_coefficient)?;

        // Calculate the basic winding parameters
        let t = periodicity(
            builder.slots,
            builder.pole_pairs,
            builder.phases,
            builder.layers,
        )
        .get();
        let slots_basic = builder.slots.get() / t;
        let pole_pairs_basic = builder.pole_pairs.get() / t;

        // Create the zone plan by method
        let winding_table = WindingTable::with_method(
            &builder.winding_table_method,
            NonZeroU16::new(slots_basic).expect("not zero"),
            builder.layers,
            NonZeroU16::new(pole_pairs_basic).expect("not zero"),
            builder.phases,
            1, // Equals 1 by definition
        )?;

        let mut winding = ToothCoilWinding {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            layers: builder.layers,
            turns_per_coil: builder.turns_per_coil,
            parallel_paths: builder.parallel_paths,
            connection: builder.connection,
            end_winding_leakage_coefficient: builder.end_winding_leakage_coefficient,
            wire: builder.wire,
            coils: Coils::with_capacity((builder.slots.get() * builder.layers.get() / 2).into()),
            winding_table_method: builder.winding_table_method,
        };
        winding.create_coils(&winding_table, true)?;

        // Check if the number of parallel paths is valid
        if winding
            .possible_parallel_paths()
            .all(|possible_path| possible_path != winding.parallel_paths)
        {
            return Err(Error::InvalidNumberParallelPaths);
        }

        if winding.equal_winding_factors() {
            return Ok(winding);
        } else {
            return Err(WindingTableCreationError::NotSymmetric.into());
        }
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ToothCoilMinimalBuilder {
    pub slots: NonZeroU16,
    pub pole_pairs: NonZeroU16,
    pub phases: NonZeroU16,
    pub layers: NonZeroU16,
    pub winding_table_method: WindingTableMethod,
}

impl TryFrom<ToothCoilMinimalBuilder> for ToothCoilWinding {
    type Error = Error;

    fn try_from(builder: ToothCoilMinimalBuilder) -> Result<Self, Self::Error> {
        ToothCoilBuilder {
            slots: builder.slots,
            pole_pairs: builder.pole_pairs,
            phases: builder.phases,
            layers: builder.layers,
            turns_per_coil: NonZeroUsize::MIN,
            parallel_paths: NonZeroU16::MIN,
            connection: Connection::Star,
            end_winding_leakage_coefficient: 0.0,
            wire: Box::new(RoundWire::default()),
            winding_table_method: builder.winding_table_method,
        }
        .try_into()
    }
}
