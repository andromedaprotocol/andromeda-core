use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[andr_instantiate("no_modules")]
#[cw_serde]
pub struct InstantiateMsg {}

#[andr_exec("no_modules")]
#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[andr_query("no_modules")]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cfg(test)]
mod tests {
    use super::*;
}
