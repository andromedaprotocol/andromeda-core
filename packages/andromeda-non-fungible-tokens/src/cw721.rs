use andromeda_std::{
    amp::addresses::AndrAddr, andr_exec, andr_instantiate, andr_instantiate_modules, andr_query,
};
use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Binary, Coin, CustomMsg};
use cw721::Expiration;

use cw721_base::{ExecuteMsg as Cw721ExecuteMsg, QueryMsg as Cw721QueryMsg};

#[andr_instantiate]
#[andr_instantiate_modules]
#[cw_serde]
pub struct InstantiateMsg {
    /// Name of the NFT contract
    pub name: String,
    /// Symbol of the NFT contract
    pub symbol: String,
    /// The minter is the only one who can create new NFTs.
    /// This is designed for a base NFT that is controlled by an external program
    /// or contract. You will likely replace this with custom logic in custom NFTs
    pub minter: AndrAddr,
}

#[cw_serde]
/// A struct used to represent an agreed transfer of a token. The `purchaser` may use the `Transfer` message for this token as long as funds are provided equalling the `amount` defined in the agreement.
pub struct TransferAgreement {
    /// The amount required for the purchaser to transfer ownership of the token
    pub amount: Coin,
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
    /// The original publisher of the token
    pub publisher: String,
}

impl CustomMsg for ExecuteMsg {}
impl CustomMsg for QueryMsg {}

#[cw_serde]
pub struct MintMsg {
    /// Unique ID of the NFT
    pub token_id: String,
    /// The owner of the newly minter NFT
    pub owner: String,
    /// Universal resource identifier for this NFT
    /// Should point to a JSON file that conforms to the ERC721
    /// Metadata JSON Schema
    pub token_uri: Option<String>,
    /// Any custom extension used by this contract
    pub extension: TokenExtension,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Mints a token
    Mint {
        /// Unique ID of the NFT
        token_id: String,
        /// The owner of the newly minter NFT
        owner: String,
        /// Universal resource identifier for this NFT
        /// Should point to a JSON file that conforms to the ERC721
        /// Metadata JSON Schema
        token_uri: Option<String>,
        /// Any custom extension used by this contract
        extension: TokenExtension,
    },
    /// Transfers ownership of a token
    TransferNft {
        recipient: AndrAddr,
        token_id: String,
    },
    /// Sends a token to another contract
    SendNft {
        contract: AndrAddr,
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
    Revoke { spender: String, token_id: String },
    /// Approves an address for all tokens owned by the sender
    ApproveAll {
        operator: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted ApproveAll permission
    RevokeAll { operator: String },
    /// Burns a token, removing all data related to it. The ID of the token is still reserved.
    Burn { token_id: String },
    /// Archives a token, causing it to be immutable but readable
    Archive { token_id: String },
    /// Assigns a `TransferAgreement` for a token
    TransferAgreement {
        token_id: String,
        agreement: Option<TransferAgreement>,
    },
    /// Mint multiple tokens at a time
    BatchMint { tokens: Vec<MintMsg> },
}

impl From<ExecuteMsg> for Cw721ExecuteMsg<TokenExtension, ExecuteMsg> {
    fn from(msg: ExecuteMsg) -> Self {
        match msg {
            ExecuteMsg::TransferNft {
                recipient,
                token_id,
            } => Cw721ExecuteMsg::TransferNft {
                recipient: recipient.to_string(),
                token_id,
            },
            ExecuteMsg::SendNft {
                contract,
                token_id,
                msg,
            } => Cw721ExecuteMsg::SendNft {
                contract: contract.to_string(),
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
            ExecuteMsg::Mint {
                extension,
                token_id,
                token_uri,
                owner,
            } => Cw721ExecuteMsg::Mint {
                extension,
                token_id,
                token_uri,
                owner,
            },
            ExecuteMsg::Burn { token_id } => Cw721ExecuteMsg::Burn { token_id },
            _ => panic!("Unsupported message"),
        }
    }
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Owner of the given token by ID
    #[returns(cw721::OwnerOfResponse)]
    OwnerOf {
        token_id: String,
        include_expired: Option<bool>,
    },
    /// Approvals for a given address (paginated)
    #[returns(cw721::OperatorsResponse)]
    AllOperators {
        owner: String,
        include_expired: Option<bool>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Amount of tokens minted by the contract
    #[returns(cw721::NumTokensResponse)]
    NumTokens {},
    /// The data of a token
    #[returns(cw721::NftInfoResponse<TokenExtension>)]
    NftInfo { token_id: String },
    /// The data of a token and any approvals assigned to it
    #[returns(cw721::AllNftInfoResponse<TokenExtension>)]
    AllNftInfo {
        token_id: String,
        include_expired: Option<bool>,
    },
    /// All tokens minted by the contract owned by a given address (paginated)
    #[returns(cw721::TokensResponse)]
    Tokens {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// All tokens minted by the contract (paginated)
    #[returns(cw721::TokensResponse)]
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
    #[returns(cw721::ContractInfoResponse)]
    ContractInfo {},
    #[returns(cw721_base::MinterResponse)]
    Minter {},
    #[returns(cw721::ApprovalResponse)]
    Approval {
        token_id: String,
        spender: String,
        include_expired: Option<bool>,
    },
    /// Return approvals that a token has
    /// Return type: `ApprovalsResponse`
    #[returns(cw721::ApprovalsResponse)]
    Approvals {
        token_id: String,
        include_expired: Option<bool>,
    },
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
            QueryMsg::Minter {} => Cw721QueryMsg::Minter {},
            QueryMsg::Approval {
                token_id,
                spender,
                include_expired,
            } => Cw721QueryMsg::Approval {
                token_id,
                spender,
                include_expired,
            },
            QueryMsg::Approvals {
                token_id,
                include_expired,
            } => Cw721QueryMsg::Approvals {
                token_id,
                include_expired,
            },
            _ => panic!("Unsupported message"),
        }
    }
}
