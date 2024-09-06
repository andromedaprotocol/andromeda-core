#[cfg(not(target_arch = "wasm32"))]
pub mod faucet;

#[cfg(not(target_arch = "wasm32"))]
pub mod interface_macro;

#[cfg(not(target_arch = "wasm32"))]
pub mod mock;

#[cfg(not(target_arch = "wasm32"))]
pub mod kernel;

#[cfg(not(target_arch = "wasm32"))]
pub mod adodb;

#[cfg(not(target_arch = "wasm32"))]
pub mod vfs;

#[cfg(not(target_arch = "wasm32"))]
pub mod economics;

#[cfg(not(target_arch = "wasm32"))]
pub mod chains;
