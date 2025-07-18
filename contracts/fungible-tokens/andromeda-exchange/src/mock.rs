#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_fungible_tokens::cw20::ExecuteMsg as Cw20ExecuteMsg;
use andromeda_fungible_tokens::exchange::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, RedeemResponse, SaleResponse,
};
use andromeda_std::{
    amp::{AndrAddr, Recipient},
    common::{denom::Asset, schedule::Schedule},
};
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{MockADO, MockContract},
};
use cosmwasm_std::{to_json_binary, Addr, Binary, Decimal256, Empty, Uint128};
use cw_multi_test::{AppResponse, Contract, ContractWrapper, Executor};

pub struct MockExchange(Addr);
mock_ado!(MockExchange, ExecuteMsg, QueryMsg);

pub fn mock_andromeda_exchange() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_exchange_instantiate_msg(
    token_address: AndrAddr,
    kernel_address: String,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        token_address,
        kernel_address,
        owner,
    }
}

impl MockExchange {
    pub fn execute_cancel_redeem(
        &self,
        app: &mut MockApp,
        sender: Addr,
        asset: Asset,
    ) -> AppResponse {
        let msg = mock_cancel_redeem_msg(asset);
        app.execute_contract(sender, self.addr().clone(), &msg, &[])
            .unwrap()
    }

    pub fn execute_cancel_sale(
        &self,
        app: &mut MockApp,
        sender: Addr,
        asset: Asset,
    ) -> AppResponse {
        let msg = mock_cancel_sale_msg(asset);
        app.execute_contract(sender, self.addr().clone(), &msg, &[])
            .unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_cw20_start_redeem(
        &self,
        app: &mut MockApp,
        sender: Addr,
        asset: Asset,
        amount: Uint128,
        exchange_rate: Decimal256,
        cw20_addr: Addr,
        schedule: Schedule,
    ) -> AppResponse {
        let msg = mock_start_redeem_cw20_msg(None, asset, exchange_rate, schedule);
        let cw20_send_msg =
            mock_cw20_send(self.addr().clone(), amount, to_json_binary(&msg).unwrap());
        app.execute_contract(sender, cw20_addr, &cw20_send_msg, &[])
            .unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_cw20_start_sale(
        &self,
        app: &mut MockApp,
        sender: Addr,
        asset: Asset,
        amount: Uint128,
        exchange_rate: Uint128,
        cw20_addr: Addr,
        schedule: Schedule,
    ) -> AppResponse {
        let msg = mock_exchange_start_sale_msg(asset, exchange_rate, None, schedule);
        let cw20_send_msg =
            mock_cw20_send(self.addr().clone(), amount, to_json_binary(&msg).unwrap());
        app.execute_contract(sender, cw20_addr, &cw20_send_msg, &[])
            .unwrap()
    }

    pub fn execute_cw20_purchase(
        &self,
        app: &mut MockApp,
        sender: Addr,
        recipient: Option<Recipient>,
        amount: Uint128,
        cw20_addr: Addr,
    ) -> AppResponse {
        let msg = mock_exchange_purchase_msg(recipient);
        let cw20_send_msg =
            mock_cw20_send(self.addr().clone(), amount, to_json_binary(&msg).unwrap());
        app.execute_contract(sender, cw20_addr, &cw20_send_msg, &[])
            .unwrap()
    }

    pub fn query_redeem(&self, app: &mut MockApp, asset: Asset) -> RedeemResponse {
        let msg = mock_redeem_query_msg(asset);
        let res: RedeemResponse = self.query(app, msg);
        res
    }

    pub fn query_sale(&self, app: &mut MockApp, asset: String) -> SaleResponse {
        let msg = mock_sale_query_msg(asset);
        let res: SaleResponse = self.query(app, msg);
        res
    }
}

pub fn mock_exchange_start_sale_msg(
    asset: Asset,
    exchange_rate: Uint128,
    recipient: Option<Recipient>,
    schedule: Schedule,
) -> Cw20HookMsg {
    Cw20HookMsg::StartSale {
        asset,
        exchange_rate,
        recipient,
        schedule,
    }
}

pub fn mock_exchange_hook_purchase_msg(recipient: Option<Recipient>) -> Cw20HookMsg {
    Cw20HookMsg::Purchase { recipient }
}

pub fn mock_exchange_purchase_msg(recipient: Option<Recipient>) -> ExecuteMsg {
    ExecuteMsg::Purchase { recipient }
}

pub fn mock_redeem_cw20_msg(recipient: Option<Recipient>) -> Cw20HookMsg {
    Cw20HookMsg::Redeem { recipient }
}

pub fn mock_replenish_redeem_cw20_msg(redeem_asset: Asset) -> Cw20HookMsg {
    Cw20HookMsg::ReplenishRedeem { redeem_asset }
}

pub fn mock_redeem_native_msg(recipient: Option<Recipient>) -> ExecuteMsg {
    ExecuteMsg::Redeem { recipient }
}

pub fn mock_replenish_redeem_native_msg(redeem_asset: Asset) -> ExecuteMsg {
    ExecuteMsg::ReplenishRedeem { redeem_asset }
}

pub fn mock_start_redeem_cw20_msg(
    recipient: Option<Recipient>,
    redeem_asset: Asset,
    exchange_rate: Decimal256,
    schedule: Schedule,
) -> Cw20HookMsg {
    Cw20HookMsg::StartRedeem {
        recipient,
        redeem_asset,
        exchange_rate,
        schedule,
    }
}

pub fn mock_cw20_send(contract: impl Into<String>, amount: Uint128, msg: Binary) -> Cw20ExecuteMsg {
    Cw20ExecuteMsg::Send {
        contract: AndrAddr::from_string(contract.into()),
        amount,
        msg,
    }
}

pub fn mock_set_redeem_condition_native_msg(
    redeem_asset: Asset,
    exchange_rate: Decimal256,
    recipient: Option<Recipient>,
    schedule: Schedule,
) -> ExecuteMsg {
    ExecuteMsg::StartRedeem {
        redeem_asset,
        exchange_rate,
        recipient,
        schedule,
    }
}

pub fn mock_redeem_query_msg(asset: Asset) -> QueryMsg {
    QueryMsg::Redeem { asset }
}

pub fn mock_sale_query_msg(asset: String) -> QueryMsg {
    QueryMsg::Sale { asset }
}

pub fn mock_cancel_sale_msg(asset: Asset) -> ExecuteMsg {
    ExecuteMsg::CancelSale { asset }
}

pub fn mock_cancel_redeem_msg(asset: Asset) -> ExecuteMsg {
    ExecuteMsg::CancelRedeem { asset }
}
