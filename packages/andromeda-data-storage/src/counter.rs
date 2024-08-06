use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub restriction: CounterRestriction,
    pub initial_state: State,
}

#[cw_serde]
pub struct State {
    pub initial_amount: Option<u64>,
    pub increase_amount: Option<u64>,
    pub decrease_amount: Option<u64>,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    Increment {},
    Decrement {},
    Reset {},
    UpdateRestriction { restriction: CounterRestriction },
    SetIncreaseAmount { increase_amount: u64 },
    SetDecreaseAmount { decrease_amount: u64 },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetInitialAmountResponse)]
    GetInitialAmount {},
    #[returns(GetCurrentAmountResponse)]
    GetCurrentAmount {},
    #[returns(GetIncreaseAmountResponse)]
    GetIncreaseAmount {},
    #[returns(GetDecreaseAmountResponse)]
    GetDecreaseAmount {},
    #[returns(GetRestrictionResponse)]
    GetRestriction {},
}

#[cw_serde]
pub enum CounterRestriction {
    Private,
    Public,
}

#[cw_serde]
pub struct GetInitialAmountResponse {
    pub initial_amount: u64,
}

#[cw_serde]
pub struct GetCurrentAmountResponse {
    pub current_amount: u64,
}

#[cw_serde]
pub struct GetIncreaseAmountResponse {
    pub increase_amount: u64,
}

#[cw_serde]
pub struct GetDecreaseAmountResponse {
    pub decrease_amount: u64,
}

#[cw_serde]
pub struct GetRestrictionResponse {
    pub restriction: CounterRestriction,
}
