use std::fmt::{Display, Formatter, Result as FMTResult};

use crate::error::ContractError;
use crate::{ado_contract::ADOContract, os::vfs::vfs_resolve_path};
use cosmwasm_std::{Addr, Api, Deps};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An address that can be used within the Andromeda ecosystem.
/// Inspired by the cosmwasm-std `Addr` type. https://github.com/CosmWasm/cosmwasm/blob/2a1c698520a1aacedfe3f4803b0d7d653892217a/packages/std/src/addresses.rs#L33
///
/// This address can be one of two things:
/// 1. A valid human readable address e.g. `cosmos1...`
/// 2. A valid Andromeda VFS path e.g. `/home/user/app/component`
///
/// VFS paths can be local in the case of an app and can be done by referencing `./component` they can also contain protocols for cross chain communication. A VFS path is usually structured as so:
///
/// `<protocol>://<chain (required if ibc used)>/<path>` or `ibc://cosmoshub-4/user/app/component`
#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema,
)]
pub struct AndrAddr(String);

impl AndrAddr {
    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    #[inline]
    pub fn into_string(self) -> String {
        self.0
    }

    #[inline]
    pub fn from_string(addr: impl Into<String>) -> AndrAddr {
        AndrAddr(addr.into())
    }

    /// Retrieves the raw address represented by the AndrAddr.
    ///
    /// If the address is a valid human readable address then that is returned, otherwise it is assumed to be a Andromeda VFS path and is resolved accordingly.
    ///
    /// If the address is assumed to be a VFS path and no VFS contract address is provided then an appropriate error is returned.
    pub fn get_raw_address(&self, deps: &Deps) -> Result<Addr, ContractError> {
        match deps.api.addr_validate(&self.0) {
            // Assume to be a valid address
            Ok(addr) => Ok(addr),
            // Otherwise assume to be VFS path
            Err(..) => {
                let contract = ADOContract::default();
                if self.0.starts_with("./") {
                    Ok(contract.get_app_component_address(deps.storage, &deps.querier, &self.0))
                } else {
                    let vfs_addr = contract.get_vfs_address(deps.storage, &deps.querier)?;
                    vfs_resolve_path(&self.0, vfs_addr, &deps.querier)
                }
            }
        }
    }

    /// Retrieves the raw address represented by the AndrAddr from the given VFS contract.
    ///     
    /// If the address is a valid human readable address then that is returned, otherwise it is assumed to be a Andromeda VFS path and is resolved accordingly.
    ///
    /// If the address is assumed to be a VFS path and no VFS contract address is provided then an appropriate error is returned.
    pub fn get_raw_address_from_vfs(
        &self,
        deps: &Deps,
        vfs_contract: String,
    ) -> Result<Addr, ContractError> {
        match deps.api.addr_validate(&self.0) {
            // Assume to be a valid address
            Ok(addr) => Ok(addr),
            // Otherwise assume to be VFS path
            Err(..) => {
                let vfs_addr = Addr::unchecked(vfs_contract);
                vfs_resolve_path(&self.0, vfs_addr, &deps.querier)
            }
        }
    }

    /// Whether the provided address is local to the app
    pub fn is_local_path(&self) -> bool {
        self.0.starts_with("./")
    }

    /// Whether the provided address is a VFS path
    pub fn is_vfs_path(&self) -> bool {
        self.is_local_path()
            || self.0.starts_with("/")
            || self.0.split("://").count() > 1
            || self.0.split("/").count() > 1
    }

    /// Whether the provided address is a valid human readable address
    pub fn is_addr(&self, api: &dyn Api) -> bool {
        !self.is_vfs_path() && api.addr_validate(&self.0).is_ok()
    }

    /// Gets the chain for a given AndrAddr if it exists
    pub fn get_chain(&self) -> Option<&str> {
        match self.get_protocol() {
            None => None,
            Some(..) => {
                let start = self.0.find("://").unwrap() + 3;
                let end = self.0[start..].find('/').unwrap_or(self.0[start..].len());
                Some(&self.0[start..start + end])
            }
        }
    }

