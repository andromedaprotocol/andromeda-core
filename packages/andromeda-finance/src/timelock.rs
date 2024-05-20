use andromeda_std::{
    amp::recipient::Recipient,
    andr_exec, andr_instantiate, andr_query,
    common::{merge_coins, MillisecondsExpiration},
    error::ContractError,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Api, BlockInfo, Coin};

#[cw_serde]
/// Enum used to specify the condition which must be met in order for the Escrow to unlock.
pub enum EscrowCondition {
    /// Requires a given time or block height to be reached.
    Expiration(MillisecondsExpiration),
    /// Requires a minimum amount of funds to be deposited.
    MinimumFunds(Vec<Coin>),
}

#[cw_serde]
/// Struct used to define funds being held in Escrow
pub struct Escrow {
    /// Funds being held within the Escrow
    pub coins: Vec<Coin>,
    /// Optional condition for the Escrow
    pub condition: Option<EscrowCondition>,
    /// The recipient of the funds once Condition is satisfied
    pub recipient: Recipient,
    /// Used for indexing.
    pub recipient_addr: String,
}

impl Escrow {
    /// Used to check the validity of an Escrow before it is stored.
    ///
    /// * Escrowed funds cannot be empty
    /// * The Escrow recipient must be a valid address
    /// * Expiration cannot be "Never" or before current time/block
    pub fn validate(&self, api: &dyn Api, block: &BlockInfo) -> Result<(), ContractError> {
        ensure!(
            !self.coins.is_empty(),
            ContractError::InvalidFunds {
                msg: "At least one coin should be sent".to_string(),
            }
        );
        ensure!(
            api.addr_validate(&self.recipient_addr).is_ok(),
            ContractError::InvalidAddress {}
        );

        if let Some(EscrowCondition::MinimumFunds(funds)) = &self.condition {
            ensure!(
                !funds.is_empty(),
                ContractError::InvalidFunds {
                    msg: "Minumum funds must not be empty".to_string(),
                }
            );
            let mut funds: Vec<Coin> = funds.clone();
            funds.sort_by(|a, b| a.denom.cmp(&b.denom));
            for i in 0..funds.len() - 1 {
                ensure!(
                    funds[i].denom != funds[i + 1].denom,
                    ContractError::DuplicateCoinDenoms {}
                );
            }
            // Explicitly stop here as it is alright if the Escrow is unlocked in this case, ie,
            // the intially deposited funds are greater or equal to the minimum imposed by this
            // condition.
            return Ok(());
        }

        ensure!(
            self.is_locked(block)? || self.condition.is_none(),
            ContractError::ExpirationInPast {}
        );
        Ok(())
    }

    /// Checks if the unlock condition has been met.
    pub fn is_locked(&self, block: &BlockInfo) -> Result<bool, ContractError> {
        match &self.condition {
            None => Ok(false),
            Some(condition) => match condition {
                EscrowCondition::Expiration(expiration) => Ok(!expiration.is_in_past(block)),
                EscrowCondition::MinimumFunds(funds) => {
                    Ok(!self.min_funds_deposited(funds.clone()))
                }
            },
        }
    }

    /// Checks if funds deposited in escrow are a subset of `required_funds`. In practice this is
    /// used for the `EscrowCondition::MinimumFunds(funds)` condition.
    fn min_funds_deposited(&self, required_funds: Vec<Coin>) -> bool {
        required_funds.iter().all(|required_coin| {
            self.coins.iter().any(|deposited_coin| {
                deposited_coin.denom == required_coin.denom
                    && required_coin.amount <= deposited_coin.amount
            })
        })
    }

