use std::str::FromStr;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Addr, Api, Uint128};
use schemars::JsonSchema;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{ado_base::ownership::OwnershipMessage, error::ContractError};

#[cw_serde]
pub struct InstantiateMsg {
    pub kernel_address: String,
    pub owner: Option<String>,
}

#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
pub enum ExecuteMsg {
    Publish {
        code_id: u64,
        ado_type: String,
        action_fees: Option<Vec<ActionFee>>,
        version: String,
        publisher: Option<String>,
    },
    Unpublish {
        ado_type: String,
        version: String,
    },
    UpdateActionFees {
        ado_type: String,
        action_fees: Vec<ActionFee>,
    },
    RemoveActionFees {
        ado_type: String,
        actions: Vec<String>,
    },
    UpdatePublisher {
        ado_type: String,
        publisher: String,
    },
    // Base message
    Ownership(OwnershipMessage),
}

#[cw_serde]
pub struct ActionFee {
    pub action: String,
    pub asset: String,
    pub amount: Uint128,
    pub receiver: Option<Addr>,
}

impl ActionFee {
    pub fn new(action: String, asset: String, amount: Uint128) -> Self {
        Self {
            action,
            asset,
            amount,
            receiver: None,
        }
    }

    pub fn with_receive(&self, receiver: Addr) -> Self {
        Self {
            action: self.action.clone(),
            asset: self.asset.clone(),
            amount: self.amount,
            receiver: Some(receiver),
        }
    }

    /// Valiades the provided asset for an action fee
    /// An asset is valid if it fits the format "cw20:address" or "native:denom"
    /// If the asset type is cw20 the address is also validated
    /// TODO: Add denom validation in future cosmwasm version
    pub fn validate_asset(&self, api: &dyn Api) -> Result<(), ContractError> {
        let asset_split = self.asset.split(':').collect::<Vec<&str>>();
        // Ensure asset is in the format "cw20:address" or "native:denom"
        // This is double validated as the asset type in the ADODB contract for fees is validated as cw20:* or native:*
        ensure!(
            asset_split.len() == 2 && !asset_split.is_empty(),
            ContractError::InvalidAsset {
                asset: self.asset.clone()
            }
        );
        let asset_type = asset_split[0];
        ensure!(
            asset_type == "cw20" || asset_type == "native",
            ContractError::InvalidAsset {
                asset: self.asset.clone()
            }
        );

        if asset_type == "cw20" {
            api.addr_validate(asset_split[1])?;
        }

        Ok(())
    }

    /// Gets the asset string without the asset type
    ///
    /// i.e. **cw20:address** would return **"address"** or native:denom would return **"denom"**
    pub fn get_asset_string(&self) -> Result<&str, ContractError> {
        ensure!(
            self.asset.contains(':'),
            ContractError::InvalidAsset {
                asset: self.asset.clone()
            }
        );
        match self.asset.split(':').last() {
            Some(asset) => Ok(asset),
            None => Err(ContractError::InvalidAsset {
                asset: self.asset.clone(),
            }),
        }
    }
}

#[cw_serde]
pub struct ADOMetadata {
    pub publisher: String,
    pub latest_version: String,
}

#[cw_serde]
#[derive(cw_orch::QueryFns, QueryResponses)]
pub enum QueryMsg {
    #[returns(u64)]
    CodeId { key: String },
    // #[returns(Vec<u64>)]
    // UnpublishedCodeIds {},
    #[returns(IsUnpublishedCodeIdResponse)]
    IsUnpublishedCodeId { code_id: u64 },
    #[returns(Option<String>)]
    #[serde(rename = "ado_type")]
    ADOType { code_id: u64 },
    #[returns(Vec<String>)]
    #[serde(rename = "all_ado_types")]
    AllADOTypes {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(Vec<String>)]
    #[serde(rename = "ado_versions")]
    ADOVersions {
        ado_type: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    // #[returns(Vec<String>)]
    // #[serde(rename = "unpublished_ado_versions")]
    // UnpublishedADOVersions { ado_type: String },
    #[returns(Option<ADOMetadata>)]
    #[serde(rename = "ado_metadata")]
    ADOMetadata { ado_type: String },
    #[returns(Option<ActionFee>)]
    ActionFee { ado_type: String, action: String },
    #[returns(Option<ActionFee>)]
    ActionFeeByCodeId { code_id: u64, action: String },
    // Base queries
    #[returns(crate::ado_base::version::VersionResponse)]
    Version {},
    #[serde(rename = "type")]
    #[returns(crate::ado_base::ado_type::TypeResponse)]
    ContractType {},
    #[returns(crate::ado_base::ownership::ContractOwnerResponse)]
    Owner {},
    #[returns(crate::ado_base::kernel_address::KernelAddressResponse)]
    KernelAddress {},
}

#[cw_serde]
pub struct IsUnpublishedCodeIdResponse {
    pub is_unpublished_code_id: bool,
}

#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema,
)]
pub struct ADOVersion(String);

