use crate::error::ContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Decimal, Uint128};
use std::cmp;

#[cw_serde]
pub struct Withdrawal {
    pub token: String,
    pub withdrawal_type: Option<WithdrawalType>,
}

#[cw_serde]
pub enum WithdrawalType {
    Amount(Uint128),
    Percentage(Decimal),
}

impl Withdrawal {
    /// Calculates the amount to withdraw given the withdrawal type and passed in `balance`.
    pub fn get_amount(&self, balance: Uint128) -> Result<Decimal, ContractError> {
        match self.withdrawal_type.clone() {
            None => Ok(Decimal::from_ratio(balance, Uint128::one())),
            Some(withdrawal_type) => withdrawal_type.get_amount(balance),
        }
    }
}

impl WithdrawalType {
    /// Calculates the amount to withdraw given the withdrawal type and passed in `balance`.
    pub fn get_amount(&self, balance: Uint128) -> Result<Decimal, ContractError> {
        match self {
            WithdrawalType::Percentage(percent) => {
                ensure!(*percent <= Decimal::one(), ContractError::InvalidRate {});
                Ok(Decimal::from_ratio(balance, Uint128::one()).checked_mul(*percent)?)
            }
            WithdrawalType::Amount(amount) => Ok(cmp::min(
                Decimal::from_ratio(*amount, Uint128::one()),
                Decimal::from_ratio(balance, Uint128::one()),
            )),
        }
    }

    /// Checks if the underlying value is zero or not.
    pub fn is_zero(&self) -> bool {
        match self {
            WithdrawalType::Percentage(percent) => percent.is_zero(),
            WithdrawalType::Amount(amount) => amount.is_zero(),
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
        assert_eq!(
            Decimal::from_ratio(balance, Uint128::one()),
            withdrawal.get_amount(balance).unwrap()
        );
    }

    #[test]
    fn test_get_amount_percentage() {
        let withdrawal = Withdrawal {
            token: "token".to_string(),
            withdrawal_type: Some(WithdrawalType::Percentage(Decimal::percent(10))),
        };
        let balance = Uint128::from(100u128);
        assert_eq!(
            Decimal::from_ratio(Uint128::from(10u128), Uint128::one()),
            withdrawal.get_amount(balance).unwrap()
        );
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
        assert_eq!(
            Decimal::from_ratio(Uint128::from(5u128), Uint128::one()),
            withdrawal.get_amount(balance).unwrap()
        );
    }

    #[test]
    fn test_get_too_high_amount() {
        let balance = Uint128::from(10u128);
        let withdrawal = Withdrawal {
            token: "token".to_string(),
            withdrawal_type: Some(WithdrawalType::Amount(balance + Uint128::from(1u128))),
        };
        assert_eq!(
            Decimal::from_ratio(Uint128::from(10u128), Uint128::one()),
            withdrawal.get_amount(balance).unwrap()
        );
    }
}
