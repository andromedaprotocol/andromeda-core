pub mod contract;
#[cfg(not(target_arch = "wasm32"))]
pub mod mock;
pub mod state;
