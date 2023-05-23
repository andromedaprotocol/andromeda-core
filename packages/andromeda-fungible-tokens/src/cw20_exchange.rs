use andromeda_std::{amp::AndrAddr, andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use cw_asset::AssetInfo;
use serde::{Deserialize, Serialize};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the CW20 token to be sold
    pub token_address: AndrAddr,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Cancels an ongoing sale
    CancelSale { asset: AssetInfo },
    /// Purchases tokens with native funds
    Purchase { recipient: Option<String> },
    /// Receive for CW20 tokens, used for purchasing and starting sales
    Receive(Cw20ReceiveMsg),
}

/// Struct used to define a token sale. The asset used for the sale is defined as the key for the storage map.
#[cw_serde]
pub struct Sale {
    /// The rate at which to exchange tokens (amount of exchanged asset to purchase sale asset)
    pub exchange_rate: Uint128,
    /// The amount for sale at the given rate
    pub amount: Uint128,
    /// The recipient of the sale proceeds
    pub recipient: String,
}

#[derive(Deserialize, Serialize)]
pub enum Cw20HookMsg {
    /// Starts a sale
    StartSale {
        /// The asset that may be used to purchase tokens
        asset: AssetInfo,
        /// The amount of the above asset required to purchase a single token
        exchange_rate: Uint128,
        /// The recipient of the sale proceeds
        /// Sender is used if `None` provided
        recipient: Option<String>,
    },
    /// Purchases tokens
    Purchase {
        /// Optional recipient to purchase on behalf of another address
        recipient: Option<String>,
    },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Sale info for a given asset
    #[returns(SaleResponse)]
    Sale { asset: AssetInfo },
    /// The address of the token being purchased
    #[returns(TokenAddressResponse)]
    TokenAddress {},
    #[returns(SaleAssetsResponse)]
    SaleAssets {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct SaleAssetsResponse {
    pub assets: Vec<String>,
}

#[cw_serde]
pub struct SaleResponse {
    /// The sale data if it exists
    pub sale: Option<Sale>,
}

#[cw_serde]
pub struct TokenAddressResponse {
    /// The address of the token being sold
    pub address: String,
}

#[cw_serde]
pub struct MigrateMsg {}
