use super::mock_querier::{
    mock_dependencies_custom, MOCK_MIRROR_GOV_ADDR, MOCK_MIRROR_MINT_ADDR, MOCK_MIRROR_STAKING_ADDR,
};
use crate::contract::{execute, instantiate, query};
use andromeda_protocol::mirror_wrapped_cdp::InstantiateMsg;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::CanonicalAddr;

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);

    let msg = InstantiateMsg {
        mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string(),
        mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
        mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
    };

    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
}
