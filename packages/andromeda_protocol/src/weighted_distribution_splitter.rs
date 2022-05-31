use crate::{modules::address_list::AddressListModule, require};
use cosmwasm_std::{StdError, StdResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AddressWeight {
    pub addr: String,
    pub weight: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// A config struct for a `Splitter` contract.
pub struct Splitter {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is sent the amount sent will be divided amongst these recipients depending on their assigned weight.
    pub recipients: Vec<AddressWeight>,
    /// Whether or not the contract is currently locked. This restricts updating any config related fields.
    pub locked: bool,
    /// An optional address list to restrict access to the `Splitter` contract.     
    pub address_list: Option<AddressListModule>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is sent the amount sent will be divided amongst these recipients depending on their assigned weight.
    pub recipients: Vec<AddressWeight>,
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
    UpdateRecipients { recipients: Vec<AddressWeight> },
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
/// * Currently there's no limit on weights, but will make them u16s for now to avoid absurd numbers
pub fn validate_recipient_list(recipients: Vec<AddressWeight>) -> StdResult<bool> {
    require(
        recipients.len() > 0,
        StdError::generic_err("The recipients list must include at least one recipient"),
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

        let valid_recipients = vec![
            AddressWeight {
                addr: String::from("some address"),
                weight: 50,
            },
            AddressWeight {
                addr: String::from("some address"),
                weight: 50,
            },
        ];

        let res = validate_recipient_list(valid_recipients).unwrap();
        assert_eq!(true, res);
    }
}
