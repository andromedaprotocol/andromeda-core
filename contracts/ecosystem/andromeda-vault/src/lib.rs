pub mod contract;
#[cfg(test)]
pub mod testing;

#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub mod mock;
