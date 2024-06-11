#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query, reply};
use andromeda_non_fungible_tokens::crowdfund::{
    CampaignSummaryResponse, Cw20HookMsg, ExecuteMsg, InitialCampaignConfig, InstantiateMsg,
    PresaleTierOrder, QueryMsg, RawTier, SimpleTierOrder, TierMetaData,
};
use andromeda_std::common::Milliseconds;
use andromeda_testing::{
    mock::MockApp,
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Coin, Empty, Uint128, Uint64};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockCrowdfund(Addr);
mock_ado!(MockCrowdfund, ExecuteMsg, QueryMsg);

impl MockCrowdfund {
    #[allow(clippy::too_many_arguments)]
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        campaign_config: InitialCampaignConfig,
        tiers: Vec<RawTier>,
        kernel_address: impl Into<String>,
        owner: Option<String>,
    ) -> MockCrowdfund {
        let msg = mock_crowdfund_instantiate_msg(campaign_config, tiers, kernel_address, owner);
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
    pub fn execute_add_tier(
        &self,
        sender: Addr,
        app: &mut MockApp,
        level: Uint64,
        label: String,
        price: Uint128,
        limit: Option<Uint128>,
        metadata: TierMetaData,
    ) -> ExecuteResult {
        let msg = mock_add_tier_msg(level, label, price, limit, metadata);
        self.execute(app, &msg, sender, &[])
    }

    pub fn execute_start_campaign(
        &self,
        sender: Addr,
        app: &mut MockApp,
        start_time: Option<Milliseconds>,
        end_time: Milliseconds,
        presale: Option<Vec<PresaleTierOrder>>,
    ) -> ExecuteResult {
        let msg = mock_start_campaign_msg(start_time, end_time, presale);
        self.execute(app, &msg, sender, &[])
    }

    pub fn execute_purchase(
        &self,
        sender: Addr,
        app: &mut MockApp,
        orders: Vec<SimpleTierOrder>,
        funds: Vec<Coin>,
    ) -> ExecuteResult {
        let msg = mock_purchase_msg(orders);
        self.execute(app, &msg, sender, &funds)
    }

    pub fn execute_end_campaign(&self, sender: Addr, app: &mut MockApp) -> ExecuteResult {
        let msg = mock_end_campaign_msg();
        self.execute(app, &msg, sender, &[])
    }

    pub fn execute_discard_campaign(&self, sender: Addr, app: &mut MockApp) -> ExecuteResult {
        let msg = mock_discard_campaign_msg();
        self.execute(app, &msg, sender, &[])
    }

    pub fn execute_claim(&self, sender: Addr, app: &mut MockApp) -> ExecuteResult {
        let msg = mock_claim_msg();
        self.execute(app, &msg, sender, &[])
    }

    pub fn query_campaign_summary(&self, app: &mut MockApp) -> CampaignSummaryResponse {
        let msg = QueryMsg::CampaignSummary {};
        self.query(app, msg)
    }
}

pub fn mock_andromeda_crowdfund() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query).with_reply(reply);
    Box::new(contract)
}

pub fn mock_crowdfund_instantiate_msg(
    campaign_config: InitialCampaignConfig,
    tiers: Vec<RawTier>,
    kernel_address: impl Into<String>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        campaign_config,
        tiers,
        kernel_address: kernel_address.into(),
        owner,
    }
}

pub fn mock_add_tier_msg(
    level: Uint64,
    label: String,
    price: Uint128,
    limit: Option<Uint128>,
    metadata: TierMetaData,
) -> ExecuteMsg {
    ExecuteMsg::AddTier {
        tier: RawTier {
            level,
            label,
            price,
            limit,
            metadata,
        },
    }
}

pub fn mock_start_campaign_msg(
    start_time: Option<Milliseconds>,
    end_time: Milliseconds,
    presale: Option<Vec<PresaleTierOrder>>,
) -> ExecuteMsg {
    ExecuteMsg::StartCampaign {
        start_time,
        end_time,
        presale,
    }
}

pub fn mock_purchase_msg(orders: Vec<SimpleTierOrder>) -> ExecuteMsg {
    ExecuteMsg::PurchaseTiers { orders }
}

pub fn mock_end_campaign_msg() -> ExecuteMsg {
    ExecuteMsg::EndCampaign {}
}

pub fn mock_discard_campaign_msg() -> ExecuteMsg {
    ExecuteMsg::DiscardCampaign {}
}

pub fn mock_claim_msg() -> ExecuteMsg {
    ExecuteMsg::Claim {}
}

pub fn mock_purchase_cw20_msg(orders: Vec<SimpleTierOrder>) -> Cw20HookMsg {
    Cw20HookMsg::PurchaseTiers { orders }
}
