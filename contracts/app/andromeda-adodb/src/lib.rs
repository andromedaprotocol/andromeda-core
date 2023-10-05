pub mod contract;
#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub mod mock;
mod state;

#[cfg(test)]
mod testing;
