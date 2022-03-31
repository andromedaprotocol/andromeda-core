use crate::state::{
    can_mint_receipt, increment_num_receipt, read_receipt, store_config, store_receipt, CONFIG,
};
use ado_base::state::ADOContract;
use andromeda_protocol::receipt::{
    generate_receipt_message, Config, ContractInfoResponse, ExecuteMsg, InstantiateMsg, MigrateMsg,
    QueryMsg, Receipt, ReceiptResponse,
};
use common::{
    ado_base::{
        hooks::{AndromedaHook, OnFundsTransferResponse},
        InstantiateMsg as BaseInstantiateMsg,
    },
    encode_binary,
    error::ContractError,
    parse_message, require,
};
use cosmwasm_std::{
    attr, entry_point, Binary, Deps, DepsMut, Env, Event, MessageInfo, Response, Uint128,
};
use cw2::{get_contract_version, set_contract_version};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-receipt";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    store_config(deps.storage, &Config { minter: msg.minter })?;
    ADOContract::default().instantiate(
        deps.storage,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "receipt".to_string(),
            operators: msg.operators,
            modules: None,
            primitive_contract: None,
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
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
        }
        ExecuteMsg::StoreReceipt { receipt } => execute_store_receipt(deps, info, receipt),
        ExecuteMsg::EditReceipt {
            receipt,
            receipt_id,
        } => execute_edit_receipt(deps, info, receipt_id, receipt),
    }
}

fn execute_store_receipt(
    deps: DepsMut,
    info: MessageInfo,
    receipt: Receipt,
) -> Result<Response, ContractError> {
    require(
        can_mint_receipt(deps.storage, &info.sender.to_string())?,
        ContractError::Unauthorized {},
    )?;
    let receipt_id = increment_num_receipt(deps.storage)?;
    store_receipt(deps.storage, receipt_id, &receipt)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "mint_receipt"),
        attr("receipt_id", receipt_id.to_string()),
    ]))
}

fn execute_edit_receipt(
    deps: DepsMut,
    info: MessageInfo,
    receipt_id: Uint128,
    receipt: Receipt,
) -> Result<Response, ContractError> {
    require(
        can_mint_receipt(deps.storage, &info.sender.to_string())?,
        ContractError::Unauthorized {},
    )?;
    read_receipt(deps.storage, receipt_id)?;
    store_receipt(deps.storage, receipt_id, &receipt)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "edit_receipt"),
        attr("receipt_id", receipt_id.to_string()),
        attr("receipt_edited_by", info.sender.to_string()),
    ]))
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
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::Receipt { receipt_id } => encode_binary(&query_receipt(deps, receipt_id)?),
        QueryMsg::ContractInfo {} => encode_binary(&query_config(deps)?),
        QueryMsg::AndrHook(msg) => handle_andr_hook(env, msg),
    }
}

fn handle_andr_hook(env: Env, msg: AndromedaHook) -> Result<Binary, ContractError> {
    match msg {
        AndromedaHook::OnFundsTransfer {
            sender: _,
            payload,
            amount,
        } => {
            let events: Vec<Event> = parse_message(&Some(payload))?;
            let msg = generate_receipt_message(env.contract.address.to_string(), events)?;
            encode_binary(&OnFundsTransferResponse {
                msgs: vec![msg],
                leftover_funds: amount,
                events: vec![],
            })
        }
        _ => Err(ContractError::UnsupportedOperation {}),
    }
}

fn query_receipt(deps: Deps, receipt_id: Uint128) -> Result<ReceiptResponse, ContractError> {
    let receipt = read_receipt(deps.storage, receipt_id)?;
    Ok(ReceiptResponse { receipt })
}

