#![cfg(not(target_arch = "wasm32"))]

use crate::contract::{execute, instantiate, query};
use andromeda_non_fungible_tokens::auction::{Cw721HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};
use common::ado_base::modules::Module;
use cosmwasm_std::{Addr, Empty, Uint128};
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_auction() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_auction_instantiate_msg(modules: Option<Vec<Module>>) -> InstantiateMsg {
    InstantiateMsg { modules }
}

pub fn mock_start_auction(
    start_time: u64,
    duration: u64,
    coin_denom: String,
    min_bid: Option<Uint128>,
    whitelist: Option<Vec<Addr>>,
) -> Cw721HookMsg {
    Cw721HookMsg::StartAuction {
        start_time,
        duration,
        coin_denom,
        min_bid,
        whitelist,
    }
}

pub fn mock_get_auction_ids(token_id: String, token_address: String) -> QueryMsg {
    QueryMsg::AuctionIds {
        token_id,
        token_address,
    }
}

pub fn mock_get_auction_state(auction_id: Uint128) -> QueryMsg {
    QueryMsg::AuctionState { auction_id }
}

pub fn mock_place_bid(token_id: String, token_address: String) -> ExecuteMsg {
    ExecuteMsg::PlaceBid {
        token_id,
        token_address,
    }
}

pub fn mock_get_bids(auction_id: Uint128) -> QueryMsg {
    QueryMsg::Bids {
        auction_id,
        start_after: None,
        limit: None,
        order_by: None,
    }
}

pub fn mock_claim_auction(token_id: String, token_address: String) -> ExecuteMsg {
    ExecuteMsg::Claim {
        token_id,
        token_address,
    }
}
