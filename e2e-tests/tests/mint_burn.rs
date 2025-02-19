use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, MockAppContract};
use andromeda_cw20::mock::{mock_andromeda_cw20, mock_cw20_instantiate_msg, MockCW20};
use andromeda_cw721::mock::{mock_andromeda_cw721, mock_cw721_instantiate_msg, MockCW721};
use andromeda_finance::mint_burn::{
    Cw20HookMsg, Cw721HookMsg, ExecuteMsg, OrderStatus, Resource, ResourceRequirement,
};
use andromeda_mint_burn::mock::{
    mock_andromeda_mint_burn, mock_mint_burn_instantiate_msg, MockMintBurn,
};
use andromeda_std::{amp::AndrAddr, error::ContractError};
use andromeda_testing::{
    mock::{mock_app, MockApp},
    mock_builder::MockAndromedaBuilder,
    MockAndromeda, MockContract,
};
use cosmwasm_std::{coin, to_json_binary, Coin, Empty, Uint128};
use cw20::{Cw20Coin, MinterResponse};
use cw_multi_test::Contract;
use rstest::{fixture, rstest};

struct TestCase {
    router: MockApp,
    andr: MockAndromeda,
    mint_burn: MockMintBurn,
    cw20: MockCW20,
    cw721: MockCW721,
}

#[fixture]
fn wallets() -> Vec<(&'static str, Vec<Coin>)> {
    vec![
        ("owner", vec![coin(1000000, "uandr")]),
        ("user1", vec![]),
        ("user2", vec![]),
        ("user3", vec![]),
    ]
}

#[fixture]
fn contracts() -> Vec<(&'static str, Box<dyn Contract<Empty>>)> {
    vec![
        ("cw20", mock_andromeda_cw20()),
        ("cw721", mock_andromeda_cw721()),
        ("mint-burn", mock_andromeda_mint_burn()),
        ("app-contract", mock_andromeda_app()),
    ]
}

