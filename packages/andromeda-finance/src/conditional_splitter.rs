use andromeda_std::{
    amp::recipient::Recipient,
    andr_exec, andr_instantiate, andr_query,
    common::{MillisecondsDuration, MillisecondsExpiration},
    error::ContractError,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Decimal, Deps, Uint128};
use std::collections::HashSet;

#[cw_serde]
// The range of received funds which will correspond to a certain percentage.
pub struct Range {
    // Lower bound of the range
    pub min: Uint128,
    // Upper bound of the range
    pub max: Uint128,
}
impl Range {
    pub fn new(min: Uint128, max: Uint128) -> Self {
        Self { min, max }
    }
    pub fn verify_range(&self) -> Result<(), ContractError> {
        if self.min < self.max {
            Ok(())
        } else {
            Err(ContractError::InvalidRange {})
        }
    }
    pub fn contains(&self, num: Uint128) -> bool {
        num >= self.min && num <= self.max
    }
}

// The contract owner will input a vector of Threshold
#[cw_serde]
pub struct Threshold {
    pub range: Range,
    pub percentage: Decimal,
}
impl Threshold {
    pub fn new(range: Range, percentage: Decimal) -> Self {
        Self { range, percentage }
    }
    pub fn contains(&self, num: Uint128) -> bool {
        self.range.contains(num)
    }
}

pub fn find_threshold(thresholds: &[Threshold], num: Uint128) -> Result<&Threshold, ContractError> {
    let threshold = thresholds.iter().find(|&threshold| threshold.contains(num));
    if let Some(threshold) = threshold {
        Ok(threshold)
    } else {
        Err(ContractError::InvalidRange {})
    }
}

#[cw_serde]
pub struct AddressFunds {
    pub recipient: Recipient,
    pub funds: Uint128,
}
impl AddressFunds {
    pub fn new(recipient: Recipient) -> Self {
        Self {
            recipient,
            funds: Uint128::zero(),
        }
    }
}

#[cw_serde]
/// A config struct for a `Conditional Splitter` contract.
pub struct ConditionalSplitter {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is sent the amount sent will be divided amongst these recipients depending on the threshold.
    pub recipients: Vec<AddressFunds>,
    /// The vector of thresholds which assign a percentage for a certain range of received funds
    pub thresholds: Vec<Threshold>,
    /// The lock's expiration time
    pub lock: MillisecondsExpiration,
}
impl ConditionalSplitter {
    pub fn validate(&self, deps: Deps) -> Result<(), ContractError> {
        validate_recipient_list(deps, self.recipients.clone())?;
        validate_thresholds(&self.thresholds, &self.recipients)
    }
}

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is
    /// sent the amount sent will be divided amongst these recipients depending on their assigned percentage.
    pub recipients: Vec<Recipient>,
    pub thresholds: Vec<Threshold>,
    pub lock_time: Option<MillisecondsDuration>,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Update the recipients list. Only executable by the contract owner when the contract is not locked.
    UpdateRecipients { recipients: Vec<Recipient> },
    /// Used to lock/unlock the contract allowing the config to be updated.
    UpdateLock {
        // Milliseconds from current time
        lock_time: MillisecondsDuration,
    },
    /// Divides any attached funds to the message amongst the recipients list.
    Send {},
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// The current config of the Splitter contract
    #[returns(GetConditionalSplitterConfigResponse)]
    GetSplitterConfig {},
}

#[cw_serde]
pub struct GetConditionalSplitterConfigResponse {
    pub config: ConditionalSplitter,
}

/// Ensures that a given list of recipients for a `splitter` contract is valid:
///
/// * Must include at least one recipient
/// * The number of recipients must not exceed 100
/// * The recipient addresses must be unique
pub fn validate_recipient_list(
    deps: Deps,
    recipients: Vec<AddressFunds>,
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

    for recipient in recipients {
        recipient.recipient.validate(&deps)?;
        let recipient_address = recipient.recipient.address.get_raw_address(&deps)?;
        ensure!(
            !recipient_address_set.contains(&recipient_address),
            ContractError::DuplicateRecipient {}
        );
        recipient_address_set.insert(recipient_address);
    }

    Ok(())
}

pub fn validate_thresholds(
    thresholds: &Vec<Threshold>,
    recipients: &Vec<AddressFunds>,
) -> Result<(), ContractError> {
    let number_of_recipients = recipients.len() as u128;
    let mut prev_max: Option<Uint128> = None;

    for threshold in thresholds {
        // Check that the range is valid (min > max)
        threshold.range.verify_range()?;

        // The percentage multiplied by the number of recipients shouldn't exceed 100
        ensure!(
            threshold.percentage * Uint128::new(number_of_recipients) <= Uint128::new(100),
            ContractError::AmountExceededHundredPrecent {}
        );

        // The ranges shouldn't overlap
        if let Some(max) = prev_max {
            if threshold.range.min <= max {
                return Err(ContractError::OverlappingRanges {});
            }
        }

        // Update prev_max
        prev_max = Some(threshold.range.max);
    }

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use cosmwasm_std::testing::mock_dependencies;

//     use super::*;

//     #[test]
//     fn test_validate_recipient_list() {
//         let deps = mock_dependencies();
//         let empty_recipients = vec![];
//         let res = validate_recipient_list(deps.as_ref(), empty_recipients).unwrap_err();
//         assert_eq!(res, ContractError::EmptyRecipientsList {});

//         let inadequate_recipients = vec![AddressPercent {
//             recipient: Recipient::from_string(String::from("abc")),
//             percent: Decimal::percent(150),
//         }];
//         let res = validate_recipient_list(deps.as_ref(), inadequate_recipients).unwrap_err();
//         assert_eq!(res, ContractError::AmountExceededHundredPrecent {});

//         let duplicate_recipients = vec![
//             AddressPercent {
//                 recipient: Recipient::from_string(String::from("abc")),
//                 percent: Decimal::percent(50),
//             },
//             AddressPercent {
//                 recipient: Recipient::from_string(String::from("abc")),
//                 percent: Decimal::percent(50),
//             },
//         ];

//         let err = validate_recipient_list(deps.as_ref(), duplicate_recipients).unwrap_err();
//         assert_eq!(err, ContractError::DuplicateRecipient {});

//         let valid_recipients = vec![
//             AddressPercent {
//                 recipient: Recipient::from_string(String::from("abc")),
//                 percent: Decimal::percent(50),
//             },
//             AddressPercent {
//                 recipient: Recipient::from_string(String::from("xyz")),
//                 percent: Decimal::percent(50),
//             },
//         ];

//         let res = validate_recipient_list(deps.as_ref(), valid_recipients);
//         assert!(res.is_ok());

//         let one_valid_recipient = vec![AddressPercent {
//             recipient: Recipient::from_string(String::from("abc")),
//             percent: Decimal::percent(50),
//         }];

//         let res = validate_recipient_list(deps.as_ref(), one_valid_recipient);
//         assert!(res.is_ok());
//     }
// }
