use crate::{
    ado_contract::ADOContract,
    amp::{AndrAddr, Recipient},
    error::ContractError,
    os::aos_querier::AOSQuerier,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Coin, Decimal, Deps, Fraction};

#[cw_serde]
pub enum RatesMessage {
    SetRate { action: String, rate: Rate },
    RemoveRate { action: String },
}

#[cw_serde]
pub struct PaymentsResponse {
    pub payments: Vec<Rate>,
}

/// An attribute struct used for any events that involve a payment
pub struct PaymentAttribute {
    /// The amount paid
    pub amount: Coin,
    /// The address the payment was made to
    pub receiver: String,
}

impl ToString for PaymentAttribute {
    fn to_string(&self) -> String {
        format!("{}<{}", self.receiver, self.amount)
    }
}

#[cw_serde]
pub enum LocalRateType {
    Additive,
    Deductive,
}
impl LocalRateType {
    pub fn is_additive(&self) -> bool {
        self == &LocalRateType::Additive
    }
}

#[cw_serde]
pub enum LocalRateValue {
    // Percent fee
    Percent(PercentRate),
    // Flat fee
    Flat(Coin),
}
impl LocalRateValue {
    pub fn validate(&self) -> Result<(), ContractError> {
        match self {
            // If it's a coin, make sure it's non-zero
            LocalRateValue::Flat(coin) => {
                ensure!(!coin.amount.is_zero(), ContractError::InvalidRate {});
            }
            // If it's a percentage, make sure it's greater than zero and less than or equal to 1 of type decimal (which represents 100%)
            LocalRateValue::Percent(percent_rate) => {
                ensure!(
                    !percent_rate.percent.is_zero() && percent_rate.percent <= Decimal::one(),
                    ContractError::InvalidRate {}
                );
            }
        }
        Ok(())
    }
}

#[cw_serde]
pub struct LocalRate {
    pub rate_type: LocalRateType,
    pub recipients: Vec<Recipient>,
    pub value: LocalRateValue,
    pub description: Option<String>,
}

impl LocalRate {}

#[cw_serde]
pub enum Rate {
    Local(LocalRate),
    Contract(AndrAddr),
}

impl Rate {
    // Makes sure that the contract address is that of a Rates contract verified by the ADODB and validates the local rate value
    pub fn validate_rate(&self, deps: Deps) -> Result<(), ContractError> {
        match self {
            Rate::Contract(address) => {
                let raw_address = address.get_raw_address(&deps)?;
                let contract_info = deps.querier.query_wasm_contract_info(raw_address)?;
                let adodb_addr =
                    ADOContract::default().get_adodb_address(deps.storage, &deps.querier)?;
                let ado_type =
                    AOSQuerier::ado_type_getter(&deps.querier, &adodb_addr, contract_info.code_id)?;
                match ado_type {
                    Some(ado_type) => {
                        ensure!(ado_type == *"rates", ContractError::InvalidAddress {});
                        Ok(())
                    }
                    None => Err(ContractError::InvalidAddress {}),
                }
            }
            Rate::Local(local_rate) => {
                // Validate the local rate value
                local_rate.value.validate()?;
                Ok(())
            }
        }
    }
    pub fn is_local(&self) -> bool {
        match self {
            Rate::Contract(_) => false,
            Rate::Local(_) => true,
        }
    }
}

#[cw_serde] // This is added such that both Rate::Flat and Rate::Percent have the same level of nesting which
            // makes it easier to work with on the frontend.
pub struct PercentRate {
    pub percent: Decimal,
}

/// Calculates a fee amount given a `Rate` and payment amount.
///
/// ## Arguments
/// * `fee_rate` - The `Rate` of the fee to be paid
/// * `payment` - The amount used to calculate the fee
///
/// Returns the fee amount in a `Coin` struct.
pub fn calculate_fee(fee_rate: LocalRateValue, payment: &Coin) -> Result<Coin, ContractError> {
    match fee_rate {
        LocalRateValue::Flat(rate) => Ok(Coin::new(rate.amount.u128(), rate.denom)),
        LocalRateValue::Percent(percent_rate) => {
            // [COM-03] Make sure that fee_rate between 0 and 100.
            ensure!(
                // No need for rate >=0 due to type limits (Question: Should add or remove?)
                percent_rate.percent <= Decimal::one() && !percent_rate.percent.is_zero(),
                ContractError::InvalidRate {}
            );
            let mut fee_amount = payment.amount * percent_rate.percent;

            // Always round any remainder up and prioritise the fee receiver.
            // Inverse of percent will always exist.
            let reversed_fee = fee_amount * percent_rate.percent.inv().unwrap();
            if payment.amount > reversed_fee {
                // [COM-1] Added checked add to fee_amount rather than direct increment
                fee_amount = fee_amount.checked_add(1u128.into())?;
            }
            Ok(Coin::new(fee_amount.u128(), payment.denom.clone()))
        } // Rate::External(_) => Err(ContractError::UnexpectedExternalRate {}),
    }
}
