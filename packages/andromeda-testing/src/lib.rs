pub mod economics_msg;
// pub mod reply;
// pub mod testing;
pub mod adodb;
pub mod economics;
pub mod kernel;
pub mod vfs;

#[cfg(all(not(target_arch = "wasm32")))]
pub mod mock;
#[cfg(all(not(target_arch = "wasm32")))]
pub mod mock_contract;
#[cfg(all(not(target_arch = "wasm32")))]
pub use adodb::MockADODB;
#[cfg(all(not(target_arch = "wasm32")))]
pub use economics::MockEconomics;
#[cfg(all(not(target_arch = "wasm32")))]
pub use kernel::MockKernel;
#[cfg(all(not(target_arch = "wasm32")))]
pub use mock::MockAndromeda;
#[cfg(all(not(target_arch = "wasm32")))]
pub use mock_contract::MockADO;
#[cfg(all(not(target_arch = "wasm32")))]
pub use mock_contract::MockContract;
#[cfg(all(not(target_arch = "wasm32")))]
pub use vfs::MockVFS;
