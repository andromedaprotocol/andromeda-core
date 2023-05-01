pub mod app;
mod auth;
mod execute;
#[cfg(feature = "instantiate")]
mod instantiate;
// #[cfg(test)]
// mod mock_querier;
#[cfg(feature = "modules")]
pub mod modules;

//TODO: Redo this feature
// #[cfg(feature = "primitive")]
// pub mod primitive;

mod query;
pub mod state;
#[cfg(feature = "withdraw")]
pub mod withdraw;

pub use crate::ado_contract::state::ADOContract;