impl ADOVersion {
    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    #[inline]
    pub fn into_string(self) -> String {
        self.0
    }

    #[inline]
    pub fn from_string(string: impl Into<String>) -> ADOVersion {
        ADOVersion(string.into())
    }

    #[inline]
    pub fn from_type(ado_type: impl Into<String>) -> ADOVersion {
        ADOVersion(ado_type.into())
    }

    #[inline]
    pub fn with_version(&self, version: impl Into<String>) -> ADOVersion {
        let mut ado_version = self.clone();
        // Remove any previous version string if present
        ado_version.0 = ado_version.get_type();
        ado_version.0.push('@');
        ado_version.0.push_str(&version.into());
        ado_version
    }

    /// Validates a given ADOVersion
    ///
    /// A valid ADOVersion must:
    /// 1. Be non-empty
    /// 2. Have at most one `@` symbol
    ///
    /// ### Examples
    /// - `ado_type@0.1.0`
    /// - `ado_type`
    /// - `ado_type@latest`
    pub fn validate(&self) -> bool {
        !self.clone().into_string().is_empty()
            && self.clone().into_string().split('@').count() <= 2
            && (self.get_version() == "latest"
                || Version::from_str(self.get_version().as_str()).is_ok())
    }

    /// Gets the version for the given ADOVersion
    ///
    /// Returns `"latest"` if no version provided
    pub fn get_version(&self) -> String {
        match self
            .clone()
            .into_string()
            .split('@')
            .collect::<Vec<&str>>()
            .len()
        {
            1 => "latest".to_string(),
            _ => self.clone().into_string().split('@').collect::<Vec<&str>>()[1].to_string(),
        }
    }

    /// Gets the type for the given ADOVersion
    pub fn get_type(&self) -> String {
        self.clone().into_string().split('@').collect::<Vec<&str>>()[0].to_string()
    }

    /// Gets the type for the given ADOVersion
    pub fn get_tuple(&self) -> (String, String) {
        (self.get_type(), self.get_version())
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::mock_dependencies;

    use super::*;

    #[test]
    fn test_validate() {
        let ado_version = ADOVersion::from_string("valid_version");
        assert!(ado_version.validate());

        let ado_version = ADOVersion::from_string("valid_version@0.1.0");
        assert!(ado_version.validate());

        let ado_version = ADOVersion::from_string("");
        assert!(!ado_version.validate());

        let ado_version = ADOVersion::from_string("not@valid@version");
        assert!(!ado_version.validate());
    }

    #[test]
    fn test_get_version() {
        let ado_version = ADOVersion::from_string("ado_type");
        assert_eq!(ado_version.get_version(), "latest");

        let ado_version = ADOVersion::from_string("ado_type@0.1.0");
        assert_eq!(ado_version.get_version(), "0.1.0");

        let ado_version = ADOVersion::from_string("ado_type@latest");
        assert_eq!(ado_version.get_version(), "latest");
    }

    #[test]
    fn test_get_type() {
        let ado_version = ADOVersion::from_string("ado_type");
        assert_eq!(ado_version.get_type(), "ado_type");

        let ado_version = ADOVersion::from_string("ado_type@0.1.0");
        assert_eq!(ado_version.get_type(), "ado_type");

        let ado_version = ADOVersion::from_string("ado_type@latest");
        assert_eq!(ado_version.get_type(), "ado_type");
    }

    #[test]
    fn test_action_fee_asset() {
        let deps = mock_dependencies();
        let action_fee = ActionFee::new(
            "action".to_string(),
            "cw20:address".to_string(),
            Uint128::zero(),
        );
        assert!(action_fee.validate_asset(deps.as_ref().api).is_ok());

        let action_fee = ActionFee::new(
            "action".to_string(),
            "native:denom".to_string(),
            Uint128::zero(),
        );
        assert!(action_fee.validate_asset(deps.as_ref().api).is_ok());

        let action_fee =
            ActionFee::new("action".to_string(), "cw20:aw".to_string(), Uint128::zero());
        assert!(action_fee.validate_asset(deps.as_ref().api).is_err());

        let action_fee =
            ActionFee::new("action".to_string(), "invalid".to_string(), Uint128::zero());
        assert!(action_fee.validate_asset(deps.as_ref().api).is_err());
    }
}
