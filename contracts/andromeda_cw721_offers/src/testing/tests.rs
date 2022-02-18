use crate::{
    contract::{execute, instantiate},
    state::{offers, Offer, CW721_CONTRACT},
};
use andromeda_protocol::{
    cw721::ExecuteMsg as Cw721ExecuteMsg,
    cw721_offers::{ExecuteMsg, InstantiateMsg},
    error::ContractError,
    testing::mock_querier::{
        bank_sub_msg, mock_dependencies_custom, MOCK_CW721_CONTRACT, MOCK_RATES_RECIPIENT,
        MOCK_TOKEN_TRANSFER_AGREEMENT,
    },
};
use cosmwasm_std::{
    coins,
    testing::{mock_env, mock_info},
    to_binary, BankMsg, CosmosMsg, DepsMut, Event, MessageInfo, Response, SubMsg, WasmMsg,
};
use cw721::Expiration;

fn init(deps: DepsMut, info: MessageInfo) -> Result<(), ContractError> {
    let _res = instantiate(
        deps,
        mock_env(),
        info,
        InstantiateMsg {
            andromeda_cw721_contract: MOCK_CW721_CONTRACT.to_owned(),
        },
    )
    .unwrap();
    Ok(())
}

#[test]
fn test_place_offer_accept_offer() {
    let mut deps = mock_dependencies_custom(&[]);
    let token_id = String::from("offer_token");
    let creator = String::from("creator");
    let purchaser = String::from("purchaser");
    let other_purchaser = String::from("other_purchaser");

    let info = mock_info("creator", &[]);
    init(deps.as_mut(), info).unwrap();
    assert_eq!(
        MOCK_CW721_CONTRACT,
        CW721_CONTRACT.load(deps.as_ref().storage).unwrap()
    );

    let msg = ExecuteMsg::PlaceOffer {
        token_id: token_id.clone(),
        expiration: Expiration::Never {},
        offer_amount: 100u128.into(),
    };

    let info = mock_info(&creator, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    assert_eq!(ContractError::TokenOwnerCannotBid {}, res.unwrap_err());

    let info = mock_info(&purchaser, &coins(100u128, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    // Tax not included therefore insufficient funds.
    assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());

    // Add 10uusd for tax.
    let info = mock_info(&purchaser, &coins(100u128 + 10u128, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        Response::new()
            .add_attribute("action", "place_offer")
            .add_attribute("purchaser", &purchaser)
            .add_attribute("offer_amount", "100")
            .add_attribute("token_id", &token_id),
        res
    );

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::OfferAlreadyPlaced {}, res.unwrap_err());

    let msg = ExecuteMsg::PlaceOffer {
        token_id: token_id.clone(),
        expiration: Expiration::Never {},
        offer_amount: 50u128.into(),
    };

    // 5 extra uusd for tax.
    let info = mock_info(&other_purchaser, &coins(50u128 + 5u128, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::OfferLowerThanCurrent {}, res.unwrap_err());

    let msg = ExecuteMsg::PlaceOffer {
        token_id: token_id.clone(),
        expiration: Expiration::Never {},
        offer_amount: 150u128.into(),
    };

    // 15 extra uusd for tax.
    let info = mock_info(&other_purchaser, &coins(150u128 + 15u128, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        Response::new()
            .add_submessage(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: purchaser,
                amount: coins(110u128, "uusd"),
            })))
            .add_attribute("action", "place_offer")
            .add_attribute("purchaser", &other_purchaser)
            .add_attribute("offer_amount", "150")
            .add_attribute("token_id", &token_id),
        res
    );

    let msg = ExecuteMsg::AcceptOffer {
        token_id: token_id.clone(),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    let info = mock_info(&creator, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let msgs: Vec<SubMsg> = vec![
        bank_sub_msg(15, MOCK_RATES_RECIPIENT),
        bank_sub_msg(15, MOCK_RATES_RECIPIENT),
        bank_sub_msg(135, &creator),
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_CW721_CONTRACT.to_owned(),
            funds: vec![],
            msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: other_purchaser,
                token_id: token_id.clone(),
            })
            .unwrap(),
        })),
    ];
    assert_eq!(
        Response::new()
            .add_submessages(msgs)
            .add_event(Event::new("Royalty"))
            .add_event(Event::new("Tax"))
            .add_attribute("action", "accept_offer")
            .add_attribute("token_id", &token_id),
        res
    );

    assert_eq!(
        None,
        offers().may_load(deps.as_ref().storage, &token_id).unwrap()
    );
}

#[test]
fn test_place_offer_expired() {
    let mut deps = mock_dependencies_custom(&[]);
    let token_id = String::from("offer_token");
    let creator = String::from("creator");
    let purchaser = String::from("purchaser");
    let mut env = mock_env();

    let info = mock_info(&creator, &[]);
    init(deps.as_mut(), info).unwrap();

    let msg = ExecuteMsg::PlaceOffer {
        token_id,
        expiration: Expiration::AtHeight(10),
        offer_amount: 100u128.into(),
    };

    env.block.height = 12;
    let info = mock_info(&purchaser, &[]);
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::Expired {}, res.unwrap_err());
}

