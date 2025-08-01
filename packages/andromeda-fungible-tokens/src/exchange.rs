use std::fmt::Display;

use andromeda_std::{
    amp::{AndrAddr, Recipient},
    andr_exec, andr_instantiate, andr_query,
    common::{denom::Asset, schedule::Schedule, Milliseconds},
    error::ContractError,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ConversionOverflowError, Decimal256, StdError, StdResult, Uint128, Uint256};
use cw20::Cw20ReceiveMsg;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the CW20 token to be sold
    pub token_address: AndrAddr,
}

#[cw_serde]
pub enum ExchangeRate {
    Fixed(Decimal256),
    Variable(Uint256),
}
impl Display for ExchangeRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExchangeRate::Fixed(rate) => write!(f, "Fixed({})", rate),
            ExchangeRate::Variable(rate) => write!(f, "Variable({})", rate),
        }
    }
}
impl ExchangeRate {
    /// Returns the rate as is if it's fixed, but if it's dynamic, the exchange rate will be influenced by the amount of funds sent alongside the message. The higher the funds, the higher the rate and vice versa.
    pub fn get_exchange_rate(&self, amount_sent: Uint128) -> Result<Decimal256, ContractError> {
        match self {
            ExchangeRate::Fixed(rate) => Ok(*rate),
            ExchangeRate::Variable(rate) => {
                let amount_decimal = Decimal256::from_ratio(amount_sent, 1u128);
                let rate_decimal = Decimal256::from_ratio(*rate, 1u128);
                Ok(amount_decimal
                    .checked_div(rate_decimal)
                    .map_err(|_| ContractError::Overflow {})?)
            }
        }
    }
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Cancels an ongoing sale
    #[attrs(restricted)]
    CancelSale { asset: Asset },
    /// Cancels an ongoing redeem
    #[attrs(restricted)]
    CancelRedeem { asset: Asset },
    /// Purchases tokens with native funds
    Purchase { recipient: Option<Recipient> },
    /// Starts a redeem
    StartRedeem {
        /// The accepted asset for redemption
        redeem_asset: Asset,
        /// The rate at which to exchange tokens (amount of exchanged asset to purchase sale asset)
        exchange_rate: ExchangeRate,
        /// The recipient of the sale proceeds
        recipient: Option<Recipient>,
        schedule: Schedule,
    },

    /// Replenishes a redeem
    ReplenishRedeem {
        /// The accepted asset for redemption
        redeem_asset: Asset,
        /// The type of exchange rate whether it was fixed or variable
        exchange_rate_type: Option<ExchangeRate>,
    },

    Redeem {
        /// Optional recipient to redeem on behalf of another address
        recipient: Option<Recipient>,
    },
    /// Receive for CW20 tokens, used for purchasing and starting sales
    #[attrs(nonpayable)]
    Receive(Cw20ReceiveMsg),
}

/// Struct used to define a token sale. The asset used for the sale is defined as the key for the storage map.
#[cw_serde]
pub struct Sale {
    /// The rate at which to exchange tokens (amount of exchanged asset to purchase sale asset)
    pub exchange_rate: Uint128,
    pub remaining_amount: Uint128,
    pub start_amount: Uint128,
    /// The recipient of the sale proceeds
    pub recipient: Recipient,
    /// The time when the sale starts
    pub start_time: Milliseconds,
    /// The time when the sale ends
    pub end_time: Option<Milliseconds>,
}

/// Struct used to define a token sale. The asset used for the sale is defined as the key for the storage map.
#[cw_serde]
pub struct Redeem {
    /// The asset that will be given in return for the redeemed asset
    pub asset: Asset,
    /// The rate at which to exchange tokens (amount of exchanged asset to purchase sale asset)
    pub exchange_rate: Decimal256,
    /// The type of exchange rate whether it was fixed or variable
    pub exchange_type: ExchangeRate,
    /// The amount for sale at the given rate
    pub amount: Uint128,
    /// The amount paid out
    pub amount_paid_out: Uint128,
    /// The recipient of the sale proceeds
    pub recipient: Recipient,
    /// The time when the sale starts
    pub start_time: Milliseconds,
    /// The time when the sale ends
    pub end_time: Option<Milliseconds>,
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Starts a sale
    StartSale {
        /// The asset that may be used to purchase tokens
        asset: Asset,
        /// The amount of the above asset required to purchase a single token
        exchange_rate: Uint128,
        /// The recipient of the sale proceeds
        /// Sender is used if `None` provided
        recipient: Option<Recipient>,
        schedule: Schedule,
    },
    /// Purchases tokens
    Purchase {
        /// Optional recipient to purchase on behalf of another address
        recipient: Option<Recipient>,
    },
    /// Starts a redeem
    StartRedeem {
        /// The accepted asset for redemption
        redeem_asset: Asset,
        /// An exchange rate of 2 would grant the redeemer 2 asset for 1 redeem_asset
        /// An exchange rate of 0.5 would grant the redeemer 2 asset for 4 redeem_asset
        exchange_rate: ExchangeRate,
        /// The recipient of the sale proceeds
        recipient: Option<Recipient>,
        schedule: Schedule,
    },
    /// Replenishes a redeem
    ReplenishRedeem {
        /// The accepted asset for redemption
        redeem_asset: Asset,
        /// The type of exchange rate whether it was fixed or variable
        exchange_rate_type: Option<ExchangeRate>,
    },
    /// Redeems tokens
    Redeem {
        /// Optional recipient to redeem on behalf of another address
        recipient: Option<Recipient>,
    },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Sale info for a given asset
    #[returns(SaleResponse)]
    Sale { asset: String },
    /// Redeem info
    #[returns(RedeemResponse)]
    Redeem { asset: Asset },
    /// The address of the token being purchased
    #[returns(TokenAddressResponse)]
    TokenAddress {},
    #[returns(SaleAssetsResponse)]
    SaleAssets {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct SaleAssetsResponse {
    pub assets: Vec<String>,
}

#[cw_serde]
pub struct SaleResponse {
    /// The sale data if it exists
    pub sale: Option<Sale>,
}

#[cw_serde]
pub struct RedeemResponse {
    /// The redeem data if it exists
    pub redeem: Option<Redeem>,
}

#[cw_serde]
pub struct TokenAddressResponse {
    /// The address of the token being sold
    pub address: String,
}

pub fn to_uint128_with_precision(value: &Decimal256) -> StdResult<Uint128> {
    let uint_value = value.atomics();

    uint_value
        .checked_div(10u128.pow(value.decimal_places() - 1).into())?
        .try_into()
        .map_err(|o: ConversionOverflowError| {
            StdError::generic_err(format!("Error converting {}", o))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(Decimal256::percent(50), Uint128::new(5))] // 0.5 with 1 decimal place
    #[case(Decimal256::permille(500), Uint128::new(5))] // 0.5 with 2 decimal places
    #[case(Decimal256::from_ratio(500u128, 1000u128), Uint128::new(5))] // 0.5 with 3 decimal places
    #[case(Decimal256::zero(), Uint128::zero())] // zero
    #[case(Decimal256::from_ratio(1234567u128, 1000u128), Uint128::new(12345))] // large number
    fn test_to_uint128_with_precision_success(
        #[case] value: Decimal256,
        #[case] expected: Uint128,
    ) {
        let result = to_uint128_with_precision(&value).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_to_uint128_with_precision_overflow() {
        let value = Decimal256::MAX;
        let result = to_uint128_with_precision(&value);
        assert!(result.is_err());
    }
}
