use std::str::FromStr;

use andromeda_app::app::AppComponent;
use andromeda_app_contract::AppContract;
use andromeda_finance::splitter::{self, AddressPercent};
use andromeda_fungible_tokens::cw20::{self as andr_cw20, ExecuteMsgFns as Cw20ExecuteMsgFns};
use andromeda_non_fungible_tokens::{
    crowdfund::{
        self, CampaignConfig, CampaignStage, Cw20HookMsg, ExecuteMsgFns as CrowdfundExecuteMsgFns,
        PresaleTierOrder, SimpleTierOrder, Tier, TierMetaData,
    },
    cw721::{self, TokenExtension},
};
use andromeda_std::{
    amp::{AndrAddr, Recipient},
    common::{denom::Asset, expiration::Expiry, Milliseconds},
    os::adodb::ExecuteMsgFns as AdodbExecuteMsgFns,
};
use andromeda_testing_e2e::{
    faucet::fund,
    mock::{mock_app, MockAndromeda},
};
use cosmwasm_std::{coin, to_json_binary, Decimal, Uint128, Uint64};
use cw20::{Cw20Coin, MinterResponse};
use cw_orch::prelude::*;
use cw_orch_daemon::{DaemonBase, TxSender, Wallet};
use e2e::constants::{
    LOCAL_TERRA, LOCAL_WASM, PURCHASER_MNEMONIC_1, RECIPIENT_MNEMONIC_1, RECIPIENT_MNEMONIC_2,
    USER_MNEMONIC,
};

use andromeda_crowdfund::CrowdfundContract;
use andromeda_cw20::CW20Contract;
use andromeda_cw721::CW721Contract;
use andromeda_splitter::SplitterContract;

use andromeda_app::app;

use rstest::{fixture, rstest};

struct TestCase {
    daemon: DaemonBase<Wallet>,
    crowdfund_contract: CrowdfundContract<DaemonBase<Wallet>>,
    cw20_contract: CW20Contract<DaemonBase<Wallet>>,
    cw721_contract: CW721Contract<DaemonBase<Wallet>>,
    splitter_contract: SplitterContract<DaemonBase<Wallet>>,
    presale: Vec<PresaleTierOrder>,
}

