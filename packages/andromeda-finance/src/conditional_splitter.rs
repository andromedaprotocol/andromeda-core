use andromeda_std::{
    amp::recipient::Recipient,
    andr_exec, andr_instantiate, andr_query,
    common::{MillisecondsDuration, MillisecondsExpiration},
    error::ContractError,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Decimal, Deps, Uint128};
use std::collections::HashSet;

use crate::splitter::AddressPercent;

// The contract owner will input a vector of Threshold
#[cw_serde]
pub struct Threshold {
    pub min: Uint128,
    pub address_percent: Vec<AddressPercent>,
}
impl Threshold {
    pub fn new(min: Uint128, address_percent: Vec<AddressPercent>) -> Self {
        Self {
            min,
            address_percent,
        }
    }
    pub fn in_range(&self, num: Uint128) -> bool {
        num >= self.min
    }
}

pub fn find_threshold(
    thresholds: &[Threshold],
    num: Uint128,
) -> Result<(Threshold, usize), ContractError> {
    // Create a vector of tuples containing the original index and the threshold
    let mut indexed_thresholds: Vec<(usize, &Threshold)> = thresholds.iter().enumerate().collect();

    // Sort thresholds by min values in decreasing order
    indexed_thresholds.sort_by(|a, b| b.1.min.cmp(&a.1.min));

    // Iterate over the sorted indexed thresholds
    for (index, threshold) in indexed_thresholds {
        if threshold.in_range(num) {
            // Return the threshold and its original index
            return Ok((threshold.clone(), index));
        }
    }
    Err(ContractError::InvalidRange {})
}

#[cw_serde]
pub struct AddressPercentages {
    pub recipient: Recipient,
    // The sequence of the the percentages should correspond to each threshold.
    // For example the first value in percentages should correspond to the first threshold
    pub percentages: Vec<Decimal>,
}

impl AddressPercentages {
    pub fn new(recipient: Recipient, percentages: Vec<Decimal>) -> Self {
        Self {
            recipient,
            percentages,
        }
    }
}

#[cw_serde]
/// A config struct for a `Conditional Splitter` contract.
pub struct ConditionalSplitter {
    /// The vector of thresholds which assign a percentage for a certain range of received funds
    pub thresholds: Vec<Threshold>,
    /// The lock's expiration time
    pub lock: MillisecondsExpiration,
}
impl ConditionalSplitter {
    pub fn validate(&self, deps: Deps) -> Result<(), ContractError> {
        validate_thresholds(deps, &self.thresholds)
    }
}

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is
    /// sent the amount sent will be divided amongst these recipients depending on their assigned percentage.
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
    GetConditionalSplitterConfig {},
}

#[cw_serde]
pub struct GetConditionalSplitterConfigResponse {
    pub config: ConditionalSplitter,
}

/// Ensures that a given list of recipients for a `conditional splitter` contract is valid:
/// * Percentages of each threshold should not exceed 100
/// * Each threshold must include at least one recipient
/// * The number of recipients for each threshold must not exceed 100
/// * The recipient addresses must be unique for each threshold
/// * Make sure there are no duplicate min values between the thresholds

pub fn validate_thresholds(deps: Deps, thresholds: &Vec<Threshold>) -> Result<(), ContractError> {
    let mut min_value_set = HashSet::new();

    for threshold in thresholds {
        // Make sure the threshold has recipients
        ensure!(
            !threshold.address_percent.is_empty(),
            ContractError::EmptyRecipientsList {}
        );
        // Make sure the threshold's number of recipients doesn't exceed 100
        ensure!(
            threshold.address_percent.len() <= 100,
            ContractError::ReachedRecipientLimit {}
        );

        let mut total_percent = Decimal::zero();
        let mut recipient_address_set = HashSet::new();

        for address_percent in &threshold.address_percent {
            total_percent = total_percent.checked_add(address_percent.percent)?;

            // Checks for duplicate and invalid recipients
            address_percent.recipient.validate(&deps)?;
            let recipient_address = address_percent.recipient.address.get_raw_address(&deps)?;
            ensure!(
                !recipient_address_set.contains(&recipient_address),
                ContractError::DuplicateRecipient {}
            );
            recipient_address_set.insert(recipient_address);
        }
        ensure!(
            total_percent <= Decimal::one(),
            ContractError::AmountExceededHundredPrecent {}
        );

        // Checks for duplicate minimum values
        let min_value = threshold.min.u128();
        ensure!(
            !min_value_set.contains(&min_value),
            ContractError::DuplicateRecipient {}
        );

        min_value_set.insert(min_value);
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
