use crate::cw721::TokenExtension;
use common::{
    ado_base::{modules::Module, recipient::Recipient, AndromedaMsg, AndromedaQuery},
    mission::AndrAddress,
};
use cosmwasm_std::{Coin, Uint128};
use cw0::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub token_address: AndrAddress,
    pub can_mint_after_sale: bool,
    pub modules: Option<Vec<Module>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    /// Mints a new token to be sold in a future sale. Only possible when the sale is not ongoing.
    Mint(Vec<CrowdfundMintMsg>),
    /// Starts the sale if one is not already ongoing.
    StartSale {
        /// When the sale ends.
        expiration: Expiration,
        /// The price per token.
        price: Coin,
        /// The minimum amount of tokens sold to go through with the sale.
        min_tokens_sold: Uint128,
        /// The amount of tokens a wallet can purchase, default is 1.
        max_amount_per_wallet: Option<Uint128>,
        /// The recipient of the funds if the sale met the minimum sold.
        recipient: Recipient,
    },
    /// Puchases a token in an ongoing sale.
    Purchase {
        token_id: String,
    },
    /// Allow a user to claim their own refund if the minimum number of tokens are not sold.
    ClaimRefund {},
    /// Ends the ongoing sale by completing `limit` number of operations depending on if the minimum number
    /// of tokens was sold.
    EndSale {
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    State {},
    Config {},
    AvailableTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    IsTokenAvailable {
        id: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CrowdfundMintMsg {
    /// Unique ID of the NFT
    pub token_id: String,
    /// The owner of the newly minter NFT
    pub owner: Option<String>,
    /// Universal resource identifier for this NFT
    /// Should point to a JSON file that conforms to the ERC721
    /// Metadata JSON Schema
    pub token_uri: Option<String>,
    /// Any custom extension used by this contract
    pub extension: TokenExtension,
}
