use crate::contract::query;
use andromeda_std::{
    error::ContractError,
    os::ibc_registry::{AllDenomInfoResponse, DenomInfoResponse, QueryMsg},
};
use cosmwasm_std::{from_json, testing::mock_env, Deps};

pub fn query_denom_info(deps: Deps, denom: String) -> Result<DenomInfoResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::DenomInfo { denom });
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_all_denom_info(
    deps: Deps,
    limit: Option<u64>,
    start_after: Option<u64>,
) -> Result<AllDenomInfoResponse, ContractError> {
    let res = query(
        deps,
        mock_env(),
        QueryMsg::AllDenomInfo { limit, start_after },
    );
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}
