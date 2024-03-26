#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_non_fungible_tokens::auction::{Cw721HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::ado_base::permissioning::{Permission, PermissioningMessage};
use andromeda_std::amp::messages::AMPPkt;
use andromeda_std::{ado_base::modules::Module, amp::AndrAddr};
use cosmwasm_std::{Addr, Empty, Uint128};
use cw_multi_test::{Contract, ContractWrapper};
use cw_utils::Expiration;

pub fn mock_andromeda_auction() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_auction_instantiate_msg(
    modules: Option<Vec<Module>>,
    kernel_address: impl Into<String>,
    owner: Option<String>,
    authorized_token_addresses: Option<Vec<AndrAddr>>,
) -> InstantiateMsg {
    InstantiateMsg {
        modules,
        kernel_address: kernel_address.into(),
        owner,
        authorized_token_addresses,
    }
}

pub fn mock_start_auction(
    start_time: Option<u64>,
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

pub fn mock_authorize_token_address(
    token_address: impl Into<String>,
    expiration: Option<Expiration>,
) -> ExecuteMsg {
    ExecuteMsg::AuthorizeTokenContract {
        addr: AndrAddr::from_string(token_address.into()),
        expiration,
    }
}

pub fn mock_set_permission(actor: AndrAddr, action: String, permission: Permission) -> ExecuteMsg {
    ExecuteMsg::Permissioning(PermissioningMessage::SetPermission {
        actor,
        action,
        permission,
    })
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

pub fn mock_receive_packet(packet: AMPPkt) -> ExecuteMsg {
    ExecuteMsg::AMPReceive(packet)
}
