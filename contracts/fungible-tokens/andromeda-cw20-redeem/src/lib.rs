pub mod contract;
#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub mod mock;

mod state;

#[cfg(test)]
mod testing;

#[cfg(not(target_arch = "wasm32"))]
mod interface;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::interface::Cw20RedeemContract;
