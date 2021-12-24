use super::mock_querier::{
    mock_dependencies_custom, mock_mint_config_response, mock_staking_config_response,
    MOCK_MIRROR_GOV_ADDR, MOCK_MIRROR_MINT_ADDR, MOCK_MIRROR_STAKING_ADDR,
};
use crate::contract::{execute, instantiate, query};
use andromeda_protocol::mirror_wrapped_cdp::{
    ConfigResponse, InstantiateMsg, MirrorMintQueryMsg, MirrorStakingQueryMsg, QueryMsg,
};
use cosmwasm_std::from_binary;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::CanonicalAddr;
use mirror_protocol::mint::ConfigResponse as MintConfigResponse;
use mirror_protocol::staking::ConfigResponse as StakingConfigResponse;

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

    // Verify that we can query the mirror mint contract.
    let msg = QueryMsg::MirrorMintQueryMsg(MirrorMintQueryMsg::Config {});
    let res: MintConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(mock_mint_config_response(), res);

    // Verify that we can query the mirror staking contract.
    let msg = QueryMsg::MirrorStakingQueryMsg(MirrorStakingQueryMsg::Config {});
    let res: StakingConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(mock_staking_config_response(), res);

    // Verify that we can query our contract's config.
    let msg = QueryMsg::Config {};
    let res: ConfigResponse = from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(
        ConfigResponse {
            mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
            mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
            mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string()
        },
        res
    );
}
