use common::error::ContractError;
use cosmwasm_std::{ensure, Addr, Api, StdError, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Map, MultiIndex};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct PathInfo {
    pub name: String,
    pub address: Addr,
}

pub struct PathIndices<'a> {
    /// PK: parent_address + component_name
    /// Secondary key: component_name
    pub index: MultiIndex<'a, String, PathInfo, String>,
}

impl<'a> IndexList<PathInfo> for PathIndices<'a> {
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn cw_storage_plus::Index<PathInfo>> + '_> {
        let v: Vec<&dyn Index<PathInfo>> = vec![&self.index];
        Box::new(v.into_iter())
    }
}

pub fn paths<'a>() -> IndexedMap<'a, &'a str, PathInfo, PathIndices<'a>> {
    let indexes = PathIndices {
        index: MultiIndex::new(|_pk: &[u8], r| r.name.clone(), "path", "path_index"),
    };
    IndexedMap::new("path", indexes)
}

pub const USERS: Map<&str, Addr> = Map::new("users");

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

        address = paths()
            .load(storage, (address.to_string() + part.as_str()).as_str())?
            .address;
    }

    Ok(address)
}

pub fn add_pathname(
    storage: &mut dyn Storage,
    parent_addr: Addr,
    name: String,
    address: Addr,
) -> Result<(), StdError> {
    paths().save(
        storage,
        &(parent_addr.to_string() + name.as_str()),
        &PathInfo { name, address },
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
                (username_address.to_string() + first_directory).as_str(),
                &PathInfo {
                    name: first_directory.to_string(),
                    address: first_directory_address.clone(),
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
                (first_directory_address.to_string() + second_directory).as_str(),
                &PathInfo {
                    name: second_directory.to_string(),
                    address: second_directory_address.clone(),
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
                (second_directory_address.to_string() + file).as_str(),
                &PathInfo {
                    name: file.to_string(),
                    address: file_address.clone(),
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
