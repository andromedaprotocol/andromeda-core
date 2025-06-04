use andromeda_std::{
    amp::{messages::AMPCtx, AndrAddr, Recipient},
    common::denom::Asset,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Decimal, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use andromeda_socket::astroport::{AssetEntry, AssetInfo, PairType};

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
pub struct ForwardReplyState {
    /// Forward Recipient
    pub recipient: Recipient,
    /// Refund Address
    pub refund_addr: AndrAddr,
    /// Amp ctx to be used for ibc communication
    pub amp_ctx: Option<AMPCtx>,
    /// Offered asset to the astroport
    pub from_asset: Asset,
    /// Asked asset returning from the astroport
    pub to_asset: Asset,
}
#[cw_serde]
pub struct LiquidityProvisionState {
    /// The assets to deposit as liquidity
    pub assets: Vec<AssetEntry>,
    /// The slippage tolerance for the liquidity provision
    pub slippage_tolerance: Option<Decimal>,
    /// Determines whether the LP tokens minted for the user are auto staked in the Generator contract
    pub auto_stake: Option<bool>,
    /// The receiver of LP tokens (if different from sender)
    pub receiver: Option<String>,
}

// Astroport factory message format
#[cw_serde]
pub enum AstroportFactoryExecuteMsg {
    CreatePair {
        pair_type: PairType,
        asset_infos: Vec<AssetInfo>,
        init_params: Option<Binary>,
    },
    WithdrawLiquidity {},
}

pub const FORWARD_REPLY_STATE: Item<ForwardReplyState> = Item::new("forward_reply_state");

pub const SWAP_ROUTER: Item<AndrAddr> = Item::new("swap_router");

pub const FACTORY: Item<AndrAddr> = Item::new("factory");

pub const PREV_BALANCE: Item<Uint128> = Item::new("prev_balance");

// Store the created pair address
pub const PAIR_ADDRESS: Item<AndrAddr> = Item::new("pair_address");

pub const LP_PAIR_ADDRESS: Item<AndrAddr> = Item::new("lp_pair_address");

// Store liquidity provision parameters during pair creation
pub const LIQUIDITY_PROVISION_STATE: Item<LiquidityProvisionState> =
    Item::new("liquidity_provision_state");

// Store withdrawal information during liquidity withdrawal
pub const WITHDRAWAL_STATE: Item<String> = Item::new("withdrawal_receiver");
