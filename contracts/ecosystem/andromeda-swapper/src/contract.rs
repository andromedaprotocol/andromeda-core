use crate::state::SWAPPER_IMPL_ADDR;
use ado_base::ADOContract;
use andromeda_ecosystem::swapper::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SwapperCw20HookMsg, SwapperImpl,
    SwapperImplCw20HookMsg, SwapperImplExecuteMsg, SwapperMsg,
};
use common::{
    ado_base::{recipient::Recipient, InstantiateMsg as BaseInstantiateMsg},
    app::AndrAddress,
    encode_binary,
    error::ContractError,
    response::get_reply_address,
};
use cosmwasm_std::{
    ensure, entry_point, from_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Reply, Response, StdError, SubMsg, Uint128, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20Coin, Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_asset::AssetInfo;
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-swapper";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let contract = ADOContract::default();
    let resp = contract.instantiate(
        deps.storage,
        env,
        deps.api,
        info.clone(),
        BaseInstantiateMsg {
            ado_type: "swapper".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
            primitive_contract: Some(msg.primitive_contract),
            kernel_address: msg.kernel_address,
        },
    )?;
    let mut msgs: Vec<SubMsg> = vec![];
    match msg.swapper_impl {
        SwapperImpl::Reference(andr_address) => {
            SWAPPER_IMPL_ADDR.save(deps.storage, &andr_address)?
        }
        SwapperImpl::New(instantiate_info) => {
            let msg = contract.generate_instantiate_msg(
                deps.storage,
                &deps.querier,
                1,
                instantiate_info.msg,
                instantiate_info.ado_type,
                info.sender.to_string(),
            )?;
            msgs.push(msg);
        }
    }
    Ok(resp.add_submessages(msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }
    ensure!(msg.id == 1, ContractError::InvalidReplyId {});

    let addr = get_reply_address(msg)?;
    SWAPPER_IMPL_ADDR.save(deps.storage, &AndrAddress { identifier: addr })?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
        }
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Swap {
            ask_asset_info,
            recipient,
        } => execute_swap(deps, env, info, ask_asset_info, recipient),
        ExecuteMsg::Send {
            ask_asset_info,
            recipient,
        } => execute_send(deps, env, info, ask_asset_info, recipient),
    }
}

fn execute_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    ask_asset_info: AssetInfo,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let recipient = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));

    ensure!(
        info.funds.len() <= 1,
        ContractError::InvalidFunds {
            msg: "Must send at most one native coin".to_string(),
        }
    );
    ensure!(
        !info.funds.is_empty() && info.funds[0].amount > Uint128::zero(),
        ContractError::InvalidFunds {
            msg: "Must send funds to swap".to_string(),
        }
    );

    let coin = &info.funds[0];
    if let AssetInfo::Native(denom) = &ask_asset_info {
        if denom == &coin.denom {
            // Send coins as is as there is no need to swap.
            let msg = recipient.generate_msg_native(
                deps.api,
                &deps.querier,
                ADOContract::default().get_app_contract(deps.storage)?,
                info.funds,
            )?;
            return Ok(Response::new()
                .add_submessage(msg)
                .add_attribute("action", "swap"));
        }
    }
    let andr_address = SWAPPER_IMPL_ADDR.load(deps.storage)?;
    let app_contract = ADOContract::default().get_app_contract(deps.storage)?;
    let contract_addr = andr_address.get_address(deps.api, &deps.querier, app_contract)?;

    let denom = coin.denom.clone();
    Ok(Response::new()
        .add_attribute("action", "swap")
        .add_attribute("offer_denom", &denom)
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            funds: info.funds,
            msg: encode_binary(&SwapperImplExecuteMsg::Swapper(SwapperMsg::Swap {
                offer_asset_info: AssetInfo::Native(denom),
                ask_asset_info: ask_asset_info.clone(),
            }))?,
        }))
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            funds: vec![],
            msg: encode_binary(&ExecuteMsg::Send {
                ask_asset_info,
                recipient,
            })?,
        })))
}

