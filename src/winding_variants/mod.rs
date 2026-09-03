pub mod coil_assembly;
pub mod winding_cage;
pub mod winding_distributed;
pub mod winding_distributed_tooth_coil;
pub mod winding_quadruple_layer_tooth_coil;
pub mod winding_tooth_coil;

pub use coil_assembly::CoilAssembly;
pub use winding_cage::SquirrelCageWinding;
pub use winding_distributed::WindingDistributed;
pub use winding_distributed_tooth_coil::WindingDistributedToothCoil;
pub use winding_quadruple_layer_tooth_coil::WindingQuadrupleLayerToothCoil;
pub use winding_tooth_coil::WindingToothCoil;