#[test]
fn test_place_offer_previous_expired() {
    let mut deps = mock_dependencies_custom(&[]);
    let token_id = String::from("offer_token");
    let creator = String::from("creator");
    let purchaser = String::from("purchaser");
    let other_purchaser = String::from("other_purchaser");
    let mut env = mock_env();

    let info = mock_info(&creator, &[]);
    init(deps.as_mut(), info).unwrap();

    let offer = Offer {
        denom: "uusd".to_string(),
        offer_amount: 100u128.into(),
        tax_amount: 10u128.into(),
        expiration: Expiration::AtHeight(10),
        purchaser: purchaser.clone(),
        msgs: vec![],
        events: vec![],
    };

    offers()
        .save(deps.as_mut().storage, &token_id, &offer)
        .unwrap();

    env.block.height = 12;

    let msg = ExecuteMsg::PlaceOffer {
        token_id: token_id.clone(),
        expiration: Expiration::AtHeight(15),
        offer_amount: 50u128.into(),
    };

    let info = mock_info(&other_purchaser, &coins(50 + 5, "uusd"));
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    let msg: SubMsg = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: purchaser,
        amount: coins(110, "uusd"),
    }));
    assert_eq!(
        Response::new()
            .add_submessage(msg)
            .add_attribute("action", "place_offer")
            .add_attribute("purchaser", &other_purchaser)
            .add_attribute("offer_amount", "50")
            .add_attribute("token_id", &token_id),
        res
    );

    assert_eq!(
        Offer {
            denom: "uusd".to_string(),
            offer_amount: 50u128.into(),
            tax_amount: 5u128.into(),
            expiration: Expiration::AtHeight(15),
            purchaser: other_purchaser,
            msgs: vec![
                bank_sub_msg(5, MOCK_RATES_RECIPIENT),
                bank_sub_msg(5, MOCK_RATES_RECIPIENT),
                bank_sub_msg(45, &creator),
            ],
            events: vec![Event::new("Royalty"), Event::new("Tax")],
        },
        offers().load(deps.as_ref().storage, &token_id).unwrap()
    );
}

#[test]
fn test_accept_offer_expired() {
    let mut deps = mock_dependencies_custom(&[]);
    let token_id = String::from("offer_token");
    let creator = String::from("creator");
    let purchaser = String::from("purchaser");
    let mut env = mock_env();

    let info = mock_info(&creator, &[]);
    init(deps.as_mut(), info).unwrap();

    let offer = Offer {
        denom: "uusd".to_string(),
        offer_amount: 50u128.into(),
        tax_amount: 5u128.into(),
        expiration: Expiration::AtHeight(10),
        purchaser,
        msgs: vec![],
        events: vec![],
    };
    offers()
        .save(deps.as_mut().storage, &token_id, &offer)
        .unwrap();

    let msg = ExecuteMsg::AcceptOffer { token_id };

    env.block.height = 12;

    let info = mock_info(&creator, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::Expired {}, res.unwrap_err());
}

#[test]
fn test_accept_offer_existing_transfer_agreement() {
    let mut deps = mock_dependencies_custom(&[]);
    let token_id = String::from(MOCK_TOKEN_TRANSFER_AGREEMENT);
    let creator = String::from("creator");
    let purchaser = String::from("purchaser");

    let info = mock_info(&creator, &[]);
    init(deps.as_mut(), info).unwrap();

    let offer = Offer {
        denom: "uusd".to_string(),
        offer_amount: 100u128.into(),
        tax_amount: 10u128.into(),
        expiration: Expiration::Never {},
        purchaser,
        msgs: vec![],
        events: vec![],
    };
    offers()
        .save(deps.as_mut().storage, &token_id, &offer)
        .unwrap();

    let msg = ExecuteMsg::AcceptOffer { token_id };

    let info = mock_info(&creator, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::TransferAgreementExists {}, res.unwrap_err());
}

#[test]
fn test_cancel_offer() {
    let mut deps = mock_dependencies_custom(&[]);
    let token_id = String::from(MOCK_TOKEN_TRANSFER_AGREEMENT);
    let creator = String::from("creator");
    let purchaser = String::from("purchaser");

    let info = mock_info(&creator, &[]);
    init(deps.as_mut(), info).unwrap();

    let offer = Offer {
        denom: "uusd".to_string(),
        offer_amount: 100u128.into(),
        tax_amount: 10u128.into(),
        expiration: Expiration::Never {},
        purchaser: purchaser.clone(),
        msgs: vec![],
        events: vec![],
    };
    offers()
        .save(deps.as_mut().storage, &token_id, &offer)
        .unwrap();

    let msg = ExecuteMsg::CancelOffer {
        token_id: token_id.clone(),
    };

    let info = mock_info(&creator, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

    let info = mock_info(&purchaser, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        Response::new()
            .add_submessage(bank_sub_msg(100 + 10, &purchaser))
            .add_attribute("action", "cancel_offer")
            .add_attribute("token_id", token_id),
        res
    );
}
