use andromeda_std::{amp::addresses::AndrAddr, andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Coin};
use cw721::Expiration;

#[andr_instantiate]
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
pub struct MintMsg {
    /// Unique ID of the NFT
    pub token_id: String,
    /// The owner of the newly minter NFT
    pub owner: AndrAddr,
    /// Universal resource identifier for this NFT
    /// Should point to a JSON file that conforms to the ERC721
    /// Metadata JSON Schema
    pub token_uri: Option<String>,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Mints a token
    Mint(MintMsg),
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
    // / Burns a token, removing all data related to it. The ID of the token is still reserved.
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
        tokens: Vec<MintMsg>,
    },
    /// Batch sends multiple NFTs to different contracts
    BatchSend {
        batch: Vec<BatchSendMsg>,
    },
}

#[cw_serde]
pub struct BatchSendMsg {
    pub token_id: String,
    pub contract_addr: AndrAddr,
    pub msg: Binary,
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Owner of the given token by ID
    #[returns(cw721::msg::OwnerOfResponse)]
    OwnerOf {
        token_id: String,
        include_expired: Option<bool>,
    },
    /// Approvals for a given address (paginated)
    #[returns(cw721::msg::OperatorsResponse)]
    AllOperators {
        owner: String,
        include_expired: Option<bool>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Amount of tokens minted by the contract
    #[returns(cw721::msg::NumTokensResponse)]
    NumTokens {},
    /// The data of a token
    #[returns(cw721::msg::NftInfoResponse)]
    NftInfo { token_id: String },
    // The data of a token and any approvals assigned to it
    #[returns(cw721::msg::AllNftInfoResponse)]
    AllNftInfo {
        token_id: String,
        include_expired: Option<bool>,
    },
    /// All tokens minted by the contract owned by a given address (paginated)
    #[returns(cw721::msg::TokensResponse)]
    Tokens {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// All tokens minted by the contract (paginated)
    #[returns(cw721::msg::TokensResponse)]
    AllTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// If the token is archived
    #[returns(IsArchivedResponse)]
    IsArchived { token_id: String },
    // /// The transfer agreement for the token
    #[returns(Option<TransferAgreement>)]
    TransferAgreement { token_id: String },
    /// The current config of the contract
    #[returns(cw721::msg::CollectionInfoAndExtensionResponse)]
    ContractInfo {},
    #[returns(cw721::msg::MinterResponse)]
    Minter {},
    #[returns(cw721::msg::ApprovalResponse)]
    Approval {
        token_id: String,
        spender: String,
        include_expired: Option<bool>,
    },
    /// Return approvals that a token has
    /// Return type: `ApprovalsResponse`
    #[returns(cw721::msg::ApprovalsResponse)]
    Approvals {
        token_id: String,
        include_expired: Option<bool>,
    },
}
#[cw_serde]
pub struct IsArchivedResponse {
    pub is_archived: bool,
}
