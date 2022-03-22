use crate::state::CONFIG;
use ado_base::state::ADOContract;
use andromeda_protocol::vault::{Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use common::{
    ado_base::{
        recipient::Recipient, AndromedaMsg, AndromedaQuery, InstantiateMsg as BaseInstantiateMsg,
    },
    error::ContractError,
    require,
};
use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use cw2::{get_contract_version, set_contract_version};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-rates";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config {
        default_yield_strategy: msg.default_yield_strategy,
    };
    CONFIG.save(deps.storage, &config)?;
    ADOContract::default().instantiate(
        deps,
        info,
        BaseInstantiateMsg {
            ado_type: "vault".to_string(),
            operators: None,
        },
    )
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AndrReceive(msg) => execute_andr_receive(deps, env, info, msg),
        ExecuteMsg::Deposit { recipient } => execute_deposit(deps, env, info, recipient),
    }
}

fn execute_andr_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    match msg {
        // AndromedaMsg::Receive(data) => {
        //     let rates: Vec<RateInfo> = parse_message(&data)?;
        //     execute_update_rates(deps, info, rates)
        // }
        _ => ADOContract::default().execute(deps, env, info, msg, execute),
    }
}

fn execute_deposit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    require(info.funds.len() > 0, ContractError::InsufficientFunds {})?;

    let config = CONFIG.load(deps.storage)?;
    let recipient = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));
    let mut resp = Response::default();

    for funds in info.funds {
        let deposit_msg_opt =
            config
                .default_yield_strategy
                .deposit(deps.storage, funds, &recipient.get_addr())?;

        if let Some(deposit_msg) = deposit_msg_opt {
            resp = resp.add_submessage(deposit_msg);
        }
    }

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, env, msg),
    }
}

fn handle_andromeda_query(
    deps: Deps,
    env: Env,
    msg: AndromedaQuery,
) -> Result<Binary, ContractError> {
    match msg {
        _ => ADOContract::default().query(deps, env, msg, query),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_protocol::vault::{InstantiateMsg, YieldStrategy};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn test_instantiate() {
        let yield_strategy = YieldStrategy::Anchor {
            address: "terra1anchoraddress".to_string(),
        };
        let inst_msg = InstantiateMsg {
            default_yield_strategy: yield_strategy.clone(),
        };
        let env = mock_env();
        let info = mock_info("minter", &[]);
        let mut deps = mock_dependencies(&[]);

        instantiate(deps.as_mut(), env, info, inst_msg).unwrap();

        let config = CONFIG.load(deps.as_ref().storage).unwrap();

        assert_eq!(config.default_yield_strategy, yield_strategy)
    }

    #[test]
    fn test_deposit() {}

    #[test]
    fn test_deposit_invalid_funds() {}
}
