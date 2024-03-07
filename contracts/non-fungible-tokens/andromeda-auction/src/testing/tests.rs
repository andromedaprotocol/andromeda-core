use crate::{
    contract::{execute, instantiate, query},
    state::{auction_infos, TOKEN_AUCTION_STATE},
    testing::mock_querier::{
        mock_dependencies_custom, MOCK_TOKEN_ADDR, MOCK_TOKEN_OWNER, MOCK_UNCLAIMED_TOKEN,
    },
};
use andromeda_non_fungible_tokens::{
    auction::{
        AuctionInfo, AuctionStateResponse, Cw721HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
        TokenAuctionState,
    },
    cw721::ExecuteMsg as Cw721ExecuteMsg,
};
use andromeda_std::{
    ado_base::modules::Module,
    common::{encode_binary, expiration::MILLISECONDS_TO_NANOSECONDS_RATIO},
    error::ContractError,
    testing::mock_querier::MOCK_KERNEL_CONTRACT,
};
use cosmwasm_std::{
    attr, coin, coins, from_json,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, BankMsg, CosmosMsg, Deps, DepsMut, Env, Response, Timestamp, Uint128, WasmMsg,
};
use cw721::Cw721ReceiveMsg;
use cw_utils::Expiration;

// const ADDRESS_LIST: &str = "addresslist";
// const RATES: &str = "rates";

fn init(deps: DepsMut, modules: Option<Vec<Module>>) -> Response {
    let msg = InstantiateMsg {
        owner: None,
        modules,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
    };

    let info = mock_info("owner", &[]);
    instantiate(deps, mock_env(), info, msg).unwrap()
}

fn query_latest_auction_state_helper(deps: Deps, env: Env) -> AuctionStateResponse {
    let query_msg = QueryMsg::LatestAuctionState {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_owned(),
    };
    from_json(query(deps, env, query_msg).unwrap()).unwrap()
}

fn start_auction(deps: DepsMut, whitelist: Option<Vec<Addr>>, min_bid: Option<Uint128>) {
    let hook_msg = Cw721HookMsg::StartAuction {
        start_time: 100000,
        duration: 100000,
        coin_denom: "uusd".to_string(),
        whitelist,
        min_bid,
    };
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: MOCK_TOKEN_OWNER.to_owned(),
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        msg: encode_binary(&hook_msg).unwrap(),
    });
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(0u64);

    let info = mock_info(MOCK_TOKEN_ADDR, &[]);
    let _res = execute(deps, env, info, msg).unwrap();
}

fn assert_auction_created(deps: Deps, whitelist: Option<Vec<Addr>>, min_bid: Option<Uint128>) {
    assert_eq!(
        TokenAuctionState {
            start_time: Expiration::AtTime(Timestamp::from_seconds(100)),
            end_time: Expiration::AtTime(Timestamp::from_seconds(200)),
            high_bidder_addr: Addr::unchecked(""),
            high_bidder_amount: Uint128::zero(),
            coin_denom: "uusd".to_string(),
            auction_id: 1u128.into(),
            whitelist,
            owner: MOCK_TOKEN_OWNER.to_string(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_owned(),
            is_cancelled: false,
            min_bid,
        },
        TOKEN_AUCTION_STATE.load(deps.storage, 1u128).unwrap()
    );

    assert_eq!(
        AuctionInfo {
            auction_ids: vec![Uint128::from(1u128)],
            token_address: MOCK_TOKEN_ADDR.to_owned(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        },
        auction_infos()
            .load(
                deps.storage,
                &(MOCK_UNCLAIMED_TOKEN.to_owned() + MOCK_TOKEN_ADDR)
            )
            .unwrap()
    );
}

#[test]
fn test_auction_instantiate() {
    let mut deps = mock_dependencies();
    let res = init(deps.as_mut(), None);
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_execute_place_bid_non_existing_auction() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();
    let _res = init(deps.as_mut(), None);

    let msg = ExecuteMsg::PlaceBid {
        token_id: MOCK_UNCLAIMED_TOKEN.to_string(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };
    let info = mock_info("bidder", &coins(100, "uusd"));
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::AuctionDoesNotExist {}, res.unwrap_err());
}

#[test]
fn execute_place_bid_auction_not_started() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);
    assert_auction_created(deps.as_ref(), None, None);

    let msg = ExecuteMsg::PlaceBid {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    env.block.time = Timestamp::from_seconds(50u64);

    let info = mock_info("sender", &coins(100, "uusd".to_string()));
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::AuctionNotStarted {}, res.unwrap_err());
}

