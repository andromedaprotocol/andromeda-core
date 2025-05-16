pub mod contract;
pub mod osmosis;
pub mod state;

#[cfg(not(target_arch = "wasm32"))]
mod interface;

#[cfg(not(target_arch = "wasm32"))]
pub use crate::interface::SocketOsmosisContract;
