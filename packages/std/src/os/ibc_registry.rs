use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use strum_macros::AsRefStr;

use crate::amp::{messages::AMPPkt, AndrAddr};

#[cw_serde]
pub struct InstantiateMsg {
    pub kernel_address: Addr,
    pub owner: Option<String>,
    pub service_address: AndrAddr,
}
#[cw_serde]
pub struct DenomInfo {
    pub path: String,
    pub base_denom: String,
}
#[cw_serde]
pub struct IBCDenomInfo {
    pub denom: String,
    pub denom_info: DenomInfo,
}

#[cw_serde]
#[derive(AsRefStr)]
pub enum ExecuteMsg {
    /// Receives an AMP Packet for relaying
    #[serde(rename = "amp_receive")]
    AMPReceive(AMPPkt),
    StoreDenomInfo {
        ibc_denom_info: Vec<IBCDenomInfo>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(DenomInfoResponse)]
    DenomInfo { denom: String },
    #[returns(AllDenomInfoResponse)]
    AllDenomInfo {
        limit: Option<u64>, // Defaults to 100,
        start_after: Option<u64>,
    },
}

#[cw_serde]
pub struct DenomInfoResponse {
    pub denom_info: DenomInfo,
}

#[cw_serde]
pub struct AllDenomInfoResponse {
    pub denom_info: Vec<DenomInfo>,
}