#[test]
fn execute_place_bid_auction_ended() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);
    assert_auction_created(deps.as_ref(), None, None);

    let msg = ExecuteMsg::PlaceBid {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    env.block.time = Timestamp::from_seconds(300);

    let info = mock_info("sender", &coins(100, "uusd".to_string()));
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::AuctionEnded {}, res.unwrap_err());
}

#[test]
fn execute_place_bid_token_owner_cannot_bid() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);
    assert_auction_created(deps.as_ref(), None, None);

    let msg = ExecuteMsg::PlaceBid {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    env.block.time = Timestamp::from_seconds(150);

    let info = mock_info(MOCK_TOKEN_OWNER, &coins(100, "uusd".to_string()));
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::TokenOwnerCannotBid {}, res.unwrap_err());
}

#[test]
fn execute_place_bid_whitelist() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), Some(vec![Addr::unchecked("sender")]), None);
    assert_auction_created(deps.as_ref(), Some(vec![Addr::unchecked("sender")]), None);

    let msg = ExecuteMsg::PlaceBid {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    env.block.time = Timestamp::from_seconds(150);
    let info = mock_info("not_sender", &coins(100, "uusd".to_string()));
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    let info = mock_info("sender", &coins(100, "uusd".to_string()));
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();
}

#[test]
fn execute_place_bid_highest_bidder_cannot_outbid() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);
    assert_auction_created(deps.as_ref(), None, None);

    let msg = ExecuteMsg::PlaceBid {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    env.block.time = Timestamp::from_seconds(150);
    let info = mock_info("sender", &coins(100, "uusd".to_string()));
    let _res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();

    env.block.time = Timestamp::from_seconds(160);
    let info = mock_info("sender", &coins(200, "uusd".to_string()));
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(
        ContractError::HighestBidderCannotOutBid {},
        res.unwrap_err()
    );
}

#[test]
fn execute_place_bid_bid_smaller_than_highest_bid() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);
    assert_auction_created(deps.as_ref(), None, None);

    let msg = ExecuteMsg::PlaceBid {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    env.block.time = Timestamp::from_seconds(150);
    let info = mock_info("sender", &coins(100, "uusd".to_string()));
    let _res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();

    env.block.time = Timestamp::from_seconds(160);
    let info = mock_info("other", &coins(50, "uusd".to_string()));
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::BidSmallerThanHighestBid {}, res.unwrap_err());
}

