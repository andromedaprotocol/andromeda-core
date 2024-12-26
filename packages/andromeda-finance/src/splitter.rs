use std::collections::HashSet;

use andromeda_std::{
    amp::recipient::Recipient,
    andr_exec, andr_instantiate, andr_query,
    common::{expiration::Expiry, Milliseconds, MillisecondsExpiration},
    error::ContractError,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, BlockInfo, Decimal, Deps};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct AddressPercent {
    pub recipient: Recipient,
    pub percent: Decimal,
}

impl AddressPercent {
    pub fn new(recipient: Recipient, percent: Decimal) -> Self {
        Self { recipient, percent }
    }
}

#[cw_serde]
/// A config struct for a `Splitter` contract.
pub struct Splitter {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is sent the amount sent will be divided amongst these recipients depending on their assigned percentage.
    pub recipients: Vec<AddressPercent>,
    /// The lock's expiration time
    pub lock: MillisecondsExpiration,
    /// The address that will receive any surplus funds, defaults to the message sender.
    pub default_recipient: Option<Recipient>,
}

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is
    /// sent the amount sent will be divided amongst these recipients depending on their assigned percentage.
    pub recipients: Vec<AddressPercent>,
    pub lock_time: Option<Expiry>,
    pub default_recipient: Option<Recipient>,
}

impl InstantiateMsg {
    pub fn validate(&self, deps: Deps) -> Result<(), ContractError> {
        validate_recipient_list(deps, self.recipients.clone())
    }
}

#[cw_serde]
pub enum Cw20HookMsg {
    Send { config: Option<Vec<AddressPercent>> },
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Update the recipients list. Only executable by the contract owner when the contract is not locked.
    #[attrs(restricted, nonpayable)]
    UpdateRecipients { recipients: Vec<AddressPercent> },
    /// Used to lock/unlock the contract allowing the config to be updated.
    #[attrs(restricted, nonpayable)]
    UpdateLock { lock_time: Expiry },
    /// Update the default recipient. Only executable by the contract owner when the contract is not locked.
    #[attrs(restricted, nonpayable)]
    UpdateDefaultRecipient { recipient: Option<Recipient> },
    #[attrs(nonpayable)]
    Receive(Cw20ReceiveMsg),
    /// Divides any attached funds to the message amongst the recipients list.
    Send { config: Option<Vec<AddressPercent>> },
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
/// * The combined percentage of the recipients must not exceed 100
/// * The recipient addresses must be unique
pub fn validate_recipient_list(
    deps: Deps,
    recipients: Vec<AddressPercent>,
) -> Result<(), ContractError> {
    ensure!(
        !recipients.is_empty(),
        ContractError::EmptyRecipientsList {}
    );

    ensure!(
        recipients.len() <= 100,
        ContractError::ReachedRecipientLimit {}
    );

    let mut percent_sum: Decimal = Decimal::zero();
    let mut recipient_address_set = HashSet::new();

    for rec in recipients {
        rec.recipient.validate(&deps)?;
        percent_sum = percent_sum.checked_add(rec.percent)?;
        ensure!(
            percent_sum <= Decimal::one(),
            ContractError::AmountExceededHundredPrecent {}
        );

        let recipient_address = rec.recipient.address.get_raw_address(&deps)?;
        ensure!(
            !recipient_address_set.contains(&recipient_address),
            ContractError::DuplicateRecipient {}
        );
        recipient_address_set.insert(recipient_address);
    }

    Ok(())
}
// 1 day in milliseconds
const ONE_DAY: u64 = 86_400_000;
// 1 year in milliseconds
const ONE_YEAR: u64 = 31_536_000_000;

/// Ensures the expiry is between one day and one year, also returns a Milliseconds representation of Expiry
pub fn validate_expiry_duration(
    lock_time: &Expiry,
    block: &BlockInfo,
) -> Result<Milliseconds, ContractError> {
    let new_lock_time_expiration = lock_time.get_time(block);
    // New lock time can't be too short
    ensure!(
        new_lock_time_expiration
            >= Milliseconds::from_seconds(block.time.seconds())
                .plus_milliseconds(Milliseconds(ONE_DAY)),
        ContractError::LockTimeTooShort {}
    );

    // New lock time can't be too long
    ensure!(
        new_lock_time_expiration
            <= Milliseconds::from_seconds(block.time.seconds())
                .plus_milliseconds(Milliseconds(ONE_YEAR)),
        ContractError::LockTimeTooLong {}
    );
    Ok(new_lock_time_expiration)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::mock_dependencies;

    use super::*;

    #[test]
    fn test_validate_recipient_list() {
        let deps = mock_dependencies();
        let empty_recipients = vec![];
        let res = validate_recipient_list(deps.as_ref(), empty_recipients).unwrap_err();
        assert_eq!(res, ContractError::EmptyRecipientsList {});

        let inadequate_recipients = vec![AddressPercent {
            recipient: Recipient::from_string(String::from("abc")),
            percent: Decimal::percent(150),
        }];
        let res = validate_recipient_list(deps.as_ref(), inadequate_recipients).unwrap_err();
        assert_eq!(res, ContractError::AmountExceededHundredPrecent {});

        let duplicate_recipients = vec![
            AddressPercent {
                recipient: Recipient::from_string(String::from("abc")),
                percent: Decimal::percent(50),
            },
            AddressPercent {
                recipient: Recipient::from_string(String::from("abc")),
                percent: Decimal::percent(50),
            },
        ];

        let err = validate_recipient_list(deps.as_ref(), duplicate_recipients).unwrap_err();
        assert_eq!(err, ContractError::DuplicateRecipient {});

        let valid_recipients = vec![
            AddressPercent {
                recipient: Recipient::from_string(String::from("abc")),
                percent: Decimal::percent(50),
            },
            AddressPercent {
                recipient: Recipient::from_string(String::from("xyz")),
                percent: Decimal::percent(50),
            },
        ];

        let res = validate_recipient_list(deps.as_ref(), valid_recipients);
        assert!(res.is_ok());

        let one_valid_recipient = vec![AddressPercent {
            recipient: Recipient::from_string(String::from("abc")),
            percent: Decimal::percent(50),
        }];

        let res = validate_recipient_list(deps.as_ref(), one_valid_recipient);
        assert!(res.is_ok());
    }
}
