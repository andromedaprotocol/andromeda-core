use andromeda_std::{amp::AndrAddr, error::ContractError};
use cosmwasm_std::{ensure, Addr, Api, StdError, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Map, MultiIndex};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct PathInfo {
    pub name: String,
    pub address: Addr,
    pub parent_address: Addr,
    pub symlink: Option<AndrAddr>,
}

pub struct PathIndices<'a> {
    /// PK: parent_address + component_name
    /// Secondary key: address
    pub address: MultiIndex<'a, Addr, PathInfo, (Addr, String)>,

    /// PK: parent_address + component_name
    /// Secondary key: parent_address
    pub parent: MultiIndex<'a, Addr, PathInfo, (Addr, String)>,
}

impl<'a> IndexList<PathInfo> for PathIndices<'a> {
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn cw_storage_plus::Index<PathInfo>> + '_> {
        let v: Vec<&dyn Index<PathInfo>> = vec![&self.address, &self.parent];
        Box::new(v.into_iter())
    }
}

pub fn paths<'a>() -> IndexedMap<'a, &'a (Addr, String), PathInfo, PathIndices<'a>> {
    let indexes = PathIndices {
        address: MultiIndex::new(|_pk: &[u8], r| r.address.clone(), "path", "path_index"),
        parent: MultiIndex::new(
            |_pk: &[u8], r| r.parent_address.clone(),
            "path",
            "parent_index",
        ),
    };
    IndexedMap::new("path", indexes)
}

pub const USERS: Map<&str, Addr> = Map::new("users");
pub const LIBRARIES: Map<&str, Addr> = Map::new("libraries");
pub const ADDRESS_USERNAME: Map<&str, String> = Map::new("address_username");
pub const ADDRESS_LIBRARY: Map<&str, String> = Map::new("address_library");

pub fn split_pathname(path: String) -> Vec<String> {
    path.split('/')
        .filter(|string| !string.is_empty())
        .map(|string| string.to_string())
        .collect::<Vec<String>>()
}

pub fn resolve_pathname(
    storage: &dyn Storage,
    api: &dyn Api,
    pathname: AndrAddr,
) -> Result<Addr, ContractError> {
    // As cross-chain queries are not currently possible we need to ensure the pathname being resolved is local
    ensure!(
        pathname.get_protocol().is_none(),
        ContractError::InvalidAddress {}
    );

    if pathname.is_vfs_path() {
        match pathname.get_root_dir() {
            "home" => resolve_home_path(storage, api, pathname),
            "lib" => resolve_lib_path(storage, api, pathname),
            &_ => Err(ContractError::InvalidAddress {}),
        }
    } else {
        Ok(api.addr_validate(pathname.as_str())?)
    }
}

fn resolve_home_path(
    storage: &dyn Storage,
    api: &dyn Api,
    pathname: AndrAddr,
) -> Result<Addr, ContractError> {
    let mut parts = split_pathname(pathname.to_string());

    let username_or_address = if parts[0].starts_with('~') && !parts[0].eq("~") {
        parts[0].remove(0);
        parts[0].as_str()
    } else {
        parts[1].as_str()
    };
    let user_address = match api.addr_validate(username_or_address) {
        Ok(addr) => addr,
        Err(_e) => USERS.load(storage, username_or_address)?,
    };

    let remaining_parts = parts
        .to_vec()
        .iter()
        .filter(|part| {
            !SKIP_PARTS.contains(&part.as_str()) && part.to_string() != *username_or_address
        })
        .map(|part| part.to_string())
        .collect::<Vec<String>>();

    resolve_path(storage, api, remaining_parts, user_address)
}

const SKIP_PARTS: [&str; 3] = ["home", "lib", "~"];

fn resolve_lib_path(
    storage: &dyn Storage,
    api: &dyn Api,
    pathname: AndrAddr,
) -> Result<Addr, ContractError> {
    let parts = split_pathname(pathname.to_string());

    let username_or_address = parts[1].as_str();

    let user_address = match api.addr_validate(username_or_address) {
        Ok(addr) => addr,
        Err(_e) => LIBRARIES.load(storage, username_or_address)?,
    };
    let remaining_parts = parts
        .to_vec()
        .iter()
        .filter(|part| {
            !SKIP_PARTS.contains(&part.as_str()) && part.to_string() != *username_or_address
        })
        .map(|part| part.to_string())
        .collect::<Vec<String>>();

    resolve_path(storage, api, remaining_parts, user_address)
}

