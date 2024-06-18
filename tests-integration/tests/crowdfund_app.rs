use std::str::FromStr;

use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, MockAppContract};
use andromeda_crowdfund::mock::{
    mock_andromeda_crowdfund, mock_crowdfund_instantiate_msg, mock_purchase_cw20_msg, MockCrowdfund,
};
use andromeda_cw20::mock::{mock_andromeda_cw20, mock_cw20_instantiate_msg, mock_minter, MockCW20};
use andromeda_cw721::mock::{mock_andromeda_cw721, mock_cw721_instantiate_msg, MockCW721};
use andromeda_finance::splitter::AddressPercent;
use andromeda_non_fungible_tokens::{
    crowdfund::{CampaignConfig, CampaignStage, PresaleTierOrder, SimpleTierOrder, TierMetaData},
    cw721::TokenExtension,
};
use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, mock_splitter_send_msg,
};
use andromeda_std::{
    amp::{AndrAddr, Recipient},
    common::{denom::Asset, encode_binary, Milliseconds},
};
use andromeda_testing::{
    mock::{mock_app, MockApp},
    mock_builder::MockAndromedaBuilder,
    MockAndromeda, MockContract,
};
use cosmwasm_std::{coin, to_json_binary, BlockInfo, Coin, Decimal, Empty, Uint128, Uint64};
use cw20::Cw20Coin;
use cw_multi_test::Contract;
use rstest::{fixture, rstest};

struct TestCase {
    router: MockApp,
    andr: MockAndromeda,
    crowdfund: MockCrowdfund,
    cw20: Option<MockCW20>,
    cw721: MockCW721,
    presale: Vec<PresaleTierOrder>,
}

#[fixture]
fn wallets() -> Vec<(&'static str, Vec<Coin>)> {
    vec![
        ("owner", vec![]),
        ("buyer_one", vec![coin(1000000, "uandr")]),
        ("recipient", vec![]),
        ("recipient1", vec![]),
        ("recipient2", vec![]),
    ]
}

#[fixture]
fn contracts() -> Vec<(&'static str, Box<dyn Contract<Empty>>)> {
    vec![
        ("cw20", mock_andromeda_cw20()),
        ("cw721", mock_andromeda_cw721()),
        ("crowdfund", mock_andromeda_crowdfund()),
        ("splitter", mock_andromeda_splitter()),
        ("app-contract", mock_andromeda_app()),
    ]
}

