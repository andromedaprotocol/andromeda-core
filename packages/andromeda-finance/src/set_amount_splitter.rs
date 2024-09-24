use std::collections::HashSet;

use andromeda_std::{
    amp::recipient::Recipient,
    andr_exec, andr_instantiate, andr_query,
    common::{expiration::Expiry, MillisecondsExpiration},
    error::ContractError,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Coin, Deps};

#[cw_serde]
pub struct AddressAmount {
    pub recipient: Recipient,
    pub coins: Vec<Coin>,
}

impl AddressAmount {
    pub fn new(recipient: Recipient, coins: Vec<Coin>) -> Self {
        Self { recipient, coins }
    }
}

#[cw_serde]
/// A config struct for a `Splitter` contract.
pub struct Splitter {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is sent the amount sent will be divided amongst these recipients depending on their assigned amount.
    pub recipients: Vec<AddressAmount>,
    /// The lock's expiration time
    pub lock: MillisecondsExpiration,
}

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is
    /// sent the amount sent will be divided amongst these recipients depending on their assigned amount.
    pub recipients: Vec<AddressAmount>,
    pub lock_time: Option<Expiry>,
}

impl InstantiateMsg {
    pub fn validate(&self, deps: Deps) -> Result<(), ContractError> {
        validate_recipient_list(deps, self.recipients.clone())
    }
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Update the recipients list. Only executable by the contract owner when the contract is not locked.
    UpdateRecipients { recipients: Vec<AddressAmount> },
    /// Used to lock/unlock the contract allowing the config to be updated.
    UpdateLock {
        // Milliseconds from current time
        lock_time: Expiry,
    },
    /// Divides any attached funds to the message amongst the recipients list.
    Send {},
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// The current config of the Splitter contract
    #[returns(GetSplitterConfigResponse)]
    GetSplitterConfig {},
}

#[cw_serde]
pub struct GetSplitterConfigResponse {
    pub config: Splitter,
}

/// Ensures that a given list of recipients for a `splitter` contract is valid:
///
/// * Must include at least one recipient
/// * The number of recipients must not exceed 100
/// * The recipient addresses must be unique
/// * The recipient amount must be above zero
/// * Each recipient can't have more than two coins assigned.
/// * No duplicate coins

pub fn validate_recipient_list(
    deps: Deps,
    recipients: Vec<AddressAmount>,
) -> Result<(), ContractError> {
    ensure!(
        !recipients.is_empty(),
        ContractError::EmptyRecipientsList {}
    );

    ensure!(
        recipients.len() <= 100,
        ContractError::ReachedRecipientLimit {}
    );

    let mut recipient_address_set = HashSet::new();

    for rec in recipients {
        ensure!(
            rec.coins.len() == 1 || rec.coins.len() == 2,
            ContractError::InvalidFunds {
                msg: "A minimim of 1 and a maximum of 2 coins are allowed".to_string(),
            }
        );

        let mut denom_set = HashSet::new();
        for coin in rec.coins {
            ensure!(!coin.amount.is_zero(), ContractError::InvalidZeroAmount {});
            ensure!(
                !denom_set.contains(&coin.denom),
                ContractError::DuplicateCoinDenoms {}
            );
            denom_set.insert(coin.denom);
        }

        rec.recipient.validate(&deps)?;

        let recipient_address = rec.recipient.address.get_raw_address(&deps)?;
        ensure!(
            !recipient_address_set.contains(&recipient_address),
            ContractError::DuplicateRecipient {}
        );
        recipient_address_set.insert(recipient_address);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, coins, testing::mock_dependencies};

    use super::*;

    #[test]
    fn test_validate_recipient_list() {
        let deps = mock_dependencies();
        let empty_recipients = vec![];
        let err = validate_recipient_list(deps.as_ref(), empty_recipients).unwrap_err();
        assert_eq!(err, ContractError::EmptyRecipientsList {});

        let recipients_zero_amount = vec![
            AddressAmount {
                recipient: Recipient::from_string(String::from("xyz")),
                coins: coins(1_u128, "uandr"),
            },
            AddressAmount {
                recipient: Recipient::from_string(String::from("abc")),
                coins: coins(0_u128, "usdc"),
            },
        ];
        let err = validate_recipient_list(deps.as_ref(), recipients_zero_amount).unwrap_err();
        assert_eq!(err, ContractError::InvalidZeroAmount {});

        let recipients_zero_amount = vec![
            AddressAmount {
                recipient: Recipient::from_string(String::from("xyz")),
                coins: coins(1_u128, "uandr"),
            },
            AddressAmount {
                recipient: Recipient::from_string(String::from("abc")),
                coins: vec![
                    coin(1_u128, "uandr"),
                    coin(12_u128, "usdc"),
                    coin(13_u128, "usdt"),
                ],
            },
        ];
        let err = validate_recipient_list(deps.as_ref(), recipients_zero_amount).unwrap_err();
        assert_eq!(
            err,
            ContractError::InvalidFunds {
                msg: "A minimim of 1 and a maximum of 2 coins are allowed".to_string(),
            }
        );
        let recipients_zero_amount = vec![
            AddressAmount {
                recipient: Recipient::from_string(String::from("xyz")),
                coins: vec![],
            },
            AddressAmount {
                recipient: Recipient::from_string(String::from("abc")),
                coins: vec![
                    coin(1_u128, "uandr"),
                    coin(12_u128, "usdc"),
                    coin(13_u128, "usdt"),
                ],
            },
        ];
        let err = validate_recipient_list(deps.as_ref(), recipients_zero_amount).unwrap_err();
        assert_eq!(
            err,
            ContractError::InvalidFunds {
                msg: "A minimim of 1 and a maximum of 2 coins are allowed".to_string(),
            }
        );

        let recipients_zero_amount = vec![
            AddressAmount {
                recipient: Recipient::from_string(String::from("xyz")),
                coins: coins(1_u128, "uandr"),
            },
            AddressAmount {
                recipient: Recipient::from_string(String::from("abc")),
                coins: vec![coin(1_u128, "uandr"), coin(12_u128, "uandr")],
            },
        ];
        let err = validate_recipient_list(deps.as_ref(), recipients_zero_amount).unwrap_err();
        assert_eq!(err, ContractError::DuplicateCoinDenoms {});

        let duplicate_recipients = vec![
            AddressAmount {
                recipient: Recipient::from_string(String::from("abc")),
                coins: coins(1_u128, "denom"),
            },
            AddressAmount {
                recipient: Recipient::from_string(String::from("abc")),
                coins: coins(1_u128, "uandr"),
            },
        ];

        let err = validate_recipient_list(deps.as_ref(), duplicate_recipients).unwrap_err();
        assert_eq!(err, ContractError::DuplicateRecipient {});

        let valid_recipients = vec![
            AddressAmount {
                recipient: Recipient::from_string(String::from("abc")),
                coins: coins(1_u128, "uandr"),
            },
            AddressAmount {
                recipient: Recipient::from_string(String::from("xyz")),
                coins: coins(1_u128, "denom"),
            },
        ];

        let res = validate_recipient_list(deps.as_ref(), valid_recipients);
        assert!(res.is_ok());

        let one_valid_recipient = vec![AddressAmount {
            recipient: Recipient::from_string(String::from("abc")),
            coins: coins(1_u128, "denom"),
        }];

        let res = validate_recipient_list(deps.as_ref(), one_valid_recipient);
        assert!(res.is_ok());
    }
}
