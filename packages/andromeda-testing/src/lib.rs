pub mod economics_msg;
pub mod reply;
pub mod testing;

#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub mod mock;
#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub mod mock_contract;