#[fixture]
fn setup(
    #[default(true)] use_native_token: bool,
    #[default(None)] withdrawal_recipient: Option<Recipient>,
    wallets: Vec<(&'static str, Vec<Coin>)>,
    contracts: Vec<(&'static str, Box<dyn Contract<Empty>>)>,
) -> TestCase {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(wallets)
        .with_contracts(contracts)
        .build(&mut router);

    let owner = andr.get_wallet("owner");
    let buyer_one = andr.get_wallet("buyer_one");
    let recipient =
        withdrawal_recipient.unwrap_or(Recipient::new(andr.get_wallet("recipient"), None));

    // Prepare Splitter component which can be used as a withdrawal address for some test cases
    let recipient_1 = andr.get_wallet("recipient1");
    let recipient_2 = andr.get_wallet("recipient2");

    let splitter_recipients = vec![
        AddressPercent {
            recipient: Recipient::from_string(recipient_1.to_string()),
            percent: Decimal::from_str("0.2").unwrap(),
        },
        AddressPercent {
            recipient: Recipient::from_string(recipient_2.to_string()),
            percent: Decimal::from_str("0.8").unwrap(),
        },
    ];
    let splitter_init_msg =
        mock_splitter_instantiate_msg(splitter_recipients, andr.kernel.addr().clone(), None, None);
    let splitter_component = AppComponent::new(
        "splitter".to_string(),
        "splitter".to_string(),
        to_json_binary(&splitter_init_msg).unwrap(),
    );
    // Add cw721 component
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Campaign Tier".to_string(),
        "CT".to_string(),
        "./crowdfund".to_string(),
        andr.kernel.addr().to_string(),
        None,
    );

    let cw721_component = AppComponent::new(
        "cw721".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    let mut app_components = vec![splitter_component, cw721_component.clone()];

    // Add cw20 components for test cases using cw20
    let cw20_component: Option<AppComponent> = match use_native_token {
        true => None,
        false => {
            let initial_balances = vec![Cw20Coin {
                address: buyer_one.to_string(),
                amount: Uint128::from(1000000u128),
            }];
            let cw20_init_msg = mock_cw20_instantiate_msg(
                None,
                "Test Tokens".to_string(),
                "TTT".to_string(),
                6,
                initial_balances,
                Some(mock_minter(
                    owner.to_string(),
                    Some(Uint128::from(1000000u128)),
                )),
                andr.kernel.addr().to_string(),
            );
            let cw20_component = AppComponent::new(
                "cw20".to_string(),
                "cw20".to_string(),
                to_json_binary(&cw20_init_msg).unwrap(),
            );
            app_components.push(cw20_component.clone());
            Some(cw20_component)
        }
    };
    let denom = match use_native_token {
        true => Asset::NativeToken("uandr".to_string()),
        false => Asset::Cw20Token(AndrAddr::from_string(format!(
            "./{}",
            cw20_component.clone().unwrap().name
        ))),
    };
    // Add campaign component
    // *** IMPORTANT ***
    //    campaign component should be added in the last order
    //    as it is using vfs to other components(potentially components in the same app)

    let campaign_config = mock_campaign_config(
        denom,
        AndrAddr::from_string(format!("./{}", cw721_component.name)),
        recipient.clone(),
        Some(Uint128::new(1000)),
    );
    let crowdfund_init_msg = mock_crowdfund_instantiate_msg(
        campaign_config,
        vec![],
        andr.kernel.addr(),
        Some(owner.to_string()),
    );
    let crowdfund_component = AppComponent::new(
        "crowdfund".to_string(),
        "crowdfund".to_string(),
        to_json_binary(&crowdfund_init_msg).unwrap(),
    );

    app_components.push(crowdfund_component.clone());

    let app = MockAppContract::instantiate(
        andr.get_code_id(&mut router, "app-contract"),
        owner,
        &mut router,
        "Crowdfund App",
        app_components.clone(),
        andr.kernel.addr(),
        Some(owner.to_string()),
    );

    let crowdfund: MockCrowdfund =
        app.query_ado_by_component_name(&router, crowdfund_component.name);

    let cw721: MockCW721 = app.query_ado_by_component_name(&router, cw721_component.name);

    let cw20: Option<MockCW20> = match use_native_token {
        true => None,
        false => Some(app.query_ado_by_component_name(&router, cw20_component.unwrap().name)),
    };

    let meta_data = TierMetaData {
        token_uri: None,
        extension: TokenExtension {
            ..Default::default()
        },
    };
    crowdfund
        .execute_add_tier(
            owner.clone(),
            &mut router,
            Uint64::one(),
            "Tier 1".to_string(),
            Uint128::new(100),
            None,
            meta_data.clone(),
        )
        .unwrap();
    crowdfund
        .execute_add_tier(
            owner.clone(),
            &mut router,
            Uint64::new(2u64),
            "Tier 2".to_string(),
            Uint128::new(200),
            Some(Uint128::new(100)),
            meta_data,
        )
        .unwrap();

    let presale = vec![PresaleTierOrder {
        level: Uint64::one(),
        amount: Uint128::new(10u128),
        orderer: buyer_one.clone(),
    }];

    TestCase {
        router,
        andr,
        crowdfund,
        cw20,
        cw721,
        presale,
    }
}

#[rstest]
fn test_successful_crowdfund_app_native(setup: TestCase) {
    let TestCase {
        mut router,
        andr,
        crowdfund,
        cw721,
        presale,
        ..
    } = setup;

    let owner = andr.get_wallet("owner");
    let buyer_one = andr.get_wallet("buyer_one");
    let recipient = Recipient::new(andr.get_wallet("recipient"), None);

    // Start campaign
    let start_time = None;
    let end_time = Milliseconds::from_nanos(router.block_info().time.plus_days(1).nanos());

    let _ = crowdfund.execute_start_campaign(
        owner.clone(),
        &mut router,
        start_time,
        end_time,
        Some(presale),
    );
    let summary = crowdfund.query_campaign_summary(&mut router);
    assert_eq!(summary.current_capital, 0);
    assert_eq!(summary.current_stage, CampaignStage::ONGOING.to_string());

    // Purchase tiers
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: router.block_info().time.plus_seconds(1),
        chain_id: router.block_info().chain_id,
    });

    let orders = vec![
        SimpleTierOrder {
            level: Uint64::one(),
            amount: Uint128::new(10),
        },
        SimpleTierOrder {
            level: Uint64::new(2),
            amount: Uint128::new(10),
        },
    ];
    let buyer_one_original_balance = router
        .wrap()
        .query_balance(buyer_one.clone(), "uandr")
        .unwrap()
        .amount;
    let _ = crowdfund.execute_purchase(
        buyer_one.clone(),
        &mut router,
        orders,
        vec![coin(5000, "uandr")],
    );
    let buyer_one_balance = router
        .wrap()
        .query_balance(buyer_one.clone(), "uandr")
        .unwrap()
        .amount;

    assert_eq!(
        buyer_one_balance,
        buyer_one_original_balance - Uint128::new(10 * 100 + 200 * 10)
    );

    // End campaign
    let _ = crowdfund.execute_end_campaign(owner.clone(), &mut router);
    let summary = crowdfund.query_campaign_summary(&mut router);
    assert_eq!(summary.current_capital, 10 * 100 + 200 * 10);
    assert_eq!(summary.current_stage, CampaignStage::SUCCESS.to_string());
    let recipient_balance = router
        .wrap()
        .query_balance(recipient.clone().address, "uandr")
        .unwrap()
        .amount;
    assert_eq!(summary.current_capital, recipient_balance.into());

    // Claim tier
    let _ = crowdfund
        .execute_claim(buyer_one.clone(), &mut router)
        .unwrap();
    // buyer_one should own 30 tiers now (10 pre order + 20 purchased)
    let owner_resp = cw721.query_owner_of(&router, "0".to_string());
    assert_eq!(owner_resp, buyer_one.to_string());
    let owner_resp = cw721.query_owner_of(&router, "29".to_string());
    assert_eq!(owner_resp, buyer_one.to_string());
}

