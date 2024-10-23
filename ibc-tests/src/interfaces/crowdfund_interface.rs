use crate::contract_interface;
use andromeda_non_fungible_tokens::crowdfund;
use andromeda_non_fungible_tokens::crowdfund::PresaleTierOrder;
use andromeda_non_fungible_tokens::crowdfund::SimpleTierOrder;
use andromeda_non_fungible_tokens::crowdfund::Tier;
use andromeda_non_fungible_tokens::crowdfund::TierMetaData;
use andromeda_non_fungible_tokens::crowdfund::{CampaignSummaryResponse, Cw20HookMsg};
use andromeda_std::ado_base::MigrateMsg;
use andromeda_std::common::Milliseconds;
use andromeda_testing_e2e::mock::MockAndromeda;
use cosmwasm_std::Uint128;
use cosmwasm_std::Uint64;
use cw_orch::interface;
use cw_orch::prelude::*;
use cw_orch_daemon::DaemonBase;
use cw_orch_daemon::Wallet;

contract_interface!(
    CrowdfundContract,
    andromeda_crowdfund,
    crowdfund,
    "andromeda_crowdfund_contract",
    "crowdfund"
);

type Chain = DaemonBase<Wallet>;

impl CrowdfundContract<Chain> {
    pub fn execute_add_tier(
        &self,
        label: String,
        level: Uint64,
        price: Uint128,
        limit: Option<Uint128>,
        metadata: TierMetaData,
    ) {
        self.execute(
            &crowdfund::ExecuteMsg::AddTier {
                tier: Tier {
                    label,
                    level,
                    price,
                    limit,
                    metadata,
                },
            },
            None,
        )
        .unwrap();
    }

    pub fn execute_start_campaign(
        &self,
        start_time: Option<Milliseconds>,
        end_time: Milliseconds,
        presale: Option<Vec<PresaleTierOrder>>,
    ) {
        self.execute(
            &crowdfund::ExecuteMsg::StartCampaign {
                start_time,
                end_time,
                presale,
            },
            None,
        )
        .unwrap();
    }
    pub fn execute_purchase(&self, orders: Vec<SimpleTierOrder>, funds: Option<&[Coin]>) {
        self.execute(&crowdfund::ExecuteMsg::PurchaseTiers { orders }, funds)
            .unwrap();
    }

    pub fn execute_end_campaign(&self) {
        self.execute(&crowdfund::ExecuteMsg::EndCampaign {}, None)
            .unwrap();
    }

    pub fn execute_claim(&self) {
        self.execute(&crowdfund::ExecuteMsg::Claim {}, None)
            .unwrap();
    }

    pub fn query_campaign_summary(&self) -> CampaignSummaryResponse {
        let query_msg = crowdfund::QueryMsg::CampaignSummary {};
        self.query(&query_msg).unwrap()
    }
}

pub fn purchase_cw20_msg(orders: Vec<SimpleTierOrder>) -> Cw20HookMsg {
    Cw20HookMsg::PurchaseTiers { orders }
}

pub fn prepare(
    daemon: &DaemonBase<Wallet>,
    andr_os: &MockAndromeda,
) -> CrowdfundContract<DaemonBase<Wallet>> {
    let crowdfund_contract = CrowdfundContract::new(daemon.clone());
    crowdfund_contract.upload().unwrap();

    let MockAndromeda { adodb_contract, .. } = &andr_os;

    adodb_contract.clone().execute_publish(
        crowdfund_contract.code_id().unwrap(),
        "crowdfund".to_string(),
        "0.1.0".to_string(),
    );
    crowdfund_contract
}
