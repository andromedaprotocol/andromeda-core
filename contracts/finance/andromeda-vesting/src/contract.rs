#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Timestamp, Uint128,
};

use cw0::{Duration, Expiration};

use ado_base::ADOContract;
use andromeda_finance::vesting::{ExecuteMsg, InstantiateMsg, QueryMsg};
use common::{
    ado_base::InstantiateMsg as BaseInstantiateMsg, error::ContractError, require,
    withdraw::WithdrawalType,
};

use crate::state::{save_batch, Batch, Config, BATCHES, CONFIG};

const CONTRACT_NAME: &str = "crates.io:andromeda-vesting";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        is_multi_batch_enabled: msg.is_multi_batch_enabled,
        recipient: msg.recipient,
        denom: msg.denom,
    };

    CONFIG.save(deps.storage, &config)?;

    ADOContract::default().instantiate(
        deps.storage,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "vesting".to_string(),
            operators: None,
            modules: None,
            primitive_contract: None,
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
        ExecuteMsg::CreateBatch {
            lockup_duration,
            release_unit,
            release_amount,
            stake,
        } => execute_create_batch(
            deps,
            info,
            env,
            lockup_duration,
            release_unit,
            release_amount,
            stake,
        ),
        ExecuteMsg::Claim {
            number_of_claims,
            batch_id,
        } => panic!(),
        ExecuteMsg::ClaimAll {
            start_after,
            limit,
            up_to_time,
        } => panic!(),
        ExecuteMsg::Stake { amount } => panic!(),
        ExecuteMsg::Unstake { amount } => panic!(),
    }
}

fn execute_create_batch(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    lockup_duration: Option<Duration>,
    release_unit: Duration,
    release_amount: WithdrawalType,
    stake: bool,
) -> Result<Response, ContractError> {
    require(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    let config = CONFIG.load(deps.storage)?;

    require(
        info.funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Creating a batch must be accompanied with a single native fund".to_string(),
        },
    )?;

    let funds = info.funds[0];

    require(
        funds.denom == config.denom,
        ContractError::InvalidFunds {
            msg: "Invalid denom".to_string(),
        },
    )?;

    require(
        !funds.amount.is_zero(),
        ContractError::InvalidFunds {
            msg: "Funds must be non-zero".to_string(),
        },
    )?;

    // TODO: Check for non-zero duration.

    let lockup_end = if let Some(duration) = lockup_duration {
        Some(duration.after(&env.block))
    } else {
        None
    };

    let last_claim_time = match release_unit {
        Duration::Time(_) => Expiration::AtTime(Timestamp::default()),
        Duration::Height(_) => Expiration::AtHeight(0),
    };

    let batch = Batch {
        amount: funds.amount,
        lockup_end,
        release_unit,
        release_amount,
        last_claim_time,
    };

    save_batch(deps.storage, batch)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    Ok(to_binary(&"")?)
}
