use andromeda_std::amp::{AndrAddr, Recipient};
use andromeda_std::common::denom::Asset;
use andromeda_std::common::expiration::Expiry;
use andromeda_std::common::{MillisecondsExpiration, OrderBy};
use andromeda_std::{andr_exec, andr_instantiate, andr_query};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;
use cw721::{Cw721ReceiveMsg, Expiration};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub authorized_token_addresses: Option<Vec<AndrAddr>>,
    pub authorized_cw20_address: Option<AndrAddr>,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    ReceiveNft(Cw721ReceiveMsg),
    // for cw20
    Receive(Cw20ReceiveMsg),
    /// Places a bid on the current auction for the given token_id. The previous largest bid gets
    /// automatically sent back to the bidder when they are outbid.
    PlaceBid {
        token_id: String,
        token_address: String,
    },
    BuyNow {
        token_id: String,
        token_address: String,
    },
    /// Transfers the given token to the auction winner's address once the auction is over.
    Claim {
        token_id: String,
        token_address: String,
    },
    UpdateAuction {
        token_id: String,
        token_address: String,
        start_time: Option<Expiry>,
        end_time: Expiry,
        coin_denom: Asset,
        whitelist: Option<Vec<Addr>>,
        min_bid: Option<Uint128>,
        min_raise: Option<Uint128>,
        recipient: Option<Recipient>,
    },
    CancelAuction {
        token_id: String,
        token_address: String,
    },
    /// Restricted to owner
    AuthorizeTokenContract {
        addr: AndrAddr,
        expiration: Option<Expiry>,
    },
    /// Restricted to owner
    DeauthorizeTokenContract {
        addr: AndrAddr,
    },
}

#[cw_serde]
pub enum Cw721HookMsg {
    /// Starts a new auction with the given parameters. The auction info can be modified before it
    /// has started but is immutable after that.
    StartAuction {
        /// Start time in milliseconds since epoch
        start_time: Option<Expiry>,
        /// Duration in milliseconds
        end_time: Expiry,
        coin_denom: Asset,
        buy_now_price: Option<Uint128>,
        min_bid: Option<Uint128>,
        min_raise: Option<Uint128>,
        whitelist: Option<Vec<Addr>>,
        recipient: Option<Recipient>,
    },
}
#[cw_serde]
pub enum Cw20HookMsg {
    PlaceBid {
        token_id: String,
        token_address: String,
    },
    BuyNow {
        token_id: String,
        token_address: String,
    },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Gets the latest auction state for the given token. This will either be the current auction
    /// if there is one in progress or the last completed one.
    #[returns(AuctionStateResponse)]
    LatestAuctionState {
        token_id: String,
        token_address: String,
    },
    /// Gets the auction state for the given auction id.
    #[returns(AuctionStateResponse)]
    AuctionState { auction_id: Uint128 },
    /// Gets the auction ids for the given token.
    #[returns(AuctionIdsResponse)]
    AuctionIds {
        token_id: String,
        token_address: String,
    },
    /// Gets all of the auction infos for a given token address.
    #[returns(AuctionInfo)]
    AuctionInfosForAddress {
        token_address: String,
        start_after: Option<String>,
        limit: Option<u64>,
    },
    /// Gets all of the authorized addresses for the auction
    #[returns(AuthorizedAddressesResponse)]
    AuthorizedAddresses {
        start_after: Option<String>,
        limit: Option<u32>,
        order_by: Option<OrderBy>,
    },

    /// Gets the bids for the given auction id. Start_after starts indexing at 0.
    #[returns(BidsResponse)]
    Bids {
        auction_id: Uint128,
        start_after: Option<u64>,
        limit: Option<u64>,
        order_by: Option<OrderBy>,
    },

    #[returns(IsCancelledResponse)]
    IsCancelled {
        token_id: String,
        token_address: String,
    },

    /// Returns true only if the auction has been cancelled, the token has been claimed, or the end time has expired
    #[returns(IsClosedResponse)]
    IsClosed {
        token_id: String,
        token_address: String,
    },

    #[returns(IsClaimedResponse)]
    IsClaimed {
        token_id: String,
        token_address: String,
    },
}

#[cw_serde]
#[derive(Default)]
pub struct AuctionInfo {
    pub auction_ids: Vec<Uint128>,
    pub token_address: String,
    pub token_id: String,
}

impl AuctionInfo {
    pub fn last(&self) -> Option<&Uint128> {
        self.auction_ids.last()
    }

    pub fn push(&mut self, e: Uint128) {
        self.auction_ids.push(e)
    }
}

impl From<TokenAuctionState> for AuctionStateResponse {
    fn from(token_auction_state: TokenAuctionState) -> AuctionStateResponse {
        AuctionStateResponse {
            start_time: token_auction_state.start_time,
            end_time: token_auction_state.end_time,
            high_bidder_addr: token_auction_state.high_bidder_addr.to_string(),
            high_bidder_amount: token_auction_state.high_bidder_amount,
            coin_denom: token_auction_state.coin_denom,
            uses_cw20: token_auction_state.uses_cw20,
            auction_id: token_auction_state.auction_id,
            whitelist: token_auction_state.whitelist,
            is_cancelled: token_auction_state.is_cancelled,
            min_bid: token_auction_state.min_bid,
            min_raise: token_auction_state.min_raise,
            owner: token_auction_state.owner,
            recipient: token_auction_state.recipient,
        }
    }
}

#[cw_serde]
pub struct TokenAuctionState {
    pub start_time: Expiration,
    pub end_time: Expiration,
    pub high_bidder_addr: Addr,
    pub high_bidder_amount: Uint128,
    pub buy_now_price: Option<Uint128>,
    pub coin_denom: String,
    pub auction_id: Uint128,
    pub whitelist: Option<Vec<Addr>>,
    pub min_bid: Option<Uint128>,
    pub min_raise: Option<Uint128>,
    pub owner: String,
    pub token_id: String,
    pub token_address: String,
    pub is_cancelled: bool,
    pub is_bought: bool,
    pub uses_cw20: bool,
    pub recipient: Option<Recipient>,
}

#[cw_serde]
pub struct Bid {
    pub bidder: String,
    pub amount: Uint128,
    pub timestamp: MillisecondsExpiration,
}

#[cw_serde]
pub struct AuctionStateResponse {
    pub start_time: Expiration,
    pub end_time: Expiration,
    pub high_bidder_addr: String,
    pub high_bidder_amount: Uint128,
    pub auction_id: Uint128,
    pub coin_denom: String,
    pub uses_cw20: bool,
    pub whitelist: Option<Vec<Addr>>,
    pub min_bid: Option<Uint128>,
    pub min_raise: Option<Uint128>,
    pub is_cancelled: bool,
    pub owner: String,
    pub recipient: Option<Recipient>,
}

#[cw_serde]
pub struct AuthorizedAddressesResponse {
    pub addresses: Vec<String>,
}

#[cw_serde]
pub struct AuctionIdsResponse {
    pub auction_ids: Vec<Uint128>,
}

#[cw_serde]
pub struct BidsResponse {
    pub bids: Vec<Bid>,
}

#[cw_serde]
pub struct IsCancelledResponse {
    pub is_cancelled: bool,
}

#[cw_serde]
pub struct IsClosedResponse {
    pub is_closed: bool,
}

#[cw_serde]
pub struct IsClaimedResponse {
    pub is_claimed: bool,
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
