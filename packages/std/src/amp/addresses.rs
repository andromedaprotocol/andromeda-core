use std::fmt::{Display, Formatter, Result as FMTResult};

use crate::ado_contract::ADOContract;
use crate::error::ContractError;
use cosmwasm_std::{Addr, Deps};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An address that can be used within the Andromeda ecosystem.
/// Inspired by the cosmwasm-std `Addr` type. https://github.com/CosmWasm/cosmwasm/blob/2a1c698520a1aacedfe3f4803b0d7d653892217a/packages/std/src/addresses.rs#L33
///
/// This address can be one of two things:
/// 1. A valid human readable address e.g. `cosmos1...`
/// 2. A valid Andromeda VFS path e.g. `/home/user/app/component`
///
/// VFS paths can be local in the case of an app and can be done by referencing `./component`
///
/// This struct allows for ease of access in validating and resolving a valid Andromeda ecosystem address.
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
                if self.0.starts_with("./") {
                    Ok(Addr::unchecked(&self.0))
                } else {
                    Ok(Addr::unchecked(""))
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