#[test]
fn execute_place_bid_invalid_coins_sent() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);
    assert_auction_created(deps.as_ref(), None, None);

    env.block.time = Timestamp::from_seconds(150);

    let error = ContractError::InvalidFunds {
        msg: "Auctions ensure! exactly one coin to be sent.".to_string(),
    };
    let msg = ExecuteMsg::PlaceBid {
        token_id: MOCK_UNCLAIMED_TOKEN.to_string(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    // No coins sent
    let info = mock_info("sender", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_eq!(error, res.unwrap_err());

    // Multiple coins sent
    let info = mock_info("sender", &[coin(100, "uusd"), coin(100, "uluna")]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_eq!(error, res.unwrap_err());

    let error = ContractError::InvalidFunds {
        msg: "No uusd assets are provided to auction".to_string(),
    };

    // Invalid denom sent
    let info = mock_info("sender", &[coin(100, "uluna")]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_eq!(error, res.unwrap_err());

    // Correct denom but empty
    let info = mock_info("sender", &[coin(0, "uusd")]);
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(error, res.unwrap_err());
}

#[test]
fn execute_place_bid_multiple_bids() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);
    assert_auction_created(deps.as_ref(), None, None);

    let msg = ExecuteMsg::PlaceBid {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    env.block.time = Timestamp::from_seconds(150);

    let info = mock_info("sender", &coins(100, "uusd".to_string()));
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        Response::new().add_attributes(vec![
            attr("action", "bid"),
            attr("token_id", MOCK_UNCLAIMED_TOKEN),
            attr("bider", info.sender),
            attr("amount", "100"),
        ]),
        res
    );
    let mut expected_response = AuctionStateResponse {
        start_time: Expiration::AtTime(Timestamp::from_seconds(100)),
        end_time: Expiration::AtTime(Timestamp::from_seconds(200)),
        high_bidder_addr: "sender".to_string(),
        high_bidder_amount: Uint128::from(100u128),
        auction_id: Uint128::from(1u128),
        coin_denom: "uusd".to_string(),
        whitelist: None,
        is_cancelled: false,
        min_bid: None,
        owner: "owner".to_string(),
    };

    let res = query_latest_auction_state_helper(deps.as_ref(), env.clone());
    assert_eq!(expected_response, res);

    env.block.time = Timestamp::from_seconds(160);
    let info = mock_info("other", &coins(200, "uusd".to_string()));
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        Response::new()
            .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: "sender".to_string(),
                amount: coins(100, "uusd")
            }))
            .add_attributes(vec![
                attr("action", "bid"),
                attr("token_id", MOCK_UNCLAIMED_TOKEN),
                attr("bider", info.sender),
                attr("amount", "200"),
            ]),
        res
    );

    expected_response.high_bidder_addr = "other".to_string();
    expected_response.high_bidder_amount = Uint128::from(200u128);
    let res = query_latest_auction_state_helper(deps.as_ref(), env.clone());
    assert_eq!(expected_response, res);

    env.block.time = Timestamp::from_seconds(170);
    let info = mock_info("sender", &coins(250, "uusd".to_string()));
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: "other".to_string(),
                amount: coins(200, "uusd")
            }))
            .add_attributes(vec![
                attr("action", "bid"),
                attr("token_id", MOCK_UNCLAIMED_TOKEN),
                attr("bider", info.sender),
                attr("amount", "250"),
            ]),
        res
    );

    expected_response.high_bidder_addr = "sender".to_string();
    expected_response.high_bidder_amount = Uint128::from(250u128);
    let res = query_latest_auction_state_helper(deps.as_ref(), env);
    assert_eq!(expected_response, res);
}

#[test]
fn execute_place_bid_auction_cancelled() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);
    assert_auction_created(deps.as_ref(), None, None);

    let msg = ExecuteMsg::CancelAuction {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    env.block.time = Timestamp::from_seconds(150);
    let info = mock_info(MOCK_TOKEN_OWNER, &[]);
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let msg = ExecuteMsg::PlaceBid {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info("sender", &coins(100, "uusd".to_string()));
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::AuctionCancelled {}, res.unwrap_err());
}

#[test]
fn test_execute_start_auction() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut(), None);

    let hook_msg = Cw721HookMsg::StartAuction {
        start_time: 100000,
        duration: 100000,
        coin_denom: "uusd".to_string(),
        whitelist: None,
        min_bid: None,
    };
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: MOCK_TOKEN_OWNER.to_owned(),
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        msg: encode_binary(&hook_msg).unwrap(),
    });
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(0u64);

    let info = mock_info(MOCK_TOKEN_ADDR, &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        res,
        Response::new().add_attributes(vec![
            attr("action", "start_auction"),
            attr("start_time", "expiration time: 100.000000000"),
            attr("end_time", "expiration time: 200.000000000"),
            attr("coin_denom", "uusd"),
            attr("auction_id", "1"),
            attr("whitelist", "None"),
        ]),
    );
    assert_auction_created(deps.as_ref(), None, None);
}

