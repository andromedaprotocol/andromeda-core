pub mod contract;
pub mod state;

#[cfg(not(target_arch = "wasm32"))]
pub mod mock;
