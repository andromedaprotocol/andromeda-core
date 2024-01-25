mod adodb;
mod economics;
mod kernel;
mod vfs;

#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub mod mock;
#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub mod mock_contract;
#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub use adodb::MockADODB;
#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub use economics::MockEconomics;
#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub use kernel::MockKernel;
#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub use mock::MockAndromeda;
#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub use mock_contract::MockADO;
#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub use mock_contract::MockContract;
#[cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
pub use vfs::MockVFS;
