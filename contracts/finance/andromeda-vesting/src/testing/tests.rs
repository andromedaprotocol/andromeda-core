use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Response,
};

use crate::{
    contract::{execute, instantiate, query},
    state::{batches, Batch, Config, CONFIG},
};

use andromeda_finance::vesting::{ExecuteMsg, InstantiateMsg, QueryMsg};
use common::ado_base::recipient::Recipient;

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("owner", &[]);

    let msg = InstantiateMsg {
        recipient: Recipient::Addr("recipient".to_string()),
        is_multi_batch_enabled: true,
        denom: "uusd".to_string(),
    };

    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("method", "instantiate")
            .add_attribute("type", "vesting"),
        res
    );

    assert_eq!(
        Config {
            recipient: Recipient::Addr("recipient".to_string()),
            is_multi_batch_enabled: true,
            denom: "uusd".to_string()
        },
        CONFIG.load(deps.as_ref().storage).unwrap()
    );
}
