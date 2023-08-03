use andromeda_std::error::ContractError;
use cosmwasm_std::{ensure, Addr, Api, StdError, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Map, MultiIndex};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct PathInfo {
    pub name: String,
    pub address: Addr,
    pub parent_address: Addr,
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
pub const ADDRESS_USERNAME: Map<&str, String> = Map::new("address_username");

pub fn split_pathname(path: String) -> Vec<String> {
    path.split('/')
        .filter(|string| !string.is_empty())
        .map(|string| string.to_string())
        .collect::<Vec<String>>()
}

pub fn resolve_pathname(
    storage: &dyn Storage,
    api: &dyn Api,
    pathname: String,
) -> Result<Addr, ContractError> {
    let parts = split_pathname(pathname);

    let username_or_address = parts.first().unwrap();
    let user_address = match api.addr_validate(username_or_address) {
        Ok(addr) => addr,
        Err(_e) => USERS.load(storage, username_or_address.as_str())?,
    };
    let mut address = user_address;
    for (idx, part) in parts.iter().enumerate() {
        // Skip username
        if idx == 0 {
            continue;
        }
        address = paths().load(storage, &(address, part.clone()))?.address;
    }

    Ok(address)
}

pub fn get_subdir(
    storage: &dyn Storage,
    api: &dyn Api,
    pathname: String,
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
        },
    )
}

pub fn validate_username(username: String) -> Result<bool, ContractError> {
    ensure!(
        !username.is_empty(),
        ContractError::InvalidUsername {
            error: Some("Username cannot be empty.".to_string())
        }
    );
    ensure!(
        username.chars().all(|ch| ch.is_alphanumeric()),
        ContractError::InvalidUsername {
            error: Some(
                "Username contains invalid characters. All characters must be alphanumeric."
                    .to_string()
            )
        }
    );

    Ok(true)
}

#[cfg(test)]
mod test {
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
    }

    #[test]
    fn test_resolve_pathname() {
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

        let res = resolve_pathname(
            deps.as_ref().storage,
            deps.as_ref().api,
            username.to_string(),
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
                },
            )
            .unwrap();

        let res = resolve_pathname(
            deps.as_ref().storage,
            deps.as_ref().api,
            "/u1/d1".to_string(),
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
                },
            )
            .unwrap();

        let res = resolve_pathname(
            deps.as_ref().storage,
            deps.as_ref().api,
            "/u1/d1/d2".to_string(),
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
                },
            )
            .unwrap();

        let res = resolve_pathname(
            deps.as_ref().storage,
            deps.as_ref().api,
            "/u1/d1/d2/f1".to_string(),
        )
        .unwrap();
        assert_eq!(res, file_address)
    }
}