#[rstest]
fn test_crowdfund_app_native_discard(
    #[with(true, Some(mock_recipient_with_invalid_msg("./splitter")))] setup: TestCase,
) {
    let TestCase {
        mut router,
        andr,
        crowdfund,
        presale,
        ..
    } = setup;

    let owner = andr.get_wallet("owner");
    let buyer_one = andr.get_wallet("buyer_one");

    // Start campaign
    let start_time = None;
    let end_time = Milliseconds::from_nanos(router.block_info().time.plus_days(1).nanos());

    let _ = crowdfund.execute_start_campaign(
        owner.clone(),
        &mut router,
        start_time,
        end_time,
        Some(presale),
    );
    let summary = crowdfund.query_campaign_summary(&mut router);
    assert_eq!(summary.current_capital, 0);
    assert_eq!(summary.current_stage, CampaignStage::ONGOING.to_string());

    // Purchase tiers
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: router.block_info().time.plus_seconds(1),
        chain_id: router.block_info().chain_id,
    });

    let orders = vec![
        SimpleTierOrder {
            level: Uint64::one(),
            amount: Uint128::new(10),
        },
        SimpleTierOrder {
            level: Uint64::new(2),
            amount: Uint128::new(10),
        },
    ];
    let buyer_one_original_balance = router
        .wrap()
        .query_balance(buyer_one.clone(), "uandr")
        .unwrap()
        .amount;
    let _ = crowdfund.execute_purchase(
        buyer_one.clone(),
        &mut router,
        orders,
        vec![coin(5000, "uandr")],
    );
    let buyer_one_balance = router
        .wrap()
        .query_balance(buyer_one.clone(), "uandr")
        .unwrap()
        .amount;

    assert_eq!(
        buyer_one_balance,
        buyer_one_original_balance - Uint128::new(10 * 100 + 200 * 10)
    );

    let _ = crowdfund.execute_end_campaign(owner.clone(), &mut router);

    let summary = crowdfund.query_campaign_summary(&mut router);

    // Campaign could not be ended due to invalid withdrawal recipient msg
    assert_eq!(summary.current_stage, CampaignStage::ONGOING.to_string());

    // Discard campaign
    let _ = crowdfund.execute_discard_campaign(owner.clone(), &mut router);
    let summary = crowdfund.query_campaign_summary(&mut router);
    assert_eq!(summary.current_stage, CampaignStage::FAILED.to_string());

    // Refund
    let buyer_one_original_balance = router
        .wrap()
        .query_balance(buyer_one.clone(), "uandr")
        .unwrap()
        .amount;
    let _ = crowdfund
        .execute_claim(buyer_one.clone(), &mut router)
        .unwrap();
    let buyer_one_balance = router
        .wrap()
        .query_balance(buyer_one.clone(), "uandr")
        .unwrap()
        .amount;
    assert_eq!(
        buyer_one_balance,
        buyer_one_original_balance + Uint128::new(10 * 100 + 200 * 10)
    );
}

