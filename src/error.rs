use compare_variables::Comparison;
use keyring_map::InsertionError;
use stem_coil_layout::Zone;
use stem_wire::prelude::stem_material::si::Length;

use crate::coils::Coil;

#[derive(Debug)]
pub enum Error {
    /// A given physical [`Length`] is not within its allowed value range.
    InvalidLength(Comparison<Length>),
    /// A given [`usize`] is not within its allowed value range.
    InvalidUsize(Comparison<usize>),
    CoilInsertionFailed(InsertionError<(Vec<Zone>, Coil)>),
    /// If a [`Coil`] requires specifying multiple [`Zone`]s it occupies in a
    /// [`WindingTable`], those zones must not be equal. This error variant is
    /// returned if they are.
    EqualCoilZones(Zone),
    WindingTableCreationError(WindingTableCreationError),
}

impl From<InsertionError<(Vec<Zone>, Coil)>> for Error {
    fn from(value: InsertionError<(Vec<Zone>, Coil)>) -> Self {
        Error::CoilInsertionFailed(value)
    }
}

impl From<Comparison<Length>> for Error {
    fn from(value: Comparison<Length>) -> Self {
        return Error::InvalidLength(value);
    }
}

impl From<Comparison<usize>> for Error {
    fn from(value: Comparison<usize>) -> Self {
        return Error::InvalidUsize(value);
    }
}

impl From<WindingTableCreationError> for Error {
    fn from(value: WindingTableCreationError) -> Self {
        return Error::WindingTableCreationError(value);
    }
}

// impl From<Comparison<usize>> for Error {
//     fn from(value: Comparison<usize>) -> Self {
//         return Error::InvalidUsize(value);
//     }
// }

#[derive(Debug)]
pub enum WindingTableCreationError {
    SingleLayerOddSlotNumber,
    EmptyZone(Option<Zone>),
    InequalPositiveNegativeZones(u16),
    /// Only two or three phases
    TingleyInvalidNumberPhases,
    /// Only single layer
    CoilSideInvalidNumberLayers,
    CoilSideCouldNotDistribute,
    /// Number of phases must be greater than two, only works for odd phases
    AlgebraicAlgorithmInvalidNumberPhases,
    AlgebraicAlgorithmInvalidStep,
    /// Only odd phase numbers
    StarOfSlotsInvalidNumberPhases,
    /// Only single or double layer
    StarOfSlotsInvalidNumberLayers,
    /// Only single or double layer
    DistributionTableInvalidNumberLayers,
}
