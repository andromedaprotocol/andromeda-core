#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_non_fungible_tokens::auction::{
    AuctionIdsResponse, AuctionStateResponse, Bid, BidsResponse, Cw721HookMsg, ExecuteMsg,
    InstantiateMsg, QueryMsg,
};
use andromeda_std::ado_base::permissioning::{Permission, PermissioningMessage};
use andromeda_std::ado_base::rates::{Rate, RatesMessage};
use andromeda_std::amp::messages::AMPPkt;
use andromeda_std::amp::AndrAddr;
use andromeda_std::amp::Recipient;
use andromeda_std::common::denom::{Asset, PermissionAction};
use andromeda_std::common::expiration::Expiry;
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Coin, Empty, Uint128};
use cw20::Cw20ReceiveMsg;
use cw_multi_test::{AppResponse, Contract, ContractWrapper, Executor};

pub struct MockAuction(Addr);
mock_ado!(MockAuction, ExecuteMsg, QueryMsg);

impl MockAuction {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,

        kernel_address: impl Into<String>,
        owner: Option<String>,
    ) -> MockAuction {
        let msg = mock_auction_instantiate_msg(kernel_address, owner, None, None);
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
        app: &mut MockApp,
        sender: Addr,
        start_time: Option<Expiry>,
        end_time: Expiry,
        buy_now_price: Option<Uint128>,
        coin_denom: Asset,
        min_bid: Option<Uint128>,
        min_raise: Option<Uint128>,
        whitelist: Option<Vec<Addr>>,
        recipient: Option<Recipient>,
    ) -> AppResponse {
        let msg = mock_start_auction(
            start_time,
            end_time,
            buy_now_price,
            coin_denom,
            min_bid,
            min_raise,
            whitelist,
            recipient,
        );
        app.execute_contract(sender, self.addr().clone(), &msg, &[])
            .unwrap()
    }

    pub fn execute_place_bid(
        &self,
        app: &mut MockApp,
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
        app: &mut MockApp,
        sender: Addr,
        token_id: String,
        token_address: String,
    ) -> ExecuteResult {
        let msg = mock_claim_auction(token_id, token_address);
        self.execute(app, &msg, sender, &[])
    }

    pub fn execute_authorize_token_address(
        &self,
        app: &mut MockApp,
        sender: Addr,
        token_address: impl Into<String>,
        expiration: Option<Expiry>,
    ) -> ExecuteResult {
        let msg = mock_authorize_token_address(token_address, expiration);
        self.execute(app, &msg, sender, &[])
    }

    pub fn execute_add_rate(
        &self,
        app: &mut MockApp,
        sender: Addr,
        action: String,
        rate: Rate,
    ) -> ExecuteResult {
        self.execute(app, &mock_set_rate_msg(action, rate), sender, &[])
    }

    pub fn execute_set_permission(
        &self,
        app: &mut MockApp,
        sender: Addr,
        actors: Vec<AndrAddr>,
        action: String,
        permission: Permission,
    ) -> ExecuteResult {
        let msg = mock_set_permission(actors, action, permission);
        self.execute(app, &msg, sender, &[])
    }

    pub fn query_auction_ids(
        &self,
        app: &mut MockApp,
        token_id: String,
        token_address: String,
    ) -> Vec<Uint128> {
        let msg = mock_get_auction_ids(token_id, token_address);
        let res: AuctionIdsResponse = self.query(app, msg);
        res.auction_ids
    }

    pub fn query_auction_state(
        &self,
        app: &mut MockApp,
        auction_id: Uint128,
    ) -> AuctionStateResponse {
        let msg = mock_get_auction_state(auction_id);
        self.query(app, msg)
    }

    pub fn query_bids(&self, app: &mut MockApp, auction_id: Uint128) -> Vec<Bid> {
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
    kernel_address: impl Into<String>,
    owner: Option<String>,
    authorized_token_addresses: Option<Vec<AndrAddr>>,
    authorized_cw20_addresses: Option<Vec<AndrAddr>>,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address: kernel_address.into(),
        owner,
        authorized_token_addresses,
        authorized_cw20_addresses,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn mock_start_auction(
    start_time: Option<Expiry>,
    end_time: Expiry,
    buy_now_price: Option<Uint128>,
    coin_denom: Asset,
    min_bid: Option<Uint128>,
    min_raise: Option<Uint128>,
    whitelist: Option<Vec<Addr>>,
    recipient: Option<Recipient>,
) -> Cw721HookMsg {
    Cw721HookMsg::StartAuction {
        start_time,
        end_time,
        buy_now_price,
        coin_denom,
        min_bid,
        min_raise,
        whitelist,
        recipient,
    }
}

pub fn mock_auction_cw20_receive(msg: Cw20ReceiveMsg) -> ExecuteMsg {
    ExecuteMsg::Receive(msg)
}

pub fn mock_authorize_token_address(
    token_address: impl Into<String>,
    expiration: Option<Expiry>,
) -> ExecuteMsg {
    ExecuteMsg::AuthorizeContract {
        action: PermissionAction::SendNft,
        addr: AndrAddr::from_string(token_address.into()),
        expiration,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn mock_update_auction(
    token_id: String,
    token_address: String,
    start_time: Option<Expiry>,
    end_time: Expiry,
    coin_denom: Asset,
    min_bid: Option<Uint128>,
    min_raise: Option<Uint128>,
    buy_now_price: Option<Uint128>,
    whitelist: Option<Vec<Addr>>,
    recipient: Option<Recipient>,
) -> ExecuteMsg {
    ExecuteMsg::UpdateAuction {
        token_id,
        token_address,
        start_time,
        end_time,
        coin_denom,
        whitelist,
        min_bid,
        min_raise,
        buy_now_price,
        recipient,
    }
}

pub fn mock_set_rate_msg(action: String, rate: Rate) -> ExecuteMsg {
    ExecuteMsg::Rates(RatesMessage::SetRate { action, rate })
}

pub fn mock_set_permission(
    actors: Vec<AndrAddr>,
    action: String,
    permission: Permission,
) -> ExecuteMsg {
    ExecuteMsg::Permissioning(PermissioningMessage::SetPermission {
        actors,
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
