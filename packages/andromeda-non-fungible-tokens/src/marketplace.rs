use common::ado_base::{modules::Module, AndromedaMsg, AndromedaQuery};
use cosmwasm_std::{Addr, Uint128};
use cw721::Cw721ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub modules: Option<Vec<Module>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
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
        whitelist: Option<Vec<Addr>>,
    },
    CancelSale {
        token_id: String,
        token_address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw721HookMsg {
    /// Starts a new sale with the given parameters. The sale info can be modified before it
    /// has started but is immutable after that.
    StartSale {
        price: Uint128,
        coin_denom: String,
        whitelist: Option<Vec<Addr>>,
    },
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]

pub enum Status {
    Open,
    Executed,
    Cancelled,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    /// Gets the latest sale state for the given token. This will either be the current sale
    /// if there is one in progress or the last completed one.
    LatestSaleState {
        token_id: String,
        token_address: String,
    },
    /// Gets the sale state for the given sale id.
    SaleState {
        sale_id: Uint128,
    },
    /// Gets the sale ids for the given token.
    SaleIds {
        token_id: String,
        token_address: String,
    },
    /// Gets all of the sale infos for a given token address.
    SaleInfosForAddress {
        token_address: String,
        start_after: Option<String>,
        limit: Option<u64>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct SaleStateResponse {
    pub sale_id: Uint128,
    pub coin_denom: String,
    pub price: Uint128,
    pub whitelist: Option<Vec<Addr>>,
    pub status: Status,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct SaleIdsResponse {
    pub sale_ids: Vec<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
