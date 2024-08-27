use andromeda_std::{andr_exec, andr_instantiate, andr_query};
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
    GetDateTime { timezone: Option<Timezone> },
}

#[cw_serde]
pub struct GetDateTimeResponse {
    pub day_of_week: String,
    pub date_time: String,
}

#[cw_serde]
pub enum Timezone {
    UtcMinus12 = -1200,
    UtcMinus11 = -1100,
    UtcMinus10 = -1000,
    UtcMinus9_30 = -950,
    UtcMinus9 = -900,
    UtcMinus8 = -800,
    UtcMinus7 = -700,
    UtcMinus6 = -600,
    UtcMinus5 = -500,
    UtcMinus4 = -400,
    UtcMinus3 = -300,
    UtcMinus2_30 = -250,
    UtcMinus2 = -200,
    UtcMinus1 = -100,
    Utc = 0,
    UtcPlus1 = 100,
    UtcPlus2 = 200,
    UtcPlus3 = 300,
    UtcPlus3_30 = 350,
    UtcPlus4 = 400,
    UtcPlus4_30 = 450,
    UtcPlus5 = 500,
    UtcPlus5_45 = 575,
    UtcPlus5_30 = 550,
    UtcPlus6 = 600,
    UtcPlus6_30 = 650,
    UtcPlus7 = 700,
    UtcPlus8 = 800,
    UtcPlus8_45 = 875,
    UtcPlus9 = 900,
    UtcPlus9_30 = 950,
    UtcPlus10 = 1000,
    UtcPlus10_30 = 1050,
    UtcPlus11 = 1100,
    UtcPlus12 = 1200,
    UtcPlus12_45 = 1275,
    UtcPlus13 = 1300,
    UtcPlus14 = 1400,
}
