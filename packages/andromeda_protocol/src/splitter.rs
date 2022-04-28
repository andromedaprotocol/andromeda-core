use crate::{modules::address_list::AddressListModule, require};
use cosmwasm_std::{StdError, StdResult, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AddressPercent {
    pub addr: String,
    pub percent: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// A config struct for a `Splitter` contract.
pub struct Splitter {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is sent the amount sent will be divided amongst these recipients depending on their assigned percentage.
    pub recipients: Vec<AddressPercent>,
    /// Whether or not the contract is currently locked. This restricts updating any config related fields.
    pub locked: bool,
    /// An optional address list to restrict access to the `Splitter` contract.     
    pub address_list: Option<AddressListModule>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is sent the amount sent will be divided amongst these recipients depending on their assigned percentage.
    pub recipients: Vec<AddressPercent>,
    /// An optional address list to restrict access to the `Splitter` contract.    
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
    /// Update the recipients list. Only executable by the contract owner when the contract is not locked.
    UpdateRecipients { recipients: Vec<AddressPercent> },
    /// Used to lock/unlock the contract allowing the config to be updated.
    UpdateLock { lock: bool },
    /// Update the optional address list module. Only executable by the contract owner when the contract is not locked.
    UpdateAddressList {
        address_list: Option<AddressListModule>,
    },
    /// Divides any attached funds to the message amongst the recipients list.
    Send {},
    /// Update ownership of the contract. Only executable by the current contract owner.
    UpdateOwner {
        /// The address of the new contract owner.
        address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// The current config of the Splitter contract
    GetSplitterConfig {},
    /// The current contract owner.
    ContractOwner {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct GetSplitterConfigResponse {
    pub config: Splitter,
    /// The address of the address list contract (if it exists)
    pub address_list_contract: Option<String>,
}

/// Ensures that a given list of recipients for a `splitter` contract is valid:
///
/// * Must include at least one recipient
/// * The combined percentage of the recipients must not exceed 100
pub fn validate_recipient_list(recipients: Vec<AddressPercent>) -> StdResult<bool> {
    require(
        recipients.len() > 0,
        StdError::generic_err("The recipients list must include at least one recipient"),
    )?;

    let mut percent_sum: Uint128 = Uint128::from(0_u128);
    for rec in recipients {
        percent_sum += rec.percent;
    }

    require(
        percent_sum <= Uint128::from(100u128),
        StdError::generic_err("The amount received by the recipients should not exceed 100%"),
    )?;

    Ok(true)
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
            percent: Uint128::from(150_u128),
        }];
        let res = validate_recipient_list(inadequate_recipients).unwrap_err();
        assert_eq!(
            res,
            StdError::generic_err("The amount received by the recipients should not exceed 100%")
        );

        let valid_recipients = vec![
            AddressPercent {
                addr: String::from("some address"),
                percent: Uint128::from(50_u128),
            },
            AddressPercent {
                addr: String::from("some address"),
                percent: Uint128::from(50_u128),
            },
        ];

        let res = validate_recipient_list(valid_recipients).unwrap();
        assert_eq!(true, res);
    }
}