const MAX_DEPTH: u8 = 50;

fn resolve_path(
    storage: &dyn Storage,
    api: &dyn Api,
    parts: Vec<String>,
    user_address: Addr,
) -> Result<Addr, ContractError> {
    let mut address = user_address;
    ensure!(
        parts.len() as u8 <= MAX_DEPTH,
        ContractError::InvalidAddress {}
    );
    for part in parts.iter() {
        let info = paths().load(storage, &(address, part.clone()))?;
        address = match info.symlink {
            Some(symlink) => resolve_pathname(storage, api, symlink)?,
            None => info.address,
        };
    }

    Ok(address)
}

pub fn get_subdir(
    storage: &dyn Storage,
    api: &dyn Api,
    pathname: AndrAddr,
) -> Result<Vec<PathInfo>, ContractError> {
    let address = resolve_pathname(storage, api, pathname)?;

    let subdirs = paths()
        .idx
        .parent
        .prefix(address)
        .range(storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|r| r.unwrap().1)
        .collect();

    Ok(subdirs)
}

pub fn get_paths(storage: &dyn Storage, addr: Addr) -> Result<Vec<String>, ContractError> {
    let mut resolved_paths: Vec<String> = vec![];
    let parent_dirs: Vec<PathInfo> = paths()
        .idx
        .address
        .prefix(addr.clone())
        .range(storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|r| r.unwrap().1)
        .collect();
    if parent_dirs.is_empty() {
        // Its a user address
        let username_or_address = ADDRESS_USERNAME
            .load(storage, addr.to_string().as_str())
            .unwrap_or(addr.to_string());
        resolved_paths.push(username_or_address)
    }
    for parent_dir in parent_dirs {
        let parent_paths = get_paths(storage, parent_dir.clone().parent_address)?;
        for parent_path in parent_paths {
            resolved_paths.push(parent_path + "/" + parent_dir.name.as_str());
        }
    }

    Ok(resolved_paths)
}

pub fn add_pathname(
    storage: &mut dyn Storage,
    parent_addr: Addr,
    name: String,
    address: Addr,
) -> Result<(), StdError> {
    paths().save(
        storage,
        &(parent_addr.clone(), name.clone()),
        &PathInfo {
            name,
            address,
            parent_address: parent_addr,
            symlink: None,
        },
    )
}

pub fn add_path_symlink(
    storage: &mut dyn Storage,
    parent_addr: Addr,
    name: String,
    symlink: AndrAddr,
) -> Result<(), StdError> {
    paths().save(
        storage,
        &(parent_addr.clone(), name.clone()),
        &PathInfo {
            name,
            address: Addr::unchecked("invalidaddress"),
            parent_address: parent_addr,
            symlink: Some(symlink),
        },
    )
}

pub fn resolve_symlink(
    storage: &dyn Storage,
    api: &dyn Api,
    path: AndrAddr,
) -> Result<AndrAddr, ContractError> {
    let mut parts = split_pathname(path.to_string());
    if !path.is_vfs_path() || path.get_protocol().is_some() || parts.len() <= 2 {
        return Ok(path);
    }
    let final_part = parts.pop().unwrap();
    let reconstructed_addr = parts.join("/");
    // Need to prepend a '/' unless the path starts with '~'
    let remaining_path = if reconstructed_addr.starts_with('~') {
        AndrAddr::from_string(reconstructed_addr)
    } else {
        AndrAddr::from_string(format!("/{reconstructed_addr}"))
    };
    let addr = resolve_pathname(storage, api, remaining_path)?;
    let info = paths().load(storage, &(addr, final_part))?;
    match info.symlink {
        Some(symlink) => Ok(symlink),
        None => Ok(path),
    }
}

#[cfg(test)]
mod test {
    use andromeda_std::os::vfs::validate_username;
    use cosmwasm_std::testing::mock_dependencies;

    use super::*;

    #[test]
    fn test_split_pathname() {
        let pathname = "//username/dir1/dir2/file";

        let res = split_pathname(pathname.to_string());
        let expected = vec!["username", "dir1", "dir2", "file"];

        assert_eq!(res, expected)
    }

