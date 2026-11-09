/*!
[`Winding`]: crate::winding::Winding
[`Winding::zone_drawables`]: crate::winding::Winding::zone_drawables
[`Winding::coil_drawables`]: crate::winding::Winding::coil_drawables
[`SquirrelCageWinding`]: crate::winding::squirrel_cage::SquirrelCageWinding
[`DistributedWinding`]: crate::winding::distributed::DistributedWinding
[`DistributedToothCoilWinding`]: crate::winding::distributed_tooth_coil::DistributedToothCoilWinding
[`ToothCoilWinding`]: crate::winding::tooth_coil::ToothCoilWinding
[`QuadrupleLayerToothCoilWinding`]: crate::winding::quadruple_layer_tooth_coil::QuadrupleLayerToothCoilWinding
[`CoilAssembly`]: crate::winding::coil_assembly::CoilAssembly
[`Coil`]: crate::coils::Coil
[`WindingTable`]: crate::winding_table::WindingTable

[stem_core]: https://crates.io/crates/stem_core

Winding definition for stem – a Simulation Toolbox for Electric Motors. The
[stem book](https://stefanmathis.github.io/stem_book/) provides a comprehensive
guide to the framework, while this crate's documentation describes its API.
 */
#![cfg_attr(feature = "doc-images",
cfg_attr(all(),
doc = ::embed_doc_image::embed_image!("phd_stator.svg", "docs/img/phd_stator.svg"),
doc = ::embed_doc_image::embed_image!("table_tooth_coil_winding.svg", "docs/img/table_tooth_coil_winding.svg"),
doc = ::embed_doc_image::embed_image!("coils_tooth_coil_winding.svg", "docs/img/coils_tooth_coil_winding.svg"),
))]
#![cfg_attr(
    not(feature = "doc-images"),
    doc = "**Doc images not enabled**. Compile docs with `cargo doc --features 'doc-images'` and Rust version >= 1.54."
)]
#![doc = include_str!("../docs/main.md")]
#![deny(missing_docs)]

pub mod coils;
pub mod error;

#[cfg(feature = "stem_core")]
pub mod core_support;

pub mod winding;
pub mod winding_table;

pub use stem_types;
pub use stem_wire::{self, stem_material, stem_material::var_quantity};

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
    pub use crate::winding::{Connection, Winding};

    pub use crate::winding::coil_assembly::*;
    pub use crate::winding::distributed::*;
    pub use crate::winding::distributed_tooth_coil::*;
    pub use crate::winding::quadruple_layer_tooth_coil::{
        QuadrupleLayerToothCoilBuilder, QuadrupleLayerToothCoilMinimalBuilder,
        QuadrupleLayerToothCoilWinding,
    };
    pub use crate::winding::squirrel_cage::{
        SquirrelCageBuilder, SquirrelCageMinimalBuilder, SquirrelCageWinding,
    };
    pub use crate::winding::tooth_coil::*;

    pub use crate::winding_table::*;

    #[cfg(feature = "cairo")]
    pub use crate::draw::{
        CoilDrawablesConfig, DrawableKind, EndWindingStyle, ZoneArrowConfig, ZoneBackgroundColor,
        ZoneCenterConfig, ZoneDrawablesConfig, phase_color,
    };

    #[doc(hidden)]
    pub use stem_wire;

    #[doc(hidden)]
    pub use stem_wire::prelude::*;

    // Integrate these docs into stem_winding
    pub use stem_types;
    pub use stem_types::*;

    #[cfg(feature = "stem_core")]
    pub use crate::core_support::*;

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
    pub use stem_material::var_quantity;
}