#[rstest]
fn test_crowdfund_app_native_with_ado_recipient(
    #[with(true, Some(mock_recipient_with_valid_msg("./splitter")))] setup: TestCase,
) {
    let TestCase {
        mut router,
        andr,
        crowdfund,
        presale,
        ..
    } = setup;

    let owner = andr.get_wallet("owner");
    let buyer_one = andr.get_wallet("buyer_one");
    let recipient_1 = andr.get_wallet("recipient1");
    let recipient_2 = andr.get_wallet("recipient2");

    // Start campaign
    let start_time = None;
    let end_time = Milliseconds::from_nanos(router.block_info().time.plus_days(1).nanos());

    let _ = crowdfund.execute_start_campaign(
        owner.clone(),
        &mut router,
        start_time,
        end_time,
        Some(presale),
    );
    let summary = crowdfund.query_campaign_summary(&mut router);
    assert_eq!(summary.current_capital, 0);
    assert_eq!(summary.current_stage, CampaignStage::ONGOING.to_string());

    // Purchase tiers
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: router.block_info().time.plus_seconds(1),
        chain_id: router.block_info().chain_id,
    });

    let orders = vec![
        SimpleTierOrder {
            level: Uint64::one(),
            amount: Uint128::new(10),
        },
        SimpleTierOrder {
            level: Uint64::new(2),
            amount: Uint128::new(10),
        },
    ];
    let buyer_one_original_balance = router
        .wrap()
        .query_balance(buyer_one.clone(), "uandr")
        .unwrap()
        .amount;
    let _ = crowdfund.execute_purchase(
        buyer_one.clone(),
        &mut router,
        orders,
        vec![coin(5000, "uandr")],
    );
    let buyer_one_balance = router
        .wrap()
        .query_balance(buyer_one.clone(), "uandr")
        .unwrap()
        .amount;

    assert_eq!(
        buyer_one_balance,
        buyer_one_original_balance - Uint128::new(10 * 100 + 200 * 10)
    );

    let _ = crowdfund.execute_end_campaign(owner.clone(), &mut router);

    let summary = crowdfund.query_campaign_summary(&mut router);

    // Campaign could not be ended due to invalid withdrawal recipient msg
    assert_eq!(summary.current_stage, CampaignStage::SUCCESS.to_string());

    let recipient_1_balance = router
        .wrap()
        .query_balance(recipient_1, "uandr")
        .unwrap()
        .amount;
    let recipient_2_balance = router
        .wrap()
        .query_balance(recipient_2, "uandr")
        .unwrap()
        .amount;
    assert_eq!(recipient_1_balance.u128(), summary.current_capital / 5);
    assert_eq!(recipient_2_balance.u128(), summary.current_capital * 4 / 5);
}

