#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_non_fungible_tokens::marketplace::{
    Cw721HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
};
use andromeda_std::{
    ado_base::rates::{Rate, RatesMessage},
    amp::messages::AMPPkt,
};
use andromeda_testing::{mock_ado, mock_contract::ExecuteResult, MockADO, MockContract};
use cosmwasm_std::{Addr, Empty, Uint128};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

pub struct MockMarketplace(Addr);
mock_ado!(MockMarketplace, ExecuteMsg, QueryMsg);

impl MockMarketplace {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut App,
        kernel_address: impl Into<String>,
        owner: Option<String>,
    ) -> MockMarketplace {
        let msg = mock_marketplace_instantiate_msg(kernel_address.into(), owner);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Marketplace Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockMarketplace(addr)
    }

    pub fn execute_buy_token(
        &self,
        app: &mut App,
        sender: Addr,
        token_address: impl Into<String>,
        token_id: impl Into<String>,
    ) -> ExecuteResult {
        self.execute(app, &mock_buy_token(token_address, token_id), sender, &[])
    }

    pub fn execute_set_rate(
        &self,
        app: &mut App,
        sender: Addr,
        action: impl Into<String>,
        rate: Rate,
    ) -> ExecuteResult {
        self.execute(app, &mock_set_rates(action, rate), sender, &[])
    }
}

pub fn mock_andromeda_marketplace() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_marketplace_instantiate_msg(
    kernel_address: String,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address,
        owner,
    }
}

pub fn mock_start_sale(price: Uint128, coin_denom: impl Into<String>) -> Cw721HookMsg {
    Cw721HookMsg::StartSale {
        price,
        coin_denom: coin_denom.into(),
        start_time: None,
        duration: None,
    }
}

pub fn mock_buy_token(token_address: impl Into<String>, token_id: impl Into<String>) -> ExecuteMsg {
    ExecuteMsg::Buy {
        token_id: token_id.into(),
        token_address: token_address.into(),
    }
}

pub fn mock_set_rates(action: impl Into<String>, rate: Rate) -> ExecuteMsg {
    ExecuteMsg::Rates(RatesMessage::SetRate {
        action: action.into(),
        rate,
    })
}

pub fn mock_receive_packet(packet: AMPPkt) -> ExecuteMsg {
    ExecuteMsg::AMPReceive(packet)
}
