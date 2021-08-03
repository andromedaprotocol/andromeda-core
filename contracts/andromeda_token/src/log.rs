use std::fmt;

use cosmwasm_std::String;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MintLog {
    pub token_id: TokenId,
    pub owner: String,
}

impl fmt::Display for MintLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{token_id: {}, owner: {}}}", self.token_id, self.owner)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TransferLog {
    pub token_id: TokenId,
    pub from: String,
    pub to: String,
}
impl fmt::Display for TransferLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{token_id: {}, from: {}, to: {}}}",
            self.token_id, self.from, self.to
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BurnLog {
    pub token_id: TokenId,
    pub burner: String,
}

impl fmt::Display for BurnLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{token_id: {}, burner: {}}}",
            self.token_id, self.burner
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ArchiveLog {
    pub token_id: TokenId,
    pub archiver: String,
}

impl fmt::Display for ArchiveLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{token_id: {}, archiver: {}}}",
            self.token_id, self.archiver
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WhitelistLog {
    pub address: String,
    pub whitelister: String,
    pub whitelisted: bool,
}

impl fmt::Display for WhitelistLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{address: {}, whitelister: {}, whitelisted: {}}}",
            self.address, self.whitelister, self.whitelisted
        )
    }
}
