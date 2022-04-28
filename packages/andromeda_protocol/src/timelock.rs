use cosmwasm_std::{Api, BlockInfo, Coin, StdError, StdResult, Storage};
use cw721::Expiration;
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{modules::address_list::AddressListModule, require};

pub const HELD_FUNDS: Map<String, Escrow> = Map::new("funds");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// Struct used to define funds being held in Escrow
pub struct Escrow {
    /// Funds being held within the Escrow
    pub coins: Vec<Coin>,
    /// Optional expiration for the Escrow
    pub expiration: Option<Expiration>,
    /// The recipient of the funds once Expiration is reached
    pub recipient: String,
}

impl Escrow {
    /// Used to check the validity of an Escrow before it is stored.
    ///
    /// * Escrowed funds cannot be empty
    /// * The Escrow recipient must be a valid address
    /// * Expiration cannot be "Never" or before current time/block
    pub fn validate(self, api: &dyn Api, block: &BlockInfo) -> StdResult<bool> {
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
                //ACK-01 Change (Check before deleting comment)
                Expiration::AtTime(time) => {
                    if time < block.time {
                        return Err(StdError::generic_err("Cannot set expiration in the past"));
                    }
                }
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
    /// An optional address list module to restrict usage of the contract
    pub address_list: Option<AddressListModule>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Hold funds in Escrow
    HoldFunds {
        expiration: Option<Expiration>,
        recipient: Option<String>,
    },
    /// Update the optional address list module
    UpdateAddressList {
        address_list: Option<AddressListModule>,
    },
    /// Release funds held in Escrow
    ReleaseFunds {},
    /// Update ownership of the contract. Only executable by the current contract owner.
    UpdateOwner {
        /// The address of the new contract owner.
        address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Queries funds held by an address
    GetLockedFunds { address: String },
    /// The current config of the contract
    GetTimelockConfig {},
    /// The current owner of the contract
    ContractOwner {},
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

/// Stores an Escrow struct for a given address. Used to store funds from an address.
pub fn hold_funds(funds: Escrow, storage: &mut dyn Storage, addr: String) -> StdResult<()> {
    require(
        // Makes sure that HELD_FUNDS is empty before allowing writing into HELD_FUNDS.
        //Decided to use unwrap instead of unwrap_or_else (correct me if wrong)
        HELD_FUNDS.may_load(storage, addr.clone()).unwrap() == None,
        StdError::generic_err("Cannot overwrite Held Funds"),
    )?;
    HELD_FUNDS.save(storage, addr, &funds)
}

/// Removes the stored Escrow struct for a given address.
pub fn release_funds(storage: &mut dyn Storage, addr: String) -> StdResult<()> {
    require(
        // Makes sure that HELD_FUNDS is NOT empty before allowing removing into HELD_FUNDS.
        //Decided to use unwrap instead of unwrap_or_else (correct me if wrong)
        HELD_FUNDS.may_load(storage, addr.clone()).unwrap() != None,
        StdError::generic_err("Cannot overwrite Held Funds"),
    )?;
    HELD_FUNDS.remove(storage, addr);
    Ok(())
}

/// Retrieves the stored Escrow struct for a given address
pub fn get_funds(storage: &dyn Storage, addr: String) -> StdResult<Option<Escrow>> {
    HELD_FUNDS.may_load(storage, addr)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::{coin, Timestamp};

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
        let block = BlockInfo {
            height: 1000,
            time: Timestamp::from_seconds(4444),
            chain_id: "foo".to_string(),
        };
        let resp = valid_escrow.validate(deps.as_ref().api, &block).unwrap();
        assert!(resp);

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
        let resp = valid_escrow.validate(deps.as_ref().api, &block).unwrap();
        assert!(resp);

        let invalid_recipient_escrow = Escrow {
            recipient: String::default(),
            coins: coins.clone(),
            expiration: Some(expiration.clone()),
        };

        let resp = invalid_recipient_escrow
            .validate(deps.as_ref().api, &block)
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
            .validate(deps.as_ref().api, &block)
            .unwrap_err();
        assert_eq!(StdError::generic_err("Cannot escrow empty funds"), resp);

        let invalid_expiration_escrow = Escrow {
            recipient: recipient.clone(),
            coins: coins.clone(),
            expiration: Some(Expiration::Never {}),
        };

        let resp = invalid_expiration_escrow
            .validate(deps.as_ref().api, &block)
            .unwrap_err();
        assert_eq!(
            StdError::generic_err("Cannot escrow funds with no expiration"),
            resp
        );
    }
}
