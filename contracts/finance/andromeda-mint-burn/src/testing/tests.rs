use super::mock::{
    cancel_order, create_order, proper_initialization, query_authorized_addresses,
    query_order_info, query_orders_by_status, query_user_deposited_orders, receive_cw20,
    receive_nft, MOCK_CW20_CONTRACT_2, MOCK_NFT_CONTRACT, MOCK_NFT_CONTRACT_TO_MINT,
};
use crate::state::ORDERS;
use andromeda_finance::mint_burn::{
    Cw20HookMsg, Cw721HookMsg, GetOrderInfoResponse, GetOrdersByStatusResponse, OrderStatus,
    Resource, ResourceRequirement,
};
use andromeda_std::{
    amp::AndrAddr,
    common::{
        denom::{AuthorizedAddressesResponse, PermissionAction},
        encode_binary,
    },
    error::ContractError,
    testing::mock_querier::MOCK_CW20_CONTRACT,
};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use cw721::Cw721ReceiveMsg;
use std::collections::HashMap;

#[test]
fn test_instantiation() {
    let (deps, _) = proper_initialization(
        Some(vec![AndrAddr::from_string(MOCK_NFT_CONTRACT)]),
        Some(vec![
            AndrAddr::from_string(MOCK_CW20_CONTRACT),
            AndrAddr::from_string(MOCK_CW20_CONTRACT_2),
        ]),
    );
    let authorized_nft_addresses: AuthorizedAddressesResponse =
        query_authorized_addresses(deps.as_ref(), PermissionAction::SendNft, None, None, None)
            .unwrap();
    assert_eq!(authorized_nft_addresses.addresses, vec![MOCK_NFT_CONTRACT]);

    let authorized_cw20_addresses: AuthorizedAddressesResponse =
        query_authorized_addresses(deps.as_ref(), PermissionAction::SendCw20, None, None, None)
            .unwrap();
    assert_eq!(
        authorized_cw20_addresses.addresses,
        vec![MOCK_CW20_CONTRACT, MOCK_CW20_CONTRACT_2]
    );
}

#[test]
fn test_create_order() {
    let (mut deps, info) = proper_initialization(
        Some(vec![AndrAddr::from_string(MOCK_NFT_CONTRACT)]),
        Some(vec![
            AndrAddr::from_string(MOCK_CW20_CONTRACT),
            AndrAddr::from_string(MOCK_CW20_CONTRACT_2),
        ]),
    );

    let requirements = vec![
        ResourceRequirement {
            resource: Resource::Cw20Token {
                cw20_addr: AndrAddr::from_string(MOCK_CW20_CONTRACT.to_string()),
            },
            amount: Uint128::new(10000),
            deposits: HashMap::new(),
        },
        ResourceRequirement {
            resource: Resource::Cw20Token {
                cw20_addr: AndrAddr::from_string(MOCK_CW20_CONTRACT_2.to_string()),
            },
            amount: Uint128::new(20000),
            deposits: HashMap::new(),
        },
    ];

    let output = Resource::Nft {
        cw721_addr: AndrAddr::from_string(MOCK_NFT_CONTRACT_TO_MINT.to_string()),
        token_id: "test_nft_to_mint_1".to_string(),
    };

    let sender = info.sender.as_str();

    let res = create_order(deps.as_mut(), requirements.clone(), output.clone(), sender).unwrap();

    // Verify response
    assert_eq!(
        res.attributes,
        vec![
            ("method", "create_order"),
            ("order_id", "1"),
            ("sender", sender),
        ]
    );

    // Verify order state
    let order = ORDERS.load(&deps.storage, 1).unwrap();
    assert_eq!(order.requirements, requirements);
    assert_eq!(order.output, output);
    assert_eq!(order.order_status, OrderStatus::NotCompleted);
    assert_eq!(order.output_recipient, None);
}