    /// Gets the protocol for a given AndrAddr if it exists
    pub fn get_protocol(&self) -> Option<&str> {
        if !self.is_vfs_path() {
            None
        } else {
            let mut split = self.0.split("://");
            if split.clone().count() == 1 {
                None
            } else {
                Some(split.next().unwrap())
            }
        }
    }

    /// Gets the raw path for a given AndrAddr by stripping away any protocols or chain declarations.
    ///
    /// E.g. `ibc://cosmoshub-4/user/app/component` would return `/user/app/component`
    ///
    /// Returns the human readable address if the address is not a VFS path.
    pub fn get_raw_path(&self) -> &str {
        if !self.is_vfs_path() {
            self.0.as_str()
        } else {
            match self.get_protocol() {
                None => self.0.as_str(),
                Some(..) => {
                    let start = self.0.find("://").unwrap() + 3;
                    let end = self.0[start..].find('/').unwrap_or(self.0[start..].len());
                    &self.0[start + end..]
                }
            }
        }
    }
}

impl Display for AndrAddr {
    fn fmt(&self, f: &mut Formatter) -> FMTResult {
        write!(f, "{}", &self.0)
    }
}

impl AsRef<str> for AndrAddr {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<&str> for AndrAddr {
    fn eq(&self, rhs: &&str) -> bool {
        self.0 == *rhs
    }
}

impl PartialEq<AndrAddr> for &str {
    fn eq(&self, rhs: &AndrAddr) -> bool {
        *self == rhs.0
    }
}

impl PartialEq<String> for AndrAddr {
    fn eq(&self, rhs: &String) -> bool {
        &self.0 == rhs
    }
}

impl PartialEq<AndrAddr> for String {
    fn eq(&self, rhs: &AndrAddr) -> bool {
        self == &rhs.0
    }
}

impl From<AndrAddr> for String {
    fn from(addr: AndrAddr) -> Self {
        addr.0
    }
}

impl From<&AndrAddr> for String {
    fn from(addr: &AndrAddr) -> Self {
        addr.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::mock_dependencies;

    use super::*;

    #[test]
    fn test_is_vfs() {
        let deps = mock_dependencies();
        let addr = AndrAddr("/home/user/app/component".to_string());
        assert!(addr.is_vfs_path());

        let addr = AndrAddr("ibc://home/user/app/component".to_string());
        assert!(addr.is_vfs_path());
        assert!(!addr.is_addr(&deps.api));

        let addr = AndrAddr("cosmos1...".to_string());
        assert!(!addr.is_vfs_path());
    }

    #[test]
    fn test_is_addr() {
        let deps = mock_dependencies();
        let addr = AndrAddr("cosmos1...".to_string());
        assert!(addr.is_addr(&deps.api));
        assert!(!addr.is_vfs_path());
    }

    #[test]
    fn test_is_local_path() {
        let deps = mock_dependencies();
        let addr = AndrAddr("./component".to_string());
        assert!(addr.is_local_path());
        assert!(addr.is_vfs_path());
        assert!(!addr.is_addr(&deps.api));
    }

    #[test]
    fn test_get_protocol() {
        let addr = AndrAddr("cosmos1...".to_string());
        assert!(addr.get_protocol().is_none());

        let addr = AndrAddr("ibc://chain/user/app/component".to_string());
        assert_eq!(addr.get_protocol().unwrap(), "ibc");
    }

    #[test]
    fn test_get_chain() {
        let addr = AndrAddr("cosmos1...".to_string());
        assert!(addr.get_chain().is_none());

        let addr = AndrAddr("ibc://chain/user/app/component".to_string());
        assert_eq!(addr.get_chain().unwrap(), "chain");

        let addr = AndrAddr("/chain/user/app/component".to_string());
        assert!(addr.get_chain().is_none());
    }

    #[test]
    fn test_get_raw_path() {
        let addr = AndrAddr("cosmos1...".to_string());
        assert_eq!(addr.get_raw_path(), "cosmos1...");

        let addr = AndrAddr("ibc://chain/user/app/component".to_string());
        assert_eq!(addr.get_raw_path(), "/user/app/component");

        let addr = AndrAddr("/chain/user/app/component".to_string());
        assert_eq!(addr.get_raw_path(), "/chain/user/app/component");
    }
}
