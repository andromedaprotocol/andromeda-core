use andromeda_std::{andr_exec, andr_instantiate, andr_instantiate_modules, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw721::{Cw721ReceiveMsg, Expiration};
use std::fmt::{Display, Formatter, Result};

#[andr_instantiate]
#[andr_instantiate_modules]
#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    ReceiveNft(Cw721ReceiveMsg),
    /// Transfers NFT to buyer and sends funds to seller
    Buy {
        token_id: String,
        token_address: String,
    },
    /// Updates the sale's price, demomination, and whitelist
    UpdateSale {
        token_id: String,
        token_address: String,
        price: Uint128,
        coin_denom: String,
    },
    CancelSale {
        token_id: String,
        token_address: String,
    },
}

#[cw_serde]
pub enum Cw721HookMsg {
    /// Starts a new sale with the given parameters. The sale info can be modified before it
    /// has started but is immutable after that.
    StartSale {
        price: Uint128,
        coin_denom: String,
        start_time: Option<u64>,
        duration: Option<u64>,
    },
}
#[cw_serde]
pub enum Status {
    Open,
    Expired,
    Executed,
    Cancelled,
}
impl Display for Status {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Status::Open => f.write_str("Open"),
            Status::Expired => f.write_str("Expired"),
            Status::Executed => f.write_str("Executed"),
            Status::Cancelled => f.write_str("Cancelled"),
        }
    }
}

#[cw_serde]
pub struct SaleInfo {
    pub sale_ids: Vec<Uint128>,
    pub token_address: String,
    pub token_id: String,
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Gets the latest sale state for the given token. This will either be the current sale
    /// if there is one in progress or the last completed one.
    #[returns(SaleStateResponse)]
    LatestSaleState {
        token_id: String,
        token_address: String,
    },
    #[returns(SaleStateResponse)]
    /// Gets the sale state for the given sale id.
    SaleState { sale_id: Uint128 },
    #[returns(SaleIdsResponse)]
    /// Gets the sale ids for the given token.
    SaleIds {
        token_id: String,
        token_address: String,
    },
    #[returns(Vec<SaleInfo>)]
    /// Gets all of the sale infos for a given token address.
    SaleInfosForAddress {
        token_address: String,
        start_after: Option<String>,
        limit: Option<u64>,
    },
}

#[cw_serde]
pub struct SaleStateResponse {
    pub sale_id: Uint128,
    pub coin_denom: String,
    pub price: Uint128,
    pub status: Status,
    pub start_time: Expiration,
    pub end_time: Expiration,
}

#[cw_serde]
pub struct SaleIdsResponse {
    pub sale_ids: Vec<Uint128>,
}
