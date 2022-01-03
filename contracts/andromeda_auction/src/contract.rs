use crate::state::{Config, TokenAuctionState, CONFIG, FUNDS_BY_BIDDER, TOKEN_AUCTION_STATE};
use andromeda_protocol::{
    auction::{AuctionStateResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    token::QueryMsg as TokenQueryMsg,
};
use cosmwasm_std::{
    attr, coin, entry_point, to_binary, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, QuerierWrapper, QueryRequest, Response, StdError, StdResult, Uint128, WasmQuery,
};
use cw721::OwnerOfResponse;
use std::ops::Add;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config: Config = Config {
        token_addr: msg.token_addr,
        stable_denom: msg.stable_denom,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::PlaceBid { token_id } => execute_place_bid(deps, env, info, token_id),
        ExecuteMsg::Withdraw { token_id } => execute_withdraw(deps, env, info, token_id),
        ExecuteMsg::StartAuction {
            token_id,
            start_time,
            end_time,
        } => execute_start_auction(deps, env, info, token_id, start_time, end_time),
    }
}

pub fn execute_place_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let coin_denom = config.stable_denom.clone();

    if info.funds.len() > 1usize {
        return Err(StdError::generic_err(
            "More than one coin is sent; only one asset is supported",
        ));
    }
    let payment = info
        .funds
        .iter()
        .find(|x| x.denom == coin_denom && x.amount > Uint128::zero())
        .ok_or_else(|| {
            StdError::generic_err(format!("No {} assets are provided to auction", coin_denom))
        })?;
    //check after start
    let token_auction_state = TOKEN_AUCTION_STATE.may_load(deps.storage, token_id.clone())?;
    if token_auction_state.is_none() {
        return Err(StdError::generic_err("not started"));
    }
    let mut token_auction_state = token_auction_state.unwrap();

    if env.block.time.seconds() < token_auction_state.start_time {
        return Err(StdError::generic_err("not started"));
    }
    // check before end
    if env.block.time.seconds() > token_auction_state.end_time {
        return Err(StdError::generic_err("ended"));
    }

    //can't bid token's owner
    let token_owner = query_token_info(deps.querier, config.token_addr, token_id.clone())?;

    if token_owner.owner == info.sender {
        return Err(StdError::generic_err(
            "token owner is not allowed to withdraw",
        ));
    }

    let mut funds_by_bidder_token = FUNDS_BY_BIDDER
        .load(deps.storage, token_id.clone())
        .unwrap_or_default();

    let funds_by_bidder_option = funds_by_bidder_token
        .iter_mut()
        .find(|x| x.0 == info.sender.to_string());

    if let Some(funds_by_bidder) = funds_by_bidder_option {
        if funds_by_bidder.1.checked_add(payment.amount)? <= token_auction_state.high_bidder_amount
        {
            return Err(StdError::generic_err(
                "bid amount is small than highest_binding_bid",
            ));
        }
        funds_by_bidder.1 = funds_by_bidder.1.add(payment.amount);
        token_auction_state.high_bidder_amount = funds_by_bidder.1;
    } else {
        if payment.amount <= token_auction_state.high_bidder_amount {
            return Err(StdError::generic_err(
                "bid amount is small than highest_binding_bid",
            ));
        }
        funds_by_bidder_token.push((info.sender.to_string(), payment.amount));
        token_auction_state.high_bidder_amount = payment.amount;
    }
    token_auction_state.high_bidder_addr = info.sender.clone();

    TOKEN_AUCTION_STATE.save(deps.storage, token_id.clone(), &token_auction_state)?;
    FUNDS_BY_BIDDER.save(deps.storage, token_id.clone(), &funds_by_bidder_token)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "bid"),
        attr("token_id", token_id.clone()),
        attr("bider", info.sender.to_string()),
        attr("amount", payment.amount.to_string()),
    ]))
}
// no high bidder allows to withdraw
// only withdraw by no high bidder or no owner

