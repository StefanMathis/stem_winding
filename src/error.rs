/*!
This module contains the [`Error`] enum, which represents the different ways
building a winding can fail due to invalid input data. The
[`Error::Other`] variant supports arbitrary errors resulting from user-created
core types.

Specifically for winding table construction, the
[`WindingTableConstructionError`] covers any errors resulting from the
construction of a [`WindingTable`](crate::winding_table::WindingTable).
*/

use std::num::NonZeroU16;

use compare_variables::Comparison;
use keyring_map::InsertionError;
use stem_types::Zone;
use stem_wire::prelude::stem_material::si::Length;

use crate::{coils::Coil, winding_table::WindingTableConstructionError};

/// An enum representing errors resulting from attempting to build / change a
/// [`Winding`](crate::winding::Winding)
#[derive(Debug)]
pub enum Error {
    /// A given physical [`Length`] is not within its allowed value range.
    InvalidLength(Comparison<Length>),
    /// A given [`usize`] is not within its allowed value range.
    InvalidUsize(Comparison<usize>),
    /// A given [`u16`] is not within its allowed value range.
    InvalidU16(Comparison<u16>),
    /// A given [`f64`] is not within its allowed value range.
    InvalidF64(Comparison<f64>),
    /// Insertion of a coil failed because at least one of its zones is already
    /// occupied by another coil. See the wrapped [`InsertionError`] for
    /// details.
    CoilInsertionFailed(InsertionError<(Vec<Zone>, Coil)>),
    /// If a [`Coil`] requires specifying multiple [`Zone`]s it occupies in a
    /// [`WindingTable`](crate::winding_table::WindingTable), each zone must be
    /// unique. If they aren't, this error variant is created which holds the
    /// "overbooked" zone in question.
    EqualCoilZones(Zone),
    /// Creating a [`WindingTable`](crate::winding_table::WindingTable) failed
    /// due to the wrapped [`WindingTableConstructionError`]
    WindingTableConstructionError(WindingTableConstructionError),
    /// The number of parallel paths must be a divisor of the maximum number of
    /// parallel paths. For symmetric windings, see
    /// [`max_parallel_paths_symmetric`](crate::winding::max_parallel_paths_symmetric)
    InvalidNumberParallelPaths {
        /// The maximum possible number of parallel paths for the given winding
        /// topology.
        maximum: NonZeroU16,
        /// The specified number of parallel paths which is not a divisor of the
        /// maximum.
        given: NonZeroU16,
    },
    /// Fallback variant for arbitrary other errors (e.g., from custom
    /// [`Winding`](crate::winding::Winding) implementations).
    Other(Box<dyn std::error::Error>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidLength(comparison) => comparison.fmt(f),
            Error::InvalidUsize(comparison) => comparison.fmt(f),
            Error::InvalidU16(comparison) => comparison.fmt(f),
            Error::InvalidF64(comparison) => comparison.fmt(f),
            Error::CoilInsertionFailed(insertion_error) => insertion_error.fmt(f),
            Error::EqualCoilZones(zone) => {
                write!(f, "coil tries to occupy zone {} more than once", zone)
            }
            Error::WindingTableConstructionError(winding_table_construction_error) => {
                winding_table_construction_error.fmt(f)
            }
            Error::InvalidNumberParallelPaths { maximum, given } => write!(
                f,
                "given number of parallel paths {} is not a divisor of the maximum possible number {}",
                given.get(),
                maximum.get()
            ),
            Error::Other(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl From<InsertionError<(Vec<Zone>, Coil)>> for Error {
    fn from(value: InsertionError<(Vec<Zone>, Coil)>) -> Self {
        Error::CoilInsertionFailed(value)
    }
}

impl From<Comparison<Length>> for Error {
    fn from(value: Comparison<Length>) -> Self {
        Error::InvalidLength(value)
    }
}

impl From<Comparison<u16>> for Error {
    fn from(value: Comparison<u16>) -> Self {
        Error::InvalidU16(value)
    }
}

impl From<Comparison<usize>> for Error {
    fn from(value: Comparison<usize>) -> Self {
        Error::InvalidUsize(value)
    }
}

impl From<Comparison<f64>> for Error {
    fn from(value: Comparison<f64>) -> Self {
        Error::InvalidF64(value)
    }
}

impl From<WindingTableConstructionError> for Error {
    fn from(value: WindingTableConstructionError) -> Self {
        Error::WindingTableConstructionError(value)
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(value: std::convert::Infallible) -> Self {
        match value {}
    }
}
