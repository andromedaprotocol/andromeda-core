use crate::ado_base::hooks::{AndromedaHook, HookMsg, OnFundsTransferResponse};
use crate::amp::{recipient::Recipient, AndrAddr};
use crate::common::context::ExecuteContext;
use crate::common::{deduct_funds, Funds};
use crate::error::ContractError;
use crate::os::aos_querier::AOSQuerier;
use cw20::Cw20Coin;
use cw_storage_plus::Item;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    coin as create_coin, ensure, Binary, Coin, Decimal, Deps, Event, Fraction, QuerierWrapper,
    Response, StdError, Storage, SubMsg,
};
use serde::de::DeserializeOwned;

use super::ADOContract;

#[cw_serde]
pub struct PaymentsResponse {
    pub payments: Vec<Rate>,
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
    Percent(Decimal),
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
            LocalRateValue::Percent(percent) => {
                ensure!(
                    !percent.is_zero() && percent <= &Decimal::one(),
                    ContractError::InvalidRate {}
                );
            }
        }
        Ok(())
    }
}

#[cw_serde]
pub struct LocalRate {
    rate_type: LocalRateType,
    recipients: Vec<Recipient>,
    value: LocalRateValue,
    description: Option<String>,
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
}

#[cw_serde]
pub struct Config {
    pub rates: Vec<Rate>,
}

#[cw_serde] // This is added such that both Rate::Flat and Rate::Percent have the same level of nesting which
            // makes it easier to work with on the frontend.
pub struct PercentRate {
    pub percent: Decimal,
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
        LocalRateValue::Percent(percent) => {
            // [COM-03] Make sure that fee_rate between 0 and 100.
            ensure!(
                // No need for rate >=0 due to type limits (Question: Should add or remove?)
                percent <= Decimal::one() && !percent.is_zero(),
                ContractError::InvalidRate {}
            );
            let mut fee_amount = payment.amount * percent;

            // Always round any remainder up and prioritise the fee receiver.
            // Inverse of percent will always exist.
            let reversed_fee = fee_amount * percent.inv().unwrap();
            if payment.amount > reversed_fee {
                // [COM-1] Added checked add to fee_amount rather than direct increment
                fee_amount = fee_amount.checked_add(1u128.into())?;
            }
            Ok(Coin::new(fee_amount.u128(), payment.denom.clone()))
        } // Rate::External(_) => Err(ContractError::UnexpectedExternalRate {}),
    }
}

/// Processes the given module response by hiding the error if it is `UnsupportedOperation` and
/// bubbling up any other one. A return value of Ok(None) signifies that the operation was not
/// supported.
fn process_module_response<T>(
    mod_resp: Result<Option<T>, StdError>,
) -> Result<Option<T>, ContractError> {
    match mod_resp {
        Ok(mod_resp) => Ok(mod_resp),
        Err(StdError::NotFound { kind }) => {
            if kind.contains("operation") {
                Ok(None)
            } else {
                Err(ContractError::Std(StdError::NotFound { kind }))
            }
        }
        Err(e) => Err(e.into()),
    }
}

/// Queries the given address with the given hook message and returns the processed result.
fn hook_query<T: DeserializeOwned>(
    querier: &QuerierWrapper,
    hook_msg: AndromedaHook,
    addr: impl Into<String>,
) -> Result<Option<T>, ContractError> {
    let msg = HookMsg::AndrHook(hook_msg);
    let mod_resp: Result<Option<T>, StdError> = querier.query_wasm_smart(addr, &msg);
    process_module_response(mod_resp)
}

pub fn rates<'a>() -> Item<'a, Config> {
    Item::new("rates")
}

