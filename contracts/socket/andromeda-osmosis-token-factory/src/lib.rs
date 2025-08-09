pub mod contract;
pub mod state;

#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub mod mock;

#[cfg(not(target_arch = "wasm32"))]
mod interface;

#[cfg(not(target_arch = "wasm32"))]
pub use crate::interface::OsmosisTokenFactoryContract;
