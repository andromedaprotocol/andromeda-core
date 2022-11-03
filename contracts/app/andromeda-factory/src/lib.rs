pub mod contract;
#[cfg(not(target_arch = "wasm32"))]
pub mod mock;
mod reply;
mod state;

#[cfg(test)]
mod testing;
