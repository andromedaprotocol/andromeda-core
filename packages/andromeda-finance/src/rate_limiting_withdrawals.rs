use common::{
    ado_base::{modules::Module, recipient::Recipient, AndromedaMsg, AndromedaQuery},
    error::ContractError,
    require,
};
use cosmwasm_std::{Coin, Decimal, Timestamp, Uint128};
use cw20::Cw20ReceiveMsg;
use cw_utils::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AddressPercent {
    pub recipient: Recipient,
    pub percent: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// Keeps track of the account's balance and time of latest withdrawal
pub struct AccountDetails {
    pub balance: Vec<Coin>,
    pub latest_withdrawal: Option<Timestamp>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    ///
    pub allowed_coins: Vec<Coin>,
    pub minimum_withdrawal_time: u64,
    pub modules: Option<Vec<Module>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Deposit {},
    Withdraw { coin: Coin },
    AndrReceive(AndromedaMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    /// The current config of the Splitter contract
    MinimalWithdrawalFrequency {},
    CoinWithdrawalLimit {
        coin: String,
    },
    AccountDetails {
        account: String,
    },
}

/// Ensures that a given list of recipients for a `splitter` contract is valid:
///
/// * Must include at least one recipient
/// * The combined percentage of the recipients must not exceed 100
pub fn validate_recipient_list(recipients: Vec<AddressPercent>) -> Result<bool, ContractError> {
    require(
        !recipients.is_empty(),
        ContractError::EmptyRecipientsList {},
    )?;

    let mut percent_sum: Decimal = Decimal::zero();
    for rec in recipients {
        // += operation is not supported for decimal.
        percent_sum += rec.percent;
    }

    require(
        percent_sum <= Decimal::one(),
        ContractError::AmountExceededHundredPrecent {},
    )?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_recipient_list() {
        let empty_recipients = vec![];
        let res = validate_recipient_list(empty_recipients).unwrap_err();
        assert_eq!(res, ContractError::EmptyRecipientsList {});

        let inadequate_recipients = vec![AddressPercent {
            recipient: Recipient::from_string(String::from("Some Address")),
            percent: Decimal::percent(150),
        }];
        let res = validate_recipient_list(inadequate_recipients).unwrap_err();
        assert_eq!(res, ContractError::AmountExceededHundredPrecent {});

        let valid_recipients = vec![
            AddressPercent {
                recipient: Recipient::from_string(String::from("Some Address")),
                percent: Decimal::percent(50),
            },
            AddressPercent {
                recipient: Recipient::from_string(String::from("Some Address")),
                percent: Decimal::percent(50),
            },
        ];

        let res = validate_recipient_list(valid_recipients).unwrap();
        assert!(res);
    }
}
