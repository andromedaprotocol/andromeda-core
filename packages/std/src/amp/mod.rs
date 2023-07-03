pub mod addresses;
pub mod messages;
pub mod recipient;

pub const ADO_DB_KEY: &str = "adodb";
pub const VFS_KEY: &str = "vfs";
pub const OSMOSIS_ROUTER_KEY: &str = "osmosis_router";
pub const ECONOMICS_KEY: &str = "economics";

pub use addresses::AndrAddr;
pub use recipient::Recipient;
