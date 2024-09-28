use crate::{
    amp::{messages::AMPPkt, AndrAddr},
    error::ContractError,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Addr};
use sha2::{Digest, Sha256};
use strum_macros::AsRefStr;

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
impl DenomInfo {
    pub fn get_ibc_denom(&self) -> String {
        // Concatenate the path and base with "/"
        let input = format!("{}/{}", self.path, self.base_denom);

        // Hash the concatenated string using SHA-256
        let hash = Sha256::digest(input.as_bytes());
        // Return the result in the format "ibc/<SHA-256 hash in hex>"
        format!("ibc/{:X}", hash).to_lowercase()
    }
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
pub fn verify_denom(denom: &str, denom_info: &DenomInfo) -> Result<(), ContractError> {
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
    // Ensure that the hash and base match the provided denom
    let hashed_denom = denom_info.get_ibc_denom();
    ensure!(
        denom.to_lowercase() == hashed_denom.to_lowercase(),
        ContractError::InvalidDenom {
            msg: Some(format!(
                "Denom hash does not match. Expected: {expected}, Actual: {actual}",
                expected = hashed_denom,
                actual = denom
            )),
        }
    );

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
    let default_denom_info = DenomInfo {
        path: "path".to_string(),
        base_denom: "base_denom".to_string(),
    };
    let err = verify_denom(&empty_denom, &default_denom_info).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidDenom {
            msg: Some("The denom should start with 'ibc/'".to_string()),
        }
    );
    // Denom that doesn't start with ibc/
    let invalid_denom = "random".to_string();
    let err = verify_denom(&invalid_denom, &default_denom_info).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidDenom {
            msg: Some("The denom should start with 'ibc/'".to_string()),
        }
    );
    // Denom that's just ibc/
    let empty_ibc_denom = "ibc/".to_string();
    let err = verify_denom(&empty_ibc_denom, &default_denom_info).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidDenom {
            msg: Some("The denom must have exactly 64 characters after 'ibc/'".to_string()),
        }
    );

    let valid_denom = default_denom_info.get_ibc_denom();
    verify_denom(&valid_denom, &default_denom_info).unwrap()
}
