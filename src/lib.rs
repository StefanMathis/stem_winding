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

pub mod prelude {
    /*!
    This module reexports the core, air gap and flux barrier types defined in
    this crate as well as the [`Magnets`] and [`WindingZones`] iterators. In
    addition, it reexports the prelude modules of the [`stem_core`] and
    [`stem_wire`] dependencies (and therefore also [`stem_material::prelude`] [`stem_slot::prelude`] [`stem_magnet::prelude`]).
     */

    // Reexporting
    pub use crate::coils::*;
    pub use crate::winding::Winding;
    pub use crate::winding::*;
    pub use crate::winding_table::*;
    // pub use core_integration::*;
    //
    // pub use drawing::*;
    // pub use fec_spectral_analysis::*;
    // pub use is_winding::*;
    // pub use iterators::*;
    // pub use slot::CoilLayout;
    // pub use uom;
    // pub use winding_variants::*;
    // pub use wire;
    // pub use zones::*;

    #[doc(hidden)]
    pub use stem_wire;

    #[doc(hidden)]
    pub use stem_wire::prelude::*;

    // Integrate these docs into stem_winding
    pub use stem_coil_layout;
    pub use stem_coil_layout::*;

    #[doc(hidden)]
    #[cfg(feature = "stem_core")]
    pub use stem_core;

    #[doc(hidden)]
    #[cfg(feature = "stem_core")]
    pub use stem_core::stem_magnet;

    #[doc(hidden)]
    #[cfg(feature = "stem_core")]
    pub use stem_core::stem_slot;

    #[doc(hidden)]
    pub use stem_material;

    // Prevent rustdoc from documenting
    #[doc(hidden)]
    #[cfg(feature = "stem_core")]
    pub use stem_slot::prelude::*;
}
