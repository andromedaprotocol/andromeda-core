use crate::{
    ado_base::{
        ado_type::TypeResponse, kernel_address::KernelAddressResponse,
        ownership::ContractOwnerResponse, version::VersionResponse,
    },
    amp::AndrAddr,
    error::ContractError,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Addr, Api, QuerierWrapper};
use regex::Regex;

pub const COMPONENT_NAME_REGEX: &str = r"^[A-Za-z0-9\.\-_]{1,40}$";
pub const USERNAME_REGEX: &str = r"^[a-z0-9]{1, 40}$";
pub const PATH_REGEX: &str = r"^((([A-Za-z0-9]+://)?([A-Za-z0-9\.\-_]{1,40}/)?((home|lib))/)|(~(/)?)|(\./))([A-Za-z0-9\.\-]{1,40}(/)?)+$";
pub fn convert_component_name(path: String) -> String {
    path.replace(' ', "_")
}

pub fn validate_component_name(path: String) -> Result<bool, ContractError> {
    ensure!(
        path.chars().any(|c| c.is_alphanumeric()),
        ContractError::InvalidPathname {
            error: Some("Pathname must contain at least one alphanumeric character".to_string())
        }
    );
    let re = Regex::new(COMPONENT_NAME_REGEX).unwrap();

    ensure!(
        re.is_match(&path),
        ContractError::InvalidPathname {
            error: Some("Pathname includes an invalid character".to_string())
        }
    );

    Ok(true)
}

pub fn validate_username(username: String) -> Result<bool, ContractError> {
    ensure!(
        !username.is_empty(),
        ContractError::InvalidUsername {
            error: Some("Username cannot be empty.".to_string())
        }
    );
    let re = Regex::new(USERNAME_REGEX).unwrap();
    ensure!(
        re.is_match(&username),
        ContractError::InvalidPathname {
            error: Some(
                "Username contains invalid characters. All characters must be alphanumeric."
                    .to_string()
            )
        }
    );
    Ok(true)
}

