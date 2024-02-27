use crate::state::DEFAULT_VALIDATOR;
use cosmwasm_std::{entry_point, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;

use andromeda_finance::validator_staking::InstantiateMsg;

use andromeda_std::{
    ado_base::InstantiateMsg as BaseInstantiateMsg, ado_contract::ADOContract, error::ContractError,
};

const CONTRACT_NAME: &str = "crates.io:andromeda-validator-staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    msg.validate(&deps)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    DEFAULT_VALIDATOR.save(deps.storage, &msg.default_validator)?;

    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "validator-staking".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;
    Ok(inst_resp)
}
