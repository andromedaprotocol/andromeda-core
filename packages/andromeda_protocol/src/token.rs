use crate::hook::InitHook;

use andromeda_modules::modules::{as_modules, Module, ModuleDefinition};
use cosmwasm_std::{HumanAddr, StdResult, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TransferAgreement {
    pub denom: String,
    pub amount: Uint128,
    pub purchaser: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub creator: HumanAddr,
    pub name: String,
    pub symbol: String,
    pub modules: Vec<ModuleDefinition>,
    pub init_hook: Option<InitHook>,
}

impl InitMsg {
    pub fn validate(&self) -> StdResult<bool> {
        let mapped_modules = as_modules(self.modules.to_vec());
        for module in mapped_modules {
            module.validate(self.modules.to_vec())?;
        }

        Ok(true)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Mint { token_id: i64 },
    // Transfer {
    //     collection_symbol: String,
    //     from: HumanAddr,
    //     to: HumanAddr,
    //     token_id: i64,
    // },
    // Burn {
    //     collection_symbol: String,
    //     token_id: i64,
    // },
    // Archive {
    //     collection_symbol: String,
    //     token_id: i64,
    // },
    // CreateTransferAgreement {
    //     collection_symbol: String,
    //     token_id: i64,
    //     denom: String,
    //     amount: Uint128,
    //     purchaser: HumanAddr,
    // },
    // Whitelist {
    //     collection_symbol: String,
    //     address: HumanAddr,
    // },
    // Dewhitelist {
    //     collection_symbol: String,
    //     address: HumanAddr,
    // },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetBalance {
    //     collection_symbol: String,
    //     address: HumanAddr,
    // },
    GetOwner { token_id: i64 },
    // GetArchived {
    //     collection_symbol: String,
    //     token_id: i64,
    // },
    // GetTransferAgreement {
    //     collection_symbol: String,
    //     token_id: i64,
    // },
    // GetExtensions {
    //     collection_symbol: String,
    // },
    // GetWhitelisted {
    //     collection_symbol: String,
    //     address: HumanAddr,
    // },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BalanceResponse {
    pub balance: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OwnerResponse {
    pub owner: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ArchivedResponse {
    pub archived: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ExtensionsResponse {
    pub extensions: Vec<ModuleDefinition>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AgreementResponse {
    pub agreement: TransferAgreement,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WhitelistedResponse {
    pub whitelisted: bool,
}
