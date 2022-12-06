use common::ado_base::hooks::AndromedaHook;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Event, SubMsg, Uint128};
use cw721::Expiration;

#[cw_serde]
pub struct Bid {
    pub denom: String,
    /// What the purchaser offers.
    pub bid_amount: Uint128,
    /// What the owner of the token will get if they accept (royalties deducted).
    pub remaining_amount: Uint128,
    /// The amount of tax the purchaser paid.
    pub tax_amount: Uint128,
    pub expiration: Expiration,
    pub purchaser: String,
    pub msgs: Vec<SubMsg>,
    pub events: Vec<Event>,
}

impl Bid {
    pub fn get_full_amount(&self) -> Coin {
        Coin {
            denom: self.denom.clone(),
            amount: self.bid_amount + self.tax_amount,
        }
    }
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub andromeda_cw721_contract: String,
    pub valid_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    PlaceBid {
        token_id: String,
        expiration: Expiration,
        bid_amount: Uint128,
    },
    CancelBid {
        token_id: String,
    },
    /// Restricted to Cw721 contract.
    AcceptBid {
        token_id: String,
        recipient: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaHook)]
    AndrHook(AndromedaHook),
    #[returns(BidResponse)]
    Bid { token_id: String },
    #[returns(AllBidsResponse)]
    AllBids {
        purchaser: String,
        limit: Option<u32>,
        start_after: Option<String>,
    },
}

#[cw_serde]
pub struct BidResponse {
    pub denom: String,
    pub bid_amount: Uint128,
    pub remaining_amount: Uint128,
    pub tax_amount: Uint128,
    pub expiration: Expiration,
    pub purchaser: String,
}

#[cw_serde]
pub struct AllBidsResponse {
    pub bids: Vec<BidResponse>,
}

impl From<Bid> for BidResponse {
    fn from(bid: Bid) -> BidResponse {
        BidResponse {
            denom: bid.denom,
            bid_amount: bid.bid_amount,
            remaining_amount: bid.remaining_amount,
            tax_amount: bid.tax_amount,
            expiration: bid.expiration,
            purchaser: bid.purchaser,
        }
    }
}

#[cw_serde]
pub struct MigrateMsg {}
