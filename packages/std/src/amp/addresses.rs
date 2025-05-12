use std::fmt::{Display, Formatter, Result as FMTResult};

use crate::error::ContractError;
use crate::os::vfs::{vfs_resolve_symlink, PATH_REGEX, PROTOCOL_PATH_REGEX};
use crate::{ado_contract::ADOContract, os::vfs::vfs_resolve_path};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Deps, QuerierWrapper, Storage};
use lazy_static::lazy_static;

lazy_static! {
    static ref ANDR_ADDR_REGEX: String = format!(
        // Combine all valid regex for ANDR_ADDR schema validations
        "({re1})|({re2})|({re3})|({re4})",
        // Protocol regex
        re1 = PROTOCOL_PATH_REGEX,
        // Path regex
        re2 = PATH_REGEX,
        // Raw address
        re3 = r"^[a-z0-9]{2,}$",
        // Local path
        re4 = r"^\.(/[A-Za-z0-9.\-_]{2,40}?)*(/)?$",
    );
}

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
#[cw_serde]
pub struct AndrAddr(#[schemars(regex = "ANDR_ADDR_REGEX")] String);

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

    #[inline]
    pub fn to_lowercase(&self) -> AndrAddr {
        AndrAddr(self.0.to_lowercase())
    }

    /// Validates an `AndrAddr`, to be valid the given address must either be a human readable address or a valid VFS path.
    ///
    /// **The existence of the provided path is not validated.**
    ///
    /// **If you wish to validate the existence of the path you must use `get_raw_address`.**
    pub fn validate(&self, api: &dyn Api) -> Result<(), ContractError> {
        match self.is_vfs_path() || self.is_addr(api) {
            true => Ok(()),
            false => Err(ContractError::InvalidAddress {}),
        }
    }

    /// Retrieves the raw address represented by the AndrAddr.
    ///
    /// If the address is a valid human readable address then that is returned, otherwise it is assumed to be a Andromeda VFS path and is resolved accordingly.
    ///
    /// If the address is assumed to be a VFS path and no VFS contract address is provided then an appropriate error is returned.
    pub fn get_raw_address(&self, deps: &Deps) -> Result<Addr, ContractError> {
        if !self.is_vfs_path() {
            return Ok(deps.api.addr_validate(&self.0)?);
        }
        let contract = ADOContract::default();
        let vfs_contract = contract.get_vfs_address(deps.storage, &deps.querier)?;
        self.get_raw_address_from_vfs(deps, vfs_contract)
    }

    /// Retrieves the raw address represented by the AndrAddr from the given VFS contract.
    ///     
    /// If the address is a valid human readable address then that is returned, otherwise it is assumed to be a Andromeda VFS path and is resolved accordingly.
    ///
    /// If the address is assumed to be a VFS path and no VFS contract address is provided then an appropriate error is returned.
    pub fn get_raw_address_from_vfs(
        &self,
        deps: &Deps,
        vfs_contract: impl Into<String>,
    ) -> Result<Addr, ContractError> {
        match self.is_vfs_path() {
            false => Ok(deps.api.addr_validate(&self.0)?),
            true => {
                let vfs_contract: String = vfs_contract.into();
                // Convert local path to VFS path before querying
                let valid_vfs_path =
                    self.local_path_to_vfs_path(deps.storage, &deps.querier, vfs_contract.clone())?;
                let vfs_addr = Addr::unchecked(vfs_contract);
                match vfs_resolve_path(valid_vfs_path.clone(), vfs_addr, &deps.querier) {
                    Ok(addr) => Ok(addr),
                    Err(_) => {
                        // If the path is cross-chain then we return it as is
                        if valid_vfs_path.get_protocol().is_some() {
                            Ok(Addr::unchecked(valid_vfs_path.into_string()))
                        } else {
                            Err(ContractError::InvalidPathname {
                                error: Some(format!(
                                    "{:?} does not exist in the file system",
                                    valid_vfs_path.0
                                )),
                            })
                        }
                    }
                }
            }
        }
    }

    /// Converts a local path to a valid VFS path by replacing `./` with the app contract address
    pub fn local_path_to_vfs_path(
        &self,
        storage: &dyn Storage,
        querier: &QuerierWrapper,
        vfs_contract: impl Into<String>,
    ) -> Result<AndrAddr, ContractError> {
        match self.is_local_path() {
            true => {
                let app_contract = ADOContract::default().get_app_contract(storage)?;
                match app_contract {
                    None => Err(ContractError::AppContractNotSpecified {}),
                    Some(app_contract) => {
                        let replaced = AndrAddr(self.0.replace("./", &format!("~{app_contract}/")));
                        vfs_resolve_symlink(replaced, vfs_contract, querier)
                    }
                }
            }
            false => Ok(self.clone()),
        }
    }

    /// Whether the provided address is local to the app
    pub fn is_local_path(&self) -> bool {
        self.0.starts_with("./")
    }

    /// Whether the provided address is a VFS path
    pub fn is_vfs_path(&self) -> bool {
        self.is_local_path()
            || self.0.starts_with('/')
            || self.0.split("://").count() > 1
            || self.0.split('/').count() > 1
            || self.0.starts_with('~')
    }

    /// Whether the provided address is a valid human readable address
    pub fn is_addr(&self, api: &dyn Api) -> bool {
        api.addr_validate(&self.0).is_ok()
    }

    /// Gets the chain for a given AndrAddr if it exists
    ///
    /// E.g. `ibc://cosmoshub-4/user/app/component` would return `cosmoshub-4`
    ///
    /// A human readable address will always return `None`
    pub fn get_chain(&self) -> Option<&str> {
        match self.get_protocol() {
            None => None,
            Some(..) => {
                let start = self.0.find("://").unwrap() + 3;
                let end = self.0[start..]
                    .find('/')
                    .unwrap_or_else(|| self.0[start..].len());
                Some(&self.0[start..start + end])
            }
        }
    }

    /// Gets the protocol for a given AndrAddr if it exists
    ///
    /// E.g. `ibc://cosmoshub-4/user/app/component` would return `ibc`
    ///
    /// A human readable address will always return `None`
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
    /// E.g. `ibc://cosmoshub-4/cosmos1...` would return `cosmos1...`
    /// E.g. `ibc://chain/ibc://chain2/home/app/component` would return `ibc://chain2/home/app/component`
    ///
    /// Returns the human readable address if the address is not a VFS path.
    pub fn get_raw_path(&self) -> &str {
        if !self.is_vfs_path() {
            self.0.as_str()
        } else {
            match self.get_protocol() {
                None => self.0.as_str(),
                Some(..) => {
                    // Find the first "://" to skip the protocol part
                    let start = self.0.find("://").unwrap() + 3;

                    // Find the next '/' after the protocol+chain
                    if let Some(path_start) = self.0[start..].find('/') {
                        let path = &self.0[start + path_start + 1..];
                        // Check if the path starts with another protocol
                        if path.starts_with("ibc://") {
                            path
                        } else if path.contains('/') {
                            // If it contains '/', return the path with a leading '/'
                            &self.0[start + path_start..]
                        } else {
                            // If it doesn't contain '/', return it as is (e.g., OWNER)
                            path
                        }
                    } else {
                        // If there's no further '/', just return the remaining part
                        &self.0[start..]
                    }
                }
            }
        }
    }

    /// Gets the root directory for a given AndrAddr
    ///
    /// E.g. `/home/user/app/component` would return `home`
    ///
    /// Returns the human readable address if the address is not a VFS path or the local path if the address is a local reference
    pub fn get_root_dir(&self) -> &str {
        match self.is_vfs_path() {
            false => self.0.as_str(),
            true => match self.is_local_path() {
                true => self.0.as_str(),
                false => {
                    let raw_path = self.get_raw_path();
                    if raw_path.starts_with('~') {
                        return "home";
                    }
                    raw_path.split('/').nth(1).unwrap()
                }
            },
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

impl From<String> for AndrAddr {
    fn from(addr: String) -> Self {
        AndrAddr(addr)
    }
}

impl From<AndrAddr> for String {
    fn from(addr: AndrAddr) -> Self {
        addr.0
    }
}

impl From<Addr> for AndrAddr {
    fn from(addr: Addr) -> Self {
        AndrAddr(addr.to_string())
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
    use regex::Regex;
    pub const OWNER: &str = "cosmwasm1fsgzj6t7udv8zhf6zj32mkqhcjcpv52yph5qsdcl0qt94jgdckqs2g053y";

    use super::*;
    struct ValidateRegexTestCase {
        name: &'static str,
        input: &'static str,
        should_err: bool,
    }

    #[test]
    fn test_validate() {
        let deps = mock_dependencies();
        let addr = AndrAddr(OWNER.to_string());
        assert!(addr.validate(&deps.api).is_ok());

        let addr = AndrAddr("ibc://cosmoshub-4/home/user/app/component".to_string());
        assert!(addr.validate(&deps.api).is_ok());

        let addr = AndrAddr("/home/user/app/component".to_string());
        assert!(addr.validate(&deps.api).is_ok());

        let addr = AndrAddr("./user/app/component".to_string());
        assert!(addr.validate(&deps.api).is_ok());

        let addr = AndrAddr("1".to_string());
        assert!(addr.validate(&deps.api).is_err());
    }

    #[test]
    fn test_is_vfs() {
        let addr = AndrAddr("/home/user/app/component".to_string());
        assert!(addr.is_vfs_path());

        let addr = AndrAddr("./user/app/component".to_string());
        assert!(addr.is_vfs_path());

        let addr = AndrAddr("ibc://chain/home/user/app/component".to_string());
        assert!(addr.is_vfs_path());

        let addr = AndrAddr(OWNER.to_string());
        assert!(!addr.is_vfs_path());
    }

    #[test]
    fn test_is_addr() {
        let deps = mock_dependencies();
        let addr = AndrAddr(OWNER.to_string());
        assert!(addr.is_addr(&deps.api));
        assert!(!addr.is_vfs_path());
    }

    #[test]
    fn test_is_local_path() {
        let addr = AndrAddr("./component".to_string());
        assert!(addr.is_local_path());
        assert!(addr.is_vfs_path());
    }

    #[test]
    fn test_get_protocol() {
        let addr = AndrAddr(OWNER.to_string());
        assert!(addr.get_protocol().is_none());

        let addr = AndrAddr("ibc://chain/home/user/app/component".to_string());
        assert_eq!(addr.get_protocol().unwrap(), "ibc");
    }

    #[test]
    fn test_get_chain() {
        let addr = AndrAddr(OWNER.to_string());
        assert!(addr.get_chain().is_none());

        let addr = AndrAddr("ibc://chain/home/user/app/component".to_string());
        assert_eq!(addr.get_chain().unwrap(), "chain");

        let addr = AndrAddr("/home/user/app/component".to_string());
        assert!(addr.get_chain().is_none());
    }

    #[test]
    fn test_get_raw_path() {
        let addr = AndrAddr(OWNER.to_string());
        assert_eq!(addr.get_raw_path(), OWNER);

        let addr = AndrAddr("ibc://chain/home/app/component".to_string());
        assert_eq!(addr.get_raw_path(), "/home/app/component");

        let addr = AndrAddr(format!("ibc://chain/{}", OWNER));
        assert_eq!(addr.get_raw_path(), OWNER);

        let addr = AndrAddr("ibc://chain/ibc://chain2/home/app/component".to_string());
        assert_eq!(addr.get_raw_path(), "ibc://chain2/home/app/component");

        let addr = AndrAddr("/chain/home/app/component".to_string());
        assert_eq!(addr.get_raw_path(), "/chain/home/app/component");
    }

    #[test]
    fn test_get_root_dir() {
        let addr = AndrAddr("/home/user1".to_string());
        assert_eq!(addr.get_root_dir(), "home");

        let addr = AndrAddr("~user1".to_string());
        assert_eq!(addr.get_root_dir(), "home");

        let addr = AndrAddr("~/user1".to_string());
        assert_eq!(addr.get_root_dir(), "home");

        let addr = AndrAddr("ibc://chain/home/user1".to_string());
        assert_eq!(addr.get_root_dir(), "home");

        let addr = AndrAddr(OWNER.to_string());
        assert_eq!(addr.get_root_dir(), OWNER);

        let addr = AndrAddr("./home/user1".to_string());
        assert_eq!(addr.get_root_dir(), "./home/user1");
    }

    #[test]
    fn test_schemars_regex() {
        let test_cases: Vec<ValidateRegexTestCase> = vec![
            ValidateRegexTestCase {
                name: "Normal Path",
                input: "/home/user",
                should_err: false,
            },
            ValidateRegexTestCase {
                name: "Path with tilde",
                input: "~user/dir",
                should_err: false,
            },
            ValidateRegexTestCase {
                name: "Wrong path with tilde",
                input: "~/user/dir",
                should_err: true,
            },
            ValidateRegexTestCase {
                name: "Valid protocol",
                input: "ibc://chain/home/user/dir",
                should_err: false,
            },
            ValidateRegexTestCase {
                name: "Valid protocol with tilde",
                input: "ibc://chain/~user/dir",
                should_err: false,
            },
            ValidateRegexTestCase {
                name: "Valid Raw Address",
                input: "cosmos1234567",
                should_err: false,
            },
            ValidateRegexTestCase {
                name: "Valid Local",
                input: "./dir/file",
                should_err: false,
            },
            ValidateRegexTestCase {
                name: "Invalid Local",
                input: "../dir/file",
                should_err: true,
            },
        ];
        let re = Regex::new(&ANDR_ADDR_REGEX).unwrap();
        for test in test_cases {
            let res = re.is_match(test.input);
            assert_eq!(!res, test.should_err, "Test case: {}", test.name);
        }
    }
}