impl<'a> ADOContract<'a> {
    /// Sets rates
    pub fn set_rates(store: &mut dyn Storage, config: Config) -> Result<(), ContractError> {
        rates().save(store, &config)?;
        Ok(())
    }
    pub fn execute_set_rates(
        self,
        ctx: ExecuteContext,
        config: Config,
    ) -> Result<Response, ContractError> {
        ensure!(
            Self::is_contract_owner(&self, ctx.deps.storage, ctx.info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        // Validate rates
        for rate in config.clone().rates {
            rate.validate_rate(ctx.deps.as_ref())?;
        }
        Self::set_rates(ctx.deps.storage, config.clone())?;

        Ok(Response::default().add_attributes(vec![("action", "set_rates")]))
    }
    pub fn remove_rates(store: &mut dyn Storage) -> Result<(), ContractError> {
        rates().remove(store);
        Ok(())
    }
    pub fn execute_remove_rates(self, ctx: ExecuteContext) -> Result<Response, ContractError> {
        ensure!(
            Self::is_contract_owner(&self, ctx.deps.storage, ctx.info.sender.as_str())?,
            ContractError::Unauthorized {}
        );

        Self::remove_rates(ctx.deps.storage)?;

        Ok(Response::default().add_attributes(vec![("action", "remove_rates")]))
    }

    pub fn get_rates(self, store: &mut dyn Storage) -> Result<Option<Config>, ContractError> {
        Ok(rates().may_load(store)?)
    }

    pub fn query_deducted_funds(
        self,
        deps: Deps,
        funds: Funds,
    ) -> Result<OnFundsTransferResponse, ContractError> {
        let config = self.rates.load(deps.storage)?;
        let mut msgs: Vec<SubMsg> = vec![];
        let mut events: Vec<Event> = vec![];
        let (coin, is_native): (Coin, bool) = match funds.clone() {
            Funds::Native(coin) => (coin, true),
            Funds::Cw20(cw20_coin) => (
                create_coin(cw20_coin.amount.u128(), cw20_coin.address),
                false,
            ),
        };
        let mut leftover_funds = vec![coin.clone()];
        for rate_info in config.rates.iter() {
            match rate_info {
                Rate::Local(local_rate) => {
                    let event_name = if local_rate.rate_type.is_additive() {
                        "tax"
                    } else {
                        "royalty"
                    };
                    let mut event = Event::new(event_name);
                    if let Some(desc) = &local_rate.description {
                        event = event.add_attribute("description", desc);
                    }
                    let fee = calculate_fee(local_rate.value.clone(), &coin)?;
                    for receiver in local_rate.recipients.iter() {
                        if local_rate.rate_type == LocalRateType::Deductive {
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
                }
                Rate::Contract(rates_address) => {
                    // Restructure leftover funds from Vec<Coin> into Funds
                    // let remaining_funds = if is_native {
                    //     Funds::Native(leftover_funds[0].clone())
                    // } else {
                    //     Funds::Cw20(Cw20Coin {
                    //         address: leftover_funds[0].clone().denom,
                    //         amount: leftover_funds[0].amount,
                    //     })
                    // };
                    // Query rates contract
                    let rates_resp: Option<OnFundsTransferResponse> = hook_query(
                        &deps.querier,
                        AndromedaHook::OnFundsTransfer {
                            payload: Binary::default(),
                            sender: "sender".to_string(),
                            amount: funds.clone(),
                        },
                        rates_address,
                    )?;

                    if let Some(rates_resp) = rates_resp {
                        let leftover_coin: Coin = match rates_resp.leftover_funds {
                            Funds::Native(coin) => coin,
                            Funds::Cw20(cw20_coin) => {
                                create_coin(cw20_coin.amount.u128(), cw20_coin.address)
                            }
                        };
                        // Update leftover funds using the rates response
                        leftover_funds = vec![leftover_coin];
                        msgs = [msgs, rates_resp.msgs].concat();
                        events = [events, rates_resp.events].concat();
                    }
                }
            }
        }
        Ok(OnFundsTransferResponse {
            msgs,
            leftover_funds: if is_native {
                Funds::Native(leftover_funds[0].clone())
            } else {
                Funds::Cw20(Cw20Coin {
                    amount: leftover_funds[0].amount,
                    address: coin.denom,
                })
            },
            events,
        })
    }
}

#[cfg(test)]
mod tests {

    use cosmwasm_std::{coin, Uint128};

    use super::*;

    // #[test]
    // fn test_validate_external_rate() {
    //     let deps = mock_dependencies_custom(&[]);

    //     let rate = Rate::External(PrimitivePointer {
    //         address: MOCK_PRIMITIVE_CONTRACT.to_owned(),

    //         key: Some("percent".to_string()),
    //     });
    //     let validated_rate = rate.validate(&deps.as_ref().querier).unwrap();
    //     let expected_rate = Rate::from(Decimal::percent(1));
    //     assert_eq!(expected_rate, validated_rate);

    //     let rate = Rate::External(PrimitivePointer {
    //         address: MOCK_PRIMITIVE_CONTRACT.to_owned(),
    //         key: Some("flat".to_string()),
    //     });
    //     let validated_rate = rate.validate(&deps.as_ref().querier).unwrap();
    //     let expected_rate = Rate::Flat(coin(1u128, "uusd"));
    //     assert_eq!(expected_rate, validated_rate);
    // }

    #[test]
    fn test_calculate_fee() {
        let payment = coin(101, "uluna");
        let expected = Ok(coin(5, "uluna"));
        let fee = LocalRateValue::Percent(Decimal::percent(4));

        let received = calculate_fee(fee, &payment);

        assert_eq!(expected, received);

        assert_eq!(expected, received);

        let payment = coin(125, "uluna");
        let fee = LocalRateValue::Flat(Coin {
            amount: Uint128::from(5_u128),
            denom: "uluna".to_string(),
        });

        let received = calculate_fee(fee, &payment);

        assert_eq!(expected, received);
    }
}
