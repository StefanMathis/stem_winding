use dyn_clone::clone_box;
use num::Integer;
use num::rational::Ratio;
use slot::CoilLayout;
use wire::{IsWire, RoundWire};


use material::InfluencingQuantity;


use magnetic_core::{CoreRef, IsCoreRef};


use stem_primitives::Overrides;


use uom::si::f64::*;

use crate::CoilBuilder;
use crate::{CoilFull, Coils, WindingTable};

use compare_variables::compare_variables;

use crate::{
    Coil, Zone, WindingTableMethod,
    common::{Connection, periodicity},
    is_winding::Winding,
    zones,
};

#[cfg(feature = "serde")]
use deserialize_untagged_verbose_error::DeserializeAlias;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, DeserializeAlias))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "serde_impl::WindingToothCoilVariants")
)]
#[cfg_attr(feature = "serde", deserialize_alias(name = "NewFull"))]
pub struct WindingToothCoil {
    slots: u16,                           // Inner of slots
    pole_pairs: u16,                      // Inner of pole pairs
    phases: u16,                          // Inner of phases
    layers: u16,                          // Inner of layers
    turns_per_coil: usize,                // Inner of turns per coil
    parallel_paths: u16,                  // Inner of parallel paths
    connection: Connection, // Connection type, e.g. star (Y), delta (D) or combinations like YY (double star) or DY (star-delta)
    end_winding_leakage_coefficient: f64, // End winding flux leakage coefficient
    wire: Box<dyn IsWire>,  // Wire motor
    coils: Coils,
}

impl WindingToothCoil {
    pub fn new(
        slots: u16,
        pole_pairs: u16,
        phases: u16,
        layers: u16,
        turns_per_coil: usize,
        parallel_paths: u16,
        connection: Connection,
        end_winding_leakage_coefficient: f64,
        wire: Box<dyn IsWire>,
        winding_table_method: WindingTableMethod,
    ) -> stem_primitives::Result<Self> {
        // Calculate the basic winding parameters
        let t = periodicity(slots, pole_pairs, phases, layers);
        let slots_basic = slots / t;
        let pole_pairs_basic = pole_pairs / t;

        let throw: i32 = 1; // Equals 1 by definition

        // Create the zone plan by method
        let winding_table = zones::create_winding_table_by_method(
            winding_table_method,
            slots_basic,
            pole_pairs_basic,
            phases,
            layers,
            throw,
            true,
        )?;

        let mut winding = WindingToothCoil {
            slots,
            pole_pairs,
            phases,
            layers,
            turns_per_coil,
            parallel_paths,
            connection,
            end_winding_leakage_coefficient,
            wire,
            coils: Coils::with_capacity((slots * layers / 2).into()),
        };
        winding.create_coils(&winding_table, true)?;
        return winding.check();
    }

    pub fn wire(&self) -> &dyn IsWire {
        return &*self.wire;
    }

    fn check(self) -> stem_primitives::Result<Self> {
        // Sanity checks
        compare_variables!(0 < self.slots)?;
        compare_variables!(0 < self.pole_pairs)?;
        compare_variables!(0 < self.parallel_paths)?;
        compare_variables!(0 < self.phases)?;
        compare_variables!(0 < self.layers < 3)?;
        compare_variables!(0.0 <= self.end_winding_leakage_coefficient)?;

        // Check if the number of parallel paths is valid
        let mut valid = false;
        for pp in self.possible_parallel_paths() {
            if pp == self.parallel_paths as usize {
                valid = true
            }
        }
        if !valid {
            return Err(stem_primitives::ErrorType::Other(
                "Given number of parallel paths not possible.".into(),
            )
            .into());
        }

        if self.equal_winding_factors() {
            return Ok(self);
        } else {
            return Err(stem_primitives::ErrorType::Other("Winding is not symmetric.".into()).into());
        }
    }
}

