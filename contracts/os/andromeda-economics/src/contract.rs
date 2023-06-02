use crate::state::BALANCES;
use andromeda_std::ado_base::InstantiateMsg as BaseInstantiateMsg;
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::amp::AndrAddr;
use andromeda_std::common::encode_binary;
use andromeda_std::error::{from_semver, ContractError};
use andromeda_std::os::economics::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use cosmwasm_std::{
    attr, ensure, entry_point, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError,
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-economics";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "economics".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit { address } => execute_deposit_native(deps, info, address),
        _ => Ok(Response::default()),
    }
}

pub fn execute_deposit_native(
    deps: DepsMut,
    info: MessageInfo,
    address: Option<AndrAddr>,
) -> Result<Response, ContractError> {
    ensure!(!info.funds.is_empty(), ContractError::CoinNotFound {});

    let addr = address
        .unwrap_or(AndrAddr::from_string(info.sender.to_string()))
        .get_raw_address(&deps.as_ref())?;

    for funds in info.funds.clone() {
        let balance = BALANCES
            .load(
                deps.as_ref().storage,
                (addr.clone(), funds.denom.to_string()),
            )
            .unwrap_or_default();

        BALANCES.save(
            deps.storage,
            (addr.clone(), funds.denom.to_string()),
            &(balance + funds.amount),
        )?;
    }

    let resp = Response::default();

    Ok(resp)
}

fn execute_pay_fee(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    payee: Addr,
    action: String,
) -> Result<Response, ContractError> {
    let mut resp = Response::default();

    resp.attributes = vec![
        attr("action", action),
        attr("sender", info.sender.to_string()),
        attr("payee", payee.to_string()),
        attr("amount", "1000"),
    ];

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    let contract = ADOContract::default();

    ensure!(
        stored.contract == CONTRACT_NAME,
        ContractError::CannotMigrate {
            previous_contract: stored.contract,
        }
    );

    // New version has to be newer/greater than the old version
    ensure!(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        }
    );

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Update the ADOContract's version
    contract.execute_update_version(deps)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {}
}
