use crate::cw721::TokenExtension;
use andromeda_std::amp::{addresses::AndrAddr, recipient::Recipient};
use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint128};
use cw_utils::Expiration;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub token_address: AndrAddr,
    pub can_mint_after_sale: bool,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
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
        max_amount_per_wallet: Option<u32>,
        /// The recipient of the funds if the sale met the minimum sold.
        recipient: Recipient,
    },
    /// Puchases tokens in an ongoing sale.
    Purchase { number_of_tokens: Option<u32> },
    /// Purchases the token with the given id.
    PurchaseByTokenId { token_id: String },
    /// Allow a user to claim their own refund if the minimum number of tokens are not sold.
    ClaimRefund {},
    /// Ends the ongoing sale by completing `limit` number of operations depending on if the minimum number
    /// of tokens was sold.
    EndSale { limit: Option<u32> },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(State)]
    State {},
    #[returns(Config)]
    Config {},
    #[returns(Vec<String>)]
    AvailableTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(bool)]
    IsTokenAvailable { id: String },
}

#[cw_serde]
pub struct Config {
    /// The address of the token contract whose tokens are being sold.
    pub token_address: AndrAddr,
    /// Whether or not the owner can mint additional tokens after the sale has been conducted.
    pub can_mint_after_sale: bool,
}

#[cw_serde]
pub struct State {
    /// The expiration denoting when the sale ends.
    pub expiration: Expiration,
    /// The price of each token.
    pub price: Coin,
    /// The minimum number of tokens sold for the sale to go through.
    pub min_tokens_sold: Uint128,
    /// The max number of tokens allowed per wallet.
    pub max_amount_per_wallet: u32,
    /// Number of tokens sold.
    pub amount_sold: Uint128,
    /// The amount of funds to send to recipient if sale successful. This already
    /// takes into account the royalties and taxes.
    pub amount_to_send: Uint128,
    /// Number of tokens transferred to purchasers if sale was successful.
    pub amount_transferred: Uint128,
    /// The recipient of the raised funds if the sale is successful.
    pub recipient: Recipient,
}

#[cw_serde]
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

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
