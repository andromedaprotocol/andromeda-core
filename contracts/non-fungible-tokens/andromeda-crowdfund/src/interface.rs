use andromeda_non_fungible_tokens::crowdfund::{
    CampaignSummaryResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};
use cw_orch_daemon::{DaemonBase, Wallet};

pub const CONTRACT_ID: &str = "crowdfund";

contract_interface!(CrowdfundContract, CONTRACT_ID, "andromeda_crowdfund.wasm");

type Chain = DaemonBase<Wallet>;

impl CrowdfundContract<Chain> {
    pub fn campaign_summary(&self) -> CampaignSummaryResponse {
        let query_msg = QueryMsg::CampaignSummary {};
        self.query(&query_msg).unwrap()
    }
}