// #[test]
// fn execute_start_auction_with_block_height() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();
//     let info = mock_info("owner", &[]);
//     let msg = InstantiateMsg { modules: None, kernel_address: None };
//     let _res = instantiate(deps.as_mut(), env, &info, msg).unwrap();

//     let hook_msg = Cw721HookMsg::StartAuction {
//         start_time: Expiration::AtHeight(100),
//         end_time: Expiration::AtHeight(200),
//         coin_denom: "uusd".to_string(),
//         whitelist: None,
//         min_bid: None,
//     };
//     let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
//         sender: MOCK_TOKEN_OWNER.to_owned(),
//         token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
//         msg: encode_binary(&hook_msg).unwrap(),
//     });
//     let mut env = mock_env();
//     env.block.height = 0;

//     let info = mock_info(MOCK_TOKEN_ADDR, &[]);
//     let res = execute(deps.as_mut(), env, &info, msg).unwrap();

//     assert_eq!(
//         res,
//         Response::new().add_attributes(vec![
//             attr("action", "start_auction"),
//             attr("start_time", "expiration height: 100"),
//             attr("end_time", "expiration height: 200"),
//             attr("coin_denom", "uusd"),
//             attr("auction_id", "1"),
//             attr("whitelist", "None"),
//         ]),
//     );
// }

// #[test]
// fn execute_start_auction_with_mismatched_expirations() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();
//     let info = mock_info("owner", &[]);
//     let msg = InstantiateMsg { modules: None, kernel_address: None };
//     let _res = instantiate(deps.as_mut(), env, &info, msg).unwrap();

//     let hook_msg = Cw721HookMsg::StartAuction {
//         start_time: Expiration::AtHeight(100),
//         end_time: Expiration::AtTime(Timestamp::from_seconds(200)),
//         coin_denom: "uusd".to_string(),
//         whitelist: None,
//         min_bid: None,
//     };
//     let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
//         sender: MOCK_TOKEN_OWNER.to_owned(),
//         token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
//         msg: encode_binary(&hook_msg).unwrap(),
//     });
//     let mut env = mock_env();
//     env.block.height = 0;

//     let info = mock_info(MOCK_TOKEN_ADDR, &[]);
//     let res = execute(deps.as_mut(), env, info, msg);

//     assert_eq!(
//         ContractError::ExpirationsMustBeOfSameType {},
//         res.unwrap_err()
//     );
// }

#[test]
fn execute_start_auction_start_time_in_past() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut(), None);

    let hook_msg = Cw721HookMsg::StartAuction {
        start_time: 100000,
        duration: 100000,
        coin_denom: "uusd".to_string(),
        whitelist: None,
        min_bid: None,
    };
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: MOCK_TOKEN_OWNER.to_owned(),
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        msg: encode_binary(&hook_msg).unwrap(),
    });
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(150);

    let info = mock_info(MOCK_TOKEN_ADDR, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg);

    assert_eq!(
        ContractError::StartTimeInThePast {
            current_time: env.block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO,
            current_block: env.block.height,
        },
        res.unwrap_err()
    );
}

#[test]
fn execute_start_auction_zero_start_time() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut(), None);

    let hook_msg = Cw721HookMsg::StartAuction {
        start_time: 0,
        duration: 1,
        coin_denom: "uusd".to_string(),
        whitelist: None,
        min_bid: None,
    };
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: MOCK_TOKEN_OWNER.to_owned(),
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        msg: encode_binary(&hook_msg).unwrap(),
    });
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(0);

    let info = mock_info(MOCK_TOKEN_ADDR, &[]);
    let res = execute(deps.as_mut(), env, info, msg);

    assert_eq!(ContractError::InvalidExpiration {}, res.unwrap_err());
}