#[rstest]
fn test_failed_crowdfund_app_native(setup: TestCase) {
    let TestCase {
        mut router,
        andr,
        crowdfund,
        presale,
        ..
    } = setup;

    let owner = andr.get_wallet("owner");
    let buyer_one = andr.get_wallet("buyer_one");

    // Start campaign
    let start_time = None;
    let end_time = Milliseconds::from_nanos(router.block_info().time.plus_days(1).nanos());

    let _ = crowdfund.execute_start_campaign(
        owner.clone(),
        &mut router,
        start_time,
        end_time,
        Some(presale),
    );
    let summary = crowdfund.query_campaign_summary(&mut router);
    assert_eq!(summary.current_capital, 0);
    assert_eq!(summary.current_stage, CampaignStage::ONGOING.to_string());

    // Purchase tiers
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: router.block_info().time.plus_seconds(1),
        chain_id: router.block_info().chain_id,
    });

    let orders = vec![SimpleTierOrder {
        level: Uint64::one(),
        amount: Uint128::new(5),
    }];
    let buyer_one_original_balance = router
        .wrap()
        .query_balance(buyer_one.clone(), "uandr")
        .unwrap()
        .amount;
    let _ = crowdfund.execute_purchase(
        buyer_one.clone(),
        &mut router,
        orders,
        vec![coin(5000, "uandr")],
    );
    let buyer_one_balance = router
        .wrap()
        .query_balance(buyer_one.clone(), "uandr")
        .unwrap()
        .amount;

    assert_eq!(
        buyer_one_balance,
        buyer_one_original_balance - Uint128::new(5 * 100)
    );

    // End campaign
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: router.block_info().time.plus_days(2),
        chain_id: router.block_info().chain_id,
    });

    let _ = crowdfund.execute_end_campaign(owner.clone(), &mut router);
    let summary = crowdfund.query_campaign_summary(&mut router);
    assert_eq!(summary.current_capital, 5 * 100);
    assert_eq!(summary.current_stage, CampaignStage::FAILED.to_string());

    // Refund
    let buyer_one_original_balance = router
        .wrap()
        .query_balance(buyer_one.clone(), "uandr")
        .unwrap()
        .amount;
    let _ = crowdfund
        .execute_claim(buyer_one.clone(), &mut router)
        .unwrap();
    let buyer_one_balance = router
        .wrap()
        .query_balance(buyer_one.clone(), "uandr")
        .unwrap()
        .amount;
    assert_eq!(
        buyer_one_balance,
        buyer_one_original_balance + Uint128::new(5 * 100)
    );
}

#[rstest]
fn test_successful_crowdfund_app_cw20(#[with(false)] setup: TestCase) {
    let TestCase {
        mut router,
        andr,
        cw721,
        crowdfund,
        cw20,
        presale,
    } = setup;
    let cw20 = cw20.unwrap();
    let owner = andr.get_wallet("owner");
    let buyer_one = andr.get_wallet("buyer_one");
    let recipient = Recipient::new(andr.get_wallet("recipient"), None);

    let start_time = None;
    let end_time = Milliseconds::from_nanos(router.block_info().time.plus_days(1).nanos());

    let _ = crowdfund.execute_start_campaign(
        owner.clone(),
        &mut router,
        start_time,
        end_time,
        Some(presale),
    );
    let summary = crowdfund.query_campaign_summary(&mut router);
    assert_eq!(summary.current_capital, 0);
    assert_eq!(summary.current_stage, CampaignStage::ONGOING.to_string());

    // Purchase tiers
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: router.block_info().time.plus_seconds(1),
        chain_id: router.block_info().chain_id,
    });

    let orders = vec![
        SimpleTierOrder {
            level: Uint64::one(),
            amount: Uint128::new(10),
        },
        SimpleTierOrder {
            level: Uint64::new(2),
            amount: Uint128::new(10),
        },
    ];
    let buyer_one_original_balance = cw20.query_balance(&router, buyer_one.clone());
    let hook_msg = mock_purchase_cw20_msg(orders);

    cw20.execute_send(
        &mut router,
        buyer_one.clone(),
        crowdfund.addr(),
        Uint128::new(5000),
        &hook_msg,
    )
    .unwrap();

    let buyer_one_balance = cw20.query_balance(&router, buyer_one.clone());
    assert_eq!(
        buyer_one_balance,
        buyer_one_original_balance - Uint128::new(10 * 100 + 200 * 10)
    );

    // End campaign
    let _ = crowdfund.execute_end_campaign(owner.clone(), &mut router);
    let summary = crowdfund.query_campaign_summary(&mut router);
    assert_eq!(summary.current_capital, 10 * 100 + 200 * 10);
    assert_eq!(summary.current_stage, CampaignStage::SUCCESS.to_string());
    let recipient_balance = cw20.query_balance(&router, recipient.clone().address);
    assert_eq!(summary.current_capital, recipient_balance.into());

    // Claim tier
    let _ = crowdfund
        .execute_claim(buyer_one.clone(), &mut router)
        .unwrap();
    // buyer_one should own 30 tiers now (10 pre order + 20 purchased)
    let owner_resp = cw721.query_owner_of(&router, "0".to_string());
    assert_eq!(owner_resp, buyer_one.to_string());
    let owner_resp = cw721.query_owner_of(&router, "29".to_string());
    assert_eq!(owner_resp, buyer_one.to_string());
}

