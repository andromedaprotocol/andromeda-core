use crate::cw721::TokenExtension;
use andromeda_os::messages::AMPPkt;
use common::{
    ado_base::{recipient::Recipient, AndromedaMsg, AndromedaQuery},
    app::AndrAddress,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Coin, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub andromeda_cw721_contract: AndrAddress,
    pub randomness_source: String,
    pub required_coin: String,
    pub kernel_address: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    AMPReceive(AMPPkt),
    Mint(Vec<GumballMintMsg>),
    Buy {},
    UpdateRequiredCoin {
        new_coin: String,
    },
    /// Sets price, max amount per wallet, and recipient
    SetSaleDetails {
        /// The price per token.
        price: Coin,
        /// The amount of tokens a wallet can purchase, default is 1.
        max_amount_per_wallet: Option<Uint128>,
        /// The recipient of the funds.
        recipient: Recipient,
    },
    /// Automatically switches to opposite status.
    /// True means buying is allowed and minting is halted. False means the opposite.
    SwitchStatus {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    #[returns(NumberOfNftsResponse)]
    NumberOfNfts {},
    #[returns(crate::crowdfund::QueryMsg)]
    SaleDetails {},
    #[returns(StatusResponse)]
    Status {},
}

#[cw_serde]
pub enum RandQueryMsg {
    LatestDrand {},
    GetRandomness { round: u64 },
}

#[cw_serde]
pub struct NumberOfNftsResponse {
    pub number: usize,
}

#[cw_serde]
pub struct StatusResponse {
    pub status: bool,
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct LatestRandomResponse {
    pub round: u64,
    pub randomness: Binary,
    pub worker: String,
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct GumballMintMsg {
    /// Unique ID of the NFT
    pub token_id: String,
    /// The owner of the newly minted NFT
    pub owner: Option<String>,
    /// Universal resource identifier for this NFT
    /// Should point to a JSON file that conforms to the ERC721
    /// Metadata JSON Schema
    pub token_uri: Option<String>,
    /// Any custom extension used by this contract
    pub extension: TokenExtension,
}

#[cw_serde]
pub struct MigrateMsg {}
