use crate::modules::whitelist::Whitelist;
use crate::require::require;
use cosmwasm_std::{StdError, StdResult, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
        require(
            self.recipients.len() > 0,
            StdError::generic_err("The recipients list must include at least one recipient"),
        )?;
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
