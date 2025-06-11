use andromeda_app_contract::AppContract;
use andromeda_socket::astroport::{
    AssetEntry, AssetInfo, ExecuteMsg as SocketAstroportExecuteMsg, InstantiateMsg, PairType,
};

use andromeda_cw20::CW20Contract;
use andromeda_fungible_tokens::cw20::ExecuteMsg as Cw20ExecuteMsg;
use andromeda_std::amp::AndrAddr;
use cosmwasm_std::{coin, Uint128};
use cw_orch::prelude::*;
use cw_orch_daemon::{Daemon, DaemonBase, TxSender, Wallet};
use e2e::constants::PION_1;

use andromeda_socket_astroport::SocketAstroportContract;

use rstest::{fixture, rstest};

struct TestCase {
    cw20_contract: CW20Contract<DaemonBase<Wallet>>,
    socket_astroport_contract: SocketAstroportContract<DaemonBase<Wallet>>,
    daemon: DaemonBase<Wallet>,
}

const TEST_MNEMONIC: &str = "cereal gossip fox peace youth leader engage move brass sell gas trap issue simple dance source develop black hurt pulp burst predict patient onion";

#[fixture]
fn setup(
    #[default(11806)] app_code_id: u64,
    #[default("neutron1p3gh4zanng04zdnvs8cdh2kdsjrcp43qkp9zk32enu9waxv4wrmqpqnl9g")]
    kernel_address: String,
    #[default("neutron1jj0scx400pswhpjes589aujlqagxgcztw04srynmhf0f6zplzn2qqmhwj7")]
    factory_address: String,
) -> TestCase {
    let daemon = Daemon::builder(PION_1)
        .mnemonic(TEST_MNEMONIC)
        .build()
        .unwrap();
    let app_contract = AppContract::new(daemon.clone());
    app_contract.set_code_id(app_code_id);

    let kernel_address = kernel_address.to_string();

    let kernel = andromeda_kernel::KernelContract::new(daemon.clone());
    kernel.set_address(&Addr::unchecked(kernel_address.clone()));

    // Add CW20 component for creating native CW20 tokens on Neutron
    let cw20_init_msg = andromeda_fungible_tokens::cw20::InstantiateMsg {
        name: "TestTokensss".to_string(),
        symbol: "Test".to_string(),
        decimals: 6,
        initial_balances: vec![cw20::Cw20Coin {
            address: daemon.sender().address().to_string(),
            amount: Uint128::new(1_000_000_000), // 1000 tokens with 6 decimals
        }],
        mint: Some(cw20::MinterResponse {
            minter: daemon.sender().address().to_string(),
            cap: Some(Uint128::new(10_000_000_000)), // 10k token cap
        }),
        marketing: None,
        kernel_address: kernel_address.to_string(),
        owner: Some(daemon.sender().address().to_string()),
    };

    let cw20_contract = CW20Contract::new(daemon.clone());
    cw20_contract.upload().unwrap();
    cw20_contract
        .instantiate(&cw20_init_msg, None, &[])
        .unwrap();

    let socket_astroport_contract = SocketAstroportContract::new(daemon.clone());
    socket_astroport_contract.upload().unwrap();

    socket_astroport_contract
        .instantiate(
            &InstantiateMsg {
                kernel_address: kernel_address.to_string(),
                owner: None,
                swap_router: None,
                factory: Some(AndrAddr::from_string(factory_address.clone())),
            },
            None,
            &[],
        )
        .unwrap();

    TestCase {
        cw20_contract,
        socket_astroport_contract,
        daemon,
    }
}

#[rstest]
fn test_create_pool_and_provide_liquidity_and_withdraw(setup: TestCase) {
    let TestCase {
        cw20_contract,
        socket_astroport_contract,
        daemon,
        ..
    } = setup;

    // Use the newly instantiated contracts - no manual address overrides needed
    println!(
        "CW20 contract address: {}",
        cw20_contract.address().unwrap()
    );
    println!(
        "Socket contract address: {}",
        socket_astroport_contract.address().unwrap()
    );

    // Use much larger amounts - Astroport pools often have minimum requirements
    let cw20_amount = Uint128::new(50000); // 0.05 tokens (with 6 decimals)
    let native_amount = Uint128::new(1_000_000); // 1 untrn (with 6 decimals)

    println!(
        "Attempting to provide liquidity with {} CW20 tokens and {} untrn",
        cw20_amount, native_amount
    );

    let msg = Cw20ExecuteMsg::IncreaseAllowance {
        spender: socket_astroport_contract.address().unwrap().to_string(),
        amount: cw20_amount,
        expires: None,
    };
    cw20_contract.execute(&msg, &[]).unwrap();

    let msg = SocketAstroportExecuteMsg::CreatePairAndProvideLiquidity {
        pair_type: PairType::Xyk {},
        asset_infos: vec![
            AssetInfo::Token {
                contract_addr: cw20_contract.address().unwrap(),
            },
            AssetInfo::NativeToken {
                denom: "untrn".to_string(),
            },
        ],
        init_params: None,
        assets: vec![
            AssetEntry {
                info: AssetInfo::Token {
                    contract_addr: cw20_contract.address().unwrap(),
                },
                amount: cw20_amount,
            },
            AssetEntry {
                info: AssetInfo::NativeToken {
                    denom: "untrn".to_string(),
                },
                amount: native_amount,
            },
        ],
        slippage_tolerance: None,
        auto_stake: None,
        receiver: Some(daemon.sender().address().to_string()),
    };

    let res = socket_astroport_contract
        .execute(&msg, &[coin(native_amount.u128(), "untrn")])
        .unwrap();

    println!("Transaction successful! Result: {:?}", res);
}
