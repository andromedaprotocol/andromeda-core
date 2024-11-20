#[cfg(not(target_arch = "wasm32"))]
pub mod faucet;

#[cfg(not(target_arch = "wasm32"))]
pub mod mock;
