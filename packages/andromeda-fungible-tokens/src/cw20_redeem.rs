use andromeda_std::{
    amp::AndrAddr,
    andr_exec, andr_instantiate, andr_query,
    common::{expiration::Expiry, MillisecondsDuration},
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use cw_asset::AssetInfo;
use cw_utils::Expiration;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the CW20 token to be redeemed
    pub token_address: AndrAddr,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Cancels an ongoing sale
    #[attrs(restricted)]
    CancelRedemptionClause {},
    /// Receive for CW20 tokens, used for purchasing and starting sales
    #[attrs(nonpayable)]
    Receive(Cw20ReceiveMsg),
    /// Starts a redemption with the native funds sent
    SetRedemptionClause {
        exchange_rate: Uint128,
        start_time: Option<Expiry>,
        duration: Option<MillisecondsDuration>,
    },
    Redeem {},
}

/// Struct used to define a token redemption. The asset used for the redemption is defined as the key for the storage map.
#[cw_serde]
pub struct RedemptionClause {
    /// Recipient of the redeemed tokens
    pub recipient: String,
    /// The asset that may be used to redeem the tokens with
    pub asset: AssetInfo,
    /// The rate at which to exchange tokens (amount of exchanged asset to purchase sale asset)
    pub exchange_rate: Uint128,
    /// The amount for redemption at the given rate
    pub amount: Uint128,
    /// The time when the redemption starts
    pub start_time: Expiration,
    /// The time when the redemption ends
    pub end_time: Expiration,
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Starts a sale
    StartRedemptionClause {
        /// The amount of the above asset required to purchase a single token
        exchange_rate: Uint128,
        start_time: Option<Expiry>,
        duration: Option<MillisecondsDuration>,
    },
    /// Purchases tokens
    Redeem {},
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Redemption info for a given asset
    #[returns(RedemptionResponse)]
    RedemptionClause {},
    /// The address of the token being redeemed
    #[returns(TokenAddressResponse)]
    TokenAddress {},
    #[returns(RedemptionAssetResponse)]
    RedemptionAsset {},
    #[returns(RedemptionAssetBalanceResponse)]
    RedemptionAssetBalance {},
}

#[cw_serde]
pub struct RedemptionAssetResponse {
    pub asset: String,
}

#[cw_serde]
pub struct RedemptionAssetBalanceResponse {
    pub balance: Uint128,
}

#[cw_serde]
pub struct RedemptionResponse {
    /// The redemption data if it exists
    pub redemption: Option<RedemptionClause>,
}

#[cw_serde]
pub struct TokenAddressResponse {
    /// The address of the token being redeemed
    pub address: String,
}