pub fn validate_path_name(api: &dyn Api, path: String) -> Result<bool, ContractError> {
    let re = Regex::new(PATH_REGEX).unwrap();

    let is_path_reference = path.contains('/');
    let starts_with_tilde = path.starts_with('~');

    // Path is of the form ~/home/...
    if starts_with_tilde && is_path_reference {
        ensure!(
            re.is_match(&path),
            ContractError::InvalidPathname {
                error: Some("Pathname includes an invalid character".to_string())
            }
        );
        ensure!(
            path.chars().next().unwrap().is_alphanumeric()
                || path.starts_with('/')
                || path.starts_with('~'),
            ContractError::InvalidPathname {
                error: Some(
                    "First character in a path must be either '/', '~' or alphanumeric".to_string()
                )
            }
        );

        let mut components = path.split('/');
        let first_component = components.nth(1).unwrap();
        ensure!(
           (first_component == "home" || first_component == "lib"),
            ContractError::InvalidPathname {
                error: Some(
                    "Paths beginning with ~ must directly reference a username: ~username or root directory: ~/home/username ~/lib/library"
                        .to_string()
                )
            }
        );

        return Ok(true);
    }

    // Path is of the form /home/... or /lib/...
    if is_path_reference && !starts_with_tilde {
        ensure!(
            re.is_match(&path),
            ContractError::InvalidPathname {
                error: Some("Pathname includes an invalid character".to_string())
            }
        );
        ensure!(
            path.chars().next().unwrap().is_alphanumeric() || path.starts_with('/'),
            ContractError::InvalidPathname {
                error: Some(
                    "First character in a path must be either '/', '~' or alphanumeric".to_string()
                )
            }
        );

        return Ok(true);
    }

    //Path is of the form ~username or ~address
    if !is_path_reference && starts_with_tilde {
        let username = &path[1..path.len()];
        let is_username = validate_username(username.to_string());
        let is_address = api.addr_validate(&path);

        ensure!(
            is_address.is_ok() || is_username.is_ok(),
            ContractError::InvalidPathname {
                error: Some(
                    "Provided address is neither a valid username nor a valid address".to_string()
                )
            }
        );

        return Ok(true);
    }

    if !is_path_reference {
        let is_address = api.addr_validate(&path);
        let is_username = validate_username(path);

        ensure!(
            is_address.is_ok() || is_username.is_ok(),
            ContractError::InvalidPathname {
                error: Some(
                    "Provided address is neither a valid username nor a valid address".to_string()
                )
            }
        );

        return Ok(true);
    }

    // Does not fit any forms
    Ok(false)
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
    AddPath {
        name: String,
        address: Addr,
        parent_address: Option<AndrAddr>,
    },
    AddSymlink {
        name: String,
        symlink: AndrAddr,
        parent_address: Option<AndrAddr>,
    },
    // Registers a child, currently only accessible by an App Contract
    AddChild {
        name: String,
        parent_address: AndrAddr,
    },
    RegisterUser {
        username: String,
        address: Option<Addr>,
    },
    // Restricted to VFS owner/Kernel
    RegisterLibrary {
        lib_name: String,
        lib_address: Addr,
    },
    RegisterUserCrossChain {
        chain: String,
        address: String,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    ResolvePath { path: AndrAddr },
    #[returns(Vec<PathDetails>)]
    SubDir {
        path: AndrAddr,
        min: Option<(Addr, String)>,
        max: Option<(Addr, String)>,
        limit: Option<u32>,
    },
    #[returns(Vec<String>)]
    Paths { addr: Addr },
    #[returns(String)]
    GetUsername { address: Addr },
    #[returns(String)]
    GetLibrary { address: Addr },
    #[returns(AndrAddr)]
    ResolveSymlink { path: AndrAddr },
    // Base queries
    #[returns(VersionResponse)]
    Version {},
    #[returns(TypeResponse)]
    Type {},
    #[returns(ContractOwnerResponse)]
    Owner {},
    #[returns(KernelAddressResponse)]
    KernelAddress {},
}

/// Queries the provided VFS contract address to resolve the given path
pub fn vfs_resolve_path(
    path: impl Into<String>,
    vfs_contract: impl Into<String>,
    querier: &QuerierWrapper,
) -> Result<Addr, ContractError> {
    let query = QueryMsg::ResolvePath {
        path: AndrAddr::from_string(path.into()),
    };
    let addr = querier.query_wasm_smart::<Addr>(vfs_contract, &query);
    match addr {
        Ok(addr) => Ok(addr),
        Err(_) => Err(ContractError::InvalidAddress {}),
    }
}

/// Queries the provided VFS contract address to resolve the given path
pub fn vfs_resolve_symlink(
    path: impl Into<String>,
    vfs_contract: impl Into<String>,
    querier: &QuerierWrapper,
) -> Result<AndrAddr, ContractError> {
    let query = QueryMsg::ResolveSymlink {
        path: AndrAddr::from_string(path.into()),
    };
    let addr = querier.query_wasm_smart::<AndrAddr>(vfs_contract, &query)?;
    Ok(addr)
}

#[cfg(test)]
mod test {
    use cosmwasm_std::testing::mock_dependencies;

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

        let dot_name = ".";
        let res = validate_component_name(dot_name.to_string());
        assert!(res.is_err());

        let dot_name = "..";
        let res = validate_component_name(dot_name.to_string());
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

    struct ValidatePathNameTestCase {
        name: &'static str,
        path: &'static str,
        should_err: bool,
    }

    #[test]
    fn test_validate_path_name() {
        let test_cases: Vec<ValidatePathNameTestCase> = vec![
            ValidatePathNameTestCase {
                name: "Simple app path",
                path: "./username/app",
                should_err: true,
            },
            ValidatePathNameTestCase {
                name: "Root path",
                path: "/",
                should_err: true,
            },
            ValidatePathNameTestCase {
                name: "Relative path with parent directory",
                path: "../username/app",
                should_err: true,
            },
            ValidatePathNameTestCase {
                name: "Tilde username reference",
                // Username must be short to circumvent it being mistaken as an address
                path: "~un",
                should_err: false,
            },
            ValidatePathNameTestCase {
                name: "Tilde address reference",
                path: "~cosmos1abcde",
                should_err: false,
            },
            ValidatePathNameTestCase {
                name: "Invalid tilde username reference",
                path: "~/un",
                should_err: true,
            },
            ValidatePathNameTestCase {
                name: "Absolute path with tilde",
                path: "~/home/username",
                should_err: false,
            },
            ValidatePathNameTestCase {
                name: "Complex valid path",
                path: "/home/username/dir1/../dir2/./file",
                should_err: true,
            },
            ValidatePathNameTestCase {
                name: "Path with invalid characters",
                path: "/home/username/dir1/|file",
                should_err: true,
            },
            ValidatePathNameTestCase {
                name: "Path with space",
                path: "/home/ username/dir1/file",
                should_err: true,
            },
            ValidatePathNameTestCase {
                name: "Empty path",
                path: "",
                should_err: true,
            },
            ValidatePathNameTestCase {
                name: "Path with only special characters",
                path: "///",
                should_err: true,
            },
            ValidatePathNameTestCase {
                name: "Path with only special characters and spaces",
                path: "/// /  /// //",
                should_err: true,
            },
            ValidatePathNameTestCase {
                name: "Valid ibc protocol path",
                path: "ibc://chain/home/username/dir1/file",
                should_err: false,
            },
            ValidatePathNameTestCase {
                name: "Invalid ibc protocol path",
                path: "ibc:///home/username/dir1/file",
                should_err: true,
            },
            ValidatePathNameTestCase {
                name: "Standard address",
                path: "cosmos1abcde",
                should_err: false,
            },
            ValidatePathNameTestCase {
                name: "Only periods",
                path: "/../../../..",
                should_err: true,
            },
        ];

        for test in test_cases {
            let deps = mock_dependencies();
            let res = validate_path_name(&deps.api, test.path.to_string());
            assert_eq!(res.is_err(), test.should_err, "Test case: {}", test.name);
        }
    }

    #[test]
    fn test_convert_component_name() {
        let pre_convert = "Some Component Name";
        let converted = convert_component_name(pre_convert.to_string());

        assert_eq!("Some_Component_Name", converted)
    }
}
