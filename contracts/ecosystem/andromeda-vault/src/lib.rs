pub mod contract;
#[cfg(test)]
pub mod testing;

#[cfg(not(target_arch = "wasm32"))]
pub mod mock;
