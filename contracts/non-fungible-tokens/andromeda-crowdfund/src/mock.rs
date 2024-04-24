#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query, reply};
use andromeda_non_fungible_tokens::{
    crowdfund::{CrowdfundMintMsg, ExecuteMsg, InstantiateMsg, QueryMsg},
    cw721::TokenExtension,
};
use andromeda_std::{
    ado_base::modules::Module,
    amp::{AndrAddr, Recipient},
    common::{Milliseconds, MillisecondsExpiration},
};
use andromeda_testing::{
    mock::MockApp,
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Coin, Empty, Uint128};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockCrowdfund(Addr);
mock_ado!(MockCrowdfund, ExecuteMsg, QueryMsg);

impl MockCrowdfund {
    #[allow(clippy::too_many_arguments)]
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        token_address: AndrAddr,
        can_mint_after_sale: bool,
        modules: Option<Vec<Module>>,
        kernel_address: impl Into<String>,
        owner: Option<String>,
    ) -> MockCrowdfund {
        let msg = mock_crowdfund_instantiate_msg(
            token_address,
            can_mint_after_sale,
            modules,
            kernel_address,
            owner,
        );
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Andromeda Crowdfund Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockCrowdfund(Addr::unchecked(addr))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_start_sale(
        &self,
        sender: Addr,
        app: &mut MockApp,
        start_time: Option<Milliseconds>,
        end_time: Milliseconds,
        price: Coin,
        min_tokens_sold: Uint128,
        max_amount_per_wallet: Option<u32>,
        recipient: Recipient,
    ) -> ExecuteResult {
        let msg = mock_start_crowdfund_msg(
            start_time,
            end_time,
            price,
            min_tokens_sold,
            max_amount_per_wallet,
            recipient,
        );
        self.execute(app, &msg, sender, &[])
    }

    pub fn execute_end_sale(
        &self,
        sender: Addr,
        app: &mut MockApp,
        limit: Option<u32>,
    ) -> ExecuteResult {
        let msg = mock_end_crowdfund_msg(limit);
        self.execute(app, &msg, sender, &[])
    }

    pub fn execute_mint(
        &self,
        sender: Addr,
        app: &mut MockApp,
        token_id: String,
        extension: TokenExtension,
        token_uri: Option<String>,
        owner: Option<String>,
    ) -> ExecuteResult {
        let msg = ExecuteMsg::Mint(vec![mock_crowdfund_mint_msg(
            token_id, extension, token_uri, owner,
        )]);
        self.execute(app, &msg, sender, &[])
    }

    pub fn execute_quick_mint(
        &self,
        sender: Addr,
        app: &mut MockApp,
        amount: u32,
        publisher: String,
    ) -> ExecuteResult {
        let msg = mock_crowdfund_quick_mint_msg(amount, publisher);
        self.execute(app, &msg, sender, &[])
    }

    pub fn execute_purchase(
        &self,
        sender: Addr,
        app: &mut MockApp,
        number_of_tokens: Option<u32>,
        funds: &[Coin],
    ) -> ExecuteResult {
        let msg = mock_purchase_msg(number_of_tokens);
        self.execute(app, &msg, sender, funds)
    }
}

pub fn mock_andromeda_crowdfund() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query).with_reply(reply);
    Box::new(contract)
}

pub fn mock_crowdfund_instantiate_msg(
    token_address: AndrAddr,
    can_mint_after_sale: bool,
    modules: Option<Vec<Module>>,
    kernel_address: impl Into<String>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        token_address,
        can_mint_after_sale,
        modules,
        kernel_address: kernel_address.into(),
        owner,
    }
}

pub fn mock_start_crowdfund_msg(
    start_time: Option<Milliseconds>,
    end_time: Milliseconds,
    price: Coin,
    min_tokens_sold: Uint128,
    max_amount_per_wallet: Option<u32>,
    recipient: Recipient,
) -> ExecuteMsg {
    ExecuteMsg::StartSale {
        start_time: start_time.map(MillisecondsExpiration::from),
        end_time: end_time.into(),
        price,
        min_tokens_sold,
        max_amount_per_wallet,
        recipient,
    }
}

pub fn mock_end_crowdfund_msg(limit: Option<u32>) -> ExecuteMsg {
    ExecuteMsg::EndSale { limit }
}

pub fn mock_crowdfund_mint_msg(
    token_id: String,
    extension: TokenExtension,
    token_uri: Option<String>,
    owner: Option<String>,
) -> CrowdfundMintMsg {
    CrowdfundMintMsg {
        token_id,
        owner,
        token_uri,
        extension,
    }
}

pub fn mock_crowdfund_quick_mint_msg(amount: u32, publisher: String) -> ExecuteMsg {
    let mut mint_msgs: Vec<CrowdfundMintMsg> = Vec::new();
    for i in 0..amount {
        let extension = TokenExtension {
            publisher: publisher.clone(),
        };

        let msg = mock_crowdfund_mint_msg(i.to_string(), extension, None, None);
        mint_msgs.push(msg);
    }

    ExecuteMsg::Mint(mint_msgs)
}

pub fn mock_purchase_msg(number_of_tokens: Option<u32>) -> ExecuteMsg {
    ExecuteMsg::Purchase { number_of_tokens }
}

pub fn mock_query_ado_base_version() -> QueryMsg {
    QueryMsg::ADOBaseVersion {}
}

pub fn mock_query_ado_version() -> QueryMsg {
    QueryMsg::Version {}
}