impl Clone for WindingToothCoil {
    fn clone(&self) -> Self {
        Self {
            slots: self.slots.clone(),
            pole_pairs: self.pole_pairs.clone(),
            phases: self.phases.clone(),
            layers: self.layers.clone(),
            turns_per_coil: self.turns_per_coil.clone(),
            parallel_paths: self.parallel_paths.clone(),
            connection: self.connection.clone(),
            end_winding_leakage_coefficient: self.end_winding_leakage_coefficient.clone(),
            wire: clone_box(&*self.wire),
            coils: self.coils.clone(),
        }
    }
}

impl Default for WindingToothCoil {
    fn default() -> Self {
        return WindingToothCoil::new(
            3,
            1,
            3,
            2,
            1,
            1,
            Connection::Star,
            0.0,
            Box::new(RoundWire::default()),
            WindingTableMethod::Tingley,
        )
        .unwrap();
    }
}

#[cfg_attr(feature = "serde", typetag::serde)]
impl Winding for WindingToothCoil {
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
        return periodicity(
            self.slots(),
            self.pole_pairs(),
            self.phases(),
            self.layers(),
        );
    }

    fn coil_layout(&self) -> CoilLayout {
        if self.layers() == 1 {
            return CoilLayout::Single;
        } else {
            return CoilLayout::DoubleHorizontal;
        }
    }

    fn turns_per_phase(&self, _phase: u16) -> Ratio<usize> {
        return Ratio::new_raw(
            (usize::from(self.layers() * self.slots())) * self.turns_per_coil
                / usize::from(2 * self.phases() * self.parallel_paths()),
            1,
        );
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

    fn coil_at(&self, zone: Zone) -> Option<&Coil> {
        return self.coils.0.get(&zone);
    }

    /**
    Returns the number of coil groups per phase. This value is equal to the maximum possible number of parallel paths and can be calculated
    as described in [Seq50], p. 37: First, the number of coils per phase in a basic winding is calculated. Then, it is checked whether this
    number is even or odd. If it is even, the number of coil groups equals twice the number of basic windings (= the periodicity). If it is odd,
    the number of coil groups equals the number of basic windings.
    */
    fn coil_groups_per_phase(&self) -> u16 {
        let t = self.periodicity();
        let number_of_coils_in_basic_winding =
            self.slots() * self.layers() / (t * 2 * self.phases());
        if number_of_coils_in_basic_winding % 2 == 0 {
            // Antiparallel coil groups are possible
            return 2 * t;
        } else {
            // Only parallel coil groups are possible
            return t;
        }
    }

    fn number_coils(&self) -> usize {
        return (self.slots() * self.layers() / 2).into();
    }

    fn as_dyn(&self) -> &dyn Winding {
        self
    }

    
    fn is_symmetric(&self, _core: CoreRef<'_>, _overrides: &Overrides) -> bool {
        return true;
    }

    
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

impl CoilBuilder for WindingToothCoil {
    fn coil_map(&self) -> &Coils {
        return &self.coils;
    }

    fn coil_map_mut(&mut self) -> &mut Coils {
        return &mut self.coils;
    }

    fn create_single_coil(&self, zone: Zone, winding_table: &WindingTable) -> Option<Coil> {
        // Only create a coil if the phase is positive. Since each coil has a positive and a negative phase,
        // in the end all coils will be created anyway. However, only considering positive phases makes the
        // function much simpler.
        let phase = winding_table.get_wrapped(zone);
        if phase < 0 {
            return None;
        }
        let phase_abs: u16 = phase.try_into().expect("the number of phases cannot exceed the maximum value of an u16 (this is guaranteed by the winding constructor)");
        let slot = zone.slot;
        let layer = zone.layer;

        // Different treatment of single-and double-layer winding
        if self.layers() == 1 {
            /*
            Move "forward" through the slots of the zone plan and count the number of times that the (absolute) phase
            of the input slot is encountered. If the number is odd, the return conductor for the current zone is in "forward"
            direction as well. Otherwise, it is in the "backward direction" (with corresponding adjustment of the coil direction)
             */
            let mut counter: i32 = 0;
            for search_slot in 1..winding_table.slots() {
                let phase_search_slot = winding_table.get_wrapped(Zone::new(search_slot + slot, layer));
                if phase_search_slot.abs() != phase.abs() {
                    let negative_zone = if counter.is_odd() {
                        Zone::new((slot + 1).rem_euclid(self.slots()), layer)
                    } else {
                        Zone::new(slot.checked_sub(1).unwrap_or(self.slots() - 1), layer)
                    };

                    return CoilFull::new(
                        Zone::new(slot, layer),
                        negative_zone,
                        true,
                        counter.is_odd(),
                        self.turns_per_coil,
                        phase_abs,
                        clone_box(&*self.wire),
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
                Zone::new(slot.checked_sub(1).unwrap_or(self.slots() - 1), 1)
            } else {
                // Coil must stop in the next slot
                Zone::new((slot + 1).rem_euclid(self.slots()), 0)
            };

            return CoilFull::new(
                Zone::new(slot, layer),
                negative_zone,
                true,
                layer == 1,
                self.turns_per_coil,
                phase_abs,
                clone_box(&*self.wire),
            )
            .ok()
            .map(|coil| Coil::Full(coil));
        }
    }
}

// =================================================================================
// Deserialization

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
struct NewWindingTableMethod {
    slots: u16,
    pole_pairs: u16,
    phases: u16,
    layers: u16,
    turns_per_coil: usize,
    parallel_paths: u16,
    connection: Connection, // Connection type, e.g. star (Y), delta (D) or combinations like YY (double star) or DY (star-delta)
    end_winding_leakage_coefficient: f64, // End winding flux leakage coefficient
    wire: Box<dyn IsWire>,  // Wire
    winding_table_method: WindingTableMethod,
}

impl TryFrom<NewWindingTableMethod> for WindingToothCoil {
    type Error = stem_primitives::Error;

    fn try_from(value: NewWindingTableMethod) -> Result<Self, Self::Error> {
        return Self::new(
            value.slots,
            value.pole_pairs,
            value.phases,
            value.layers,
            value.turns_per_coil,
            value.parallel_paths,
            value.connection,
            value.end_winding_leakage_coefficient,
            value.wire,
            value.winding_table_method,
        );
    }
}

#[derive(constructor)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[constructor(
    target = "WindingToothCoil",
    fn_name = "new_minimal",
    error = "stem_primitives::Error"
)]
struct NewMinimal {
    slots: u16,
    pole_pairs: u16,
    phases: u16,
    layers: u16,
    winding_table_method: WindingTableMethod,
}

impl TryFrom<NewMinimal> for WindingToothCoil {
    type Error = stem_primitives::Error;

    fn try_from(value: NewMinimal) -> Result<Self, Self::Error> {
        // Definition of default values
        let turns_per_coil = 1;
        let parallel_paths = 1;
        let connection = Connection::Star;
        let end_winding_leakage_coefficient = 0.0;
        let wire = Box::new(RoundWire::default());

        return Self::new(
            value.slots,
            value.pole_pairs,
            value.phases,
            value.layers,
            turns_per_coil,
            parallel_paths,
            connection,
            end_winding_leakage_coefficient,
            wire,
            value.winding_table_method,
        );
    }
}

#[cfg(feature = "serde")]
mod serde_impl {

    use super::*;
    use deserialize_untagged_verbose_error::{DeserializeUntaggedVerboseError, TryFromEnum};

    #[derive(TryFromEnum, DeserializeUntaggedVerboseError)]
    #[try_from_enum(target = "WindingToothCoil", error = "stem_primitives::Error")]
    pub(super) enum WindingToothCoilVariants {
        #[try_from_enum(convert_with = "check")]
        NewFull(NewFull),
        NewWindingTableMethod(NewWindingTableMethod),
        NewMinimal(NewMinimal),
    }

    fn check(alias: NewFull) -> stem_primitives::Result<WindingToothCoil> {
        return WindingToothCoil::from(alias).check();
    }
}
