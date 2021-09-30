use crate::modules::whitelist::Whitelist;
use crate::require::require;
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
        percent_sum.eq(&Uint128::from(100 as u128)),
        StdError::generic_err("The amount received by the recipients must come to 100%"),
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
    pub use_whitelist: bool,             //Use whitelist
    pub sender_whitelist: Whitelist,     //Address list allowing to receive funds
    pub accepted_tokenlist: Vec<String>, //Token list allowing to accept
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub recipients: Vec<AddressPercent>,
    pub use_whitelist: bool,
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
    UpdateRecipients { recipients: Vec<AddressPercent> },
    UpdateLock { lock: bool },
    UpdateUseWhitelist { use_whitelist: bool },
    UpdateTokenList { accepted_tokenlist: Vec<String> },
    UpdateSenderWhitelist { sender_whitelist: Vec<String> },
    Send {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetSplitterConfig {},
    IsWhitelisted { address: String },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct GetSplitterConfigResponse {
    pub config: Splitter,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct IsWhitelistedResponse {
    pub whitelisted: bool,
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
            percent: Uint128::from(50 as u128),
        }];
        let res = validate_recipient_list(inadequate_recipients).unwrap_err();
        assert_eq!(
            res,
            StdError::generic_err("The amount received by the recipients must come to 100%")
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
