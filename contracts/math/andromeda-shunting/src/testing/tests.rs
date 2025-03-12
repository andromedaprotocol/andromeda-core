use crate::contract::{instantiate, query};
pub use andromeda_std::testing::mock_querier::MOCK_KERNEL_CONTRACT;

use andromeda_math::shunting::{EvaluateParam, InstantiateMsg, QueryMsg, ShuntingResponse};
use cosmwasm_std::{
    from_json,
    testing::{message_info, mock_dependencies, mock_env},
};

#[test]
fn test_instantiate_query() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner = deps.api.addr_make("owner");
    let info = message_info(&owner, &[]);
    let expressions = vec![
        "cos({x0})".to_string(),
        "sin({x0})".to_string(),
        "{y0}^2 + {y1}^2".to_string(),
    ];
    let msg = InstantiateMsg {
        expressions,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
    };
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
    let query_msg = QueryMsg::Evaluate {
        params: vec![EvaluateParam::Value("0.8".to_string())],
    };
    let response: ShuntingResponse =
        from_json(query(deps.as_ref(), env, query_msg).unwrap()).unwrap();

    assert_eq!(
        response,
        ShuntingResponse {
            result: "1".to_string()
        }
    );
}