    #[test]
    fn test_validate_username() {
        let valid_user = "username1980";
        validate_username(valid_user.to_string()).unwrap();

        let empty_user = "";
        let res = validate_username(empty_user.to_string());
        assert!(res.is_err());

        let invalid_user = "///////";
        let res = validate_username(invalid_user.to_string());
        assert!(res.is_err());

        let invalid_user =
            "reallyreallyreallyreallyreallyreallyreallyreallyreallyreallyreallyreallylongusername";
        let res = validate_username(invalid_user.to_string());
        assert!(res.is_err());
    }

    #[test]
    fn test_resolve_pathname() {
        let path = AndrAddr::from_string("cosmos1...");
        let res =
            resolve_pathname(&mock_dependencies().storage, &mock_dependencies().api, path).unwrap();
        assert_eq!(res, Addr::unchecked("cosmos1..."));
    }

    #[test]
    fn test_resolve_home_path() {
        let mut deps = mock_dependencies();
        let username = "u1";
        let first_directory = "d1";
        let second_directory = "d2";
        let file = "f1";

        let username_address = Addr::unchecked("useraddress");
        let first_directory_address = Addr::unchecked("dir1address");
        let second_directory_address = Addr::unchecked("dir2address");
        let file_address = Addr::unchecked("fileaddress");

        USERS
            .save(deps.as_mut().storage, username, &username_address)
            .unwrap();

        let res = resolve_home_path(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(format!("/home/{username}")),
        )
        .unwrap();
        assert_eq!(res, username_address);

        let res = resolve_home_path(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(format!("~{username}")),
        )
        .unwrap();
        assert_eq!(res, username_address);

        let res = resolve_home_path(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(format!("~/{username}")),
        )
        .unwrap();
        assert_eq!(res, username_address);

        paths()
            .save(
                deps.as_mut().storage,
                &(username_address.clone(), first_directory.to_string()),
                &PathInfo {
                    name: first_directory.to_string(),
                    address: first_directory_address.clone(),
                    parent_address: username_address,
                    symlink: None,
                },
            )
            .unwrap();

        let res = resolve_home_path(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(format!("/home/{username}/{first_directory}")),
        )
        .unwrap();
        assert_eq!(res, first_directory_address);
        let res = resolve_home_path(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(format!("~{username}/{first_directory}")),
        )
        .unwrap();
        assert_eq!(res, first_directory_address);

        paths()
            .save(
                deps.as_mut().storage,
                &(
                    first_directory_address.clone(),
                    second_directory.to_string(),
                ),
                &PathInfo {
                    name: second_directory.to_string(),
                    address: second_directory_address.clone(),
                    parent_address: first_directory_address,
                    symlink: None,
                },
            )
            .unwrap();

        let res = resolve_home_path(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(format!(
                "/home/{username}/{first_directory}/{second_directory}"
            )),
        )
        .unwrap();
        assert_eq!(res, second_directory_address);

        paths()
            .save(
                deps.as_mut().storage,
                &(second_directory_address.clone(), file.to_string()),
                &PathInfo {
                    name: file.to_string(),
                    address: file_address.clone(),
                    parent_address: second_directory_address,
                    symlink: None,
                },
            )
            .unwrap();

        let res = resolve_home_path(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(format!(
                "/home/{username}/{first_directory}/{second_directory}/{file}"
            )),
        )
        .unwrap();
        assert_eq!(res, file_address)
    }

    #[test]
    fn test_resolve_lib_path() {
        let mut deps = mock_dependencies();
        let lib_name = "l1";
        let first_directory = "d1";
        let second_directory = "d2";
        let file = "f1";

        let username_address = Addr::unchecked("useraddress");
        let first_directory_address = Addr::unchecked("dir1address");
        let second_directory_address = Addr::unchecked("dir2address");
        let file_address = Addr::unchecked("fileaddress");

        LIBRARIES
            .save(deps.as_mut().storage, lib_name, &username_address)
            .unwrap();

        let res = resolve_lib_path(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(format!("/lib/{lib_name}")),
        )
        .unwrap();
        assert_eq!(res, username_address);

        paths()
            .save(
                deps.as_mut().storage,
                &(username_address.clone(), first_directory.to_string()),
                &PathInfo {
                    name: first_directory.to_string(),
                    address: first_directory_address.clone(),
                    parent_address: username_address,
                    symlink: None,
                },
            )
            .unwrap();

        let res = resolve_lib_path(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(format!("/lib/{lib_name}/{first_directory}")),
        )
        .unwrap();
        assert_eq!(res, first_directory_address);

        paths()
            .save(
                deps.as_mut().storage,
                &(
                    first_directory_address.clone(),
                    second_directory.to_string(),
                ),
                &PathInfo {
                    name: second_directory.to_string(),
                    address: second_directory_address.clone(),
                    parent_address: first_directory_address,
                    symlink: None,
                },
            )
            .unwrap();

        let res = resolve_lib_path(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(format!(
                "/lib/{lib_name}/{first_directory}/{second_directory}"
            )),
        )
        .unwrap();
        assert_eq!(res, second_directory_address);

        paths()
            .save(
                deps.as_mut().storage,
                &(second_directory_address.clone(), file.to_string()),
                &PathInfo {
                    name: file.to_string(),
                    address: file_address.clone(),
                    parent_address: second_directory_address,
                    symlink: None,
                },
            )
            .unwrap();

        let res = resolve_lib_path(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(format!(
                "/lib/{lib_name}/{first_directory}/{second_directory}/{file}"
            )),
        )
        .unwrap();
        assert_eq!(res, file_address)
    }

