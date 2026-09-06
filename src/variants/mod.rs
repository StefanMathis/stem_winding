pub mod coil_assembly;
pub mod distributed;
pub mod distributed_tooth_coil;
pub mod quadruple_layer_tooth_coil;
pub mod squirrel_cage;
pub mod tooth_coil;

pub use coil_assembly::CoilAssembly;
pub use distributed::DistributedWinding;
pub use distributed_tooth_coil::DistributedToothCoilWinding;
pub use quadruple_layer_tooth_coil::QuadrupleLayerToothCoilWinding;
pub use squirrel_cage::SquirrelCageWinding;
pub use tooth_coil::ToothCoilWinding;
