pub mod contract;
pub mod execute_redeem;
pub mod execute_sale;
#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub mod mock;

mod state;

#[cfg(test)]
mod testing;

#[cfg(not(target_arch = "wasm32"))]
mod interface;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::interface::Cw20ExchangeContract;
