#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_non_fungible_tokens::marketplace::{
    Cw721HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
};
use andromeda_std::ado_base::permissioning::Permission;
use andromeda_std::ado_base::permissioning::PermissioningMessage;
use andromeda_std::ado_base::rates::Rate;
use andromeda_std::ado_base::rates::RatesMessage;
use andromeda_std::amp::messages::AMPPkt;

use andromeda_std::amp::AndrAddr;
use andromeda_std::amp::Recipient;
use andromeda_std::common::denom::Asset;
use andromeda_std::common::expiration::Expiry;
use andromeda_std::common::MillisecondsDuration;
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
        owner: Option<String>,
        authorized_cw20_address: Option<AndrAddr>,
    ) -> MockMarketplace {
        let msg =
            mock_marketplace_instantiate_msg(kernel_address.into(), owner, authorized_cw20_address);
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

    pub fn execute_set_rate(
        &self,
        app: &mut MockApp,
        sender: Addr,
        action: impl Into<String>,
        rate: Rate,
    ) -> ExecuteResult {
        self.execute(app, &mock_set_rates(action, rate), sender, &[])
    }

    pub fn execute_set_permissions(
        &self,
        app: &mut MockApp,
        sender: Addr,
        actor: AndrAddr,
        action: impl Into<String>,
        permission: Permission,
    ) -> ExecuteResult {
        self.execute(
            app,
            &mock_set_permissions(actor, action, permission),
            sender,
            &[],
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_update_sale(
        &self,
        app: &mut MockApp,
        sender: Addr,
        token_address: impl Into<String>,
        token_id: impl Into<String>,
        coin_denom: Asset,
        price: Uint128,
        recipient: Option<Recipient>,
    ) -> ExecuteResult {
        self.execute(
            app,
            &mock_update_sale(
                token_id.into(),
                token_address.into(),
                coin_denom,
                price,
                recipient,
            ),
            sender,
            &[],
        )
    }
}

pub fn mock_andromeda_marketplace() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_marketplace_instantiate_msg(
    kernel_address: String,
    owner: Option<String>,
    authorized_cw20_address: Option<AndrAddr>,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address,
        owner,
        authorized_cw20_address,
    }
}

pub fn mock_start_sale(
    price: Uint128,
    coin_denom: Asset,
    duration: Option<MillisecondsDuration>,
    start_time: Option<Expiry>,
    recipient: Option<Recipient>,
) -> Cw721HookMsg {
    Cw721HookMsg::StartSale {
        price,
        coin_denom,
        start_time,
        duration,
        recipient,
    }
}

pub fn mock_update_sale(
    token_id: String,
    token_address: String,
    coin_denom: Asset,
    price: Uint128,
    recipient: Option<Recipient>,
) -> ExecuteMsg {
    ExecuteMsg::UpdateSale {
        token_id,
        token_address,
        price,
        coin_denom,
        recipient,
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

pub fn mock_set_permissions(
    actor: AndrAddr,
    action: impl Into<String>,
    permission: Permission,
) -> ExecuteMsg {
    ExecuteMsg::Permissioning(PermissioningMessage::SetPermission {
        actor,
        action: action.into(),
        permission,
    })
}

pub fn mock_receive_packet(packet: AMPPkt) -> ExecuteMsg {
    ExecuteMsg::AMPReceive(packet)
}
