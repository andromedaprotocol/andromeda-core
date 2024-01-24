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
    EvalExpression {},
}

#[cw_serde]
pub struct ShuntingResponse {
    pub result: String,
}

#[cw_serde]
pub struct ShuntingObject {
    pub expression: String,
}
impl ShuntingObject {
    pub fn eval(&self) -> Result<f64, ContractError> {
        let expr = ShuntingParser::parse_str(&self.expression);
        if expr.is_err() {
            return Err(ContractError::InvalidExpression {
                msg: "Expression is not valid".to_string(),
            });
        };

        let result = MathContext::new().eval(&expr.unwrap());
        if result.is_err() {
            return Err(ContractError::InvalidExpression {
                msg: result.unwrap_err(),
            });
        }

        Ok(result.unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_expression() {
        let expression = "sin(0.2)^2 + cos(0.2)^2".to_string();
        let object = ShuntingObject { expression };
        let received = object.eval().unwrap();
        let expected: f64 = 1.0;
        assert_eq!(expected, received);
    }
}
