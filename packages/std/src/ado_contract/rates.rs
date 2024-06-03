use crate::ado_base::rates::Rate;
use crate::ado_base::rates::RatesMessage;
use crate::ado_base::rates::RatesResponse;
use crate::common::context::ExecuteContext;
use crate::common::Funds;
use crate::error::ContractError;
use crate::os::aos_querier::AOSQuerier;
use cosmwasm_std::{coin as create_coin, ensure, Coin, Deps, Response, Storage};
use cw20::Cw20Coin;
use cw_storage_plus::Map;

use super::ADOContract;

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
    pub fn execute_rates(
        &self,
        ctx: ExecuteContext,
        rates_message: RatesMessage,
    ) -> Result<Response, ContractError> {
        match rates_message {
            RatesMessage::SetRate { action, rate } => self.execute_set_rates(ctx, action, rate),
            RatesMessage::RemoveRate { action } => self.execute_remove_rates(ctx, action),
        }
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
    ) -> Result<Option<RatesResponse>, ContractError> {
        let action: String = action.into();
        let rate = self.rates.may_load(deps.storage, &action)?;
        match rate {
            Some(rate) => {
                let (coin, is_native): (Coin, bool) = match funds {
                    Funds::Native(coin) => {
                        ensure!(
                            !coin.amount.is_zero(),
                            ContractError::InvalidFunds {
                                msg: "Zero amounts are prohibited".to_string()
                            }
                        );
                        (coin, true)
                    }
                    Funds::Cw20(cw20_coin) => {
                        ensure!(
                            !cw20_coin.amount.is_zero(),
                            ContractError::InvalidFunds {
                                msg: "Zero amounts are prohibited".to_string()
                            }
                        );
                        (
                            create_coin(cw20_coin.amount.u128(), cw20_coin.address),
                            false,
                        )
                    }
                };
                let (msgs, events, leftover_funds) = match rate {
                    Rate::Local(local_rate) => {
                        local_rate.generate_response(deps, coin.clone(), is_native)?
                    }
                    Rate::Contract(rates_address) => {
                        // Query rates contract
                        let addr = rates_address.get_raw_address(&deps)?;
                        let rate = AOSQuerier::get_rate(&deps.querier, &addr, &action)?;
                        rate.generate_response(deps, coin.clone(), is_native)?
                    }
                };

                Ok(Some(RatesResponse {
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
                }))
            }
            None => Ok(None),
        }
    }
}
#[cfg(test)]
#[cfg(feature = "rates")]

mod tests {

    use cosmwasm_std::{
        coin,
        testing::{mock_dependencies, mock_env},
        Addr,
    };

    use crate::{
        ado_base::rates::{LocalRate, LocalRateType, LocalRateValue},
        amp::{AndrAddr, Recipient},
    };

    use super::*;
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
