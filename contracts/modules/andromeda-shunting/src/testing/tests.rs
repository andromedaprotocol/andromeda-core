use crate::contract::{execute, instantiate, query};
use crate::state::SHUNTING;
use crate::testing::mock_querier::{mock_dependencies_custom, MOCK_KERNEL_CONTRACT};
use andromeda_modules::shunting::{ExecuteMsg, InstantiateMsg, QueryMsg, ShuntingResponse};
use andromeda_std::ado_contract::ADOContract;

use andromeda_std::error::ContractError;

use cosmwasm_std::{attr, from_binary, DepsMut, MessageInfo};
use cosmwasm_std::{
    testing::{mock_env, mock_info},
    Response,
};

fn init(deps: DepsMut, info: MessageInfo) {
    instantiate(
        deps,
        mock_env(),
        info,
        InstantiateMsg {
            expression: "".to_string(),
            kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
            owner: None,
        },
    )
    .unwrap();
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);

    init(deps.as_mut(), info);
}

#[test]
fn test_update_expression() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let operator = "creator";
    let info = mock_info(operator, &[]);

    let expression = "3 + 2";

    init(deps.as_mut(), info.clone());

    ADOContract::default()
        .execute_update_operators(deps.as_mut(), info.clone(), vec![operator.to_owned()])
        .unwrap();

    let msg = ExecuteMsg::UpdateExpression {
        expression: expression.to_string(),
    };

    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
    let expected = Response::default().add_attributes(vec![
        attr("action", "update_expression"),
        attr("expression", expression),
    ]);
    assert_eq!(expected, res);

    let obj = SHUNTING.load(deps.as_ref().storage).unwrap();
    assert_eq!(obj.expression, expression);

    // update expression for unregistered operator
    let unauth_info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), env.clone(), unauth_info, msg).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, res);
}

#[test]
fn test_eval_expression() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let operator = "creator";
    let info = mock_info(operator, &[]);

    let expression_1 = "3 + 2";

    init(deps.as_mut(), info.clone());

    let msg = ExecuteMsg::UpdateExpression {
        expression: expression_1.to_string(),
    };
    execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();

    let msg = QueryMsg::EvalExpression {};
    let res: ShuntingResponse = from_binary(&query(deps.as_ref(), env, msg).unwrap()).unwrap();
    assert_eq!("5", res.result);
}