#[fixture]
fn setup(
    wallets: Vec<(&'static str, Vec<Coin>)>,
    contracts: Vec<(&'static str, Box<dyn Contract<Empty>>)>,
) -> TestCase {
    let mut router = mock_app(None);

    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(wallets)
        .with_contracts(contracts)
        .build(&mut router);

    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    // Step 1: Instantiate Mint-Burn contract first
    let mint_burn_init_msg = mock_mint_burn_instantiate_msg(
        andr.kernel.addr().to_string(),
        Some(owner.to_string()),
        None,
        None,
    );
    let mint_burn_component = AppComponent::new(
        "mint-burn".to_string(),
        "mint-burn".to_string(),
        to_json_binary(&mint_burn_init_msg).unwrap(),
    );

    // Step 2: Instantiate the App contract and get Mint-Burn contract's address
    let app = MockAppContract::instantiate(
        andr.get_code_id(&mut router, "app-contract"),
        owner,
        &mut router,
        "Mint Burn App",
        vec![mint_burn_component.clone()],
        andr.kernel.addr(),
        None,
    );

    let mint_burn: MockMintBurn = app.query_ado_by_component_name(&router, "mint-burn".to_string());
    let mint_burn_addr = mint_burn.addr(); // Get Mint-Burn contract address

    // Step 3: Initialize CW721 with Mint-Burn contract as the minter
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "NFT Collection".to_string(),
        "NFT".to_string(),
        mint_burn_addr.clone(), // Set Mint-Burn as minter
        andr.kernel.addr().to_string(),
        None, // Ensure minter is set correctly
    );

    let cw721_component = AppComponent::new(
        "cw721".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    // Step 4: Initialize CW20 contract
    let initial_balances = vec![
        Cw20Coin {
            address: owner.to_string(),
            amount: Uint128::from(1_000_000_000u128),
        },
        Cw20Coin {
            address: user1.to_string(),
            amount: Uint128::from(1_000_000_000u128),
        },
    ];
    let cw20_init_msg = mock_cw20_instantiate_msg(
        None,
        "Test Token".to_string(),
        "TTK".to_string(),
        6,
        initial_balances,
        Some(MinterResponse {
            minter: mint_burn_addr.clone().to_string(),
            cap: None,
        }),
        andr.kernel.addr().to_string(),
    );
    let cw20_component = AppComponent::new(
        "cw20".to_string(),
        "cw20".to_string(),
        to_json_binary(&cw20_init_msg).unwrap(),
    );

    app.execute_add_app_component(&mut router, owner.clone(), cw20_component.clone())
        .unwrap();
    app.execute_add_app_component(&mut router, owner.clone(), cw721_component.clone())
        .unwrap();

    let cw20: MockCW20 = app.query_ado_by_component_name(&router, cw20_component.name);
    let cw721: MockCW721 = app.query_ado_by_component_name(&router, cw721_component.name);

    TestCase {
        router,
        andr,
        mint_burn,
        cw20,
        cw721,
    }
}

#[rstest]
fn test_successful_cw20_to_nft(setup: TestCase) {
    let TestCase {
        mut router,
        andr,
        mint_burn,
        cw20,
        cw721,
    } = setup;

    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");
    let user2 = andr.get_wallet("user2");

    // Create order: burn 100 CW20 -> mint 1 NFT
    let order_msg = ExecuteMsg::CreateOrder {
        requirements: vec![ResourceRequirement {
            resource: Resource::Cw20Token {
                cw20_addr: AndrAddr::from_string(cw20.addr().to_string()),
            },
            amount: Uint128::new(1_000_000_u128),
            deposits: Default::default(),
        }],
        output: Resource::Nft {
            cw721_addr: AndrAddr::from_string(cw721.addr().to_string()),
            token_id: "new_nft".to_string(),
        },
    };
    mint_burn
        .execute(&mut router, &order_msg, owner.clone(), &[])
        .unwrap();

    // User1 fills the order
    let hook_msg = Cw20HookMsg::FillOrder {
        order_id: Uint128::one(),
        recipient: Some(AndrAddr::from_string(user2.as_str())),
    };

    let external_cw20_code_id = andr.get_code_id(&mut router, "cw20"); // Get CW20 code ID
    let external_cw20 = MockCW20::instantiate(
        external_cw20_code_id,
        owner.clone(),
        &mut router,
        None,
        "Invalid Cw20".to_string(),
        "ICT".to_string(),
        6,
        vec![
            Cw20Coin {
                address: owner.to_string(),
                amount: Uint128::from(500_000_000u128), // External CW20 supply
            },
            Cw20Coin {
                address: user1.to_string(),
                amount: Uint128::from(500_000_000u128),
            },
        ],
        None,
        andr.kernel.addr().to_string(),
    );

    let err_res: ContractError = external_cw20
        .execute_send(
            &mut router,
            user1.clone(),
            mint_burn.addr(),
            Uint128::new(2_000_000_u128),
            &hook_msg,
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(
        err_res,
        ContractError::CustomError {
            msg: "Invalid CW20 token sent".to_string()
        }
    );

    cw20.execute_send(
        &mut router,
        user1.clone(),
        mint_burn.addr(),
        Uint128::new(2_000_000_u128),
        &hook_msg,
    )
    .unwrap();

    // Verify NFT is minted to user1
    let nft_owner = cw721.query_owner_of(&router, "new_nft");
    assert_eq!(nft_owner, user2.to_string());

    // Verify CW20 was burned
    let mint_burn_balance = cw20.query_balance(&router, mint_burn.addr());
    assert_eq!(mint_burn_balance, Uint128::zero());

    // Verify excess CW20 was refunded
    let user1_balance = cw20.query_balance(&router, user1.clone());
    assert_eq!(user1_balance, Uint128::new(999_000_000_u128));

    let hook_msg = Cw20HookMsg::FillOrder {
        order_id: Uint128::one(),
        recipient: Some(AndrAddr::from_string(user2.as_str())),
    };

    let err_res: ContractError = cw20
        .execute_send(
            &mut router,
            user1.clone(),
            mint_burn.addr(),
            Uint128::new(2000000_u128),
            &hook_msg,
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    // Verify order is already completed
    assert_eq!(
        err_res,
        ContractError::CustomError {
            msg: "Already Completed Order".to_string()
        }
    );
}

#[rstest]
fn test_nft_and_cw20_to_nft(setup: TestCase) {
    let TestCase {
        mut router,
        andr,
        mint_burn,
        cw20,
        cw721,
    } = setup;

    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    // Mint NFT to user1
    cw721
        .execute_quick_mint(&mut router, owner.clone(), 1, user1.to_string())
        .unwrap();

    // Create order: Burn NFT + 1,000,000 CW20 → Mint new NFT
    let order_msg = ExecuteMsg::CreateOrder {
        requirements: vec![
            ResourceRequirement {
                resource: Resource::Nft {
                    cw721_addr: AndrAddr::from_string(cw721.addr().to_string()),
                    token_id: "0".to_string(),
                },
                amount: Uint128::one(),
                deposits: Default::default(),
            },
            ResourceRequirement {
                resource: Resource::Cw20Token {
                    cw20_addr: AndrAddr::from_string(cw20.addr().to_string()),
                },
                amount: Uint128::new(1_000_000_u128),
                deposits: Default::default(),
            },
        ],
        output: Resource::Nft {
            cw721_addr: AndrAddr::from_string(cw721.addr().to_string()),
            token_id: "new_nft".to_string(),
        },
    };

    mint_burn
        .execute(&mut router, &order_msg, owner.clone(), &[])
        .unwrap();

    // User1 sends NFT
    let nft_hook_msg = Cw721HookMsg::FillOrder {
        order_id: Uint128::one(),
        recipient: Some(AndrAddr::from_string(user1.as_str())),
    };
    cw721
        .execute_send_nft(
            &mut router,
            user1.clone(),
            mint_burn.addr(),
            "0".to_string(),
            &nft_hook_msg,
        )
        .unwrap();

    // User1 sends CW20
    let cw20_hook_msg = Cw20HookMsg::FillOrder {
        order_id: Uint128::one(),
        recipient: Some(AndrAddr::from_string(user1.as_str())),
    };
    cw20.execute_send(
        &mut router,
        user1.clone(),
        mint_burn.addr(),
        Uint128::new(1_000_000_u128),
        &cw20_hook_msg,
    )
    .unwrap();

    // Verify new NFT minted to user1
    let nft_owner = cw721.query_owner_of(&router, "new_nft");
    assert_eq!(nft_owner, user1.to_string());
}

#[rstest]
fn test_nft_and_cw20_to_cw20(setup: TestCase) {
    let TestCase {
        mut router,
        andr,
        mint_burn,
        cw20,
        cw721,
    } = setup;

    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    // Mint NFT to user1
    cw721
        .execute_quick_mint(&mut router, owner.clone(), 1, user1.to_string())
        .unwrap();

    // Create order: Burn NFT + CW20 → Mint CW20
    let order_msg = ExecuteMsg::CreateOrder {
        requirements: vec![
            ResourceRequirement {
                resource: Resource::Nft {
                    cw721_addr: AndrAddr::from_string(cw721.addr().to_string()),
                    token_id: "0".to_string(),
                },
                amount: Uint128::one(),
                deposits: Default::default(),
            },
            ResourceRequirement {
                resource: Resource::Cw20Token {
                    cw20_addr: AndrAddr::from_string(cw20.addr().to_string()),
                },
                amount: Uint128::new(500_000_u128),
                deposits: Default::default(),
            },
        ],
        output: Resource::Cw20Token {
            cw20_addr: AndrAddr::from_string(cw20.addr().to_string()),
        },
    };

    mint_burn
        .execute(&mut router, &order_msg, owner.clone(), &[])
        .unwrap();

    // Send NFT
    let nft_hook_msg = Cw721HookMsg::FillOrder {
        order_id: Uint128::one(),
        recipient: Some(AndrAddr::from_string(user1.as_str())),
    };
    cw721
        .execute_send_nft(
            &mut router,
            user1.clone(),
            mint_burn.addr(),
            "0".to_string(),
            &nft_hook_msg,
        )
        .unwrap();

    // Send CW20
    let cw20_hook_msg = Cw20HookMsg::FillOrder {
        order_id: Uint128::one(),
        recipient: Some(AndrAddr::from_string(user1.as_str())),
    };
    cw20.execute_send(
        &mut router,
        user1.clone(),
        mint_burn.addr(),
        Uint128::new(500_000_u128),
        &cw20_hook_msg,
    )
    .unwrap();

    // Verify CW20 minted to user1
    let user1_balance = cw20.query_balance(&router, user1.clone());
    assert_eq!(user1_balance, Uint128::new(999_500_000_u128 + 1)); // Original + Minted
}

#[rstest]
fn test_cw20_to_cw20(setup: TestCase) {
    let TestCase {
        mut router,
        andr,
        mint_burn,
        cw20,
        ..
    } = setup;

    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    // Create order: Burn 1,000,000 CW20 → Mint 1,000,000 new CW20
    let order_msg = ExecuteMsg::CreateOrder {
        requirements: vec![ResourceRequirement {
            resource: Resource::Cw20Token {
                cw20_addr: AndrAddr::from_string(cw20.addr().to_string()),
            },
            amount: Uint128::new(1_000_000_u128),
            deposits: Default::default(),
        }],
        output: Resource::Cw20Token {
            cw20_addr: AndrAddr::from_string(cw20.addr().to_string()),
        },
    };
    mint_burn
        .execute(&mut router, &order_msg, owner.clone(), &[])
        .unwrap();

    // Send CW20
    let cw20_hook_msg = Cw20HookMsg::FillOrder {
        order_id: Uint128::one(),
        recipient: Some(AndrAddr::from_string(user1.as_str())),
    };
    cw20.execute_send(
        &mut router,
        user1.clone(),
        mint_burn.addr(),
        Uint128::new(1_000_000_u128),
        &cw20_hook_msg,
    )
    .unwrap();

    // Verify CW20 was minted back
    let user1_balance = cw20.query_balance(&router, user1.clone());
    assert_eq!(user1_balance, Uint128::new(999_000_000_u128 + 1));
}

#[rstest]
fn test_nft_to_cw20(setup: TestCase) {
    let TestCase {
        mut router,
        andr,
        mint_burn,
        cw20,
        cw721,
    } = setup;

    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    // Mint NFT to user1
    cw721
        .execute_quick_mint(&mut router, owner.clone(), 1, user1.to_string())
        .unwrap();

    // Create order: Burn NFT → Mint 1,000,000 CW20
    let order_msg = ExecuteMsg::CreateOrder {
        requirements: vec![ResourceRequirement {
            resource: Resource::Nft {
                cw721_addr: AndrAddr::from_string(cw721.addr().to_string()),
                token_id: "0".to_string(),
            },
            amount: Uint128::one(),
            deposits: Default::default(),
        }],
        output: Resource::Cw20Token {
            cw20_addr: AndrAddr::from_string(cw20.addr().to_string()),
        },
    };

    mint_burn
        .execute(&mut router, &order_msg, owner.clone(), &[])
        .unwrap();

    // Send NFT
    let nft_hook_msg = Cw721HookMsg::FillOrder {
        order_id: Uint128::one(),
        recipient: Some(AndrAddr::from_string(user1.as_str())),
    };
    cw721
        .execute_send_nft(
            &mut router,
            user1.clone(),
            mint_burn.addr(),
            "0".to_string(),
            &nft_hook_msg,
        )
        .unwrap();

    // Verify CW20 was minted to user1
    let user1_balance = cw20.query_balance(&router, user1.clone());
    assert_eq!(user1_balance, Uint128::new(1_000_000_000_u128 + 1));
}

#[rstest]
fn test_cancel_order(setup: TestCase) {
    let TestCase {
        mut router,
        andr,
        mint_burn,
        cw20,
        ..
    } = setup;

    let owner = andr.get_wallet("owner");

    // Create order
    let order_msg = ExecuteMsg::CreateOrder {
        requirements: vec![ResourceRequirement {
            resource: Resource::Cw20Token {
                cw20_addr: AndrAddr::from_string(cw20.addr().to_string()),
            },
            amount: Uint128::new(1_000_000_u128),
            deposits: Default::default(),
        }],
        output: Resource::Cw20Token {
            cw20_addr: AndrAddr::from_string(cw20.addr().to_string()),
        },
    };
    mint_burn
        .execute(&mut router, &order_msg, owner.clone(), &[])
        .unwrap();

    // Cancel the order
    let cancel_msg = ExecuteMsg::CancelOrder {
        order_id: Uint128::one(),
    };
    mint_burn
        .execute(&mut router, &cancel_msg, owner.clone(), &[])
        .unwrap();

    // Verify the order is canceled
    let order = mint_burn.query_order_info(&mut router, Uint128::one());
    assert_eq!(order.status, OrderStatus::Cancelled);
}
