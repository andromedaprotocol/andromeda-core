use common::{
    ado_base::{hooks::AndromedaHook, modules::Module, AndromedaMsg, AndromedaQuery},
    mission::AndrAddress,
    primitive::Value,
};
use cosmwasm_std::{Binary, Coin};
use cw721::Expiration;
pub use cw721_base::MintMsg;
use cw721_base::{ExecuteMsg as Cw721ExecuteMsg, QueryMsg as Cw721QueryMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    pub minter: AndrAddress,
    ///The attached Andromeda modules
    pub modules: Option<Vec<Module>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// A struct used to represent an agreed transfer of a token. The `purchaser` may use the `Transfer` message for this token as long as funds are provided equalling the `amount` defined in the agreement.
pub struct TransferAgreement {
    /// The amount required for the purchaser to transfer ownership of the token
    pub amount: Value<Coin>,
    /// The address of the purchaser
    pub purchaser: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MetadataAttribute {
    /// The key for the attribute
    pub trait_type: String,
    /// The value for the attribute
    pub value: String,
    /// The string used to display the attribute, if none is provided the `key` field can be used
    pub display_type: Option<String>,
}

/// https://docs.opensea.io/docs/metadata-standards
/// Replicates OpenSea Metadata Standards
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokenExtension {
    /// The name of the token
    pub name: String,
    /// The original publisher of the token
    pub publisher: String,
    /// An optional description of the token
    pub description: Option<String>,
    /// The metadata of the token (if it exists)
    pub attributes: Vec<MetadataAttribute>,
    /// URL to token image
    pub image: String,
    /// Raw SVG image data
    pub image_data: Option<String>,
    /// A URL to the token's source
    pub external_url: Option<String>,
    /// A URL to any multi-media attachments
    pub animation_url: Option<String>,
    /// A URL to a related YouTube video
    pub youtube_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    /// Mints a token
    Mint(Box<MintMsg<TokenExtension>>),
    /// Transfers ownership of a token
    TransferNft {
        recipient: String,
        token_id: String,
    },
    /// Sends a token to another contract
    SendNft {
        contract: String,
        token_id: String,
        msg: Binary,
    },
    /// Allows operator to transfer / send the token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    Approve {
        spender: String,
        token_id: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted Approval
    Revoke {
        spender: String,
        token_id: String,
    },
    /// Approves an address for all tokens owned by the sender
    ApproveAll {
        operator: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted ApproveAll permission
    RevokeAll {
        operator: String,
    },
    /// Burns a token, removing all data related to it. The ID of the token is still reserved.
    Burn {
        token_id: String,
    },
    /// Archives a token, causing it to be immutable but readable
    Archive {
        token_id: String,
    },
    /// Assigns a `TransferAgreement` for a token
    TransferAgreement {
        token_id: String,
        agreement: Option<TransferAgreement>,
    },
}

impl From<ExecuteMsg> for Cw721ExecuteMsg<TokenExtension> {
    fn from(msg: ExecuteMsg) -> Self {
        match msg {
            ExecuteMsg::TransferNft {
                recipient,
                token_id,
            } => Cw721ExecuteMsg::TransferNft {
                recipient,
                token_id,
            },
            ExecuteMsg::SendNft {
                contract,
                token_id,
                msg,
            } => Cw721ExecuteMsg::SendNft {
                contract,
                token_id,
                msg,
            },
            ExecuteMsg::Approve {
                spender,
                token_id,
                expires,
            } => Cw721ExecuteMsg::Approve {
                spender,
                token_id,
                expires,
            },
            ExecuteMsg::Revoke { spender, token_id } => {
                Cw721ExecuteMsg::Revoke { spender, token_id }
            }
            ExecuteMsg::ApproveAll { operator, expires } => {
                Cw721ExecuteMsg::ApproveAll { operator, expires }
            }
            ExecuteMsg::RevokeAll { operator } => Cw721ExecuteMsg::RevokeAll { operator },
            ExecuteMsg::Mint(msg) => Cw721ExecuteMsg::Mint(*msg),
            _ => panic!("Unsupported message"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    AndrHook(AndromedaHook),
    /// Owner of the given token by ID
    OwnerOf {
        token_id: String,
        include_expired: Option<bool>,
    },
    /// Approvals for a given address (paginated)
    ApprovedForAll {
        owner: String,
        include_expired: Option<bool>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Amount of tokens minted by the contract
    NumTokens {},
    /// The data of a token
    NftInfo {
        token_id: String,
    },
    /// The data of a token and any approvals assigned to it
    AllNftInfo {
        token_id: String,
        include_expired: Option<bool>,
    },
    /// All tokens minted by the contract owned by a given address (paginated)
    Tokens {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// All tokens minted by the contract (paginated)
    AllTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// If the token is archived
    IsArchived {
        token_id: String,
    },
    /// The transfer agreement for the token
    TransferAgreement {
        token_id: String,
    },
    /// Info of any modules assigned to the contract
    ModuleInfo {},
    /// The current config of the contract
    ContractInfo {},
}

impl From<QueryMsg> for Cw721QueryMsg {
    fn from(msg: QueryMsg) -> Self {
        match msg {
            QueryMsg::OwnerOf {
                token_id,
                include_expired,
            } => Cw721QueryMsg::OwnerOf {
                token_id,
                include_expired,
            },
            QueryMsg::ApprovedForAll {
                owner,
                include_expired,
                start_after,
                limit,
            } => Cw721QueryMsg::ApprovedForAll {
                owner,
                include_expired,
                start_after,
                limit,
            },
            QueryMsg::NumTokens {} => Cw721QueryMsg::NumTokens {},
            QueryMsg::ContractInfo {} => Cw721QueryMsg::ContractInfo {},
            QueryMsg::NftInfo { token_id } => Cw721QueryMsg::NftInfo { token_id },
            QueryMsg::AllNftInfo {
                token_id,
                include_expired,
            } => Cw721QueryMsg::AllNftInfo {
                token_id,
                include_expired,
            },
            QueryMsg::Tokens {
                owner,
                start_after,
                limit,
            } => Cw721QueryMsg::Tokens {
                owner,
                start_after,
                limit,
            },
            QueryMsg::AllTokens { start_after, limit } => {
                Cw721QueryMsg::AllTokens { start_after, limit }
            }
            _ => panic!("Unsupported message"),
        }
    }
}
