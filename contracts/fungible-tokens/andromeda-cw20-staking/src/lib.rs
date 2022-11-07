pub mod allocated_rewards;
pub mod contract;
#[cfg(not(target_arch = "wasm32"))]
pub mod mock;

pub mod state;

#[cfg(test)]
mod testing;
