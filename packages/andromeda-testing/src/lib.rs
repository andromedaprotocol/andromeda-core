mod adodb;
mod economics;
mod kernel;
mod vfs;

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
pub use mock_contract::MockContract;
pub use vfs::MockVFS;
