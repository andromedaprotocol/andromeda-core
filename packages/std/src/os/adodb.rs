use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cw_serde]
pub struct InstantiateMsg {
    pub kernel_address: String,
    pub owner: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Publish {
        code_id: u64,
        ado_type: String,
        action_fees: Option<Vec<ActionFee>>,
        version: String,
        publisher: Option<String>,
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
}

#[cw_serde]
pub struct ADOMetadata {
    pub publisher: String,
    pub latest_version: String,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(u64)]
    CodeId { key: String },
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
    #[returns(Option<ADOMetadata>)]
    #[serde(rename = "ado_metadata")]
    ADOMetadata { ado_type: String },
    #[returns(Option<ActionFee>)]
    ActionFee { ado_type: String, action: String },
    #[returns(Option<ActionFee>)]
    ActionFeeByCodeId { code_id: u64, action: String },
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
        !self.clone().into_string().is_empty() && self.clone().into_string().split('@').count() <= 2
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
}
