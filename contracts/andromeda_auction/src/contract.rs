use crate::state::{
    Bid, Config, TokenAuctionState, AUCTION_IDS, BIDS, CONFIG, NEXT_AUCTION_ID, TOKEN_AUCTION_STATE,
};
use andromeda_protocol::{
    auction::{AuctionStateResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    error::ContractError,
    token::QueryMsg as TokenQueryMsg,
};
use cosmwasm_std::{
    attr, coin, entry_point, to_binary, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, QuerierWrapper, QueryRequest, Response, StdError, StdResult, Uint128, WasmQuery,
};
use cw721::OwnerOfResponse;
use cw_storage_plus::U128Key;
use std::ops::AddAssign;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config: Config = Config {
        token_addr: msg.token_addr,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("method", "instantiate"))
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
            coin_denom,
        } => execute_start_auction(deps, env, info, token_id, start_time, end_time, coin_denom),
    }
}

pub fn execute_place_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.funds.len() > 1usize {
        //TODO: use proper error for this.
        return Err(ContractError::Std(StdError::generic_err(
            "More than one coin is sent; only one asset is supported",
        )));
    }
    let mut token_auction_state = get_existing_token_auction_state(&deps, &token_id)?;
    let coin_denom = token_auction_state.coin_denom.clone();
    let payment = info
        .funds
        .iter()
        .find(|x| x.denom == coin_denom && x.amount > Uint128::zero())
        .ok_or_else(|| {
            StdError::generic_err(format!("No {} assets are provided to auction", coin_denom))
        })?;

    if env.block.time.seconds() < token_auction_state.start_time {
        return Err(ContractError::AuctionNotStarted {});
    }
    if env.block.time.seconds() > token_auction_state.end_time {
        return Err(ContractError::AuctionEnded {});
    }

    let token_owner = query_owner_of(deps.querier, config.token_addr, token_id.clone())?;
    if token_owner.owner == info.sender {
        return Err(ContractError::TokenOwnerCannotBid {});
    }

    if token_auction_state.high_bidder_amount >= payment.amount {
        return Err(ContractError::BidAmountSmallerThanHighestBid {});
    } else if token_auction_state.high_bidder_addr == info.sender.to_string() {
        return Err(ContractError::HighestBidderCannotOutBid {});
    }

    let bank_msg = BankMsg::Send {
        to_address: token_auction_state.high_bidder_addr.to_string(),
        amount: vec![coin(
            token_auction_state.high_bidder_amount.u128(),
            token_auction_state.coin_denom,
        )],
    };
    token_auction_state.high_bidder_addr = info.sender;
    token_auction_state.high_bidder_amount = payment.amount;

    TOKEN_AUCTION_STATE.save(
        deps.storage,
        U128Key::new(token_auction_state.auction_id.u128()),
        &token_auction_state,
    )?;
    let mut bids_for_auction = BIDS
        .load(
            deps.storage,
            U128Key::new(token_auction_state.auction_id.u128()),
        )
        .unwrap_or_default();

    bids_for_auction.push(Bid {
        bidder: info.sender.to_string(),
        amount: payment.amount,
    });

    BIDS.save(
        deps.storage,
        U128Key::new(token_auction_state.auction_id.u128()),
        &bids_for_auction,
    )?;
    Ok(Response::new()
        .add_message(CosmosMsg::Bank(bank_msg))
        .add_attributes(vec![
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
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let token_auction_state = get_existing_token_auction_state(&deps, &token_id)?;

    if env.block.time.seconds() < token_auction_state.start_time {
        return Err(ContractError::AuctionNotStarted {});
    }

    if info.sender == token_auction_state.high_bidder_addr {
        return Err(ContractError::CannotWithdrawHighestBid {});
    }

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

    if withdraw_amount.is_zero() {
        return Err(ContractError::WithdrawalIsEmpty {});
    }

    Ok(Response::new()
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![coin(withdraw_amount.u128(), token_auction_state.coin_denom)],
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
    info: MessageInfo,
    token_id: String,
    start_time: u64,
    end_time: u64,
    coin_denom: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let owner_of_response = query_owner_of(deps.querier, config.token_addr, token_id)?;
    if owner_of_response.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    if start_time >= end_time {
        return Err(ContractError::StartTimeAfterEndTime {});
    }
    if start_time <= env.block.time.seconds() {
        return Err(ContractError::StartTimeInThePast {});
    }

    let mut next_auction_id = NEXT_AUCTION_ID.load(deps.storage)?;

    let latest_auction_id: Uint128 = match AUCTION_IDS.may_load(deps.storage, &token_id)? {
        None => next_auction_id,
        Some(auction_ids) => *auction_ids.last().unwrap(),
    };

    let token_auction_state_option =
        TOKEN_AUCTION_STATE.may_load(deps.storage, U128Key::new(latest_auction_id.u128()))?;
    if let Some(token_auction_state) = token_auction_state_option {
        if env.block.time.seconds() >= token_auction_state.start_time {
            return Err(ContractError::AuctionAlreadyStarted {});
        }
    } else {
        // In this case there is no existing auction with this ID, so next_auction_id is used.
        next_auction_id.add_assign(Uint128::from(1u128));
        NEXT_AUCTION_ID.save(deps.storage, &next_auction_id)?;
    }

    TOKEN_AUCTION_STATE.save(
        deps.storage,
        U128Key::new(latest_auction_id.u128()),
        &TokenAuctionState {
            start_time,
            end_time,
            high_bidder_addr: Addr::unchecked(""),
            high_bidder_amount: Uint128::zero(),
            coin_denom,
            auction_id: latest_auction_id,
        },
    )?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "start_auction"),
        attr("start_time", start_time.to_string()),
        attr("end_time", end_time.to_string()),
    ]))
}

fn get_existing_token_auction_state(
    deps: &DepsMut,
    token_id: &str,
) -> Result<TokenAuctionState, ContractError> {
    let latest_auction_id: Uint128 = match AUCTION_IDS.may_load(deps.storage, &token_id)? {
        None => return Err(ContractError::AuctionDoesNotExist {}),
        Some(auction_ids) => *auction_ids.last().unwrap(),
    };
    let token_auction_state =
        TOKEN_AUCTION_STATE.may_load(deps.storage, U128Key::new(latest_auction_id.u128()))?;
    if let Some(token_auction_state) = token_auction_state {
        return Ok(token_auction_state);
    }
    return Err(ContractError::AuctionDoesNotExist {});
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
    let token_auction_state_option = TOKEN_AUCTION_STATE.may_load(deps.storage, &token_id)?;
    Ok(
        if let Some(token_auction_state) = token_auction_state_option {
            AuctionStateResponse {
                start_time: token_auction_state.start_time,
                end_time: token_auction_state.end_time,
                high_bidder_addr: token_auction_state.high_bidder_addr.to_string(),
                high_bidder_amount: token_auction_state.high_bidder_amount,
            }
        } else {
            AuctionStateResponse {
                start_time: 0,
                end_time: 0,
                high_bidder_addr: "".to_string(),
                high_bidder_amount: Uint128::zero(),
            }
        },
    )
}

fn query_owner_of(
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

/*#[cfg(test)]
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
}*/
