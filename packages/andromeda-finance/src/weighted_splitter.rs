use common::{
    ado_base::{modules::Module, recipient::Recipient, AndromedaMsg, AndromedaQuery},
    error::ContractError,
    require,
};
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AddressWeight {
    pub recipient: Recipient,
    pub weight: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// A config struct for a `Splitter` contract.
pub struct Splitter {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is sent the amount sent will be divided amongst these recipients depending on their assigned weight.
    pub recipients: Vec<AddressWeight>,
    /// Whether or not the contract is currently locked. This restricts updating any config related fields.
    pub locked: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is
    /// sent the amount sent will be divided amongst these recipients depending on their assigned weight.
    pub recipients: Vec<AddressWeight>,
    pub modules: Option<Vec<Module>>,
}

impl InstantiateMsg {
    pub fn validate(&self) -> Result<bool, ContractError> {
        validate_recipient_list(self.recipients.clone())?;
        Ok(true)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Update the recipients list. Only executable by the contract owner when the contract is not locked.
    UpdateRecipients {
        recipients: Vec<AddressWeight>,
    },
    AddRecipient {
        recipient: AddressWeight,
    },
    RemoveRecipient {
        recipient: Recipient,
    },
    /// Used to lock/unlock the contract allowing the config to be updated.
    UpdateLock {
        lock: bool,
    },
    /// Divides any attached funds to the message amongst the recipients list.
    Send {},
    AndrReceive(AndromedaMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    /// The current config of the Splitter contract
    GetSplitterConfig {},
    /// Gets user's allocated weight
    GetUserWeight {
        user: Recipient,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct GetSplitterConfigResponse {
    pub config: Splitter,
}
/// In addition to returning a specific recipient's weight, this function also returns the total weight of all recipients.
/// This serves to put the user's weight into perspective.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct GetUserWeightResponse {
    pub weight: Uint128,
    pub total_weight: Uint128,
}

/// Ensures that a given list of recipients for a `weighted-splitter` contract is valid:
///
/// * Must include at least one recipient
pub fn validate_recipient_list(recipients: Vec<AddressWeight>) -> Result<bool, ContractError> {
    require(
        !recipients.is_empty(),
        ContractError::EmptyRecipientsList {},
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
        assert_eq!(res, ContractError::EmptyRecipientsList {});

        let valid_recipients = vec![
            AddressWeight {
                recipient: Recipient::from_string(String::from("Some Address")),
                weight: Uint128::new(50),
            },
            AddressWeight {
                recipient: Recipient::from_string(String::from("Some Address")),
                weight: Uint128::new(50),
            },
        ];

        let res = validate_recipient_list(valid_recipients).unwrap();
        assert!(res);
    }
}