    #[test]
    fn test_resolve_symlink() {
        let mut deps = mock_dependencies();
        let username = "u1";
        let first_directory = "d1";

        let username_address = Addr::unchecked("useraddress");
        let first_directory_address = Addr::unchecked("dir1address");

        USERS
            .save(deps.as_mut().storage, username, &username_address)
            .unwrap();

        paths()
            .save(
                deps.as_mut().storage,
                &(username_address.clone(), first_directory.to_string()),
                &PathInfo {
                    name: first_directory.to_string(),
                    address: first_directory_address.clone(),
                    parent_address: username_address,
                    symlink: None,
                },
            )
            .unwrap();

        let res = resolve_home_path(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(format!("/home/{username}/{first_directory}")),
        )
        .unwrap();
        assert_eq!(res, first_directory_address);

        let symlink_parent = Addr::unchecked("parentaddress");
        let symlink_name = "symlink";
        let symlink = AndrAddr::from_string(format!("/home/{username}/{first_directory}"));
        add_path_symlink(
            deps.as_mut().storage,
            symlink_parent.clone(),
            symlink_name.to_string(),
            symlink.clone(),
        )
        .unwrap();

        let res = resolve_home_path(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(format!("/home/{symlink_parent}/{symlink_name}")),
        )
        .unwrap();
        assert_eq!(res, first_directory_address);

        let res = resolve_symlink(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(format!("/home/{symlink_parent}/{symlink_name}")),
        )
        .unwrap();

        assert_eq!(res, symlink);

        let res = resolve_symlink(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(format!("/home/{username}/{first_directory}")),
        )
        .unwrap();

        assert_eq!(
            res,
            AndrAddr::from_string(format!("/home/{username}/{first_directory}"))
        );

        let res = resolve_symlink(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(format!("ibc://chain/home/{symlink_parent}/{symlink}")),
        )
        .unwrap();

        assert_eq!(
            res,
            AndrAddr::from_string(format!("ibc://chain/home/{symlink_parent}/{symlink}"))
        );

        let res = resolve_symlink(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string("someaddress"),
        )
        .unwrap();

        assert_eq!(res, AndrAddr::from_string("someaddress"));

        let res = resolve_symlink(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string("/home/someuser"),
        )
        .unwrap();

        assert_eq!(res, AndrAddr::from_string("/home/someuser"));
    }

    #[test]
    fn test_resolve_path_too_long() {
        let mut deps = mock_dependencies();
        let mut path = "~/u1".to_owned();

        for i in 0..MAX_DEPTH + 1 {
            path.push_str(format!("/d{}", i).as_str());
        }

        USERS
            .save(deps.as_mut().storage, "u1", &Addr::unchecked("u1"))
            .unwrap();

        let res = resolve_home_path(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(path),
        );

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContractError::InvalidAddress {});

        let mut deps = mock_dependencies();
        let mut path: String = "/lib/u1".to_owned();

        for i in 0..MAX_DEPTH + 1 {
            path.push_str(format!("/d{}", i).as_str());
        }

        LIBRARIES
            .save(deps.as_mut().storage, "u1", &Addr::unchecked("u1"))
            .unwrap();

        let res = resolve_lib_path(
            deps.as_ref().storage,
            deps.as_ref().api,
            AndrAddr::from_string(path),
        );

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContractError::InvalidAddress {})
    }
}