#[rstest]
fn test_failed_crowdfund_app_cw20(#[with(false)] setup: TestCase) {
    let TestCase {
        mut router,
        andr,
        crowdfund,
        presale,
        cw20,
        ..
    } = setup;
    let cw20 = cw20.unwrap();

    let owner = andr.get_wallet("owner");
    let buyer_one = andr.get_wallet("buyer_one");

    // Start campaign
    let start_time = None;
    let end_time = Milliseconds::from_nanos(router.block_info().time.plus_days(1).nanos());

    let _ = crowdfund.execute_start_campaign(
        owner.clone(),
        &mut router,
        start_time,
        end_time,
        Some(presale),
    );
    let summary = crowdfund.query_campaign_summary(&mut router);
    assert_eq!(summary.current_capital, 0);
    assert_eq!(summary.current_stage, CampaignStage::ONGOING.to_string());

    // Purchase tiers
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: router.block_info().time.plus_seconds(1),
        chain_id: router.block_info().chain_id,
    });

    let orders = vec![SimpleTierOrder {
        level: Uint64::one(),
        amount: Uint128::new(5),
    }];

    let buyer_one_original_balance = cw20.query_balance(&router, buyer_one.clone());
    let hook_msg = mock_purchase_cw20_msg(orders);

    cw20.execute_send(
        &mut router,
        buyer_one.clone(),
        crowdfund.addr(),
        Uint128::new(5000),
        &hook_msg,
    )
    .unwrap();

    let buyer_one_balance = cw20.query_balance(&router, buyer_one.clone());

    assert_eq!(
        buyer_one_balance,
        buyer_one_original_balance - Uint128::new(5 * 100)
    );

    // End campaign
    router.set_block(BlockInfo {
        height: router.block_info().height,
        time: router.block_info().time.plus_days(2),
        chain_id: router.block_info().chain_id,
    });

    let _ = crowdfund.execute_end_campaign(owner.clone(), &mut router);
    let summary = crowdfund.query_campaign_summary(&mut router);
    assert_eq!(summary.current_capital, 5 * 100);
    assert_eq!(summary.current_stage, CampaignStage::FAILED.to_string());

    // Refund
    let buyer_one_original_balance = cw20.query_balance(&router, buyer_one.clone());
    let _ = crowdfund
        .execute_claim(buyer_one.clone(), &mut router)
        .unwrap();
    let buyer_one_balance = cw20.query_balance(&router, buyer_one.clone());
    assert_eq!(
        buyer_one_balance,
        buyer_one_original_balance + Uint128::new(5 * 100)
    );
}

fn mock_campaign_config(
    denom: Asset,
    token_address: AndrAddr,
    withdrawal_recipient: Recipient,
    soft_cap: Option<Uint128>,
) -> CampaignConfig {
    CampaignConfig {
        title: "First Crowdfund".to_string(),
        description: "Demo campaign for testing".to_string(),
        banner: "http://<campaign_banner>".to_string(),
        url: "http://<campaign_url>".to_string(),
        denom,
        token_address,
        withdrawal_recipient,
        soft_cap,
        hard_cap: None,
    }
}

fn mock_recipient_with_invalid_msg(addr: &str) -> Recipient {
    Recipient::new(addr, Some(encode_binary(b"invalid msg").unwrap()))
}

fn mock_recipient_with_valid_msg(addr: &str) -> Recipient {
    Recipient::new(
        addr,
        Some(to_json_binary(&mock_splitter_send_msg()).unwrap()),
    )
}
