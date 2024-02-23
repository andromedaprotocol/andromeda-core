use crate::ado_base::hooks::{AndromedaHook, HookMsg, OnFundsTransferResponse};
use crate::ado_base::rates::{calculate_fee, LocalRateType, PaymentAttribute, Rate};
use crate::common::context::ExecuteContext;
use crate::common::{deduct_funds, encode_binary, Funds};
use crate::error::ContractError;
use cosmwasm_std::{
    coin as create_coin, ensure, Coin, Deps, Event, QuerierWrapper, Response, StdError, Storage,
    SubMsg,
};
use cw20::Cw20Coin;
use cw_storage_plus::Map;
use serde::de::DeserializeOwned;

use super::ADOContract;

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

pub fn rates<'a>() -> Map<'a, &'a str, Rate> {
    Map::new("rates")
}

impl<'a> ADOContract<'a> {
    /// Sets rates
    pub fn set_rates(
        &self,
        store: &mut dyn Storage,
        action: impl Into<String>,
        rate: Rate,
    ) -> Result<(), ContractError> {
        let action: String = action.into();
        self.rates.save(store, &action, &rate)?;
        Ok(())
    }
    pub fn execute_set_rates(
        &self,
        ctx: ExecuteContext,
        action: impl Into<String>,
        rate: Rate,
    ) -> Result<Response, ContractError> {
        ensure!(
            Self::is_contract_owner(self, ctx.deps.storage, ctx.info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        let action: String = action.into();
        // Validate rates
        rate.validate_rate(ctx.deps.as_ref())?;
        self.set_rates(ctx.deps.storage, action, rate)?;

        Ok(Response::default().add_attributes(vec![("action", "set_rates")]))
    }
    pub fn remove_rates(
        &self,
        store: &mut dyn Storage,
        action: impl Into<String>,
    ) -> Result<(), ContractError> {
        let action: String = action.into();
        self.rates.remove(store, &action);
        Ok(())
    }
    pub fn execute_remove_rates(
        &self,
        ctx: ExecuteContext,
        action: impl Into<String>,
    ) -> Result<Response, ContractError> {
        ensure!(
            Self::is_contract_owner(self, ctx.deps.storage, ctx.info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        let action: String = action.into();
        self.remove_rates(ctx.deps.storage, action.clone())?;

        Ok(Response::default().add_attributes(vec![
            ("action", "remove_rates"),
            ("removed_action", &action),
        ]))
    }

    pub fn get_rates(
        &self,
        deps: Deps,
        action: impl Into<String>,
    ) -> Result<Option<Rate>, ContractError> {
        let action: String = action.into();
        Ok(rates().may_load(deps.storage, &action)?)
    }

    pub fn query_deducted_funds(
        self,
        deps: Deps,
        action: impl Into<String>,
        funds: Funds,
    ) -> Result<OnFundsTransferResponse, ContractError> {
        let action: String = action.into();
        let rate = self.rates.load(deps.storage, &action)?;
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
        match rate {
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
                        payload: encode_binary(&action)?,
                        sender: "sender".to_string(),
                        amount: funds,
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
#[cfg(feature = "rates")]

mod tests {

    use cosmwasm_std::{
        coin,
        testing::{mock_dependencies, mock_env},
        Addr, Decimal, Uint128,
    };

    use crate::{
        ado_base::rates::{calculate_fee, LocalRate, LocalRateValue, PercentRate},
        amp::{AndrAddr, Recipient},
    };

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
        let fee = LocalRateValue::Percent(PercentRate {
            percent: Decimal::percent(4),
        });

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
    #[test]
    fn test_rates_crud() {
        let mut deps = mock_dependencies();
        let _env = mock_env();
        let contract = ADOContract::default();
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let expected_rate = Rate::Local(LocalRate {
            rate_type: LocalRateType::Additive,
            recipients: vec![Recipient {
                address: AndrAddr::from_string("owner".to_string()),
                msg: None,
                ibc_recovery_address: None,
            }],
            value: LocalRateValue::Flat(coin(100_u128, "uandr")),
            description: None,
        });

        let action = "deposit";
        // set rates
        ADOContract::set_rates(&contract, &mut deps.storage, action, expected_rate.clone())
            .unwrap();

        let rate = ADOContract::default()
            .rates
            .load(&deps.storage, action)
            .unwrap();

        assert_eq!(rate, expected_rate);

        // get rates
        let rate = ADOContract::default()
            .get_rates(deps.as_ref(), action)
            .unwrap();
        assert_eq!(expected_rate, rate.unwrap());

        // remove rates
        ADOContract::remove_rates(&contract, &mut deps.storage, action).unwrap();
        let rate = ADOContract::default()
            .get_rates(deps.as_ref(), action)
            .unwrap();
        assert!(rate.is_none());
    }
}
