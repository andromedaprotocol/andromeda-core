pub mod addresses;
pub mod messages;
pub mod recipient;

pub const ADO_DB_KEY: &str = "adodb";
pub const VFS_KEY: &str = "vfs";

pub use addresses::AndrAddr;
pub use recipient::Recipient;
