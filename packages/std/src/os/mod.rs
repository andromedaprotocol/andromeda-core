pub mod adodb;
pub mod aos_querier;
pub mod economics;
pub mod ibc_registry;
pub mod kernel;
pub mod vfs;

// IBC transfer port
pub const TRANSFER_PORT: &str = "transfer";
pub const IBC_VERSION: &str = "andr-kernel-1";

#[cfg(test)]
mod tests;
