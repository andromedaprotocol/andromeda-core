use common::{
    ado_base::{
        hooks::{AndromedaHook, OnFundsTransferResponse},
        recipient::Recipient,
        AndromedaMsg, AndromedaQuery,
    },
    encode_binary,
    error::ContractError,
    primitive::{Primitive, PrimitivePointer},
    require, Funds,
};
use cosmwasm_std::{Addr, Api, Coin, Decimal, Fraction, QuerierWrapper, QueryRequest, WasmQuery};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub rates: Vec<RateInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    UpdateRates { rates: Vec<RateInfo> },
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    AndrHook(AndromedaHook),
    Payments {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PaymentsResponse {
    pub payments: Vec<RateInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RateInfo {
    pub rate: Rate,
    pub is_additive: bool,
    pub description: Option<String>,
    pub receivers: Vec<Recipient>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// An enum used to define various types of fees
pub enum Rate {
    /// A flat rate fee
    Flat(Coin),
    /// A percentage fee
    Percent(PercentRate),
    External(PrimitivePointer),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// This is added such that both Rate::Flat and Rate::Percent have the same level of nesting which
// makes it easier to work with on the frontend.
pub struct PercentRate {
    pub percent: Decimal,
}

impl From<Decimal> for Rate {
    fn from(decimal: Decimal) -> Self {
        Rate::Percent(PercentRate { percent: decimal })
    }
}

impl Rate {
    /// Validates that a given rate is non-zero. It is expected that the Rate is not an
    /// External Rate.
    pub fn is_non_zero(&self) -> Result<bool, ContractError> {
        match self {
            Rate::Flat(coin) => Ok(!coin.amount.is_zero()),
            Rate::Percent(PercentRate { percent }) => Ok(!percent.is_zero()),
            Rate::External(_) => Err(ContractError::UnexpectedExternalRate {}),
        }
    }

    /// Validates `self` and returns an "unwrapped" version of itself wherein if it is an External
    /// Rate, the actual rate value is retrieved from the Primitive Contract.
    pub fn validate(
        &self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        mission_contract: Option<Addr>,
    ) -> Result<Rate, ContractError> {
        let rate = self.clone().get_rate(api, querier, mission_contract)?;
        require(rate.is_non_zero()?, ContractError::InvalidRate {})?;

        if let Rate::Percent(PercentRate { percent }) = rate {
            require(percent <= Decimal::one(), ContractError::InvalidRate {})?;
        }

        Ok(rate)
    }

    /// If `self` is Flat or Percent it returns itself. Otherwise it queries the primitive contract
    /// and retrieves the actual Flat or Percent rate.
    fn get_rate(
        self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        mission_contract: Option<Addr>,
    ) -> Result<Rate, ContractError> {
        match self {
            Rate::Flat(_) => Ok(self),
            Rate::Percent(_) => Ok(self),
            Rate::External(primitive_pointer) => {
                let primitive = primitive_pointer.into_value(api, querier, mission_contract)?;
                match primitive {
                    None => Err(ContractError::ParsingError {
                        err: "Stored primitive is None".to_string(),
                    }),
                    Some(primitive) => match primitive {
                        Primitive::Coin(coin) => Ok(Rate::Flat(coin)),
                        Primitive::Decimal(value) => Ok(Rate::from(value)),
                        _ => Err(ContractError::ParsingError {
                            err: "Stored rate is not a coin or Decimal".to_string(),
                        }),
                    },
                }
            }
        }
    }
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

pub fn on_required_payments(
    querier: QuerierWrapper,
    addr: String,
    amount: Funds,
) -> Result<OnFundsTransferResponse, ContractError> {
    let res: OnFundsTransferResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: addr,
        msg: encode_binary(&QueryMsg::AndrQuery(AndromedaQuery::Get(Some(
            encode_binary(&amount)?,
        ))))?,
    }))?;

    Ok(res)
}

/// Calculates a fee amount given a `Rate` and payment amount.
///
/// ## Arguments
/// * `fee_rate` - The `Rate` of the fee to be paid
/// * `payment` - The amount used to calculate the fee
///
/// Returns the fee amount in a `Coin` struct.
pub fn calculate_fee(fee_rate: Rate, payment: &Coin) -> Result<Coin, ContractError> {
    match fee_rate {
        Rate::Flat(rate) => Ok(Coin::new(rate.amount.u128(), rate.denom)),
        Rate::Percent(PercentRate { percent }) => {
            // [COM-03] Make sure that fee_rate between 0 and 100.
            require(
                // No need for rate >=0 due to type limits (Question: Should add or remove?)
                percent <= Decimal::one() && !percent.is_zero(),
                ContractError::InvalidRate {},
            )
            .unwrap();
            let mut fee_amount = payment.amount * percent;

            // Always round any remainder up and prioritise the fee receiver.
            // Inverse of percent will always exist.
            let reversed_fee = fee_amount * percent.inv().unwrap();
            if payment.amount > reversed_fee {
                // [COM-1] Added checked add to fee_amount rather than direct increment
                fee_amount = fee_amount.checked_add(1u128.into())?;
            }
            Ok(Coin::new(fee_amount.u128(), payment.denom.clone()))
        }
        Rate::External(_) => Err(ContractError::UnexpectedExternalRate {}),
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::mock_querier::{mock_dependencies_custom, MOCK_PRIMITIVE_CONTRACT};
    use common::mission::AndrAddress;
    use cosmwasm_std::{coin, Uint128};

    use super::*;

    #[test]
    fn test_validate_external_rate() {
        let deps = mock_dependencies_custom(&[]);

        let rate = Rate::External(PrimitivePointer {
            address: AndrAddress {
                identifier: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            },
            key: Some("percent".to_string()),
        });
        let validated_rate = rate
            .validate(deps.as_ref().api, &deps.as_ref().querier, None)
            .unwrap();
        let expected_rate = Rate::from(Decimal::percent(1));
        assert_eq!(expected_rate, validated_rate);

        let rate = Rate::External(PrimitivePointer {
            address: AndrAddress {
                identifier: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            },
            key: Some("flat".to_string()),
        });
        let validated_rate = rate
            .validate(deps.as_ref().api, &deps.as_ref().querier, None)
            .unwrap();
        let expected_rate = Rate::Flat(coin(1u128, "uusd"));
        assert_eq!(expected_rate, validated_rate);
    }

    #[test]
    fn test_calculate_fee() {
        let payment = coin(101, "uluna");
        let expected = Ok(coin(5, "uluna"));
        let fee = Rate::from(Decimal::percent(4));

        let received = calculate_fee(fee, &payment);

        assert_eq!(expected, received);

        assert_eq!(expected, received);

        let payment = coin(125, "uluna");
        let fee = Rate::Flat(Coin {
            amount: Uint128::from(5_u128),
            denom: "uluna".to_string(),
        });

        let received = calculate_fee(fee, &payment);

        assert_eq!(expected, received);
    }
}