    /// Adds coins in `coins_to_add` to `self.coins` by merging those of the same denom and
    /// otherwise appending.
    ///
    /// ## Arguments
    /// * `&mut self`    - Mutable reference to an instance of Escrow
    /// * `coins_to_add` - The `Vec<Coin>` to add, it is assumed that it contains no coins of the
    ///                    same denom
    ///
    /// Returns nothing as it is done in place.
    pub fn add_funds(&mut self, coins_to_add: Vec<Coin>) {
        self.coins = merge_coins(self.coins.to_vec(), coins_to_add);
    }
}

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Hold funds in Escrow
    HoldFunds {
        condition: Option<EscrowCondition>,
        recipient: Option<Recipient>,
    },
    /// Release funds all held in Escrow for the given recipient
    ReleaseFunds {
        recipient_addr: Option<String>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    ReleaseSpecificFunds {
        owner: String,
        recipient_addr: Option<String>,
    },
}
#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Queries funds held by an address
    #[returns(GetLockedFundsResponse)]
    GetLockedFunds { owner: String, recipient: String },
    /// Queries the funds for the given recipient.
    #[returns(GetLockedFundsForRecipientResponse)]
    GetLockedFundsForRecipient {
        recipient: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct GetLockedFundsResponse {
    pub funds: Option<Escrow>,
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct GetLockedFundsForRecipientResponse {
    pub funds: Vec<Escrow>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_std::common::Milliseconds;
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::{coin, Timestamp};

    #[test]
    fn test_validate() {
        let deps = mock_dependencies();
        let condition = EscrowCondition::Expiration(Milliseconds::from_seconds(101));
        let coins = vec![coin(100u128, "uluna")];
        let recipient = Recipient::from_string("owner");

        let valid_escrow = Escrow {
            recipient: recipient.clone(),
            coins: coins.clone(),
            condition: Some(condition.clone()),
            recipient_addr: "owner".to_string(),
        };
        let block = BlockInfo {
            height: 1000,
            time: Timestamp::from_seconds(100),
            chain_id: "foo".to_string(),
        };
        valid_escrow.validate(deps.as_ref().api, &block).unwrap();

        let valid_escrow = Escrow {
            recipient: recipient.clone(),
            coins: coins.clone(),
            condition: None,
            recipient_addr: "owner".to_string(),
        };
        let block = BlockInfo {
            height: 1000,
            time: Timestamp::from_seconds(3333),
            chain_id: "foo".to_string(),
        };
        valid_escrow.validate(deps.as_ref().api, &block).unwrap();

        let invalid_recipient_escrow = Escrow {
            recipient: Recipient::from_string(String::default()),
            coins: coins.clone(),
            condition: Some(condition.clone()),
            recipient_addr: String::default(),
        };

        let resp = invalid_recipient_escrow
            .validate(deps.as_ref().api, &block)
            .unwrap_err();
        assert_eq!(ContractError::InvalidAddress {}, resp);

        let invalid_coins_escrow = Escrow {
            recipient: recipient.clone(),
            coins: vec![],
            condition: Some(condition),
            recipient_addr: "owner".to_string(),
        };

        let resp = invalid_coins_escrow
            .validate(deps.as_ref().api, &block)
            .unwrap_err();
        assert_eq!(
            ContractError::InvalidFunds {
                msg: "At least one coin should be sent".to_string()
            },
            resp
        );

        let invalid_time_escrow = Escrow {
            recipient,
            coins,
            condition: Some(EscrowCondition::Expiration(Milliseconds::from_seconds(0))),
            recipient_addr: "owner".to_string(),
        };
        let block = BlockInfo {
            height: 1000,
            time: Timestamp::from_seconds(1),
            chain_id: "foo".to_string(),
        };
        assert_eq!(
            ContractError::ExpirationInPast {},
            invalid_time_escrow
                .validate(deps.as_ref().api, &block)
                .unwrap_err()
        );
    }

    #[test]
    fn test_validate_funds_condition() {
        let deps = mock_dependencies();
        let recipient = Recipient::from_string("owner");

        let valid_escrow = Escrow {
            recipient: recipient.clone(),
            coins: vec![coin(100, "uluna")],
            condition: Some(EscrowCondition::MinimumFunds(vec![
                coin(100, "uusd"),
                coin(100, "uluna"),
            ])),
            recipient_addr: "owner".to_string(),
        };
        let block = BlockInfo {
            height: 1000,
            time: Timestamp::from_seconds(4444),
            chain_id: "foo".to_string(),
        };
        valid_escrow.validate(deps.as_ref().api, &block).unwrap();

        // Funds exceed minimum
        let valid_escrow = Escrow {
            recipient: recipient.clone(),
            coins: vec![coin(200, "uluna")],
            condition: Some(EscrowCondition::MinimumFunds(vec![coin(100, "uluna")])),
            recipient_addr: "owner".to_string(),
        };
        valid_escrow.validate(deps.as_ref().api, &block).unwrap();

        // Empty funds
        let invalid_escrow = Escrow {
            recipient: recipient.clone(),
            coins: vec![coin(100, "uluna")],
            condition: Some(EscrowCondition::MinimumFunds(vec![])),
            recipient_addr: "owner".to_string(),
        };
        assert_eq!(
            ContractError::InvalidFunds {
                msg: "Minumum funds must not be empty".to_string(),
            },
            invalid_escrow
                .validate(deps.as_ref().api, &block)
                .unwrap_err()
        );

        // Duplicate funds
        let invalid_escrow = Escrow {
            recipient,
            coins: vec![coin(100, "uluna")],
            condition: Some(EscrowCondition::MinimumFunds(vec![
                coin(100, "uusd"),
                coin(100, "uluna"),
                coin(200, "uusd"),
            ])),
            recipient_addr: "owner".to_string(),
        };
        assert_eq!(
            ContractError::DuplicateCoinDenoms {},
            invalid_escrow
                .validate(deps.as_ref().api, &block)
                .unwrap_err()
        );
    }

    #[test]
    fn test_min_funds_deposited() {
        let recipient = Recipient::from_string("owner");
        let escrow = Escrow {
            recipient: recipient.clone(),
            coins: vec![coin(100, "uluna")],
            condition: None,
            recipient_addr: "owner".to_string(),
        };
        assert!(!escrow.min_funds_deposited(vec![coin(100, "uusd")]));

        let escrow = Escrow {
            recipient: recipient.clone(),
            coins: vec![coin(100, "uluna")],
            condition: None,
            recipient_addr: "owner".to_string(),
        };
        assert!(!escrow.min_funds_deposited(vec![coin(100, "uusd"), coin(100, "uluna")]));

        let escrow = Escrow {
            recipient: recipient.clone(),
            coins: vec![coin(100, "uluna")],
            condition: None,
            recipient_addr: "owner".to_string(),
        };
        assert!(escrow.min_funds_deposited(vec![coin(100, "uluna")]));

        let escrow = Escrow {
            recipient,
            coins: vec![coin(200, "uluna")],
            condition: None,
            recipient_addr: "owner".to_string(),
        };
        assert!(escrow.min_funds_deposited(vec![coin(100, "uluna")]));
    }

    #[test]
    fn test_add_funds() {
        let mut escrow = Escrow {
            coins: vec![coin(100, "uusd"), coin(100, "uluna")],
            condition: None,
            recipient: Recipient::from_string(""),
            recipient_addr: "".to_string(),
        };
        let funds_to_add = vec![coin(25, "uluna"), coin(50, "uusd"), coin(100, "ucad")];

        escrow.add_funds(funds_to_add);
        assert_eq!(
            vec![coin(150, "uusd"), coin(125, "uluna"), coin(100, "ucad")],
            escrow.coins
        );
    }
}
