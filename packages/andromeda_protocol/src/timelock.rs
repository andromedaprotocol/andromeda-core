use cosmwasm_std::{Api, BlockInfo, Coin};
use cw721::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::communication::{AndromedaMsg, AndromedaQuery, Recipient};
use crate::error::ContractError;
use crate::{modules::address_list::AddressListModule, require};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// Struct used to define funds being held in Escrow
pub struct Escrow {
    /// Funds being held within the Escrow
    pub coins: Vec<Coin>,
    /// Optional expiration for the Escrow
    pub expiration: Option<Expiration>,
    /// The recipient of the funds once Expiration is reached
    pub recipient: Recipient,
}

impl Escrow {
    /// Used to check the validity of an Escrow before it is stored.
    ///
    /// * Escrowed funds cannot be empty
    /// * The Escrow recipient must be a valid address
    /// * Expiration cannot be "Never" or before current time/block
    pub fn validate(&self, api: &dyn Api, block: &BlockInfo) -> Result<(), ContractError> {
        require(!self.coins.is_empty(), ContractError::EmptyFunds {})?;
        require(
            api.addr_validate(&self.recipient.get_addr()).is_ok(),
            ContractError::InvalidAddress {},
        )?;

        //Ok(!self.is_expired(block)?)
        if self.is_expired(block)? && self.expiration.is_some() {
            return Err(ContractError::ExpirationInPast {});
        }
        return Ok(());
    }

    pub fn is_expired(&self, block: &BlockInfo) -> Result<bool, ContractError> {
        match self.expiration {
            None => return Ok(true),
            Some(expiration) => match expiration {
                Expiration::AtTime(t) => {
                    if t > block.time {
                        return Ok(false);
                    }
                }
                Expiration::AtHeight(h) => {
                    if h > block.height {
                        return Ok(false);
                    }
                }
                _ => return Err(ContractError::ExpirationNotSpecified {}),
            },
        }
        Ok(true)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// An optional address list module to restrict usage of the contract
    pub address_list: Option<AddressListModule>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    /// Hold funds in Escrow
    HoldFunds {
        expiration: Option<Expiration>,
        recipient: Option<Recipient>,
    },
    /// Update the optional address list module
    UpdateAddressList {
        address_list: Option<AddressListModule>,
    },
    /// Release funds held in Escrow
    ReleaseFunds {
        recipient_addr: Option<String>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    /// Queries funds held by an address
    GetLockedFunds {
        address: String,
    },
    /// The current config of the contract
    GetTimelockConfig {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetLockedFundsResponse {
    pub funds: Option<Escrow>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetTimelockConfigResponse {
    pub address_list: Option<AddressListModule>,
    pub address_list_contract: Option<String>,
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::{coin, Timestamp};

    use super::*;

    #[test]
    fn test_validate() {
        let deps = mock_dependencies(&[]);
        let expiration = Expiration::AtHeight(1500);
        let coins = vec![coin(100u128, "uluna")];
        let recipient = Recipient::Addr("owner".into());

        let valid_escrow = Escrow {
            recipient: recipient.clone(),
            coins: coins.clone(),
            expiration: Some(expiration),
        };
        let block = BlockInfo {
            height: 1000,
            time: Timestamp::from_seconds(4444),
            chain_id: "foo".to_string(),
        };
        valid_escrow.validate(deps.as_ref().api, &block).unwrap();

        let valid_escrow = Escrow {
            recipient: recipient.clone(),
            coins: coins.clone(),
            expiration: None,
        };
        let block = BlockInfo {
            height: 1000,
            time: Timestamp::from_seconds(3333),
            chain_id: "foo".to_string(),
        };
        valid_escrow.validate(deps.as_ref().api, &block).unwrap();

        let invalid_recipient_escrow = Escrow {
            recipient: Recipient::Addr(String::default()),
            coins: coins.clone(),
            expiration: Some(expiration),
        };

        let resp = invalid_recipient_escrow
            .validate(deps.as_ref().api, &block)
            .unwrap_err();
        assert_eq!(ContractError::InvalidAddress {}, resp);

        let invalid_coins_escrow = Escrow {
            recipient: recipient.clone(),
            coins: vec![],
            expiration: Some(expiration),
        };

        let resp = invalid_coins_escrow
            .validate(deps.as_ref().api, &block)
            .unwrap_err();
        assert_eq!(ContractError::EmptyFunds {}, resp);

        let invalid_expiration_escrow = Escrow {
            recipient: recipient.clone(),
            coins: coins.clone(),
            expiration: Some(Expiration::Never {}),
        };

        let resp = invalid_expiration_escrow
            .validate(deps.as_ref().api, &block)
            .unwrap_err();
        assert_eq!(ContractError::ExpirationNotSpecified {}, resp);

        let invalid_time_escrow = Escrow {
            recipient: recipient.clone(),
            coins: coins.clone(),
            expiration: Some(Expiration::AtHeight(10)),
        };
        let block = BlockInfo {
            height: 1000,
            time: Timestamp::from_seconds(4444),
            chain_id: "foo".to_string(),
        };
        assert_eq!(
            ContractError::ExpirationInPast {},
            invalid_time_escrow
                .validate(deps.as_ref().api, &block)
                .unwrap_err()
        );

        let invalid_time_escrow = Escrow {
            recipient: recipient.clone(),
            coins: coins.clone(),
            expiration: Some(Expiration::AtTime(Timestamp::from_seconds(100))),
        };
        assert_eq!(
            ContractError::ExpirationInPast {},
            invalid_time_escrow
                .validate(deps.as_ref().api, &block)
                .unwrap_err()
        );
    }
}
