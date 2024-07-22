use andromeda_std::{
    andr_exec, andr_instantiate, andr_query,
};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetDateTimeResponse)]
    GetDateTime { 
        timezone: Timezone,
    },
}

#[cw_serde]
pub struct GetDateTimeResponse {
    pub day_of_week: String,
    pub date_time: String,
}

#[cw_serde]
pub enum Timezone {
    UtcMinus12 = -12,
    UtcMinus11 = -11,
    UtcMinus10 = -10,
    UtcMinus9 = -9,
    UtcMinus8 = -8,
    UtcMinus7 = -7,
    UtcMinus6 = -6,
    UtcMinus5 = -5,
    UtcMinus4 = -4,
    UtcMinus3 = -3,
    UtcMinus2 = -2,
    UtcMinus1 = -1,
    Utc = 0,
    UtcPlus1 = 1,
    UtcPlus2 = 2,
    UtcPlus3 = 3,
    UtcPlus4 = 4,
    UtcPlus5 = 5,
    UtcPlus6 = 6,
    UtcPlus7 = 7,
    UtcPlus8 = 8,
    UtcPlus9 = 9,
    UtcPlus10 = 10,
    UtcPlus11 = 11,
    UtcPlus12 = 12,
    UtcPlus13 = 13,
    UtcPlus14 = 14,
}
