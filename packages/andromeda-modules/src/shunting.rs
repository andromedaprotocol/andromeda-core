use andromeda_std::{andr_exec, andr_instantiate, andr_query, error::ContractError};
use cosmwasm_schema::{cw_serde, QueryResponses};
use shunting::*;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub expression: String,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    UpdateExpression { expression: String },
}

#[cw_serde]
pub struct MigrateMsg {}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ShuntingResponse)]
    Result {},
}

#[cw_serde]
pub struct ShuntingResponse {
    pub result: String,
}

/// Evaluate a shunting result.
///
/// ## Arguments
/// * `expression` - Input string which represents math expressions
///
/// Returns the eval result in float64.
pub fn eval_expression(expression: &str) -> Result<f64, ContractError> {
    let expr = ShuntingParser::parse_str(expression).unwrap();
    let result = MathContext::new().eval(&expr).unwrap();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_expression() {
        let input = "sin(0.2)^2 + cos(0.2)^2";
        let received = eval_expression(input).unwrap();
        let expected: f64 = 1.0;
        assert_eq!(expected, received);
    }
}
