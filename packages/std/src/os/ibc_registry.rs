use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Addr};
use strum_macros::AsRefStr;

use crate::{
    amp::{messages::AMPPkt, AndrAddr},
    error::ContractError,
};

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

/// Ensures that the denom starts with 'ibc/'
pub fn verify_denom(denom: &str) -> Result<(), ContractError> {
    // Ensure that the denom is formatted correctly. It should start with "ibc/"
    ensure!(
        denom.starts_with("ibc/"),
        ContractError::InvalidDenom {
            msg: Some("The denom should start with 'ibc/'".to_string()),
        }
    );
    let suffix = &denom[4..]; // Get the part after "ibc/"

    // Ensure that there are exactly 64 characters after "ibc/"
    if suffix.len() != 64 {
        return Err(ContractError::InvalidDenom {
            msg: Some("The denom must have exactly 64 characters after 'ibc/'".to_string()),
        });
    }
    Ok(())
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

#[cfg(test)]
#[test]
fn test_validate_denom() {
    // Empty denom
    let empty_denom = "".to_string();
    let err = verify_denom(&empty_denom).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidDenom {
            msg: Some("The denom should start with 'ibc/'".to_string()),
        }
    );
    // Denom that doesn't start with ibc/
    let invalid_denom = "random".to_string();
    let err = verify_denom(&invalid_denom).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidDenom {
            msg: Some("The denom should start with 'ibc/'".to_string()),
        }
    );
    // Denom that's just ibc/
    let empty_ibc_denom = "ibc/".to_string();
    let err = verify_denom(&empty_ibc_denom).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidDenom {
            msg: Some("The denom must have exactly 64 characters after 'ibc/'".to_string()),
        }
    );

    // Valid denom
    let valid_denom =
        "ibc/usdcusdcusdcusdcusdcusdcusdcusdcusdcusdcusdcusdcusdcusdcusdcusdc".to_string();
    verify_denom(&valid_denom).unwrap()
}
