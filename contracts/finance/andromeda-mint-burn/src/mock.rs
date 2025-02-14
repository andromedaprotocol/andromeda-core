#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_finance::mint_burn::{
    ExecuteMsg, GetOrderInfoResponse, GetOrdersByStatusResponse, GetUserDepositedOrdersReponse,
    InstantiateMsg, OrderStatus, QueryMsg, Resource, ResourceRequirement,
};
use andromeda_std::amp::AndrAddr;
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Coin, Empty, Uint128};
use cw20::Cw20ReceiveMsg;
use cw721::Cw721ReceiveMsg;
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockMintBurn(Addr);
mock_ado!(MockMintBurn, ExecuteMsg, QueryMsg);

impl MockMintBurn {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        kernel_address: String,
        owner: Option<String>,
        authorized_nft_addresses: Option<Vec<AndrAddr>>,
        authorized_cw20_addresses: Option<Vec<AndrAddr>>,
    ) -> MockMintBurn {
        let msg = mock_mint_burn_instantiate_msg(
            kernel_address,
            owner,
            authorized_nft_addresses,
            authorized_cw20_addresses,
        );
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Mint Burn Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockMintBurn(Addr::unchecked(addr))
    }

    pub fn execute_create_order(
        &self,
        app: &mut MockApp,
        sender: Addr,
        requirements: Vec<ResourceRequirement>,
        output: Resource,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = ExecuteMsg::CreateOrder {
            requirements,
            output,
        };
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn execute_cancel_order(
        &self,
        app: &mut MockApp,
        sender: Addr,
        order_id: Uint128,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = ExecuteMsg::CancelOrder { order_id };
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn execute_receive_cw721(
        &self,
        app: &mut MockApp,
        sender: Addr,
        msg: Cw721ReceiveMsg,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = ExecuteMsg::ReceiveNft(msg);
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn execute_receive_cw20(
        &self,
        app: &mut MockApp,
        sender: Addr,
        msg: Cw20ReceiveMsg,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = ExecuteMsg::ReceiveCw20(msg);
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn query_order_info(&self, app: &mut MockApp, order_id: Uint128) -> GetOrderInfoResponse {
        let msg = QueryMsg::GetOrderInfo { order_id };
        let res: GetOrderInfoResponse = self.query(app, msg);
        res
    }

    pub fn query_orders_by_status(
        &self,
        app: &mut MockApp,
        status: OrderStatus,
        limit: Option<Uint128>,
    ) -> GetOrdersByStatusResponse {
        let msg = QueryMsg::GetOrdersByStatus { status, limit };
        let res: GetOrdersByStatusResponse = self.query(app, msg);
        res
    }

    pub fn query_user_deposited_orders(
        &self,
        app: &mut MockApp,
        user: AndrAddr,
        limit: Option<Uint128>,
    ) -> GetUserDepositedOrdersReponse {
        let msg = QueryMsg::GetUserDepositedOrders { user, limit };
        let res: GetUserDepositedOrdersReponse = self.query(app, msg);
        res
    }
}

pub fn mock_andromeda_mint_burn() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_mint_burn_instantiate_msg(
    kernel_address: String,
    owner: Option<String>,
    authorized_nft_addresses: Option<Vec<AndrAddr>>,
    authorized_cw20_addresses: Option<Vec<AndrAddr>>,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address,
        owner,
        authorized_nft_addresses,
        authorized_cw20_addresses,
    }
}