// the amount placed by high_bidder will be transfered on on_transfer()
pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    //let token_owner = query_token_info(deps.querier, config.token_addr, token_id.clone())?;

    let token_auction_state = TOKEN_AUCTION_STATE.may_load(deps.storage, token_id.clone())?;
    if token_auction_state.is_none() {
        return Err(StdError::generic_err("not started"));
    }
    let token_auction_state = token_auction_state.unwrap();

    if env.block.time.seconds() < token_auction_state.start_time {
        return Err(StdError::generic_err("not started auction"));
    }

    if info.sender == token_auction_state.high_bidder_addr {
        return Err(StdError::generic_err(
            "high bidder does not allow to withdraw",
        ));
    }

    // token_owner
    // let withdraw_amount;
    // if info.sender == token_owner.owner {
    //
    //     let owner_has_withdrawn = OWNER_HAS_WITHDRAWN.load(deps.storage, token_id.clone())?;
    //     if owner_has_withdrawn {
    //         return Err(StdError::generic_err("already withdrawn"));
    //     }
    //
    //     withdraw_amount = token_auction_state.high_bidder_amount;
    //     OWNER_HAS_WITHDRAWN.save(deps.storage, token_id.clone(), &true)?;
    //
    // }else{
    let funds_by_bidder_token = FUNDS_BY_BIDDER
        .load(deps.storage, token_id.clone())
        .unwrap_or_default();
    let funds_by_bidder_option = funds_by_bidder_token
        .iter()
        .find(|x| x.0 == info.sender.to_string());

    let withdraw_amount = if let Some(funds_by_bidder) = funds_by_bidder_option {
        funds_by_bidder.1
    } else {
        Uint128::zero()
    };
    // }

    if withdraw_amount.is_zero() {
        return Err(StdError::generic_err("Withdrawal amount is zero"));
    }

    Ok(Response::new()
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![coin(withdraw_amount.u128(), config.stable_denom)],
        }))
        .add_attributes(vec![
            attr("action", "withdraw"),
            attr("token_id", token_id.clone()),
            attr("receiver", info.sender.to_string()),
            attr("withdraw_amount", withdraw_amount.to_string()),
        ]))
}

pub fn execute_start_auction(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    token_id: String,
    start_time: u64,
    end_time: u64,
) -> StdResult<Response> {
    if start_time >= end_time {
        return Err(StdError::generic_err("auction time is wrong"));
    }
    if start_time <= env.block.time.seconds() {
        return Err(StdError::generic_err("start time is past"));
    }

    // OWNER_HAS_WITHDRAWN.save(deps.storage, token_id.clone(), &false)?;

    //check after start
    let token_auction_state_option =
        TOKEN_AUCTION_STATE.may_load(deps.storage, token_id.clone())?;
    if token_auction_state_option.is_some() {
        let token_auction_state = token_auction_state_option.unwrap();
        if env.block.time.seconds() <= token_auction_state.end_time {
            return Err(StdError::generic_err("already started"));
        }
    }

    TOKEN_AUCTION_STATE.save(
        deps.storage,
        token_id.clone(),
        &TokenAuctionState {
            start_time,
            end_time,
            high_bidder_addr: Addr::unchecked(""),
            high_bidder_amount: Uint128::zero(),
        },
    )?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "start_auction"),
        attr("start_time", start_time.to_string()),
        attr("end_time", end_time.to_string()),
    ]))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // QueryMsg::GetHighestBid {token_id} =>to_binary(&query_get_highest_bid(deps, token_id)?),
        QueryMsg::AuctionState { token_id } => to_binary(&query_get_auction_state(deps, token_id)?),
    }
}

// fn query_get_highest_bid(deps: Deps, token_id: String) -> StdResult<HighestBidResponse>{
//     let bid_payments_result = FUNDS_BY_BIDDER.load(deps.storage, token_id.clone());
//     if bid_payments_result.is_ok(){
//         let bid_payments = bid_payments_result.unwrap();
//         let max_bid = bid_payments.iter().max_by(|x,y| x.1.cmp(&y.1)).unwrap();
//         return Ok(HighestBidResponse{
//             address: max_bid.0.to_string(),
//             bid: max_bid.1.clone()
//         })
//     }else{
//         return Ok(HighestBidResponse{
//             address: "".to_string(),
//             bid: Uint128::zero(),
//         });
//     };
// }

