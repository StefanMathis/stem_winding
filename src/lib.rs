pub mod coils;
pub mod error;
pub mod iterators;

#[cfg(feature = "stem_core")]
pub mod core_support;

// pub mod drawing;
pub mod fec_spectral_analysis;
pub mod winding;
pub mod winding_table;

pub use stem_coil_layout;
pub use stem_wire::{self, stem_material};

#[cfg(feature = "stem_core")]
pub use stem_core::{self, stem_magnet, stem_slot};

#[cfg(feature = "cairo")]
pub mod draw;

pub mod prelude {
    /*!
    This module reexports the core, air gap and flux barrier types defined in
    this crate as well as the [`Magnets`] and [`WindingZones`] iterators. In
    addition, it reexports the prelude modules of the [`stem_core`] and
    [`stem_wire`] dependencies (and therefore also [`stem_material::prelude`] [`stem_slot::prelude`] [`stem_magnet::prelude`]).
     */

    // Reexporting
    pub use crate::coils::*;
    pub use crate::winding::*;
    pub use crate::winding::{Connection, Winding};
    pub use crate::winding_table::*;

    #[cfg(feature = "cairo")]
    pub use crate::draw::{
        EndWindingLayouter, ZoneArrowConfig, ZoneBackgroundColor, ZoneCenterConfig, ZoneConfig,
        get_phase_color,
    };

    #[doc(hidden)]
    pub use stem_wire;

    #[doc(hidden)]
    pub use stem_wire::prelude::*;

    // Integrate these docs into stem_winding
    pub use stem_coil_layout;
    pub use stem_coil_layout::*;

    #[cfg(feature = "stem_core")]
    pub use crate::core_support::{FromWinding, Overrides};

    #[doc(hidden)]
    #[cfg(feature = "stem_core")]
    pub use stem_core;

    #[doc(hidden)]
    #[cfg(feature = "stem_core")]
    pub use stem_core::prelude::*;

    #[doc(hidden)]
    #[cfg(feature = "stem_core")]
    pub use stem_core::stem_magnet;

    #[doc(hidden)]
    #[cfg(feature = "stem_core")]
    pub use stem_core::stem_slot;

    #[doc(hidden)]
    pub use stem_material;

    #[doc(hidden)]
    pub use si::inductance::{henry, kilohenry, megahenry, microhenry, millihenry, nanohenry};
}
