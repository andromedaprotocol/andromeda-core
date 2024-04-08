#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_non_fungible_tokens::marketplace::{
    Cw721HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
};
use andromeda_std::amp::messages::AMPPkt;

use andromeda_std::amp::AndrAddr;
use andromeda_std::common::Milliseconds;
use andromeda_std::{ado_base::modules::Module, amp::Recipient};
use andromeda_testing::{
    mock::MockApp, mock_ado, mock_contract::ExecuteResult, MockADO, MockContract,
};
use cosmwasm_std::{Addr, Empty, Uint128};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockMarketplace(Addr);
mock_ado!(MockMarketplace, ExecuteMsg, QueryMsg);

impl MockMarketplace {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        kernel_address: impl Into<String>,
        modules: Option<Vec<Module>>,
        owner: Option<String>,
        authorized_cw20_address: Option<AndrAddr>,
    ) -> MockMarketplace {
        let msg = mock_marketplace_instantiate_msg(
            kernel_address.into(),
            modules,
            owner,
            authorized_cw20_address,
        );
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
        app: &mut MockApp,
        sender: Addr,
        token_address: impl Into<String>,
        token_id: impl Into<String>,
    ) -> ExecuteResult {
        self.execute(app, &mock_buy_token(token_address, token_id), sender, &[])
    }
}

pub fn mock_andromeda_marketplace() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_marketplace_instantiate_msg(
    kernel_address: String,
    modules: Option<Vec<Module>>,
    owner: Option<String>,
    authorized_cw20_address: Option<AndrAddr>,
) -> InstantiateMsg {
    InstantiateMsg {
        modules,
        kernel_address,
        owner,
        authorized_cw20_address,
    }
}

pub fn mock_start_sale(
    price: Uint128,
    coin_denom: impl Into<String>,
    uses_cw20: bool,
    duration: Option<Milliseconds>,
    start_time: Option<Milliseconds>,
    recipient: Option<Recipient>,
) -> Cw721HookMsg {
    Cw721HookMsg::StartSale {
        price,
        coin_denom: coin_denom.into(),
        start_time,
        duration,
        uses_cw20,
        recipient,
    }
}

pub fn mock_buy_token(token_address: impl Into<String>, token_id: impl Into<String>) -> ExecuteMsg {
    ExecuteMsg::Buy {
        token_id: token_id.into(),
        token_address: token_address.into(),
    }
}

pub fn mock_receive_packet(packet: AMPPkt) -> ExecuteMsg {
    ExecuteMsg::AMPReceive(packet)
}
