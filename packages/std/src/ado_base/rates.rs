use crate::{
    ado_contract::ADOContract,
    amp::{AndrAddr, Recipient},
    common::{deduct_funds, Funds},
    error::ContractError,
    os::{adodb::ADOVersion, aos_querier::AOSQuerier},
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, has_coins, Coin, Decimal, Deps, Event, Fraction, SubMsg};
use cw20::Cw20Coin;

#[cw_serde]
pub struct RatesResponse {
    pub msgs: Vec<SubMsg>,
    pub events: Vec<Event>,
    pub leftover_funds: Funds,
}

impl Default for RatesResponse {
    fn default() -> Self {
        Self {
            msgs: Vec::new(),
            events: Vec::new(),
            leftover_funds: Funds::Native(Coin::default()),
        }
    }
}

#[cw_serde]
pub enum RatesMessage {
    SetRate { action: String, rate: Rate },
    RemoveRate { action: String },
}

#[cw_serde]
pub enum RatesQueryMessage {
    GetRate { action: String },
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
    pub fn create_event(&self) -> Event {
        if self.is_additive() {
            Event::new("tax")
        } else {
            Event::new("royalty")
        }
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
// Created this because of the very complex return value warning.
type LocalRateResponse = (Vec<SubMsg>, Vec<Event>, Vec<Coin>);

impl LocalRate {
    pub fn generate_response(
        &self,
        deps: Deps,
        coin: Coin,
        is_native: bool,
    ) -> Result<LocalRateResponse, ContractError> {
        let mut msgs: Vec<SubMsg> = vec![];
        let mut events: Vec<Event> = vec![];
        let mut leftover_funds = vec![coin.clone()];
        // Tax event if the rate type is additive, or Royalty event if the rate type is deductive.
        let mut event = self.rate_type.create_event();

        if let Some(desc) = &self.description {
            event = event.add_attribute("description", desc);
        }
        let fee = calculate_fee(self.value.clone(), &coin)?;
        for receiver in self.recipients.iter() {
            // If the rate type is deductive
            if !self.rate_type.is_additive() {
                deduct_funds(&mut leftover_funds, &fee)?;
                event = event.add_attribute("deducted", fee.to_string());
            }
            event = event.add_attribute(
                "payment",
                PaymentAttribute {
                    receiver: receiver.get_addr(),
                    amount: fee.clone(),
                }
                .to_string(),
            );
            let msg = if is_native {
                receiver.generate_direct_msg(&deps, vec![fee.clone()])?
            } else {
                receiver.generate_msg_cw20(
                    &deps,
                    Cw20Coin {
                        amount: fee.amount,
                        address: fee.denom.to_string(),
                    },
                )?
            };
            msgs.push(msg);
        }
        events.push(event);
        Ok((msgs, events, leftover_funds))
    }
}

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
                let ado_type = AOSQuerier::ado_type_getter_smart(
                    &deps.querier,
                    &adodb_addr,
                    contract_info.code_id,
                )?;
                match ado_type {
                    Some(ado_type) => {
                        let ado_type = ADOVersion::from_string(ado_type).get_type();
                        ensure!(ado_type == "rates", ContractError::InvalidAddress {});
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
// This is added such that both Rate::Flat and Rate::Percent have the same level of nesting which makes it easier to work with on the frontend.
#[cw_serde]
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
        LocalRateValue::Flat(rate) => {
            ensure!(
                has_coins(&[payment.clone()], &rate),
                ContractError::InsufficientFunds {}
            );
            Ok(Coin::new(rate.amount.u128(), rate.denom))
        }
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