fn execute_send(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    ask_asset_info: AssetInfo,
    recipient: Recipient,
) -> Result<Response, ContractError> {
    ensure!(
        info.sender == env.contract.address,
        ContractError::Unauthorized {}
    );
    let msg: SubMsg = match &ask_asset_info {
        cw_asset::AssetInfoBase::Native(denom) => {
            let amount = ask_asset_info.query_balance(&deps.querier, env.contract.address)?;
            recipient.generate_msg_native(
                deps.api,
                &deps.querier,
                ADOContract::default().get_app_contract(deps.storage)?,
                vec![Coin {
                    denom: denom.to_owned(),
                    amount,
                }],
            )?
        }
        cw_asset::AssetInfoBase::Cw20(contract_addr) => {
            let amount = ask_asset_info.query_balance(&deps.querier, env.contract.address)?;
            recipient.generate_msg_cw20(
                deps.api,
                &deps.querier,
                ADOContract::default().get_app_contract(deps.storage)?,
                Cw20Coin {
                    address: contract_addr.to_string(),
                    amount,
                },
            )?
        }
        _ => todo!(),
    };
    Ok(Response::new()
        .add_attribute("action", "send")
        .add_submessage(msg))
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    ensure!(
        !cw20_msg.amount.is_zero(),
        ContractError::InvalidFunds {
            msg: "Amount must be non-zero".to_string(),
        }
    );

    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::Swap {
            ask_asset_info,
            recipient,
        } => execute_swap_cw20(
            deps,
            env,
            info.sender.to_string(),
            cw20_msg.amount,
            ask_asset_info,
            cw20_msg.sender,
            recipient,
        ),
    }
}

fn execute_swap_cw20(
    deps: DepsMut,
    env: Env,
    offer_token: String,
    offer_amount: Uint128,
    ask_asset_info: AssetInfo,
    sender: String,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let recipient = recipient.unwrap_or(Recipient::Addr(sender));
    if let AssetInfo::Cw20(contract_addr) = &ask_asset_info {
        if *contract_addr == offer_token {
            // Send as is.
            let msg = recipient.generate_msg_cw20(
                deps.api,
                &deps.querier,
                ADOContract::default().get_app_contract(deps.storage)?,
                Cw20Coin {
                    address: offer_token,
                    amount: offer_amount,
                },
            )?;
            return Ok(Response::new()
                .add_submessage(msg)
                .add_attribute("action", "swap"));
        }
    }
    let andr_address = SWAPPER_IMPL_ADDR.load(deps.storage)?;
    let app_contract = ADOContract::default().get_app_contract(deps.storage)?;
    let contract_addr = andr_address.get_address(deps.api, &deps.querier, app_contract)?;

    Ok(Response::new()
        .add_attribute("action", "swap")
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: offer_token,
            funds: vec![],
            msg: encode_binary(&Cw20ExecuteMsg::Send {
                contract: contract_addr,
                amount: offer_amount,
                msg: encode_binary(&SwapperImplCw20HookMsg::Swapper(SwapperCw20HookMsg::Swap {
                    ask_asset_info: ask_asset_info.clone(),
                }))?,
            })?,
        }))
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            funds: vec![],
            msg: encode_binary(&ExecuteMsg::Send {
                ask_asset_info,
                recipient,
            })?,
        })))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::SwapperImpl {} => encode_binary(&query_swapper_impl(deps)?),
    }
}

fn query_swapper_impl(deps: Deps) -> Result<AndrAddress, ContractError> {
    let andr_address = SWAPPER_IMPL_ADDR.load(deps.storage)?;
    Ok(andr_address)
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

fn from_semver(err: semver::Error) -> StdError {
    StdError::generic_err(format!("Semver: {}", err))
}