fn query_config(deps: Deps) -> Result<ContractInfoResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    Ok(ContractInfoResponse { config })
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{
        ado_base::{AndromedaMsg, AndromedaQuery},
        Funds,
    };
    use cosmwasm_std::{
        coin, from_binary,
        testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR},
        CosmosMsg, Event, SubMsg, WasmMsg,
    };

    #[test]
    fn test_instantiate() {
        let owner = "creator";
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {
            minter: owner.to_string(),
            operators: None,
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_store_receipt() {
        let owner = "creator";
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let unauth_info = mock_info("anyone", &[]);

        instantiate(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            InstantiateMsg {
                minter: owner.to_string(),
                operators: None,
            },
        )
        .unwrap();

        let msg = ExecuteMsg::StoreReceipt {
            receipt: Receipt { events: vec![] },
        };

        let res_unauth = execute(deps.as_mut(), env.clone(), unauth_info, msg.clone()).unwrap_err();
        assert_eq!(res_unauth, ContractError::Unauthorized {});

        //add address for registered operator
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            Response::new().add_attributes(vec![
                attr("action", "mint_receipt"),
                attr("receipt_id", "1"),
            ]),
            res
        );
    }

    #[test]
    fn test_edit_receipt() {
        let owner = "creator";
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let unauth_info = mock_info("anyone", &[]);

        instantiate(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            InstantiateMsg {
                minter: owner.to_string(),
                operators: None,
            },
        )
        .unwrap();

        let store_msg = ExecuteMsg::StoreReceipt {
            receipt: Receipt {
                events: vec![Event::new("test")],
            },
        };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), store_msg).unwrap();
        assert_eq!(
            Response::new().add_attributes(vec![
                attr("action", "mint_receipt"),
                attr("receipt_id", "1"),
            ]),
            res
        );

        let new_receipt = Receipt {
            events: vec![Event::new("new")],
        };
        let msg = ExecuteMsg::EditReceipt {
            receipt_id: Uint128::from(1_u128),
            receipt: new_receipt.clone(),
        };

        let res_unauth = execute(deps.as_mut(), env.clone(), unauth_info, msg.clone()).unwrap_err();
        assert_eq!(res_unauth, ContractError::Unauthorized {});

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let expected = Response::default().add_attributes(vec![
            attr("action", "edit_receipt"),
            attr("receipt_id", "1"),
            attr("receipt_edited_by", info.sender.to_string()),
        ]);

        assert_eq!(res, expected);

        let query_msg = QueryMsg::Receipt {
            receipt_id: Uint128::from(1_u128),
        };
        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let val: ReceiptResponse = from_binary(&res).unwrap();

        assert_eq!(val.receipt, new_receipt)
    }

    #[test]
    fn test_andr_receive() {
        let owner = "creator";
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info(owner, &[]);
        instantiate(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            InstantiateMsg {
                minter: owner.to_string(),
                operators: None,
            },
        )
        .unwrap();

        let msg = ExecuteMsg::StoreReceipt {
            receipt: Receipt { events: vec![] },
        };

        let msg =
            ExecuteMsg::AndrReceive(AndromedaMsg::Receive(Some(encode_binary(&msg).unwrap())));

        //add address for registered operator
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(
            Response::new().add_attributes(vec![
                attr("action", "mint_receipt"),
                attr("receipt_id", "1"),
            ]),
            res
        );

        let query_msg = QueryMsg::Receipt {
            receipt_id: Uint128::from(1_u128),
        };

        let query_msg = QueryMsg::AndrQuery(AndromedaQuery::Get(Some(
            encode_binary(&query_msg).unwrap(),
        )));
        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let val: ReceiptResponse = from_binary(&res).unwrap();
        let new_receipt = Receipt { events: vec![] };
        assert_eq!(val.receipt, new_receipt)
    }

    #[test]
    fn test_on_funds_transfer_hook() {
        let deps = mock_dependencies(&[]);
        let events: Vec<Event> = vec![Event::new("Event1"), Event::new("Event2")];

        let query_msg = QueryMsg::AndrHook(AndromedaHook::OnFundsTransfer {
            sender: "sender".to_string(),
            payload: encode_binary(&events).unwrap(),
            amount: Funds::Native(coin(0, "uusd")),
        });

        let res: OnFundsTransferResponse =
            from_binary(&query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();

        assert_eq!(
            OnFundsTransferResponse {
                msgs: vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                    msg: encode_binary(&ExecuteMsg::StoreReceipt {
                        receipt: Receipt { events }
                    })
                    .unwrap(),
                    funds: vec![],
                }))],
                events: vec![],
                leftover_funds: Funds::Native(coin(0, "uusd"))
            },
            res
        );
    }
}