#[test]
fn test_fill_order_with_cw20() {
    let (mut deps, info) = proper_initialization(
        Some(vec![AndrAddr::from_string(MOCK_NFT_CONTRACT)]),
        Some(vec![
            AndrAddr::from_string(MOCK_CW20_CONTRACT),
            AndrAddr::from_string(MOCK_CW20_CONTRACT_2),
        ]),
    );

    let requirements = vec![
        ResourceRequirement {
            resource: Resource::Cw20Token {
                cw20_addr: AndrAddr::from_string(MOCK_CW20_CONTRACT.to_string()),
            },
            amount: Uint128::new(10000),
            deposits: HashMap::new(),
        },
        ResourceRequirement {
            resource: Resource::Cw20Token {
                cw20_addr: AndrAddr::from_string(MOCK_CW20_CONTRACT_2.to_string()),
            },
            amount: Uint128::new(20000),
            deposits: HashMap::new(),
        },
    ];

    let output = Resource::Nft {
        cw721_addr: AndrAddr::from_string(MOCK_NFT_CONTRACT_TO_MINT.to_string()),
        token_id: "test_nft_to_mint_1".to_string(),
    };

    let sender = info.sender.as_str();

    create_order(deps.as_mut(), requirements.clone(), output, sender).unwrap();

    let created_order: GetOrderInfoResponse =
        query_order_info(deps.as_ref(), Uint128::one()).unwrap();
    assert_eq!(created_order.requirements, requirements.clone());

    let hook_msg = Cw20HookMsg::FillOrder {
        order_id: Uint128::one(),
        recipient: None,
    };
    let cw20_receive_msg = Cw20ReceiveMsg {
        sender: "origin_cw20_sender".to_string(),
        amount: Uint128::new(5000),
        msg: encode_binary(&hook_msg).unwrap(),
    };

    receive_cw20(deps.as_mut(), cw20_receive_msg, MOCK_CW20_CONTRACT).unwrap();

    let created_order: GetOrderInfoResponse =
        query_order_info(deps.as_ref(), Uint128::one()).unwrap();
    for requirement in created_order.requirements {
        if let Resource::Cw20Token { cw20_addr } = requirement.resource {
            if cw20_addr == AndrAddr::from_string(MOCK_CW20_CONTRACT) {
                for (user, amount) in requirement.deposits {
                    if user == "origin_cw20_sender" {
                        assert_eq!(amount, Uint128::new(5000));
                        continue;
                    }
                }
            }
            continue;
        }
    }

    let order_status = created_order.order_status;
    assert_eq!(order_status, OrderStatus::NotCompleted);

    let cw20_receive_msg = Cw20ReceiveMsg {
        sender: "origin_cw20_sender".to_string(),
        amount: Uint128::new(7000),
        msg: encode_binary(&hook_msg).unwrap(),
    };

    receive_cw20(deps.as_mut(), cw20_receive_msg, MOCK_CW20_CONTRACT).unwrap();

    let created_order: GetOrderInfoResponse =
        query_order_info(deps.as_ref(), Uint128::one()).unwrap();
    for requirement in created_order.requirements {
        if let Resource::Cw20Token { cw20_addr } = requirement.resource {
            if cw20_addr == AndrAddr::from_string(MOCK_CW20_CONTRACT) {
                for (user, amount) in requirement.deposits {
                    if user == "origin_cw20_sender" {
                        assert_eq!(amount, Uint128::new(10000));
                        continue;
                    }
                }
            }
            continue;
        }
    }

    let order_status = created_order.order_status;
    assert_eq!(order_status, OrderStatus::NotCompleted);

    let cw20_receive_msg = Cw20ReceiveMsg {
        sender: "origin_cw20_sender".to_string(),
        amount: Uint128::new(24000),
        msg: encode_binary(&hook_msg).unwrap(),
    };

    receive_cw20(deps.as_mut(), cw20_receive_msg, MOCK_CW20_CONTRACT_2).unwrap();

    let created_order: GetOrderInfoResponse =
        query_order_info(deps.as_ref(), Uint128::one()).unwrap();
    for requirement in created_order.requirements {
        if let Resource::Cw20Token { cw20_addr } = requirement.resource {
            if cw20_addr == AndrAddr::from_string(MOCK_CW20_CONTRACT_2) {
                for (user, amount) in requirement.deposits {
                    if user == "origin_cw20_sender" {
                        assert_eq!(amount, Uint128::new(20000));
                        continue;
                    }
                }
            }
            continue;
        }
    }

    let order_status = created_order.order_status;
    assert_eq!(order_status, OrderStatus::Completed);
}

