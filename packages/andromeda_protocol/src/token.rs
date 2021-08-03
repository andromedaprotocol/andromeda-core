use crate::hook::InitHook;

use andromeda_modules::modules::ModuleDefinition;
use cosmwasm_std::{Coin, StdResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub type TokenId = i64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TransferAgreement {
    pub amount: Coin,
    pub purchaser: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    /// Name of the NFT contract
    pub name: String,
    /// Symbol of the NFT contract
    pub symbol: String,

    /// The minter is the only one who can create new NFTs.
    /// This is designed for a base NFT that is controlled by an external program
    /// or contract. You will likely replace this with custom logic in custom NFTs
    pub minter: String,

    //The attached Andromeda modules
    pub modules: Vec<ModuleDefinition>,

    //A hook for if the contract is instantiated by the factory
    pub init_hook: Option<InitHook>,
}

impl InstantiateMsg {
    pub fn validate(&self) -> StdResult<bool> {
        Ok(true)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MintMsg {
    pub token_id: TokenId,
    pub owner: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Mint(MintMsg),
    // Transfer {
    //     collection_symbol: String,
    //     from: String,
    //     to: String,
    //     token_id: TokenId,
    // },
    // Burn {
    //     collection_symbol: String,
    //     token_id: TokenId,
    // },
    // Archive {
    //     collection_symbol: String,
    //     token_id: TokenId,
    // },
    // CreateTransferAgreement {
    //     collection_symbol: String,
    //     token_id: TokenId,
    //     denom: String,
    //     amount: Uint128,
    //     purchaser: String,
    // },
    // Whitelist {
    //     collection_symbol: String,
    //     address: String,
    // },
    // Dewhitelist {
    //     collection_symbol: String,
    //     address: String,
    // },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetBalance {
    //     collection_symbol: String,
    //     address: String,
    // },
    GetOwner { token_id: TokenId },
    // GetArchived {
    //     collection_symbol: String,
    //     token_id: TokenId,
    // },
    // GetTransferAgreement {
    //     collection_symbol: String,
    //     token_id: TokenId,
    // },
    // GetExtensions {
    //     collection_symbol: String,
    // },
    // GetWhitelisted {
    //     collection_symbol: String,
    //     address: String,
    // },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BalanceResponse {
    pub balance: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OwnerResponse {
    pub owner: String,
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
