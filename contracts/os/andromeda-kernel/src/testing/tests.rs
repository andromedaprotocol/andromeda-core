use crate::{
    contract::{execute, instantiate},
    state::ADO_OWNER,
};
use andromeda_std::{
    amp::{ADO_DB_KEY, VFS_KEY},
    os::kernel::{ExecuteMsg, InstantiateMsg},
    testing::mock_querier::{mock_dependencies_custom, MOCK_ADODB_CONTRACT, MOCK_VFS_CONTRACT},
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Binary,
};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        owner: None,
        chain_name: None,
    };
    let env = mock_env();

    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_create_ado() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let env = mock_env();
    instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        InstantiateMsg {
            owner: None,
            chain_name: None,
        },
    )
    .unwrap();

    let assign_key_msg = ExecuteMsg::UpsertKeyAddress {
        key: ADO_DB_KEY.to_string(),
        value: MOCK_ADODB_CONTRACT.to_string(),
    };
    execute(deps.as_mut(), env.clone(), info.clone(), assign_key_msg).unwrap();
    let assign_key_msg = ExecuteMsg::UpsertKeyAddress {
        key: VFS_KEY.to_string(),
        value: MOCK_VFS_CONTRACT.to_string(),
    };
    execute(deps.as_mut(), env.clone(), info.clone(), assign_key_msg).unwrap();

    let create_msg = ExecuteMsg::Create {
        ado_type: "ado_type".to_string(),
        msg: Binary::default(),
        owner: None,
    };
    let res = execute(deps.as_mut(), env, info.clone(), create_msg).unwrap();
    assert_eq!(1, res.messages.len());
    assert_eq!(
        ADO_OWNER.load(deps.as_ref().storage).unwrap(),
        info.sender
    );
}
