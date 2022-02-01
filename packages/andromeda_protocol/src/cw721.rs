use cosmwasm_std::{attr, BankMsg, Coin, Event};
use cw721_base::{InstantiateMsg as Cw721InstantiateMsg, QueryMsg as Cw721QueryMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    communication::{modules::Module, AndromedaQuery},
    error::ContractError,
    modules::common::calculate_fee,
    modules::Rate,
};

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
    pub modules: Option<Vec<Module>>,
}

impl From<InstantiateMsg> for Cw721InstantiateMsg {
    fn from(msg: InstantiateMsg) -> Self {
        Self {
            name: msg.name,
            symbol: msg.symbol,
            minter: msg.minter,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// A struct used to represent an agreed transfer of a token. The `purchaser` may use the `Transfer` message for this token as long as funds are provided equalling the `amount` defined in the agreement.
pub struct TransferAgreement {
    /// The amount required for the purchaser to transfer ownership of the token
    pub amount: Coin,
    /// The address of the purchaser
    pub purchaser: String,
}

impl TransferAgreement {
    /// Generates a `BankMsg` for the amount defined in the transfer agreement to the provided address
    pub fn generate_payment(&self, to_address: String) -> BankMsg {
        BankMsg::Send {
            to_address,
            amount: vec![self.amount.clone()],
        }
    }
    /// Generates a `BankMsg` for a given `Rate` to a given address
    pub fn generate_fee_payment(
        &self,
        to_address: String,
        rate: Rate,
    ) -> Result<BankMsg, ContractError> {
        Ok(BankMsg::Send {
            to_address,
            amount: vec![calculate_fee(rate, &self.amount)?],
        })
    }
    /// Generates an event related to the agreed transfer of a token
    pub fn generate_event(self) -> Event {
        Event::new("agreed_transfer").add_attributes(vec![
            attr("amount", self.amount.to_string()),
            attr("purchaser", self.purchaser),
        ])
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
/// Enum used to define the type of metadata held by a token
pub enum MetadataType {
    Image,
    Video,
    Audio,
    Domain,
    Json,
    Other,
}

impl ToString for MetadataType {
    fn to_string(&self) -> String {
        match self {
            MetadataType::Image => String::from("Image"),
            MetadataType::Video => String::from("Video"),
            MetadataType::Audio => String::from("Audio"),
            MetadataType::Domain => String::from("Domain"),
            MetadataType::Json => String::from("Json"),
            MetadataType::Other => String::from("Other"),
        }
    }
}
// [TOK-02] Add approval function should have been here but maybe was removed or altered in alter commits.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MetadataAttribute {
    /// The key for the attribute
    pub key: String,
    /// The value for the attribute
    pub value: String,
    /// The string used to display the attribute, if none is provided the `key` field can be used
    pub display_label: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokenMetadata {
    /// The metadata type
    pub data_type: MetadataType,
    /// A URL to the token's source
    pub external_url: Option<String>,
    /// A URL to any off-chain data relating to the token, the response from this URL should match the defined `data_type` of the token
    pub data_url: Option<String>,
    /// On chain attributes related to the token (basic key/value)
    pub attributes: Option<Vec<MetadataAttribute>>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokenExtension {
    /// The name of the token
    pub name: String,
    /// The original publisher of the token (immutable)
    pub publisher: String,
    /// An optional description of the token
    pub description: Option<String>,
    /// The transfer agreement of the token (if it exists)
    pub transfer_agreement: Option<TransferAgreement>,
    /// The metadata of the token (if it exists)
    pub metadata: Option<TokenMetadata>,
    /// Whether the token is archived or not
    pub archived: bool,
    /// The current price listing for the token
    pub pricing: Option<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
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
            _ => panic!("Invalid CW721 query message type"),
        }
    }
}
