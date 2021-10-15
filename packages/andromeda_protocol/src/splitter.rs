use crate::{modules::address_list::AddressListModule, require::require};
use cosmwasm_std::{StdError, StdResult, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub fn validate_recipient_list(recipients: Vec<AddressPercent>) -> StdResult<bool> {
    require(
        recipients.len() > 0,
        StdError::generic_err("The recipients list must include at least one recipient"),
    )?;

    let mut percent_sum: Uint128 = Uint128::from(0 as u128);
    for rec in recipients {
        percent_sum += rec.percent;
    }

    require(
        percent_sum <= Uint128::from(100u128),
        StdError::generic_err("The amount received by the recipients should not exceed 100%"),
    )?;

    Ok(true)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AddressPercent {
    pub addr: String,
    pub percent: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Splitter {
    pub recipients: Vec<AddressPercent>, //Map for address and percentage
    pub locked: bool,                    //Lock
    pub address_list: Option<AddressListModule>, //Address list allowing to receive funds
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub recipients: Vec<AddressPercent>,
    pub address_list: Option<AddressListModule>,
}

impl InstantiateMsg {
    pub fn validate(&self) -> StdResult<bool> {
        validate_recipient_list(self.recipients.clone())?;
        Ok(true)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateRecipients {
        recipients: Vec<AddressPercent>,
    },
    UpdateLock {
        lock: bool,
    },
    UpdateAddressList {
        address_list: Option<AddressListModule>,
    },
    Send {},
    UpdateOwner {
        address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetSplitterConfig {},
    ContractOwner {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct GetSplitterConfigResponse {
    pub config: Splitter,
    pub address_list_contract: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_recipient_list() {
        let empty_recipients = vec![];
        let res = validate_recipient_list(empty_recipients).unwrap_err();
        assert_eq!(
            res,
            StdError::generic_err("The recipients list must include at least one recipient")
        );

        let inadequate_recipients = vec![AddressPercent {
            addr: String::from("some address"),
            percent: Uint128::from(150 as u128),
        }];
        let res = validate_recipient_list(inadequate_recipients).unwrap_err();
        assert_eq!(
            res,
            StdError::generic_err("The amount received by the recipients should not exceed 100%")
        );

        let valid_recipients = vec![
            AddressPercent {
                addr: String::from("some address"),
                percent: Uint128::from(50 as u128),
            },
            AddressPercent {
                addr: String::from("some address"),
                percent: Uint128::from(50 as u128),
            },
        ];

        let res = validate_recipient_list(valid_recipients).unwrap();
        assert_eq!(true, res);
    }
}
