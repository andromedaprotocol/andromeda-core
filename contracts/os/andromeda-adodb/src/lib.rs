pub mod contract;

pub mod execute;
#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub mod mock;
pub mod query;
mod state;

#[cfg(test)]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
mod interface;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::interface::ADODBContract;
