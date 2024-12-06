pub mod contract;
pub mod state;

#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub mod mock;

#[cfg(test)]
pub mod testing;

mod execute;
mod query;
mod reply;

#[cfg(not(target_arch = "wasm32"))]
mod interface;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::interface::AppContract;
