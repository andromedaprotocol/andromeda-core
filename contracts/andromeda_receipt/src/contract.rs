use crate::state::{increment_num_receipt, read_receipt, store_config, store_receipt, Config};
use andromeda_protocol::receipt::{ExecuteMsg, InstantiateMsg, QueryMsg, Receipt, ReceiptResponse};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            owner: info.sender.to_string(), // token contract address
        },
    )?;
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::StoreReceipt { receipt } => execute_store_receipt(deps, receipt),
    }
}

fn execute_store_receipt(deps: DepsMut, receipt: Receipt) -> StdResult<Response> {
    let receipt_id = increment_num_receipt(deps.storage)?;
    store_receipt(deps.storage, receipt_id, &receipt)?;
    Ok(Response::new().add_attribute("receipt_id", receipt_id.to_string()))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Receipt { receipt_id } => Ok(to_binary(&query_receipt(deps, receipt_id)?)?),
    }
}

fn query_receipt(deps: Deps, receipt_id: Uint128) -> StdResult<ReceiptResponse> {
    let receipt = read_receipt(deps.storage, receipt_id)?;
    Ok(ReceiptResponse { receipt })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    #[test]
    fn test_instantiate() {
        let owner = "creator";
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {
            owner: owner.to_string(),
        };
        let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_store_receipt() {
        let owner = "creator";
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let config = Config {
            owner: owner.to_string(),
        };
        store_config(deps.as_mut().storage, &config).unwrap();

        let msg = ExecuteMsg::StoreReceipt {
            receipt: Receipt {
                token_id: String::from("token_id"),
                seller: String::from("seller"),
                purchaser: String::from("purchaser"),
                amount: Uint128::from(1 as u128),
                payments_info: vec!["1>recever1".to_string()],
                payment_desc: vec![String::default()],
            },
        };
        //add address for registered moderator
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        assert_eq!(Response::new().add_attribute("receipt_id", "1"), res);
    }
}
