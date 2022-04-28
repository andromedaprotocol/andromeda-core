use crate::{error::ContractError, require};
use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Withdrawal {
    pub token: String,
    pub withdrawal_type: Option<WithdrawalType>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WithdrawalType {
    Amount(Uint128),
    Percentage(Decimal),
}

impl Withdrawal {
    /// Calculates the amount to withdraw given the withdrawal type and passed in `balance`.
    pub fn get_amount(&self, balance: Uint128) -> Result<Uint128, ContractError> {
        match self.withdrawal_type.clone() {
            None => Ok(balance),
            Some(withdrawal_type) => match withdrawal_type {
                WithdrawalType::Percentage(percent) => {
                    require(percent <= Decimal::one(), ContractError::InvalidRate {})?;
                    Ok(balance * percent)
                }
                WithdrawalType::Amount(amount) => {
                    require(
                        amount <= balance,
                        ContractError::InvalidFunds {
                            msg: "Requested withdrawal amount exceeds token balance".to_string(),
                        },
                    )?;
                    Ok(amount)
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_amount_no_withdrawal_type() {
        let withdrawal = Withdrawal {
            token: "token".to_string(),
            withdrawal_type: None,
        };
        let balance = Uint128::from(100u128);
        assert_eq!(balance, withdrawal.get_amount(balance).unwrap());
    }

    #[test]
    fn test_get_amount_percentage() {
        let withdrawal = Withdrawal {
            token: "token".to_string(),
            withdrawal_type: Some(WithdrawalType::Percentage(Decimal::percent(10))),
        };
        let balance = Uint128::from(100u128);
        assert_eq!(10u128, withdrawal.get_amount(balance).unwrap().u128());
    }

    #[test]
    fn test_get_amount_invalid_percentage() {
        let withdrawal = Withdrawal {
            token: "token".to_string(),
            withdrawal_type: Some(WithdrawalType::Percentage(Decimal::percent(101))),
        };
        let balance = Uint128::from(100u128);
        assert_eq!(
            ContractError::InvalidRate {},
            withdrawal.get_amount(balance).unwrap_err()
        );
    }

    #[test]
    fn test_get_amount_amount() {
        let withdrawal = Withdrawal {
            token: "token".to_string(),
            withdrawal_type: Some(WithdrawalType::Amount(5u128.into())),
        };
        let balance = Uint128::from(10u128);
        assert_eq!(5u128, withdrawal.get_amount(balance).unwrap().u128());
    }

    #[test]
    fn test_get_invalid_amount() {
        let balance = Uint128::from(10u128);
        let withdrawal = Withdrawal {
            token: "token".to_string(),
            withdrawal_type: Some(WithdrawalType::Amount(balance + Uint128::from(1u128))),
        };
        assert_eq!(
            ContractError::InvalidFunds {
                msg: "Requested withdrawal amount exceeds token balance".to_string(),
            },
            withdrawal.get_amount(balance).unwrap_err()
        );
    }
}