#[test]
fn execute_start_auction_zero_duration() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut(), None);

    let hook_msg = Cw721HookMsg::StartAuction {
        start_time: 100,
        duration: 0,
        coin_denom: "uusd".to_string(),
        whitelist: None,
        min_bid: None,
    };
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: MOCK_TOKEN_OWNER.to_owned(),
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        msg: encode_binary(&hook_msg).unwrap(),
    });
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(0);

    let info = mock_info(MOCK_TOKEN_ADDR, &[]);
    let res = execute(deps.as_mut(), env, info, msg);

    assert_eq!(ContractError::InvalidExpiration {}, res.unwrap_err());
}

// #[test]
// fn execute_start_auction_end_time_never() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();
//     let info = mock_info("owner", &[]);
//     let msg = InstantiateMsg { modules: None, kernel_address: None };
//     let _res = instantiate(deps.as_mut(), env, &info, msg).unwrap();

//     let hook_msg = Cw721HookMsg::StartAuction {
//         end_time: Expiration::Never {},
//         start_time: Expiration::AtTime(Timestamp::from_seconds(200)),
//         coin_denom: "uusd".to_string(),
//         whitelist: None,
//         min_bid: None,
//     };
//     let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
//         sender: MOCK_TOKEN_OWNER.to_owned(),
//         token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
//         msg: encode_binary(&hook_msg).unwrap(),
//     });
//     let mut env = mock_env();
//     env.block.time = Timestamp::from_seconds(0);

//     let info = mock_info(MOCK_TOKEN_ADDR, &[]);
//     let res = execute(deps.as_mut(), env, info, msg);

//     assert_eq!(ContractError::ExpirationMustNotBeNever {}, res.unwrap_err());
// }

#[test]
fn execute_update_auction_zero_start() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);

    let msg = ExecuteMsg::UpdateAuction {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
        start_time: 0,
        duration: 1,
        coin_denom: "uusd".to_string(),
        whitelist: None,
        min_bid: None,
    };
    let mut env = mock_env();
    env.block.height = 0;
    env.block.time = Timestamp::from_seconds(0);

    let info = mock_info(MOCK_TOKEN_OWNER, &[]);
    let res = execute(deps.as_mut(), env, info, msg);

    assert_eq!(ContractError::InvalidExpiration {}, res.unwrap_err());
}

#[test]
fn execute_update_auction_start_time_in_past() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);

    let msg = ExecuteMsg::UpdateAuction {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
        start_time: 1,
        duration: 1,
        coin_denom: "uusd".to_string(),
        whitelist: None,
        min_bid: None,
    };
    let mut env = mock_env();
    env.block.height = 150;
    env.block.time = Timestamp::from_seconds(10);

    let info = mock_info(MOCK_TOKEN_OWNER, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg);

    assert_eq!(
        ContractError::StartTimeInThePast {
            current_time: env.block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO,
            current_block: env.block.height,
        },
        res.unwrap_err()
    );
}

#[test]
fn execute_update_auction_zero_duration() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);

    let msg = ExecuteMsg::UpdateAuction {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
        start_time: 100000,
        duration: 0,
        coin_denom: "uusd".to_string(),
        whitelist: None,
        min_bid: None,
    };
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(0);

    let info = mock_info(MOCK_TOKEN_OWNER, &[]);
    let res = execute(deps.as_mut(), env, info, msg);

    assert_eq!(ContractError::InvalidExpiration {}, res.unwrap_err());
}

#[test]
fn execute_update_auction_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);

    let msg = ExecuteMsg::UpdateAuction {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
        start_time: 100000,
        duration: 100,
        coin_denom: "uluna".to_string(),
        whitelist: Some(vec![Addr::unchecked("user")]),
        min_bid: None,
    };
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(150);
    env.block.height = 0;

    let info = mock_info("not_owner", &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn execute_update_auction_auction_started() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);

    let msg = ExecuteMsg::UpdateAuction {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
        start_time: 100000,
        duration: 100,
        coin_denom: "uluna".to_string(),
        whitelist: Some(vec![Addr::unchecked("user")]),
        min_bid: None,
    };
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(150);
    env.block.height = 0;

    let info = mock_info(MOCK_TOKEN_OWNER, &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::AuctionAlreadyStarted {}, res.unwrap_err());
}

