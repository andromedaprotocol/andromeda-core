#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_fungible_tokens::cw20::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::ado_base::rates::Rate;
use andromeda_std::ado_base::rates::RatesMessage;
use andromeda_std::amp::AndrAddr;
use andromeda_testing::mock_contract::ExecuteResult;
use andromeda_testing::MockADO;
use andromeda_testing::MockContract;
use andromeda_testing::{mock::MockApp, mock_ado};
use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::to_json_binary;
use cosmwasm_std::{Addr, Binary, Empty, Uint128};
use cw20::BalanceResponse;
use cw20::MinterResponse;
use cw_multi_test::Executor;
use cw_multi_test::{Contract, ContractWrapper};
pub struct MockCW20(Addr);
mock_ado!(MockCW20, ExecuteMsg, QueryMsg);

impl MockCW20 {
    #[allow(clippy::too_many_arguments)]
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        owner: Option<String>,
        name: String,
        symbol: String,
        decimals: u8,
        initial_balances: Vec<cw20::Cw20Coin>,
        mint: Option<MinterResponse>,
        kernel_address: String,
    ) -> MockCW20 {
        let msg = mock_cw20_instantiate_msg(
            owner,
            name,
            symbol,
            decimals,
            initial_balances,
            mint,
            kernel_address,
        );
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "CW20 Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockCW20(addr)
    }

    pub fn execute_send(
        &self,
        app: &mut MockApp,
        sender: Addr,
        contract: impl Into<String>,
        amount: Uint128,
        msg: &impl Serialize,
    ) -> ExecuteResult {
        self.execute(
            app,
            &mock_cw20_send(contract, amount, to_json_binary(msg).unwrap()),
            sender,
            &[],
        )
    }

    pub fn execute_send_from(
        &self,
        app: &mut MockApp,
        sender: Addr,
        contract: impl Into<String>,
        amount: Uint128,
        owner: String,
        msg: &impl Serialize,
    ) -> ExecuteResult {
        self.execute(
            app,
            &mock_cw20_send_from(contract, amount, owner, to_json_binary(msg).unwrap()),
            sender,
            &[],
        )
    }

    pub fn execute_transfer_from(
        &self,
        app: &mut MockApp,
        sender: Addr,
        recipient: impl Into<String>,
        amount: Uint128,
        owner: String,
    ) -> ExecuteResult {
        self.execute(
            app,
            &mock_cw20_transfer_from(recipient, amount, owner),
            sender,
            &[],
        )
    }

    pub fn execute_increase_allowance(
        &self,
        app: &mut MockApp,
        sender: Addr,
        spender: String,
        amount: Uint128,
    ) -> ExecuteResult {
        self.execute(
            app,
            &mock_cw20_increase_allowance(spender, amount),
            sender,
            &[],
        )
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

    pub fn query_balance(&self, app: &MockApp, address: impl Into<String>) -> Uint128 {
        self.query::<BalanceResponse>(app, mock_get_cw20_balance(address))
            .balance
    }
}

pub fn mock_andromeda_cw20() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_minter(minter: String, cap: Option<Uint128>) -> MinterResponse {
    MinterResponse { minter, cap }
}

#[allow(clippy::too_many_arguments)]
pub fn mock_cw20_instantiate_msg(
    owner: Option<String>,
    name: String,
    symbol: String,
    decimals: u8,
    initial_balances: Vec<cw20::Cw20Coin>,
    mint: Option<MinterResponse>,
    kernel_address: String,
) -> InstantiateMsg {
    InstantiateMsg {
        name,
        symbol,
        decimals,
        initial_balances,
        mint,
        marketing: None,
        kernel_address,
        owner,
    }
}

pub fn mock_get_cw20_balance(address: impl Into<String>) -> QueryMsg {
    QueryMsg::Balance {
        address: address.into(),
    }
}
pub fn mock_get_version() -> QueryMsg {
    QueryMsg::Version {}
}

pub fn mock_cw20_send(contract: impl Into<String>, amount: Uint128, msg: Binary) -> ExecuteMsg {
    ExecuteMsg::Send {
        contract: AndrAddr::from_string(contract.into()),
        amount,
        msg,
    }
}

pub fn mock_cw20_send_from(
    contract: impl Into<String>,
    amount: Uint128,
    owner: String,
    msg: Binary,
) -> ExecuteMsg {
    ExecuteMsg::SendFrom {
        contract: AndrAddr::from_string(contract.into()),
        amount,
        msg,
        owner,
    }
}

pub fn mock_cw20_transfer_from(
    contract: impl Into<String>,
    amount: Uint128,
    owner: String,
) -> ExecuteMsg {
    ExecuteMsg::TransferFrom {
        recipient: AndrAddr::from_string(contract.into()),
        amount,
        owner,
    }
}

pub fn mock_cw20_transfer(recipient: AndrAddr, amount: Uint128) -> ExecuteMsg {
    ExecuteMsg::Transfer { recipient, amount }
}

pub fn mock_cw20_increase_allowance(spender: String, amount: Uint128) -> ExecuteMsg {
    ExecuteMsg::IncreaseAllowance {
        spender,
        amount,
        expires: None,
    }
}

pub fn mock_set_rate_msg(action: String, rate: Rate) -> ExecuteMsg {
    ExecuteMsg::Rates(RatesMessage::SetRate { action, rate })
}
