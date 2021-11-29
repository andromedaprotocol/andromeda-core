use cosmwasm_std::{Api, Coin, StdError, StdResult, Storage};
use cw721::Expiration;
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{modules::address_list::AddressListModule, require::require};

pub const HELD_FUNDS: Map<String, Escrow> = Map::new("funds");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Escrow {
    pub coins: Vec<Coin>,
    pub expiration: Option<Expiration>,
    pub recipient: String,
}

impl Escrow {
    pub fn validate(self, api: &dyn Api) -> StdResult<bool> {
        require(
            self.coins.len() > 0,
            StdError::generic_err("Cannot escrow empty funds"),
        )?;
        require(
            api.addr_validate(&self.recipient.clone()).is_ok(),
            StdError::generic_err("Escrow recipient must be a valid address"),
        )?;

        if self.expiration.is_some() {
            match self.expiration.unwrap() {
                Expiration::Never {} => {
                    return Err(StdError::generic_err(
                        "Cannot escrow funds with no expiration",
                    ));
                }
                _ => {}
            }
        }

        Ok(true)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub address_list: Option<AddressListModule>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    HoldFunds {
        expiration: Option<Expiration>,
        recipient: Option<String>,
    },
    UpdateAddressList {
        address_list: Option<AddressListModule>,
    },
    ReleaseFunds {},
    UpdateOwner {
        address: String,
    },
    UpdateOperator {
        operators: Vec<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetLockedFunds { address: String },
    GetTimelockConfig {},
    ContractOwner {},
    IsOperator { address: String },
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

pub fn hold_funds(funds: Escrow, storage: &mut dyn Storage, addr: String) -> StdResult<()> {
    HELD_FUNDS.save(storage, addr.clone(), &funds)
}

pub fn release_funds(storage: &mut dyn Storage, addr: String) {
    HELD_FUNDS.remove(storage, addr.clone());
}

pub fn get_funds(storage: &dyn Storage, addr: String) -> StdResult<Option<Escrow>> {
    HELD_FUNDS.may_load(storage, addr)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::coin;
    use cosmwasm_std::testing::mock_dependencies;

    use super::*;

    #[test]
    fn test_validate() {
        let deps = mock_dependencies(&[]);
        let expiration = Expiration::AtHeight(1);
        let coins = vec![coin(100u128, "uluna")];
        let recipient = String::from("owner");

        let valid_escrow = Escrow {
            recipient: recipient.clone(),
            coins: coins.clone(),
            expiration: Some(expiration.clone()),
        };

        let resp = valid_escrow.validate(deps.as_ref().api).unwrap();
        assert!(resp);

        let valid_escrow = Escrow {
            recipient: recipient.clone(),
            coins: coins.clone(),
            expiration: None,
        };

        let resp = valid_escrow.validate(deps.as_ref().api).unwrap();
        assert!(resp);

        let invalid_recipient_escrow = Escrow {
            recipient: String::default(),
            coins: coins.clone(),
            expiration: Some(expiration.clone()),
        };

        let resp = invalid_recipient_escrow
            .validate(deps.as_ref().api)
            .unwrap_err();
        assert_eq!(
            StdError::generic_err("Escrow recipient must be a valid address"),
            resp
        );

        let invalid_coins_escrow = Escrow {
            recipient: recipient.clone(),
            coins: vec![],
            expiration: Some(expiration.clone()),
        };

        let resp = invalid_coins_escrow
            .validate(deps.as_ref().api)
            .unwrap_err();
        assert_eq!(StdError::generic_err("Cannot escrow empty funds"), resp);

        let invalid_expiration_escrow = Escrow {
            recipient: recipient.clone(),
            coins: coins.clone(),
            expiration: Some(Expiration::Never {}),
        };

        let resp = invalid_expiration_escrow
            .validate(deps.as_ref().api)
            .unwrap_err();
        assert_eq!(
            StdError::generic_err("Cannot escrow funds with no expiration"),
            resp
        );
    }
}