#[test]
fn execute_update_auction() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);

    let msg = ExecuteMsg::UpdateAuction {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
        start_time: 100000,
        duration: 100000,
        coin_denom: "uluna".to_string(),
        whitelist: Some(vec![Addr::unchecked("user")]),
        min_bid: None,
    };
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(0);
    env.block.height = 0;

    let info = mock_info(MOCK_TOKEN_OWNER, &[]);
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        TokenAuctionState {
            start_time: Expiration::AtTime(Timestamp::from_seconds(100)),
            end_time: Expiration::AtTime(Timestamp::from_seconds(200)),
            high_bidder_addr: Addr::unchecked(""),
            high_bidder_amount: Uint128::zero(),
            coin_denom: "uluna".to_string(),
            auction_id: 1u128.into(),
            whitelist: Some(vec![Addr::unchecked("user")]),
            owner: MOCK_TOKEN_OWNER.to_string(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_owned(),
            is_cancelled: false,
            min_bid: None,
        },
        TOKEN_AUCTION_STATE
            .load(deps.as_ref().storage, 1u128)
            .unwrap()
    );
}

#[test]
fn execute_start_auction_after_previous_finished() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut(), None);

    // There was a previous auction.
    start_auction(deps.as_mut(), None, None);

    let hook_msg = Cw721HookMsg::StartAuction {
        start_time: 300000,
        duration: 100000,
        coin_denom: "uusd".to_string(),
        whitelist: None,
        min_bid: None,
    };
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: MOCK_TOKEN_OWNER.to_owned(),
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        msg: encode_binary(&hook_msg).unwrap(),
    });
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(250);

    let info = mock_info(MOCK_TOKEN_ADDR, &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        Response::new().add_attributes(vec![
            attr("action", "start_auction"),
            attr("start_time", "expiration time: 300.000000000"),
            attr("end_time", "expiration time: 400.000000000"),
            attr("coin_denom", "uusd"),
            attr("auction_id", "2"),
            attr("whitelist", "None"),
        ]),
        res
    );
}

#[test]
fn execute_claim_no_bids() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);

    env.block.time = Timestamp::from_seconds(250);

    let msg = ExecuteMsg::Claim {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info("any_user", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_TOKEN_ADDR.to_owned(),
                msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: MOCK_TOKEN_OWNER.to_owned(),
                    token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
                })
                .unwrap(),
                funds: vec![],
            }))
            .add_attribute("action", "claim")
            .add_attribute("token_id", MOCK_UNCLAIMED_TOKEN)
            .add_attribute("token_contract", MOCK_TOKEN_ADDR)
            .add_attribute("recipient", MOCK_TOKEN_OWNER)
            .add_attribute("winning_bid_amount", Uint128::zero())
            .add_attribute("auction_id", "1"),
        res
    );
}

#[test]
fn execute_claim() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);

    let msg = ExecuteMsg::PlaceBid {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    env.block.time = Timestamp::from_seconds(150);

    let info = mock_info("sender", &coins(100, "uusd".to_string()));
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    env.block.time = Timestamp::from_seconds(250);

    let msg = ExecuteMsg::Claim {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info("any_user", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    let transfer_nft_msg = Cw721ExecuteMsg::TransferNft {
        recipient: "sender".to_string(),
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
    };
    assert_eq!(
        Response::new()
            .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: MOCK_TOKEN_OWNER.to_owned(),
                amount: coins(100, "uusd"),
            }))
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_TOKEN_ADDR.to_string(),
                msg: encode_binary(&transfer_nft_msg).unwrap(),
                funds: vec![],
            }))
            .add_attribute("action", "claim")
            .add_attribute("token_id", MOCK_UNCLAIMED_TOKEN)
            .add_attribute("token_contract", MOCK_TOKEN_ADDR)
            .add_attribute("recipient", "sender")
            .add_attribute("winning_bid_amount", Uint128::from(100u128))
            .add_attribute("auction_id", "1"),
        res
    );
}

