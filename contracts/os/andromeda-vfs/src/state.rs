use common::error::ContractError;
use cosmwasm_std::{ensure, Addr, Api, StdError, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Map, MultiIndex};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct PathInfo {
    name: String,
    addr: Addr,
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
    path.split("/")
        .filter(|string| string.len() > 0)
        .map(|string| string.to_string())
        .collect::<Vec<String>>()
}

pub const VALID_CHARACTERS: [&'static str; 4] = ["_", "-", "/", ":"];

pub fn validate_pathname(path: String) -> Result<bool, ContractError> {
    ensure!(
        path.len() > 0,
        ContractError::InvalidPathname {
            error: Some("Empty path".to_string())
        }
    );
    ensure!(
        path.chars().any(|character| character.is_alphanumeric()),
        ContractError::InvalidPathname {
            error: Some("Path name does not include any valid characters".to_string())
        }
    );
    // let valid_characters: Vec<&str> = vec!["_", "-"];

    ensure!(
        path.chars().all(|character| character.is_alphanumeric()
            | VALID_CHARACTERS
                .iter()
                .any(|valid| valid.chars().next().eq(&Some(character)))),
        ContractError::InvalidPathname {
            error: Some("Pathname includes an invalid character".to_string())
        }
    );
    Ok(true)
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
        Err(_e) => USERS.load(storage, &username_or_address.as_str())?,
    };
    let mut address = user_address;
    for (idx, part) in parts.iter().enumerate() {
        if idx == 0 {
            continue;
        }

        address = paths()
            .load(storage, &(address.to_string() + part.as_str()).as_str())?
            .addr;
    }

    Ok(address)
}

pub fn add_pathname(
    storage: &mut dyn Storage,
    parent_addr: Addr,
    name: String,
    addr: Addr,
) -> Result<(), StdError> {
    paths().save(
        storage,
        &(parent_addr.to_string() + name.as_str()),
        &PathInfo { name, addr },
    )
}

#[cfg(test)]
mod test {
    use cosmwasm_std::testing::mock_dependencies;

    use super::*;

    #[test]
    fn test_split_pathname() {
        let pathname = "/username/dir1/dir2/file";

        let res = split_pathname(pathname.to_string());
        let expected = vec!["username", "dir1", "dir2", "file"];

        assert_eq!(res, expected)
    }

    #[test]
    fn test_validate_pathname() {
        let valid_path = "/username/dir1/file";
        validate_pathname(valid_path.to_string()).unwrap();

        let valid_path = "username/dir1/file";
        validate_pathname(valid_path.to_string()).unwrap();

        let valid_path = "/username/dir1/file/";
        validate_pathname(valid_path.to_string()).unwrap();

        let empty_path = "";
        let res = validate_pathname(empty_path.to_string());
        assert!(res.is_err());

        let invalid_path = "///////";
        let res = validate_pathname(invalid_path.to_string());
        assert!(res.is_err());

        let invalid_path = "/username/dir1/f!le";
        let res = validate_pathname(invalid_path.to_string());
        assert!(res.is_err())
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
                    addr: first_directory_address.clone(),
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
                    addr: second_directory_address.clone(),
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
                    addr: file_address.clone(),
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
