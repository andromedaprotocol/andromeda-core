use andromeda_std::{
    amp::AndrAddr,
    andr_exec, andr_instantiate, andr_query,
    common::{expiration::Expiry, Milliseconds, MillisecondsDuration},
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use cw_asset::AssetInfo;

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
    #[attrs(restricted)]
    CancelSale { asset: AssetInfo },
    /// Cancels an ongoing redeem
    #[attrs(restricted)]
    CancelRedeem { asset: AssetInfo },
    /// Purchases tokens with native funds
    Purchase { recipient: Option<String> },
    /// Starts a redeem
    StartRedeem {
        /// The accepted asset for redemption
        redeem_asset: AssetInfo,
        /// The rate at which to exchange tokens (amount of exchanged asset to purchase sale asset)
        exchange_rate: Decimal,
        /// The recipient of the sale proceeds
        recipient: Option<String>,
        /// The time when the sale starts
        start_time: Option<Expiry>,
        /// The time when the sale ends
        end_time: Option<Milliseconds>,
    },

    Redeem {
        /// Optional recipient to redeem on behalf of another address
        recipient: Option<String>,
    },
    /// Receive for CW20 tokens, used for purchasing and starting sales
    #[attrs(nonpayable)]
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
    /// The time when the sale starts
    pub start_time: Milliseconds,
    /// The time when the sale ends
    pub end_time: Option<Milliseconds>,
}

/// Struct used to define a token sale. The asset used for the sale is defined as the key for the storage map.
#[cw_serde]
pub struct Redeem {
    /// The asset that will be given in return for the redeemed asset
    pub asset: AssetInfo,
    /// The rate at which to exchange tokens (amount of exchanged asset to purchase sale asset)
    pub exchange_rate: Decimal,
    /// The amount for sale at the given rate
    pub amount: Uint128,
    /// The amount paid out
    pub amount_paid_out: Uint128,
    /// The recipient of the sale proceeds
    pub recipient: String,
    /// The time when the sale starts
    pub start_time: Milliseconds,
    /// The time when the sale ends
    pub end_time: Option<Milliseconds>,
}

#[cw_serde]
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
        start_time: Option<Expiry>,
        duration: Option<MillisecondsDuration>,
    },
    /// Purchases tokens
    Purchase {
        /// Optional recipient to purchase on behalf of another address
        recipient: Option<String>,
    },
    /// Starts a redeem
    StartRedeem {
        /// The accepted asset for redemption
        redeem_asset: AssetInfo,
        /// The rate at which to exchange tokens (amount of exchanged asset to purchase sale asset)
        exchange_rate: Decimal,
        /// The recipient of the sale proceeds
        recipient: Option<String>,
        /// The time when the sale starts
        start_time: Option<Expiry>,
        /// The time when the sale ends
        end_time: Option<Milliseconds>,
    },
    /// Redeems tokens
    Redeem {
        /// Optional recipient to redeem on behalf of another address
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
    /// Redeem info
    #[returns(RedeemResponse)]
    Redeem { asset: String },
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
pub struct RedeemResponse {
    /// The redeem data if it exists
    pub redeem: Option<Redeem>,
}

#[cw_serde]
pub struct TokenAddressResponse {
    /// The address of the token being sold
    pub address: String,
}
