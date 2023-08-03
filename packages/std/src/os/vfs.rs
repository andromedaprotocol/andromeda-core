use crate::error::ContractError;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Addr, QuerierWrapper};
use regex::Regex;

pub const COMPONENT_NAME_REGEX: &str = r"^[A-Za-z0-9\.\-_]{1,40}$";
pub const PATH_REGEX: &str = r"^([A-Za-z0-9]+://)?(/)?([A-Za-z0-9\.\-_]{1,40}(/)?)+$";

pub fn convert_component_name(path: String) -> String {
    path.replace(' ', "_")
}

pub fn validate_component_name(path: String) -> Result<bool, ContractError> {
    let re = Regex::new(COMPONENT_NAME_REGEX).unwrap();

    ensure!(
        re.is_match(&path),
        ContractError::InvalidPathname {
            error: Some("Pathname includes an invalid character".to_string())
        }
    );
    Ok(true)
}

pub fn validate_path_name(path: String) -> Result<bool, ContractError> {
    let re = Regex::new(PATH_REGEX).unwrap();

    ensure!(
        re.is_match(&path),
        ContractError::InvalidPathname {
            error: Some("Pathname includes an invalid character".to_string())
        }
    );
    Ok(true)
}

#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the Kernel contract on chain
    pub kernel_address: String,
    pub owner: Option<String>,
}

#[cw_serde]
pub struct PathDetails {
    name: String,
    address: Addr,
}

impl PathDetails {
    pub fn new(name: impl Into<String>, address: Addr) -> PathDetails {
        PathDetails {
            name: name.into(),
            address,
        }
    }
}

#[cw_serde]
pub enum ExecuteMsg {
    AddPath { name: String, address: Addr },
    AddParentPath { name: String, parent_address: Addr },
    RegisterUser { username: String },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    ResolvePath { path: String },
    #[returns(Vec<PathDetails>)]
    SubDir { path: String },
    #[returns(Vec<String>)]
    Paths { addr: Addr },
    #[returns(String)]
    GetUsername { address: Addr },
}

/// Queries the provided VFS contract address to resolve the given path
pub fn vfs_resolve_path(
    path: impl Into<String>,
    vfs_contract: impl Into<String>,
    querier: &QuerierWrapper,
) -> Result<Addr, ContractError> {
    let query = QueryMsg::ResolvePath { path: path.into() };
    let addr = querier.query_wasm_smart::<Addr>(vfs_contract, &query)?;
    Ok(addr)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_validate_component_name() {
        let valid_name = "component1";
        validate_component_name(valid_name.to_string()).unwrap();

        let valid_name = "component-1";
        validate_component_name(valid_name.to_string()).unwrap();

        let valid_name = "component_1";
        validate_component_name(valid_name.to_string()).unwrap();

        let valid_name = ".component-1";
        validate_component_name(valid_name.to_string()).unwrap();

        let empty_name = "";
        let res = validate_component_name(empty_name.to_string());
        assert!(res.is_err());

        let invalid_name = "/ /";
        let res = validate_component_name(invalid_name.to_string());
        assert!(res.is_err());

        let invalid_name = " ";
        let res = validate_component_name(invalid_name.to_string());
        assert!(res.is_err());

        let invalid_name =
            "somereallyreallyreallyreallyreallyreallyreallyreallyreallyreallyreallyreallylongname";
        let res = validate_component_name(invalid_name.to_string());
        assert!(res.is_err());
    }

    #[test]
    fn test_validate_path_name() {
        let valid_path = "/username";
        validate_path_name(valid_path.to_string()).unwrap();

        let valid_path = "username/dir1/file";
        validate_path_name(valid_path.to_string()).unwrap();

        let valid_path = "/username/dir1/file/";
        validate_path_name(valid_path.to_string()).unwrap();

        let valid_path = "vfs://home/username/dir1/file/";
        validate_path_name(valid_path.to_string()).unwrap();

        let valid_path = "vfs://chain/username/dir1/file/";
        validate_path_name(valid_path.to_string()).unwrap();

        let empty_path = "";
        let res = validate_path_name(empty_path.to_string());
        assert!(res.is_err());

        let invalid_path = "//// ///";
        let res = validate_path_name(invalid_path.to_string());
        assert!(res.is_err());

        let invalid_path = "vfs:/username/dir1/f!le";
        let res = validate_path_name(invalid_path.to_string());
        assert!(res.is_err())
    }

    #[test]
    fn test_convert_component_name() {
        let pre_convert = "Some Component Name";
        let converted = convert_component_name(pre_convert.to_string());

        assert_eq!("Some_Component_Name", converted)
    }
}
