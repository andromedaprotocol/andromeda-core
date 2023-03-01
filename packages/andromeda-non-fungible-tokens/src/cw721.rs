use andromeda_os::messages::AMPPkt;
use common::{
    ado_base::{hooks::AndromedaHook, modules::Module, AndromedaMsg, AndromedaQuery},
    primitive::Value,
};
use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Binary, Coin, CustomMsg};
use cw721::{
    AllNftInfoResponse, ContractInfoResponse, Expiration, NftInfoResponse, NumTokensResponse,
    OperatorsResponse, OwnerOfResponse, TokensResponse,
};
pub use cw721_base::MintMsg;
use cw721_base::{ExecuteMsg as Cw721ExecuteMsg, MinterResponse, QueryMsg as Cw721QueryMsg};

#[cw_serde]
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
    ///The attached Andromeda modules
    pub modules: Option<Vec<Module>>,
    pub kernel_address: Option<String>,
}

#[cw_serde]
/// A struct used to represent an agreed transfer of a token. The `purchaser` may use the `Transfer` message for this token as long as funds are provided equalling the `amount` defined in the agreement.
pub struct TransferAgreement {
    /// The amount required for the purchaser to transfer ownership of the token
    pub amount: Value<Coin>,
    /// The address of the purchaser
    pub purchaser: String,
}

#[cw_serde]
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
#[cw_serde]
#[derive(Default)]
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
    /// A URL to a related YouTube videos
    pub youtube_url: Option<String>,
}

impl CustomMsg for ExecuteMsg {}
impl CustomMsg for QueryMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    AMPReceive(AMPPkt),
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
    /// Mint multiple tokens at a time
    BatchMint {
        tokens: Vec<MintMsg<TokenExtension>>,
    },
    Extension {
        msg: Box<ExecuteMsg>,
    },
}

impl From<ExecuteMsg> for Cw721ExecuteMsg<TokenExtension, ExecuteMsg> {
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
            ExecuteMsg::Burn { token_id } => Cw721ExecuteMsg::Burn { token_id },
            ExecuteMsg::Extension { msg } => Cw721ExecuteMsg::Extension { msg: *msg },

            _ => panic!("Unsupported message"),
        }
    }
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    #[returns(AndromedaHook)]
    AndrHook(AndromedaHook),

    /// Owner of the given token by ID
    #[returns(OwnerOfResponse)]
    OwnerOf {
        token_id: String,
        include_expired: Option<bool>,
    },
    /// Approvals for a given address (paginated)
    #[returns(OperatorsResponse)]
    AllOperators {
        owner: String,
        include_expired: Option<bool>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Amount of tokens minted by the contract
    #[returns(NumTokensResponse)]
    NumTokens {},
    /// The data of a token
    #[returns(NftInfoResponse<TokenExtension>)]
    NftInfo { token_id: String },
    /// The data of a token and any approvals assigned to it
    #[returns(AllNftInfoResponse<TokenExtension>)]
    AllNftInfo {
        token_id: String,
        include_expired: Option<bool>,
    },
    /// All tokens minted by the contract owned by a given address (paginated)
    #[returns(TokensResponse)]
    Tokens {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// All tokens minted by the contract (paginated)
    #[returns(TokensResponse)]
    AllTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// If the token is archived
    #[returns(bool)]
    IsArchived { token_id: String },
    /// The transfer agreement for the token
    #[returns(Option<TransferAgreement>)]
    TransferAgreement { token_id: String },
    /// The current config of the contract
    #[returns(ContractInfoResponse)]
    ContractInfo {},
    #[returns(TokenExtension)]
    Extension { msg: Box<QueryMsg> },
    #[returns(MinterResponse)]
    Minter {},
}

impl From<QueryMsg> for Cw721QueryMsg<QueryMsg> {
    fn from(msg: QueryMsg) -> Self {
        match msg {
            QueryMsg::OwnerOf {
                token_id,
                include_expired,
            } => Cw721QueryMsg::OwnerOf {
                token_id,
                include_expired,
            },
            QueryMsg::AllOperators {
                owner,
                include_expired,
                start_after,
                limit,
            } => Cw721QueryMsg::AllOperators {
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
            QueryMsg::Extension { msg } => Cw721QueryMsg::Extension { msg: *msg },
            QueryMsg::Minter {} => Cw721QueryMsg::Minter {},
            _ => panic!("Unsupported message"),
        }
    }
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
