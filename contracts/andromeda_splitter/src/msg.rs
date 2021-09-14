use schemars::JsonSchema;
use serde::{ Deserialize, Serialize};
use andromeda_protocol::token::TokenId;
use crate::state::AddressPercent;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub use_whitelist: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateRecipient {
        recipient: Vec<AddressPercent>,
    },
    UpdateLock {
        lock: bool,
    },
    UpdateUseWhitelist {
        use_whitelist: bool,
    },
    UpdateTokenList {
        accepted_tokenlist: Vec<TokenId>
    },
    UpdateSenderWhitelist {
        sender_whitelist: Vec<String>
    },
    Send {}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Splitter{},
    IsWhitelisted { address: String },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct IsWhitelistedResponse {
    pub whitelisted: bool,
}

