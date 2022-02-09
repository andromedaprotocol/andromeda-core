use crate::state::{ANDROMEDA_CW721_ADDR, CAN_UNWRAP};
use andromeda_protocol::{
    communication::{encode_binary, query_get},
    cw721::InstantiateMsg as Cw721InstantiateMsg,
    error::ContractError,
    factory::CodeIdResponse,
    ownership::CONTRACT_OWNER,
    require,
    response::get_reply_address,
    wrapped_cw721::{Cw721Specification, ExecuteMsg, InstantiateMsg, InstantiateType, QueryMsg},
};
use cosmwasm_std::{
    attr, coins, entry_point, Addr, BankMsg, Binary, BlockInfo, Coin, CosmosMsg, Deps, DepsMut,
    Env, MessageInfo, QuerierWrapper, QueryRequest, Reply, ReplyOn, Response, StdError, StdResult,
    Storage, SubMsg, Uint128, WasmMsg, WasmQuery,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;
    CAN_UNWRAP.save(deps.storage, &msg.can_unwrap)?;
    let mut resp: Response = Response::new();
    match msg.cw721_instantiate_type {
        InstantiateType::Address(addr) => ANDROMEDA_CW721_ADDR.save(deps.storage, &addr)?,
        InstantiateType::New(specification) => {
            let instantiate_msg = Cw721InstantiateMsg {
                name: specification.name,
                symbol: specification.symbol,
                modules: specification.modules,
                minter: env.contract.address.to_string(),
            };
            let code_id: u64 = query_get::<CodeIdResponse>(
                Some(encode_binary(&"cw721")?),
                msg.factory_contract.to_string(),
                deps.querier,
            )?
            .code_id;
            let msg: SubMsg = SubMsg {
                id: 1,
                reply_on: ReplyOn::Always,
                msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                    admin: None,
                    code_id,
                    msg: encode_binary(&instantiate_msg)?,
                    funds: vec![],
                    label: "Instantiate: andromeda_cw721".to_string(),
                }),
                gas_limit: None,
            };
            resp = resp.add_submessage(msg);
        }
    }

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }
    require(msg.id == 1, ContractError::InvalidReplyId {})?;

    let addr = get_reply_address(&msg)?;
    ANDROMEDA_CW721_ADDR.save(deps.storage, &addr)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    Ok(encode_binary(&"asdf".to_string())?)
}
