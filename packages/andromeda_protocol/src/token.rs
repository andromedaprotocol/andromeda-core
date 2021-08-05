use crate::hook::InitHook;

use andromeda_modules::modules::ModuleDefinition;
use cosmwasm_std::{Addr, Binary, BlockInfo, Coin, StdResult};
use cw721::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub type TokenId = String;

//Duplicate Approval struct from CW721-base contract: https://github.com/CosmWasm/cosmwasm-plus/blob/main/contracts/cw721-base/src/state.rs
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: Addr,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

impl Approval {
    pub fn is_expired(&self, block: &BlockInfo) -> bool {
        self.expires.is_expired(block)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Token {
    pub token_id: TokenId,
    pub owner: String,
    pub name: String,
    pub description: Option<String>,
    pub approvals: Vec<Approval>,
}

impl Token {
    pub fn filter_approval(&mut self, spender: &Addr) {
        self.approvals = self
            .approvals
            .clone()
            .into_iter()
            .filter(|a| !a.spender.eq(spender))
            .collect();
    }
}

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
    TransferNft {
        recipient: String,
        token_id: TokenId,
    },
    SendNft {
        contract: String,
        token_id: TokenId,
        msg: Binary,
    },
    /// Allows operator to transfer / send the token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    Approve {
        spender: String,
        token_id: TokenId,
        expires: Option<Expiration>,
    },
    /// Remove previously granted Approval
    Revoke {
        spender: String,
        token_id: TokenId,
    },
    ApproveAll {
        operator: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted ApproveAll permission
    RevokeAll {
        operator: String,
    },
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
    // }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    OwnerOf {
        token_id: String,
    },
    ApprovedForAll {
        owner: String,
        include_expired: Option<bool>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    NumTokens {},
    NftInfo {
        token_id: TokenId,
    },
    AllNftInfo {
        token_id: TokenId,
    },
    ContractInfo {},
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
