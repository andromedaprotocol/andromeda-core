#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_non_fungible_tokens::auction::{
    AuctionIdsResponse, AuctionStateResponse, Bid, BidsResponse, Cw721HookMsg, ExecuteMsg,
    InstantiateMsg, QueryMsg,
};
use andromeda_std::ado_base::permissioning::{Permission, PermissioningMessage};
use andromeda_std::amp::messages::AMPPkt;
use andromeda_std::{ado_base::modules::Module, amp::AndrAddr};
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Coin, Empty, Uint128};
use cw20::Expiration;
use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};

pub struct MockAuction(Addr);
mock_ado!(MockAuction, ExecuteMsg, QueryMsg);

impl MockAuction {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut App,
        modules: Option<Vec<Module>>,
        kernel_address: impl Into<String>,
        owner: Option<String>,
    ) -> MockAuction {
        let msg = mock_auction_instantiate_msg(modules, kernel_address, owner, None);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Auction Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockAuction(Addr::unchecked(addr))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_start_auction(
        &self,
        app: &mut App,
        sender: Addr,
        start_time: u64,
        duration: u64,
        coin_denom: String,
        min_bid: Option<Uint128>,
        whitelist: Option<Vec<Addr>>,
    ) -> AppResponse {
        let msg = mock_start_auction(start_time, duration, coin_denom, min_bid, whitelist);
        app.execute_contract(sender, self.addr().clone(), &msg, &[])
            .unwrap()
    }

    pub fn execute_place_bid(
        &self,
        app: &mut App,
        sender: Addr,
        token_id: String,
        token_address: String,
        funds: &[Coin],
    ) -> AppResponse {
        let msg = mock_place_bid(token_id, token_address);
        app.execute_contract(sender, self.addr().clone(), &msg, funds)
            .unwrap()
    }

    pub fn execute_claim_auction(
        &self,
        app: &mut App,
        sender: Addr,
        token_id: String,
        token_address: String,
    ) -> ExecuteResult {
        let msg = mock_claim_auction(token_id, token_address);
        self.execute(app, &msg, sender, &[])
    }

    pub fn query_auction_ids(
        &self,
        app: &mut App,
        token_id: String,
        token_address: String,
    ) -> Vec<Uint128> {
        let msg = mock_get_auction_ids(token_id, token_address);
        let res: AuctionIdsResponse = self.query(app, msg);
        res.auction_ids
    }

    pub fn query_auction_state(&self, app: &mut App, auction_id: Uint128) -> AuctionStateResponse {
        let msg = mock_get_auction_state(auction_id);
        self.query(app, msg)
    }

    pub fn query_bids(&self, app: &mut App, auction_id: Uint128) -> Vec<Bid> {
        let msg = mock_get_bids(auction_id);
        let res: BidsResponse = self.query(app, msg);
        res.bids
    }
}

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
