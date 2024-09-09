use andromeda_std::{amp::AndrAddr, andr_exec, andr_instantiate, andr_query, error::ContractError};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, StdError};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub restriction: StringStorageRestriction,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    SetValue {
        value: StringStorage,
    },
    DeleteValue {},
    UpdateRestriction {
        restriction: StringStorageRestriction,
    },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetValueResponse)]
    GetValue {},
    #[returns(GetDataOwnerResponse)]
    GetDataOwner {},
}

#[cw_serde]
pub enum StringStorage {
    String(String),
}

impl StringStorage {
    pub fn validate(&self) -> Result<(), ContractError> {
        match self {
            StringStorage::String(value) => {
                ensure!(!value.to_string().is_empty(), ContractError::EmptyString {});
            }
        }
        Ok(())
    }
}

impl From<StringStorage> for String {
    fn from(string_storage: StringStorage) -> Self {
        match string_storage {
            StringStorage::String(value) => value,
        }
    }
}

impl From<String> for StringStorage {
    fn from(value: String) -> Self {
        StringStorage::String(value)
    }
}

impl StringStorage {
    pub fn try_get_value(&self) -> Result<String, StdError> {
        match self {
            StringStorage::String(value) => Ok(value.to_string()),
        }
    }
}

#[cw_serde]
pub enum StringStorageRestriction {
    Private,
    Public,
    Restricted,
}

#[cw_serde]
pub struct GetValueResponse {
    pub value: String,
}

#[cw_serde]
pub struct GetDataOwnerResponse {
    pub owner: AndrAddr,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestValidate {
        name: &'static str,
        string_storage: StringStorage,
        expected_error: Option<ContractError>,
    }

    #[test]
    fn test_from_string() {
        let cases = vec![(
            StringStorage::String("String".to_string()),
            "String".to_string(),
        )];

        for (value, expected_str) in cases.iter() {
            assert_eq!(String::from(value.to_owned()), expected_str.to_owned());
        }
    }

    #[test]
    fn test_validate() {
        let test_cases = vec![TestValidate {
            name: "Empty string",
            string_storage: StringStorage::String("".to_string()),
            expected_error: Some(ContractError::EmptyString {}),
        }];

        for test in test_cases {
            let res = test.string_storage.validate();

            if let Some(err) = test.expected_error {
                assert_eq!(res.unwrap_err(), err, "{}", test.name);
                continue;
            }

            assert!(res.is_ok());
        }
    }

    #[test]
    fn try_get_string() {
        let string_storage = StringStorage::String("String".to_string());
        assert_eq!(
            "String".to_string(),
            string_storage.try_get_value().unwrap()
        );
    }
}
