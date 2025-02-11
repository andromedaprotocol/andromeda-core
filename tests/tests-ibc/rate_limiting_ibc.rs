#![cfg(not(target_arch = "wasm32"))]

use andromeda_app::app::{AppComponent, InstantiateMsg as AppInstantiateMsg};
use andromeda_app_contract::AppContract;
use andromeda_auction::AuctionContract;
use andromeda_cw721::CW721Contract;
use andromeda_finance::rate_limiting_withdrawals::{
    CoinAndLimit, InstantiateMsg, MinimumFrequency,
};
use andromeda_non_fungible_tokens::{
    auction::InstantiateMsg as AuctionInstantiateMsg,
    cw721::InstantiateMsg as CW721InstantiateMsg,
};
use andromeda_rate_limiting_withdrawals::RateLimitingWithdrawalsContract;
use andromeda_std::{
    amp::AndrAddr,
    common::Milliseconds,
    os,
};
use andromeda_testing::{
    ado_deployer,
    interchain::InterchainChain,
    register_ado, InterchainTestEnv,
};
use cosmwasm_std::{to_json_binary, Uint128};
use cw_orch::mock::cw_multi_test::MockApiBech32;
use cw_orch::mock::MockBase;
use cw_orch::prelude::*;
use rstest::*;

pub struct ChainMap<'a> {
    pub chains: Vec<(&'a InterchainChain, &'a InterchainChain)>,
}
ado_deployer!(
    deploy_app,
    AppContract<MockBase<MockApiBech32>>,
    &AppInstantiateMsg
);

ado_deployer!(
    deploy_auction,
    AuctionContract<MockBase>,
    &AuctionInstantiateMsg
);

ado_deployer!(deploy_cw721, CW721Contract<MockBase>, &CW721InstantiateMsg);

ado_deployer!(
    deploy_rate_limiting,
    RateLimitingWithdrawalsContract<MockBase>,
    &InstantiateMsg
);

#[rstest]
#[case::osmosis_to_juno("osmosis", "juno")]
#[case::juno_to_osmosis("juno", "osmosis")]
#[case::andromeda_to_juno("andromeda", "juno")]
fn test_rate_limiting_withdrawals_ibc(#[case] chain1_name: &str, #[case] chain2_name: &str) {
    let InterchainTestEnv {
        juno,
        osmosis,
        andromeda,
        ..
    } = InterchainTestEnv::new();
    let chains = [
        ("juno", &juno),
        ("osmosis", &osmosis),
        ("andromeda", &andromeda),
    ]
    .into_iter()
    .collect::<std::collections::HashMap<_, _>>();

    let chain1 = chains.get(chain1_name).unwrap();
    let _chain2 = chains.get(chain2_name).unwrap();

    // Upload all contracts first
    let auction = AuctionContract::new(chain1.chain.clone());
    auction.upload().unwrap();
    let cw721 = CW721Contract::new(chain1.chain.clone());
    cw721.upload().unwrap();
    let rate_limiting = RateLimitingWithdrawalsContract::new(chain1.chain.clone());
    rate_limiting.upload().unwrap();
    let app = AppContract::new(chain1.chain.clone());
    app.upload().unwrap();

    // Register all contracts
    register_ado!(chain1, auction, "auction");
    register_ado!(chain1, cw721, "cw721");
    register_ado!(chain1, rate_limiting, "rate-limiting-withdrawals");
    register_ado!(chain1, app, "app-contract");

    // Deploy the app with all components
    let _app_contract = deploy_app!(
        app,
        &AppInstantiateMsg {
            app_components: vec![
                AppComponent::new(
                    "cw721",
                    "cw721",
                    to_json_binary(&CW721InstantiateMsg {
                        minter: AndrAddr::from_string(chain1.addresses[0].clone()),
                        name: "Test".to_string(),
                        symbol: "TEST".to_string(),
                        kernel_address: chain1.aos.kernel.address().unwrap().into_string(),
                        owner: None
                    })
                    .unwrap()
                ),
                AppComponent::new(
                    "auction",
                    "auction",
                    to_json_binary(&AuctionInstantiateMsg {
                        authorized_token_addresses: None,
                        authorized_cw20_addresses: None,
                        kernel_address: chain1.aos.kernel.address().unwrap().into_string(),
                        owner: None,
                    })
                    .unwrap()
                ),
                AppComponent::new(
                    "rate-limiting",
                    "rate-limiting-withdrawals",
                    to_json_binary(&InstantiateMsg {
                        allowed_coin: CoinAndLimit {
                            coin: chain1.denom.clone(),
                            limit: Uint128::new(100),
                        },
                        minimal_withdrawal_frequency: MinimumFrequency::Time {
                            time: Milliseconds::from_seconds(1),
                        },
                        kernel_address: chain1.aos.kernel.address().unwrap().into_string(),
                        owner: None,
                    })
                    .unwrap()
                )
            ],
            name: "test_app".to_string(),
            chain_info: None,
            kernel_address: chain1.aos.kernel.address().unwrap().into_string(),
            owner: None
        },
        "app"
    );
}