fn query_get_auction_state(deps: Deps, token_id: String) -> StdResult<AuctionStateResponse> {
    let token_auction_state_option =
        TOKEN_AUCTION_STATE.may_load(deps.storage, token_id.clone())?;
    let auction_state_res = if token_auction_state_option.is_none() {
        AuctionStateResponse {
            start_time: 0,
            end_time: 0,
            high_bidder_addr: "".to_string(),
            high_bidder_amount: Uint128::zero(),
        }
    } else {
        let token_auction_state = token_auction_state_option.unwrap();
        AuctionStateResponse {
            start_time: token_auction_state.start_time,
            end_time: token_auction_state.end_time,
            high_bidder_addr: token_auction_state.high_bidder_addr.to_string(),
            high_bidder_amount: token_auction_state.high_bidder_amount,
        }
    };

    Ok(auction_state_res)
}

fn query_token_info(
    querier: QuerierWrapper,
    token_addr: String,
    token_id: String,
) -> StdResult<OwnerOfResponse> {
    let res: OwnerOfResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: token_addr,
        msg: to_binary(&TokenQueryMsg::OwnerOf { token_id })?,
    }))?;

    Ok(res)
}

#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate};
    use andromeda_protocol::auction::{ExecuteMsg, InstantiateMsg};
    use andromeda_protocol::testing::mock_querier::mock_dependencies_custom;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, coin, BankMsg, CosmosMsg, Response, StdError, Timestamp};

    #[test]
    fn test_auction_instantiate() {
        let owner = "creator";
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {
            token_addr: "token_addr_001".to_string(),
            stable_denom: "uusd".to_string(),
        };
        let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());
    }
    #[test]
    fn test_execute_place_bid() {
        let owner = "creator";
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {
            token_addr: "token_addr_001".to_string(),
            stable_denom: "uusd".to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::PlaceBid {
            token_id: "token_id1".to_string(),
        };
        let info = mock_info(owner, &[coin(100u128, "uusd".to_string())]);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "not started"),
            Err(e) => panic!("Unexpected error: {:?}", e),
            _ => panic!("Must return error"),
        }

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100u64,
            end_time: 200u64,
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0u64);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::PlaceBid {
            token_id: "token_001".to_string(),
        };
        // let info = mock_info(owner, &[coin(100u128, "uusd".to_string())]);

        env.block.time = Timestamp::from_seconds(150u64);

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        assert_eq!(
            res,
            Response::new().add_attributes(vec![
                attr("action", "bid"),
                attr("token_id", "token_001"),
                attr("bider", "creator"),
                attr("amount", "100"),
            ]),
        );
    }

    #[test]
    fn execute_start_auction() {
        let owner = "creator";
        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {
            token_addr: "token_addr_001".to_string(),
            stable_denom: "uusd".to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100u64,
            end_time: 200u64,
        };
        let info = mock_info(owner, &[]);
        env.block.time = Timestamp::from_seconds(0u64);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        assert_eq!(
            res,
            Response::new().add_attributes(vec![
                attr("action", "start_auction"),
                attr("start_time", "100"),
                attr("end_time", "200"),
            ]),
        );
    }

    #[test]
    fn execute_withdraw() {
        let owner = "creator";
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {
            token_addr: "token_addr_001".to_string(),
            stable_denom: "uusd".to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100u64,
            end_time: 200u64,
        };
        let info = mock_info(owner, &[]);
        env.block.time = Timestamp::from_seconds(0u64);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::PlaceBid {
            token_id: "token_001".to_string(),
        };

        env.block.time = Timestamp::from_seconds(150u64);
        let info = mock_info("user1", &[coin(100u128, "uusd".to_string())]);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::PlaceBid {
            token_id: "token_001".to_string(),
        };

        env.block.time = Timestamp::from_seconds(150u64);
        let info = mock_info("user2", &[coin(150u128, "uusd".to_string())]);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        env.block.time = Timestamp::from_seconds(300u64);
        let msg = ExecuteMsg::Withdraw {
            token_id: "token_001".to_string(),
        };
        let info = mock_info("user1", &[]);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        assert_eq!(
            res,
            Response::new()
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "user1".to_string(),
                    amount: vec![coin(100u128, "uusd")]
                }))
                .add_attributes(vec![
                    attr("action", "withdraw"),
                    attr("token_id", "token_001".to_string()),
                    attr("receiver", "user1".to_string()),
                    attr("withdraw_amount", "100".to_string())
                ]),
        );
    }
}