#[test]
fn execute_claim_auction_not_ended() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);

    let msg = ExecuteMsg::PlaceBid {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    env.block.time = Timestamp::from_seconds(150);

    let info = mock_info("sender", &coins(100, "uusd".to_string()));
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let msg = ExecuteMsg::Claim {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info("any_user", &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::AuctionNotEnded {}, res.unwrap_err());
}

#[test]
fn execute_claim_auction_already_claimed() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut(), None);

    let hook_msg = Cw721HookMsg::StartAuction {
        start_time: 100000,
        duration: 100000,
        coin_denom: "uusd".to_string(),
        whitelist: None,
        min_bid: None,
    };
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: MOCK_TOKEN_OWNER.to_owned(),
        token_id: "claimed_token".to_string(),
        msg: encode_binary(&hook_msg).unwrap(),
    });
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(0u64);

    let info = mock_info(MOCK_TOKEN_ADDR, &[]);
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Auction is over.
    env.block.time = Timestamp::from_seconds(300);

    let msg = ExecuteMsg::Claim {
        token_id: "claimed_token".to_string(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info("any_user", &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::AuctionAlreadyClaimed {}, res.unwrap_err());
}

#[test]
fn execute_cancel_no_bids() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);

    let msg = ExecuteMsg::CancelAuction {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    // Auction start and end are 100 and 200.
    env.block.time = Timestamp::from_seconds(150);

    let info = mock_info(MOCK_TOKEN_OWNER, &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_TOKEN_ADDR.to_owned(),
            msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: MOCK_TOKEN_OWNER.to_owned(),
                token_id: MOCK_UNCLAIMED_TOKEN.to_owned()
            })
            .unwrap(),
            funds: vec![],
        })),
        res
    );

    assert!(
        TOKEN_AUCTION_STATE
            .load(deps.as_ref().storage, 1u128)
            .unwrap()
            .is_cancelled
    );
}

#[test]
fn execute_cancel_with_bids() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);

    let msg = ExecuteMsg::PlaceBid {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };
    // Auction start and end are 100 and 200.
    env.block.time = Timestamp::from_seconds(150);

    let info = mock_info("bidder", &coins(100, "uusd"));
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let msg = ExecuteMsg::CancelAuction {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info(MOCK_TOKEN_OWNER, &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_TOKEN_ADDR.to_owned(),
                msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: MOCK_TOKEN_OWNER.to_owned(),
                    token_id: MOCK_UNCLAIMED_TOKEN.to_owned()
                })
                .unwrap(),
                funds: vec![],
            }))
            .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: "bidder".to_string(),
                amount: coins(100, "uusd")
            })),
        res
    );

    assert!(
        TOKEN_AUCTION_STATE
            .load(deps.as_ref().storage, 1u128)
            .unwrap()
            .is_cancelled
    );
}

#[test]
fn execute_cancel_not_token_owner() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);

    let msg = ExecuteMsg::CancelAuction {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    // Auction start and end are 100 and 200.
    env.block.time = Timestamp::from_seconds(150);

    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn execute_cancel_auction_ended() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, None);

    let msg = ExecuteMsg::CancelAuction {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    // Auction start and end are 100 and 200.
    env.block.time = Timestamp::from_seconds(300);

    let info = mock_info(MOCK_TOKEN_OWNER, &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::AuctionEnded {}, res.unwrap_err());
}

#[test]
fn execute_bid_below_min_price() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_auction(deps.as_mut(), None, Some(Uint128::from(100u128)));

    let msg = ExecuteMsg::PlaceBid {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };
    // Auction start and end are 100 and 200.
    env.block.time = Timestamp::from_seconds(150);

    let info = mock_info("bidder", &coins(10, "uusd"));
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();

    assert_eq!(
        res,
        ContractError::InvalidFunds {
            msg: "Must provide at least 100 uusd to bid".to_string()
        }
    );

    let info = mock_info("bidder", &coins(100, "uusd"));
    //Will error if invalid
    execute(deps.as_mut(), env, info, msg).unwrap();
}