#[test]
fn test_two_users_fill_order_with_cw20() {
    let (mut deps, info) = proper_initialization(
        Some(vec![AndrAddr::from_string(MOCK_NFT_CONTRACT)]),
        Some(vec![
            AndrAddr::from_string(MOCK_CW20_CONTRACT),
            AndrAddr::from_string(MOCK_CW20_CONTRACT_2),
        ]),
    );

    let requirements = vec![
        ResourceRequirement {
            resource: Resource::Cw20Token {
                cw20_addr: AndrAddr::from_string(MOCK_CW20_CONTRACT.to_string()),
            },
            amount: Uint128::new(10000),
            deposits: HashMap::new(),
        },
        ResourceRequirement {
            resource: Resource::Cw20Token {
                cw20_addr: AndrAddr::from_string(MOCK_CW20_CONTRACT_2.to_string()),
            },
            amount: Uint128::new(20000),
            deposits: HashMap::new(),
        },
    ];

    let output = Resource::Nft {
        cw721_addr: AndrAddr::from_string(MOCK_NFT_CONTRACT_TO_MINT.to_string()),
        token_id: "test_nft_to_mint_1".to_string(),
    };

    let sender = info.sender.as_str();

    create_order(deps.as_mut(), requirements.clone(), output, sender).unwrap();

    let created_order: GetOrderInfoResponse =
        query_order_info(deps.as_ref(), Uint128::one()).unwrap();
    assert_eq!(created_order.requirements, requirements.clone());

    let hook_msg = Cw20HookMsg::FillOrder {
        order_id: Uint128::one(),
        recipient: None,
    };
    let cw20_receive_msg = Cw20ReceiveMsg {
        sender: "origin_cw20_sender".to_string(),
        amount: Uint128::new(5000),
        msg: encode_binary(&hook_msg).unwrap(),
    };

    receive_cw20(deps.as_mut(), cw20_receive_msg, MOCK_CW20_CONTRACT).unwrap();

    let cw20_receive_msg = Cw20ReceiveMsg {
        sender: "origin_cw20_sender_2".to_string(),
        amount: Uint128::new(6000),
        msg: encode_binary(&hook_msg).unwrap(),
    };

    receive_cw20(deps.as_mut(), cw20_receive_msg, MOCK_CW20_CONTRACT).unwrap();

    let created_order: GetOrderInfoResponse =
        query_order_info(deps.as_ref(), Uint128::one()).unwrap();

    let order_status = created_order.order_status;
    assert_eq!(order_status, OrderStatus::NotCompleted);

    let cw20_receive_msg = Cw20ReceiveMsg {
        sender: "origin_cw20_sender".to_string(),
        amount: Uint128::new(7000),
        msg: encode_binary(&hook_msg).unwrap(),
    };

    receive_cw20(deps.as_mut(), cw20_receive_msg, MOCK_CW20_CONTRACT).unwrap();

    let cw20_receive_msg = Cw20ReceiveMsg {
        sender: "origin_cw20_sender_2".to_string(),
        amount: Uint128::new(4000),
        msg: encode_binary(&hook_msg).unwrap(),
    };

    receive_cw20(deps.as_mut(), cw20_receive_msg, MOCK_CW20_CONTRACT).unwrap();

    let created_order: GetOrderInfoResponse =
        query_order_info(deps.as_ref(), Uint128::one()).unwrap();

    let order_status = created_order.order_status;
    assert_eq!(order_status, OrderStatus::NotCompleted);

    let cw20_receive_msg = Cw20ReceiveMsg {
        sender: "origin_cw20_sender".to_string(),
        amount: Uint128::new(24000),
        msg: encode_binary(&hook_msg).unwrap(),
    };

    receive_cw20(deps.as_mut(), cw20_receive_msg, MOCK_CW20_CONTRACT_2).unwrap();

    let created_order: GetOrderInfoResponse =
        query_order_info(deps.as_ref(), Uint128::one()).unwrap();

    let order_status = created_order.order_status;
    assert_eq!(order_status, OrderStatus::Completed);

    let recipient = created_order.output_recipient.unwrap();
    assert_eq!(recipient, AndrAddr::from_string("origin_cw20_sender"));
}

#[test]
fn test_fill_order_with_nft() {
    let (mut deps, info) =
        proper_initialization(Some(vec![AndrAddr::from_string(MOCK_NFT_CONTRACT)]), None);

    let requirements = vec![
        ResourceRequirement {
            resource: Resource::Nft {
                cw721_addr: AndrAddr::from_string(MOCK_NFT_CONTRACT),
                token_id: "test_nft_1".to_string(),
            },
            amount: Uint128::one(),
            deposits: HashMap::new(),
        },
        ResourceRequirement {
            resource: Resource::Nft {
                cw721_addr: AndrAddr::from_string(MOCK_NFT_CONTRACT),
                token_id: "test_nft_2".to_string(),
            },
            amount: Uint128::one(),
            deposits: HashMap::new(),
        },
    ];

    let output = Resource::Nft {
        cw721_addr: AndrAddr::from_string(MOCK_NFT_CONTRACT_TO_MINT.to_string()),
        token_id: "test_nft_to_mint".to_string(),
    };

    let sender = info.sender.as_str();

    create_order(deps.as_mut(), requirements.clone(), output, sender).unwrap();

    let created_order: GetOrderInfoResponse =
        query_order_info(deps.as_ref(), Uint128::one()).unwrap();
    assert_eq!(created_order.requirements, requirements.clone());

    let hook_msg = Cw721HookMsg::FillOrder {
        order_id: Uint128::one(),
        recipient: None,
    };
    let cw721_receive_msg = Cw721ReceiveMsg {
        sender: "origin_nft_sender".to_string(),
        token_id: "test_nft_1".to_string(),
        msg: encode_binary(&hook_msg).unwrap(),
    };

    receive_nft(deps.as_mut(), cw721_receive_msg, MOCK_NFT_CONTRACT).unwrap();

    let created_order: GetOrderInfoResponse =
        query_order_info(deps.as_ref(), Uint128::one()).unwrap();

    let order_status = created_order.order_status;
    assert_eq!(order_status, OrderStatus::NotCompleted);

    let cw721_receive_msg = Cw721ReceiveMsg {
        sender: "origin_nft_sender".to_string(),
        token_id: "test_nft_2".to_string(),
        msg: encode_binary(&hook_msg).unwrap(),
    };

    receive_nft(deps.as_mut(), cw721_receive_msg, MOCK_NFT_CONTRACT).unwrap();

    let created_order: GetOrderInfoResponse =
        query_order_info(deps.as_ref(), Uint128::one()).unwrap();

    let order_status = created_order.order_status;
    assert_eq!(order_status, OrderStatus::Completed);

    let recipient = created_order.output_recipient.unwrap();
    assert_eq!(recipient, AndrAddr::from_string("origin_nft_sender"));
}