#[fixture]
fn setup(
    #[default(true)] use_native_token: bool,
    #[default(LOCAL_TERRA)] chain_info: ChainInfo,
    #[default(100000000u128)] purchaser_balance: u128,
) -> TestCase {
    let daemon = mock_app(chain_info.clone(), USER_MNEMONIC);
    let mock_andromeda = MockAndromeda::new(&daemon);

    // Preparing contracts
    let app_contract = AppContract::new(daemon.clone());
    app_contract.upload().unwrap();
    mock_andromeda
        .adodb_contract
        .clone()
        .publish(
            "app-contract".to_string(),
            app_contract.code_id().unwrap(),
            "0.1.0".to_string(),
            None,
            None,
        )
        .unwrap();

    let cw20_contract = CW20Contract::new(daemon.clone());
    cw20_contract.upload().unwrap();
    mock_andromeda
        .adodb_contract
        .clone()
        .publish(
            "cw20".to_string(),
            cw20_contract.code_id().unwrap(),
            "0.1.0".to_string(),
            None,
            None,
        )
        .unwrap();

    let cw721_contract = CW721Contract::new(daemon.clone());
    cw721_contract.upload().unwrap();
    mock_andromeda
        .adodb_contract
        .clone()
        .publish(
            "cw721".to_string(),
            cw721_contract.code_id().unwrap(),
            "0.1.0".to_string(),
            None,
            None,
        )
        .unwrap();

    let crowdfund_contract = CrowdfundContract::new(daemon.clone());
    crowdfund_contract.upload().unwrap();
    mock_andromeda
        .adodb_contract
        .clone()
        .publish(
            "crowdfund".to_string(),
            crowdfund_contract.code_id().unwrap(),
            "0.1.0".to_string(),
            None,
            None,
        )
        .unwrap();

    let splitter_contract = SplitterContract::new(daemon.clone());
    splitter_contract.upload().unwrap();
    mock_andromeda
        .adodb_contract
        .clone()
        .publish(
            "splitter".to_string(),
            splitter_contract.code_id().unwrap(),
            "0.1.0".to_string(),
            None,
            None,
        )
        .unwrap();

    // Prepare App Components
    let recipient_1_daemon = daemon
        .rebuild()
        .mnemonic(RECIPIENT_MNEMONIC_1)
        .build()
        .unwrap();
    let recipient_2_daemon = daemon
        .rebuild()
        .mnemonic(RECIPIENT_MNEMONIC_2)
        .build()
        .unwrap();

    let purchaser_1_daemon = daemon
        .rebuild()
        .mnemonic(PURCHASER_MNEMONIC_1)
        .build()
        .unwrap();
    fund(&purchaser_1_daemon, &chain_info, purchaser_balance);

    let recipients = vec![
        AddressPercent {
            recipient: Recipient::from_string(recipient_1_daemon.sender().address().to_string()),
            percent: Decimal::from_str("0.2").unwrap(),
        },
        AddressPercent {
            recipient: Recipient::from_string(recipient_2_daemon.sender().address().to_string()),
            percent: Decimal::from_str("0.8").unwrap(),
        },
    ];

    let kernel_address = mock_andromeda.kernel_contract.addr_str().unwrap();
    let splitter_init_msg = splitter::InstantiateMsg {
        recipients,
        lock_time: None,
        kernel_address: kernel_address.clone(),
        owner: None,
        default_recipient: None,
    };

    let splitter_component = AppComponent::new(
        "splitter".to_string(),
        "splitter".to_string(),
        to_json_binary(&splitter_init_msg).unwrap(),
    );

    let cw721_init_msg = cw721::InstantiateMsg {
        name: "Campaign Tier".to_string(),
        symbol: "CT".to_string(),
        minter: AndrAddr::from_string("./crowdfund".to_string()),
        kernel_address: kernel_address.clone(),
        owner: None,
    };
    let cw721_component = AppComponent::new(
        "cw721".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    let initial_balances = vec![Cw20Coin {
        address: purchaser_1_daemon.sender().address().to_string(),
        amount: Uint128::from(purchaser_balance),
    }];

    let mint = Some(MinterResponse {
        minter: daemon.sender().address().to_string(),
        cap: Some(Uint128::from(10000000000u128)),
    });
    let mut app_components = vec![splitter_component.clone(), cw721_component.clone()];
    let cw20_component: Option<AppComponent> = match use_native_token {
        true => None,
        false => {
            let cw20_init_msg = andr_cw20::InstantiateMsg {
                name: "Test Tokens".to_string(),
                symbol: "TTT".to_string(),
                decimals: 6,
                marketing: None,
                mint,
                initial_balances,
                kernel_address: kernel_address.clone(),
                owner: None,
            };
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
        true => Asset::NativeToken(chain_info.gas_denom.to_string()),
        false => Asset::Cw20Token(AndrAddr::from_string(format!(
            "./{}",
            cw20_component.clone().unwrap().name
        ))),
    };

    let withdrawal_recipient = Recipient::new(
        format!("./{}", splitter_component.name),
        Some(to_json_binary(&splitter::ExecuteMsg::Send { config: None }).unwrap()),
    );

    let campaign_config = CampaignConfig {
        title: Some("Demo Crowdfund".to_string()),
        description: Some("Crowdfund For Internal Testing".to_string()),
        banner: Some("http://<campaign_banner>".to_string()),
        url: Some("http://<campaign_url>".to_string()),
        token_address: AndrAddr::from_string(format!("./{}", cw721_component.name)),
        denom,
        withdrawal_recipient,
        soft_cap: Some(Uint128::new(100000)),
        hard_cap: None,
    };

    let crowdfund_init_msg = crowdfund::InstantiateMsg {
        campaign_config,
        kernel_address: kernel_address.clone(),
        owner: None,
        tiers: vec![],
    };

    let crowdfund_component = AppComponent::new(
        "crowdfund".to_string(),
        "crowdfund".to_string(),
        to_json_binary(&crowdfund_init_msg).unwrap(),
    );
    app_components.push(crowdfund_component.clone());

    app_contract
        .instantiate(
            &app::InstantiateMsg {
                app_components,
                name: "Crowdfund App".to_string(),
                chain_info: None,
                kernel_address: kernel_address.clone(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    let crowdfund_addr = app_contract.get_address(crowdfund_component.name);
    crowdfund_contract.set_address(&Addr::unchecked(crowdfund_addr));

    let cw721_addr = app_contract.get_address(cw721_component.name);
    cw721_contract.set_address(&Addr::unchecked(cw721_addr));

    let splitter_addr = app_contract.get_address(splitter_component.name);
    splitter_contract.set_address(&Addr::unchecked(splitter_addr));

    if !use_native_token {
        let cw20_addr = app_contract.get_address(cw20_component.unwrap().name);
        cw20_contract.set_address(&Addr::unchecked(cw20_addr));
    }

    let meta_data = TierMetaData {
        token_uri: None,
        extension: TokenExtension {
            ..Default::default()
        },
    };
    crowdfund_contract
        .add_tier(Tier {
            label: "Tier 1".to_string(),
            level: Uint64::one(),
            price: Uint128::new(10000),
            limit: None,
            metadata: meta_data.clone(),
        })
        .unwrap();

    crowdfund_contract
        .add_tier(Tier {
            label: "Tier 2".to_string(),
            level: Uint64::new(2u64),
            price: Uint128::new(20000),
            limit: Some(Uint128::new(100)),
            metadata: meta_data,
        })
        .unwrap();

    let presale = vec![PresaleTierOrder {
        level: Uint64::one(),
        amount: Uint128::new(10u128),
        orderer: Addr::unchecked(purchaser_1_daemon.sender().address()),
    }];

    TestCase {
        daemon,
        crowdfund_contract,
        cw20_contract,
        cw721_contract,
        splitter_contract,
        presale,
    }
}

#[rstest]
fn test_successful_crowdfund_app_native(#[with(true, LOCAL_WASM)] setup: TestCase) {
    let TestCase {
        daemon,
        mut crowdfund_contract,
        presale,
        cw721_contract,
        ..
    } = setup;
    let recipient_1_daemon = daemon
        .rebuild()
        .mnemonic(RECIPIENT_MNEMONIC_1)
        .build()
        .unwrap();
    let recipient_1_balance = daemon
        .balance(
            recipient_1_daemon.sender_addr(),
            Some(LOCAL_WASM.gas_denom.to_string()),
        )
        .unwrap()[0]
        .amount;

    let recipient_2_daemon = daemon
        .rebuild()
        .mnemonic(RECIPIENT_MNEMONIC_2)
        .build()
        .unwrap();
    let recipient_2_balance = daemon
        .balance(
            recipient_2_daemon.sender_addr(),
            Some(LOCAL_WASM.gas_denom.to_string()),
        )
        .unwrap()[0]
        .amount;

    let start_time = None;
    let end_time = Milliseconds::from_nanos(daemon.block_info().unwrap().time.plus_days(1).nanos());
    let end_time = Expiry::AtTime(end_time);

    crowdfund_contract
        .start_campaign(end_time, Some(presale), start_time)
        .unwrap();

    let summary = crowdfund_contract.campaign_summary();
    assert_eq!(summary.current_capital, 0);
    assert_eq!(summary.current_stage, CampaignStage::ONGOING.to_string());

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

    let purchaser_1_daemon = daemon
        .rebuild()
        .mnemonic(PURCHASER_MNEMONIC_1)
        .build()
        .unwrap();
    crowdfund_contract.set_sender(purchaser_1_daemon.sender());
    let funds = vec![coin(500000, LOCAL_WASM.gas_denom)];
    crowdfund_contract.purchase_tiers(orders, &funds).unwrap();

    crowdfund_contract.set_sender(daemon.sender());
    crowdfund_contract.end_campaign().unwrap();

    let summary = crowdfund_contract.campaign_summary();
    assert_eq!(summary.current_capital, 10 * 10000 + 20000 * 10);
    assert_eq!(summary.current_stage, CampaignStage::SUCCESS.to_string());

    let recipient_1_change = daemon
        .balance(
            recipient_1_daemon.sender_addr(),
            Some(LOCAL_WASM.gas_denom.to_string()),
        )
        .unwrap()[0]
        .amount
        - recipient_1_balance;

    let recipient_2_change = daemon
        .balance(
            recipient_2_daemon.sender_addr(),
            Some(LOCAL_WASM.gas_denom.to_string()),
        )
        .unwrap()[0]
        .amount
        - recipient_2_balance;

    assert_eq!(recipient_1_change.u128(), summary.current_capital / 5);
    assert_eq!(recipient_2_change.u128(), summary.current_capital * 4 / 5);

    crowdfund_contract.set_sender(purchaser_1_daemon.sender());
    crowdfund_contract.claim().unwrap();

    let owner_resp = cw721_contract.owner_of("0".to_string()).owner;
    assert_eq!(owner_resp, purchaser_1_daemon.sender_addr().into_string());

    let owner_resp = cw721_contract.owner_of("29".to_string()).owner;
    assert_eq!(owner_resp, purchaser_1_daemon.sender_addr().into_string());
}

#[rstest]
fn test_successful_crowdfund_app_cw20(#[with(false)] setup: TestCase) {
    let TestCase {
        daemon,
        mut crowdfund_contract,
        presale,
        mut cw20_contract,
        cw721_contract,
        splitter_contract,
        ..
    } = setup;

    let start_time = None;
    let end_time = Milliseconds::from_nanos(daemon.block_info().unwrap().time.plus_days(1).nanos());
    let end_time = Expiry::AtTime(end_time);

    crowdfund_contract
        .start_campaign(end_time, Some(presale), start_time)
        .unwrap();

    let summary = crowdfund_contract.campaign_summary();
    assert_eq!(summary.current_capital, 0);
    assert_eq!(summary.current_stage, CampaignStage::ONGOING.to_string());

    let recipient_balance = cw20_contract
        .balance(splitter_contract.addr_str().unwrap())
        .balance;

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

    let hook_msg = to_json_binary(&Cw20HookMsg::PurchaseTiers { orders }).unwrap();

    let purchaser_1_daemon = daemon
        .rebuild()
        .mnemonic(PURCHASER_MNEMONIC_1)
        .build()
        .unwrap();

    let purchaser_1_balance = cw20_contract
        .balance(purchaser_1_daemon.sender_addr())
        .balance;

    cw20_contract.set_sender(purchaser_1_daemon.sender());
    cw20_contract
        .send(
            Uint128::new(500000),
            AndrAddr::from_string(crowdfund_contract.addr_str().unwrap()),
            hook_msg,
        )
        .unwrap();
    cw20_contract.set_sender(daemon.sender());

    let purchaser_1_change = purchaser_1_balance
        - cw20_contract
            .balance(purchaser_1_daemon.sender_addr())
            .balance;

    assert_eq!(purchaser_1_change, Uint128::new(10 * 10000 + 20000 * 10));

    crowdfund_contract.end_campaign().unwrap();

    let summary = crowdfund_contract.campaign_summary();
    assert_eq!(summary.current_capital, 10 * 10000 + 20000 * 10);
    assert_eq!(summary.current_stage, CampaignStage::SUCCESS.to_string());

    // Splitter is only working for native token, not for cw20 token
    let recipient_change = cw20_contract
        .balance(splitter_contract.addr_str().unwrap())
        .balance
        - recipient_balance;

    assert_eq!(recipient_change.u128(), summary.current_capital);

    crowdfund_contract.set_sender(purchaser_1_daemon.sender());
    crowdfund_contract.claim().unwrap();

    let owner_resp = cw721_contract.owner_of("0".to_string()).owner;
    assert_eq!(owner_resp, purchaser_1_daemon.sender_addr().into_string());

    let owner_resp = cw721_contract.owner_of("29".to_string()).owner;
    assert_eq!(owner_resp, purchaser_1_daemon.sender_addr().into_string());
}
