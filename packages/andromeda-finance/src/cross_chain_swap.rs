use andromeda_std::{amp::AndrAddr, andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Coin, Decimal, Uint128};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    SwapAndForward {
        dex: String,
        to_denom: String,
        forward_addr: AndrAddr,
        forward_msg: Option<Binary>,
        slippage_percentage: Decimal,
        window_seconds: Option<u64>,
    },
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

// Source: https://github.com/osmosis-labs/osmosis/blob/main/cosmwasm/contracts/swaprouter/src/msg.rs#L20
#[cw_serde]
pub enum OsmosisSlippage {
    Twap {
        window_seconds: Option<u64>,
        slippage_percentage: Decimal,
    },
    MinOutputAmount(Uint128),
}

#[cw_serde]
pub enum OsmosisSwapMsg {
    Swap {
        input_coin: Coin,
        output_denom: String,
        slippage: OsmosisSlippage,
    },
}

#[cw_serde]
pub struct OsmosisSwapResponse {
    pub original_sender: String,
    pub token_out_denom: String,
    pub amount: Uint128,
}

#[cfg(test)]
mod tests {}