#[test]
fn test_cancel_order() {
    let (mut deps, info) = proper_initialization(
        Some(vec![AndrAddr::from_string(MOCK_NFT_CONTRACT)]),
        Some(vec![
            AndrAddr::from_string(MOCK_CW20_CONTRACT),
            AndrAddr::from_string(MOCK_CW20_CONTRACT_2),
        ]),
    );

    let requirements = vec![
        ResourceRequirement {
            resource: Resource::Cw20Token {
                cw20_addr: AndrAddr::from_string(MOCK_CW20_CONTRACT.to_string()),
            },
            amount: Uint128::new(10000),
            deposits: HashMap::new(),
        },
        ResourceRequirement {
            resource: Resource::Cw20Token {
                cw20_addr: AndrAddr::from_string(MOCK_CW20_CONTRACT_2.to_string()),
            },
            amount: Uint128::new(20000),
            deposits: HashMap::new(),
        },
    ];

    let output = Resource::Nft {
        cw721_addr: AndrAddr::from_string(MOCK_NFT_CONTRACT_TO_MINT.to_string()),
        token_id: "test_nft_to_mint_1".to_string(),
    };

    let sender = info.sender.as_str();

    create_order(deps.as_mut(), requirements.clone(), output, sender).unwrap();

    let created_order: GetOrderInfoResponse =
        query_order_info(deps.as_ref(), Uint128::one()).unwrap();
    assert_eq!(created_order.requirements, requirements.clone());

    let hook_msg = Cw20HookMsg::FillOrder {
        order_id: Uint128::one(),
        recipient: None,
    };
    let cw20_receive_msg = Cw20ReceiveMsg {
        sender: "origin_cw20_sender".to_string(),
        amount: Uint128::new(5000),
        msg: encode_binary(&hook_msg).unwrap(),
    };

    receive_cw20(deps.as_mut(), cw20_receive_msg, MOCK_CW20_CONTRACT).unwrap();

    let cw20_receive_msg = Cw20ReceiveMsg {
        sender: "origin_cw20_sender_2".to_string(),
        amount: Uint128::new(6000),
        msg: encode_binary(&hook_msg).unwrap(),
    };

    receive_cw20(deps.as_mut(), cw20_receive_msg.clone(), MOCK_CW20_CONTRACT).unwrap();

    let created_order: GetOrderInfoResponse =
        query_order_info(deps.as_ref(), Uint128::one()).unwrap();

    let order_status = created_order.order_status;
    assert_eq!(order_status, OrderStatus::NotCompleted);

    cancel_order(deps.as_mut(), Uint128::one(), "creator").unwrap();

    let created_order: GetOrderInfoResponse =
        query_order_info(deps.as_ref(), Uint128::one()).unwrap();

    let order_status = created_order.order_status;
    assert_eq!(order_status, OrderStatus::Cancelled);

    let err_res: ContractError =
        receive_cw20(deps.as_mut(), cw20_receive_msg.clone(), MOCK_CW20_CONTRACT).unwrap_err();
    assert_eq!(
        err_res,
        ContractError::CustomError {
            msg: "Already cancelled order".to_string(),
        }
    );

    let _cancelled_order: GetOrdersByStatusResponse =
        query_orders_by_status(deps.as_ref(), OrderStatus::Cancelled, None).unwrap();

    let _user_deposited_orders = query_user_deposited_orders(
        deps.as_ref(),
        AndrAddr::from_string("origin_cw20_sender"),
        None,
    )
    .unwrap();
}
