use andromeda_non_fungible_tokens::{
    crowdfund::{
        CampaignConfig, CampaignStage, ExecuteMsg, InstantiateMsg, Tier, TierMetaData, TierOrder,
    },
    cw721::TokenExtension,
};

use andromeda_std::{
    common::{reply::ReplyId, MillisecondsExpiration},
    error::ContractError,
    os::economics::ExecuteMsg as EconomicsExecuteMsg,
    testing::mock_querier::{MOCK_ADO_PUBLISHER, MOCK_KERNEL_CONTRACT},
};
use cosmwasm_std::{
    testing::{mock_env, mock_info},
    to_json_binary, Addr, CosmosMsg, DepsMut, Env, Order, Response, Storage, SubMsg, Uint128,
    Uint64, WasmMsg,
};

use crate::{
    contract::{execute, instantiate},
    state::{CAMPAIGN_CONFIG, CAMPAIGN_STAGE, CURRENT_CAP, TIERS, TIER_ORDERS},
    testing::mock_querier::{mock_dependencies_custom, mock_zero_price_tier, MOCK_DEFAULT_LIMIT},
};

use super::mock_querier::{mock_campaign_config, mock_campaign_tiers, MOCK_DEFAULT_OWNER};

fn init(deps: DepsMut, config: CampaignConfig, tiers: Vec<Tier>) -> Response {
    let msg = InstantiateMsg {
        campaign_config: config,
        tiers,
        owner: None,
        modules: None,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
    };

    let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
    instantiate(deps, mock_env(), info, msg).unwrap()
}

fn get_campaign_config(storage: &dyn Storage) -> CampaignConfig {
    CAMPAIGN_CONFIG.load(storage).unwrap()
}

fn get_tiers(storage: &dyn Storage) -> Vec<Tier> {
    TIERS
        .range_raw(storage, None, None, Order::Ascending)
        .map(|res| res.unwrap().1)
        .collect()
}

fn future_time(env: &Env) -> MillisecondsExpiration {
    MillisecondsExpiration::from_seconds(env.block.time.seconds() + 5000)
}
fn past_time() -> MillisecondsExpiration {
    MillisecondsExpiration::from_seconds(0) // Past timestamp
}
fn set_campaign_stage(store: &mut dyn Storage, stage: &CampaignStage) {
    CAMPAIGN_STAGE.save(store, stage).unwrap();
}
fn set_current_cap(store: &mut dyn Storage, cur_cap: &Uint128) {
    CURRENT_CAP.save(store, cur_cap).unwrap();
}
fn set_campaign_config(store: &mut dyn Storage, config: &CampaignConfig) {
    CAMPAIGN_CONFIG.save(store, config).unwrap();
}

#[cfg(test)]
mod test {
    use andromeda_non_fungible_tokens::crowdfund::{Cw20HookMsg, SimpleTierOrder};
    use andromeda_std::{
        amp::AndrAddr,
        common::{denom::Asset, encode_binary},
        testing::mock_querier::MOCK_CW20_CONTRACT,
    };
    use cosmwasm_std::{coin, coins, wasm_execute, BankMsg, Coin};
    use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

    use crate::{
        state::{get_current_cap, set_tiers},
        testing::mock_querier::{MOCK_DEFAULT_OWNER, MOCK_WITHDRAWAL_ADDRESS},
    };

    use super::*;

    const MOCK_NATIVE_DENOM: &str = "uandr";
    const INVA1LID_DENOM: &str = "other";

    struct InstantiateTestCase {
        name: String,
        config: CampaignConfig,
        tiers: Vec<Tier>,
        expected_res: Result<Response, ContractError>,
    }
    #[test]
    fn test_instantiate() {
        let test_cases: Vec<InstantiateTestCase> = vec![
            InstantiateTestCase {
                name: "instantiate with native token".to_string(),
                config: mock_campaign_config(Asset::NativeToken(MOCK_NATIVE_DENOM.to_string())),
                tiers: mock_campaign_tiers(),
                expected_res: Ok(Response::new()
                    .add_attribute("method", "instantiate")
                    .add_attribute("type", "crowdfund")
                    .add_attribute("kernel_address", MOCK_KERNEL_CONTRACT)
                    .add_attribute(MOCK_DEFAULT_OWNER, MOCK_DEFAULT_OWNER)),
            },
            InstantiateTestCase {
                name: "instantiate with invalid native token".to_string(),
                config: mock_campaign_config(Asset::NativeToken(INVA1LID_DENOM.to_string())),
                tiers: mock_campaign_tiers(),
                expected_res: Err(ContractError::InvalidAsset {
                    asset: Asset::NativeToken(INVA1LID_DENOM.to_string()).to_string(),
                }),
            },
            InstantiateTestCase {
                name: "instantiate with cw20".to_string(),
                config: mock_campaign_config(Asset::Cw20Token(AndrAddr::from_string(
                    MOCK_CW20_CONTRACT.to_string(),
                ))),
                tiers: mock_campaign_tiers(),
                expected_res: Ok(Response::new()
                    .add_attribute("method", "instantiate")
                    .add_attribute("type", "crowdfund")
                    .add_attribute("kernel_address", MOCK_KERNEL_CONTRACT)
                    .add_attribute(MOCK_DEFAULT_OWNER, MOCK_DEFAULT_OWNER)),
            },
            InstantiateTestCase {
                name: "instantiate with invalid cw20".to_string(),
                config: mock_campaign_config(Asset::Cw20Token(AndrAddr::from_string(
                    "cw20_contract1".to_string(),
                ))),
                tiers: mock_campaign_tiers(),
                expected_res: Err(ContractError::InvalidAsset {
                    asset: Asset::Cw20Token(AndrAddr::from_string("cw20_contract1".to_string()))
                        .to_string(),
                }),
            },
            InstantiateTestCase {
                name: "instantiate with invalid tiers including zero price tier".to_string(),
                config: mock_campaign_config(Asset::NativeToken(MOCK_NATIVE_DENOM.to_string())),
                tiers: vec![mock_zero_price_tier(Uint64::zero())],
                expected_res: Err(ContractError::InvalidTier {
                    operation: "all".to_string(),
                    msg: "Price can not be zero".to_string(),
                }),
            },
        ];

        for test in test_cases {
            let mut deps = mock_dependencies_custom(&[coin(100000, MOCK_NATIVE_DENOM)]);
            let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
            let msg = InstantiateMsg {
                campaign_config: test.config.clone(),
                tiers: test.tiers.clone(),
                owner: None,
                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                modules: None,
            };
            let res = instantiate(deps.as_mut(), mock_env(), info, msg);

            assert_eq!(res, test.expected_res, "Test case: {}", test.name);
            if res.is_ok() {
                assert_eq!(
                    get_campaign_config(&deps.storage),
                    test.config,
                    "Test case: {}",
                    test.name
                );
                assert_eq!(
                    get_tiers(deps.as_ref().storage),
                    test.tiers,
                    "Test case: {}",
                    test.name
                );
            }
        }
    }

    struct TierTestCase {
        name: String,
        tier: Tier,
        expected_res: Result<Response, ContractError>,
        payee: String,
    }

    #[test]
    fn test_add_tier() {
        let valid_tier = Tier {
            level: Uint64::new(2u64),
            label: "Tier 2".to_string(),
            limit: Some(Uint128::new(100)),
            sold_amount: Uint128::zero(),
            price: Uint128::new(100),
            meta_data: TierMetaData {
                extension: TokenExtension {
                    publisher: MOCK_ADO_PUBLISHER.to_string(),
                },
                token_uri: None,
            },
        };
        let duplicated_tier = Tier {
            level: Uint64::new(0u64),
            label: "Tier 2".to_string(),
            limit: Some(Uint128::new(100)),
            sold_amount: Uint128::zero(),
            price: Uint128::new(100),
            meta_data: TierMetaData {
                extension: TokenExtension {
                    publisher: MOCK_ADO_PUBLISHER.to_string(),
                },
                token_uri: None,
            },
        };

        let test_cases: Vec<TierTestCase> = vec![
            TierTestCase {
                name: "standard add_tier".to_string(),
                tier: valid_tier.clone(),
                payee: MOCK_DEFAULT_OWNER.to_string(),
                expected_res: Ok(Response::new()
                    .add_attribute("action", "add_tier")
                    .add_attribute("level", valid_tier.level.to_string())
                    .add_attribute("label", valid_tier.label.clone())
                    .add_attribute("price", valid_tier.price.to_string())
                    .add_attribute("limit", valid_tier.limit.unwrap().to_string())
                    // Economics message
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER),
                                action: "AddTier".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
            },
            TierTestCase {
                name: "add_tier with unauthorized sender".to_string(),
                tier: valid_tier.clone(),
                expected_res: Err(ContractError::Unauthorized {}),
                payee: "owner1".to_string(),
            },
            TierTestCase {
                name: "add_tier with zero price tier".to_string(),
                tier: mock_zero_price_tier(Uint64::new(2)),
                expected_res: Err(ContractError::InvalidTier {
                    operation: "all".to_string(),
                    msg: "Price can not be zero".to_string(),
                }),
                payee: MOCK_DEFAULT_OWNER.to_string(),
            },
            TierTestCase {
                name: "add_tier with duplicated tier".to_string(),
                tier: duplicated_tier,
                expected_res: Err(ContractError::InvalidTier {
                    operation: "add".to_string(),
                    msg: "Tier with level 0 already exist".to_string(),
                }),
                payee: MOCK_DEFAULT_OWNER.to_string(),
            },
        ];
        for test in test_cases {
            let mut deps = mock_dependencies_custom(&[coin(100000, MOCK_NATIVE_DENOM)]);
            let _ = init(
                deps.as_mut(),
                mock_campaign_config(Asset::NativeToken(MOCK_NATIVE_DENOM.to_string())),
                mock_campaign_tiers(),
            );

            let info = mock_info(&test.payee, &[]);

            let msg = ExecuteMsg::AddTier {
                tier: test.tier.clone(),
            };

            let res = execute(deps.as_mut(), mock_env(), info, msg);
            assert_eq!(res, test.expected_res, "Test case: {}", test.name);
            if res.is_ok() {
                assert_eq!(
                    test.tier,
                    TIERS
                        .load(deps.as_ref().storage, test.tier.level.into())
                        .unwrap(),
                    "Test case: {}",
                    test.name
                );
            }
        }
    }

    #[test]
    fn test_update_tier() {
        let valid_tier = Tier {
            level: Uint64::zero(),
            label: "Tier 0".to_string(),
            limit: Some(Uint128::new(100)),
            sold_amount: Uint128::zero(),
            price: Uint128::new(100),
            meta_data: TierMetaData {
                extension: TokenExtension {
                    publisher: MOCK_ADO_PUBLISHER.to_string(),
                },
                token_uri: None,
            },
        };
        let non_existing_tier = Tier {
            level: Uint64::new(2u64),
            label: "Tier 2".to_string(),
            limit: Some(Uint128::new(100)),
            sold_amount: Uint128::zero(),
            price: Uint128::new(100),
            meta_data: TierMetaData {
                extension: TokenExtension {
                    publisher: MOCK_ADO_PUBLISHER.to_string(),
                },
                token_uri: None,
            },
        };

        let test_cases: Vec<TierTestCase> = vec![
            TierTestCase {
                name: "standard update_tier".to_string(),
                tier: valid_tier.clone(),
                payee: MOCK_DEFAULT_OWNER.to_string(),
                expected_res: Ok(Response::new()
                    .add_attribute("action", "update_tier")
                    .add_attribute("level", valid_tier.level.to_string())
                    .add_attribute("label", valid_tier.label.clone())
                    .add_attribute("price", valid_tier.price.to_string())
                    .add_attribute("limit", valid_tier.limit.unwrap().to_string())
                    // Economics message
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER),
                                action: "UpdateTier".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
            },
            TierTestCase {
                name: "update_tier with unauthorized sender".to_string(),
                tier: valid_tier.clone(),
                expected_res: Err(ContractError::Unauthorized {}),
                payee: "owner1".to_string(),
            },
            TierTestCase {
                name: "update_tier with zero price tier".to_string(),
                tier: mock_zero_price_tier(Uint64::zero()),
                expected_res: Err(ContractError::InvalidTier {
                    operation: "all".to_string(),
                    msg: "Price can not be zero".to_string(),
                }),
                payee: MOCK_DEFAULT_OWNER.to_string(),
            },
            TierTestCase {
                name: "update_tier with non existing tier".to_string(),
                tier: non_existing_tier,
                expected_res: Err(ContractError::InvalidTier {
                    operation: "update".to_string(),
                    msg: "Tier with level 2 does not exist".to_string(),
                }),
                payee: MOCK_DEFAULT_OWNER.to_string(),
            },
        ];
        for test in test_cases {
            let mut deps = mock_dependencies_custom(&[coin(100000, MOCK_NATIVE_DENOM)]);
            let _ = init(
                deps.as_mut(),
                mock_campaign_config(Asset::NativeToken(MOCK_NATIVE_DENOM.to_string())),
                mock_campaign_tiers(),
            );

            let info = mock_info(&test.payee, &[]);

            let msg = ExecuteMsg::UpdateTier {
                tier: test.tier.clone(),
            };

            let res = execute(deps.as_mut(), mock_env(), info, msg);
            assert_eq!(res, test.expected_res, "Test case: {}", test.name);
            if res.is_ok() {
                assert_eq!(
                    test.tier,
                    TIERS
                        .load(deps.as_ref().storage, test.tier.level.into())
                        .unwrap(),
                    "Test case: {}",
                    test.name
                );
            }
        }
    }

    #[test]
    fn test_remove_tier() {
        let valid_tier = Tier {
            level: Uint64::zero(),
            label: "Tier 0".to_string(),
            limit: Some(Uint128::new(100)),
            price: Uint128::new(100),
            sold_amount: Uint128::zero(),
            meta_data: TierMetaData {
                extension: TokenExtension {
                    publisher: MOCK_ADO_PUBLISHER.to_string(),
                },
                token_uri: None,
            },
        };
        let non_existing_tier = Tier {
            level: Uint64::new(2u64),
            label: "Tier 2".to_string(),
            limit: Some(Uint128::new(100)),
            price: Uint128::new(100),
            sold_amount: Uint128::zero(),
            meta_data: TierMetaData {
                extension: TokenExtension {
                    publisher: MOCK_ADO_PUBLISHER.to_string(),
                },
                token_uri: None,
            },
        };

        let test_cases: Vec<TierTestCase> = vec![
            TierTestCase {
                name: "standard remove_tier".to_string(),
                tier: valid_tier.clone(),
                payee: MOCK_DEFAULT_OWNER.to_string(),
                expected_res: Ok(Response::new()
                    .add_attribute("action", "remove_tier")
                    .add_attribute("level", valid_tier.level.to_string())
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER),
                                action: "RemoveTier".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
            },
            TierTestCase {
                name: "remove_tier with unauthorized sender".to_string(),
                tier: valid_tier.clone(),
                expected_res: Err(ContractError::Unauthorized {}),
                payee: "owner1".to_string(),
            },
            TierTestCase {
                name: "remove_tier with non existing tier level".to_string(),
                tier: non_existing_tier,
                expected_res: Err(ContractError::InvalidTier {
                    operation: "remove".to_string(),
                    msg: "Tier with level 2 does not exist".to_string(),
                }),
                payee: MOCK_DEFAULT_OWNER.to_string(),
            },
        ];
        for test in test_cases {
            let mut deps = mock_dependencies_custom(&[coin(100000, MOCK_NATIVE_DENOM)]);
            let _ = init(
                deps.as_mut(),
                mock_campaign_config(Asset::NativeToken(MOCK_NATIVE_DENOM.to_string())),
                mock_campaign_tiers(),
            );

            let info = mock_info(&test.payee, &[]);

            let msg = ExecuteMsg::RemoveTier {
                level: test.tier.level,
            };

            let res = execute(deps.as_mut(), mock_env(), info, msg);
            assert_eq!(res, test.expected_res, "Test case: {}", test.name);
            if res.is_ok() {
                assert!(
                    !TIERS.has(deps.as_ref().storage, test.tier.level.into()),
                    "Test case: {}",
                    test.name
                );
            }
        }
    }

    struct StartCampaignTestCase {
        name: String,
        tiers: Vec<Tier>,
        presale: Option<Vec<TierOrder>>,
        start_time: Option<MillisecondsExpiration>,
        end_time: MillisecondsExpiration,
        expected_res: Result<Response, ContractError>,
        payee: String,
    }

    #[test]
    fn test_start_campaign() {
        let mock_orderer = Addr::unchecked("mock_orderer".to_string());
        let valid_presale = vec![TierOrder {
            amount: Uint128::new(100u128),
            level: Uint64::new(1u64),
            orderer: mock_orderer.clone(),
        }];

        let invalid_presale = vec![TierOrder {
            amount: Uint128::new(100u128),
            level: Uint64::new(2u64),
            orderer: mock_orderer.clone(),
        }];

        let invalid_tiers = vec![Tier {
            level: Uint64::new(1u64),
            label: "Tier 1".to_string(),
            limit: Some(Uint128::new(1000u128)),
            sold_amount: Uint128::zero(),
            price: Uint128::new(10u128),
            meta_data: TierMetaData {
                extension: TokenExtension {
                    publisher: MOCK_ADO_PUBLISHER.to_string(),
                },
                token_uri: None,
            },
        }];

        let env = mock_env();
        let test_cases: Vec<StartCampaignTestCase> = vec![
            StartCampaignTestCase {
                name: "standard start_campaign".to_string(),
                tiers: mock_campaign_tiers(),
                presale: Some(valid_presale.clone()),
                start_time: None,
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds() + 100),
                payee: MOCK_DEFAULT_OWNER.to_string(),
                expected_res: Ok(Response::new()
                    .add_attribute("action", "start_campaign")
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER),
                                action: "StartCampaign".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
            },
            StartCampaignTestCase {
                name: "start_campaign with unauthorized sender".to_string(),
                tiers: mock_campaign_tiers(),
                presale: Some(valid_presale.clone()),
                start_time: None,
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds() + 100),
                payee: "owner1".to_string(),
                expected_res: Err(ContractError::Unauthorized {}),
            },
            StartCampaignTestCase {
                name: "start_campaign with no unlimited tier".to_string(),
                tiers: invalid_tiers,
                presale: Some(valid_presale.clone()),
                start_time: None,
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds() + 100),
                payee: MOCK_DEFAULT_OWNER.to_string(),
                expected_res: Err(ContractError::InvalidTiers {}),
            },
            StartCampaignTestCase {
                name: "start_campaign with invalid presales".to_string(),
                tiers: mock_campaign_tiers(),
                presale: Some(invalid_presale.clone()),
                start_time: None,
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds() + 100),
                payee: MOCK_DEFAULT_OWNER.to_string(),
                expected_res: Err(ContractError::InvalidTier {
                    operation: "set_tier_orders".to_string(),
                    msg: "Tier with level 2 does not exist".to_string(),
                }),
            },
            StartCampaignTestCase {
                name: "start_campaign with invalid end_time".to_string(),
                tiers: mock_campaign_tiers(),
                presale: Some(valid_presale.clone()),
                start_time: None,
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds() - 100),
                payee: MOCK_DEFAULT_OWNER.to_string(),
                expected_res: Err(ContractError::StartTimeAfterEndTime {}),
            },
            StartCampaignTestCase {
                name: "start_campaign with invalid start_time".to_string(),
                tiers: mock_campaign_tiers(),
                presale: Some(valid_presale.clone()),
                start_time: Some(MillisecondsExpiration::from_seconds(
                    env.block.time.seconds() + 1000,
                )),
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds() + 500),
                payee: MOCK_DEFAULT_OWNER.to_string(),
                expected_res: Err(ContractError::StartTimeAfterEndTime {}),
            },
        ];
        for test in test_cases {
            let mut deps = mock_dependencies_custom(&[coin(100000, MOCK_NATIVE_DENOM)]);

            let _ = init(
                deps.as_mut(),
                mock_campaign_config(Asset::NativeToken(MOCK_NATIVE_DENOM.to_string())),
                test.tiers.clone(),
            );
            let info = mock_info(&test.payee, &[]);

            let msg = ExecuteMsg::StartCampaign {
                start_time: test.start_time,
                end_time: test.end_time,
                presale: test.presale.clone(),
            };

            let res = execute(deps.as_mut(), env.clone(), info, msg);
            assert_eq!(res, test.expected_res, "Test case: {}", test.name);

            if res.is_ok() {
                assert_eq!(
                    CAMPAIGN_CONFIG.load(&deps.storage).unwrap().start_time,
                    test.start_time
                );
                assert_eq!(
                    CAMPAIGN_CONFIG.load(&deps.storage).unwrap().end_time,
                    test.end_time
                );
                assert_eq!(
                    CAMPAIGN_STAGE.load(&deps.storage).unwrap(),
                    CampaignStage::ONGOING
                );
                for order in &test.presale.unwrap() {
                    let order_amount: u128 = order.amount.into();
                    assert_eq!(
                        TIER_ORDERS
                            .load(&deps.storage, (mock_orderer.clone(), order.level.into()))
                            .unwrap(),
                        order_amount
                    );
                    let cur_limit = TIERS.load(&deps.storage, order.level.into()).unwrap().limit;
                    if cur_limit.is_some() {
                        assert_eq!(
                            TIERS
                                .load(&deps.storage, order.level.into())
                                .unwrap()
                                .sold_amount
                                .u128(),
                            order_amount
                        );
                    }
                }
            } else {
                assert_eq!(
                    CAMPAIGN_STAGE
                        .load(&deps.storage)
                        .unwrap_or(CampaignStage::READY),
                    CampaignStage::READY
                );
            }
        }
    }

    struct PurchaseTierTestCase {
        name: String,
        stage: CampaignStage,
        expected_res: Result<Response, ContractError>,
        payee: String,
        start_time: Option<MillisecondsExpiration>,
        end_time: MillisecondsExpiration,
        orders: Vec<SimpleTierOrder>,
        initial_cap: Uint128,
        funds: Vec<Coin>,
        denom: Asset,
    }
    #[test]
    fn test_execute_purchase_tiers_native() {
        // fixed total cost to 100 for valid purchase
        let env = mock_env();
        let buyer = "buyer";
        let test_cases: Vec<PurchaseTierTestCase> = vec![
            PurchaseTierTestCase {
                name: "Standard purchase with valid order using native tokens".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Ok(Response::new()
                    .add_attribute("action", "purchase_tiers")
                    .add_attribute("payment", "1000native:uandr")
                    .add_attribute("total_cost", "100")
                    .add_attribute("refunded", "900")
                    .add_message(BankMsg::Send {
                        to_address: buyer.to_string(),
                        // Refund sent back as they only were able to mint one.
                        amount: coins(900, MOCK_NATIVE_DENOM),
                    })
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(buyer),
                                action: "PurchaseTiers".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![coin(1000, MOCK_NATIVE_DENOM)],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
            },
            PurchaseTierTestCase {
                name: "Purchasing more than limit".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::PurchaseLimitReached {}),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(MOCK_DEFAULT_LIMIT + 1),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![coin(10 * MOCK_DEFAULT_LIMIT + 20, MOCK_NATIVE_DENOM)],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
            },
            PurchaseTierTestCase {
                name: "Purchase with insufficient funds using native tokens".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::InsufficientFunds {}),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(20),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![coin(10, MOCK_NATIVE_DENOM)],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
            },
            PurchaseTierTestCase {
                name: "Purchase in wrong campaign stage using native tokens".to_string(),
                stage: CampaignStage::READY,
                expected_res: Err(ContractError::InvalidCampaignOperation {
                    operation: "purchase_tiers".to_string(),
                    stage: "READY".to_string(),
                }),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![coin(1000, MOCK_NATIVE_DENOM)],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
            },
            PurchaseTierTestCase {
                name: "Purchase before campaign start using native tokens".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::CampaignNotStarted {}),
                payee: buyer.to_string(),
                start_time: Some(future_time(&env)),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![coin(1000, MOCK_NATIVE_DENOM)],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
            },
            PurchaseTierTestCase {
                name: "Purchase after campaign end using native tokens".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::CampaignEnded {}),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: past_time(),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![coin(1000, MOCK_NATIVE_DENOM)],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
            },
            PurchaseTierTestCase {
                name: "Purchase with invalid denomination using native tokens".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::InvalidFunds {
                    msg: format!("Only native:{MOCK_NATIVE_DENOM} is accepted by the campaign."),
                }),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![coin(1000, INVA1LID_DENOM)],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
            },
        ];
        for test in test_cases {
            let mut deps = mock_dependencies_custom(&test.funds);
            let info = mock_info(&test.payee, &test.funds);

            // Mock necessary storage setup
            set_campaign_stage(deps.as_mut().storage, &test.stage);
            set_current_cap(deps.as_mut().storage, &test.initial_cap);
            set_tiers(deps.as_mut().storage, mock_campaign_tiers()).unwrap();

            let mut mock_config = mock_campaign_config(test.denom);
            mock_config.start_time = test.start_time;
            mock_config.end_time = test.end_time;
            set_campaign_config(deps.as_mut().storage, &mock_config);

            let msg = ExecuteMsg::PurchaseTiers {
                orders: test.orders.clone(),
            };

            let res = execute(deps.as_mut(), env.clone(), info, msg);
            assert_eq!(res, test.expected_res, "Test case: {}", test.name);

            if res.is_ok() {
                // Check current capital
                let updated_cap = get_current_cap(deps.as_ref().storage);
                let expected_cap = test.initial_cap + Uint128::new(100);
                assert_eq!(updated_cap, expected_cap, "Test case: {}", test.name);

                // Check tier orders
                for order in &test.orders {
                    let stored_order = TIER_ORDERS
                        .load(
                            deps.as_ref().storage,
                            (Addr::unchecked(buyer), order.level.into()),
                        )
                        .unwrap();
                    assert_eq!(
                        stored_order,
                        order.amount.u128(),
                        "Test case: {}",
                        test.name
                    );
                }

                // Check tier limits
                for order in &test.orders {
                    let tier = TIERS
                        .load(deps.as_ref().storage, order.level.into())
                        .unwrap();
                    assert_eq!(
                        tier.sold_amount.u128(),
                        order.amount.u128(),
                        "Test case: {}",
                        test.name
                    );
                }
            }
        }
    }

    #[test]
    fn test_execute_purchase_tiers_cw20() {
        // fixed total cost to 100 for valid purchase
        let env = mock_env();
        let buyer = "buyer";
        let valid_denom = Asset::Cw20Token(AndrAddr::from_string(MOCK_CW20_CONTRACT.to_string()));
        let test_cases: Vec<PurchaseTierTestCase> = vec![
            PurchaseTierTestCase {
                name: "Standard purchase with valid order using cw20 token".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Ok(Response::new()
                    .add_attribute("action", "purchase_tiers")
                    .add_attribute("payment", "1000cw20:cw20_contract")
                    .add_attribute("total_cost", "100")
                    .add_attribute("refunded", "900")
                    .add_message(
                        wasm_execute(
                            MOCK_CW20_CONTRACT.to_string(),
                            &Cw20ExecuteMsg::Transfer {
                                recipient: buyer.to_string(),
                                amount: Uint128::new(900u128),
                            },
                            vec![],
                        )
                        .unwrap(),
                    )
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_CW20_CONTRACT),
                                action: "Receive".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![],
                denom: valid_denom.clone(),
            },
            PurchaseTierTestCase {
                name: "Purchase with insufficient funds using cw20 token".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::InsufficientFunds {}),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(200000),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![],
                denom: valid_denom.clone(),
            },
            PurchaseTierTestCase {
                name: "Purchase in wrong campaign stage using cw20 token".to_string(),
                stage: CampaignStage::READY,
                expected_res: Err(ContractError::InvalidCampaignOperation {
                    operation: "purchase_tiers".to_string(),
                    stage: "READY".to_string(),
                }),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![],
                denom: valid_denom.clone(),
            },
            PurchaseTierTestCase {
                name: "Purchase before campaign start using cw20 token".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::CampaignNotStarted {}),
                payee: buyer.to_string(),
                start_time: Some(future_time(&env)),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![],
                denom: valid_denom.clone(),
            },
            PurchaseTierTestCase {
                name: "Purchase after campaign end using cw20 token".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::CampaignEnded {}),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: past_time(),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![],
                denom: valid_denom.clone(),
            },
            PurchaseTierTestCase {
                name: "Purchase with invalid denomination using cw20 token".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::InvalidFunds {
                    msg: format!("Only cw20:{MOCK_CW20_CONTRACT} is accepted by the campaign."),
                }),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![],
                denom: Asset::Cw20Token(AndrAddr::from_string("cw20_contract124".to_string())),
            },
        ];

        for test in test_cases {
            let mut deps = mock_dependencies_custom(&test.funds);
            let Asset::Cw20Token(ref cw20) = test.denom else {
                todo!();
            };
            let info = mock_info(cw20.as_ref(), &[]);

            // Mock necessary storage setup
            set_campaign_stage(deps.as_mut().storage, &test.stage);
            set_current_cap(deps.as_mut().storage, &test.initial_cap);
            set_tiers(deps.as_mut().storage, mock_campaign_tiers()).unwrap();

            let mut mock_config = mock_campaign_config(valid_denom.clone());
            mock_config.start_time = test.start_time;
            mock_config.end_time = test.end_time;
            set_campaign_config(deps.as_mut().storage, &mock_config);

            let hook_msg = Cw20HookMsg::PurchaseTiers {
                orders: test.orders.clone(),
            };
            let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: buyer.to_owned(),
                amount: Uint128::new(1000u128),
                msg: encode_binary(&hook_msg).unwrap(),
            });

            let res = execute(deps.as_mut(), env.clone(), info, msg);
            assert_eq!(res, test.expected_res, "Test case: {}", test.name);

            if res.is_ok() {
                // Check current capital
                let updated_cap = get_current_cap(deps.as_ref().storage);
                let expected_cap = test.initial_cap + Uint128::new(100);
                assert_eq!(updated_cap, expected_cap, "Test case: {}", test.name);

                // Check tier orders
                for order in &test.orders {
                    let stored_order = TIER_ORDERS
                        .load(
                            deps.as_ref().storage,
                            (Addr::unchecked(buyer), order.level.into()),
                        )
                        .unwrap();
                    assert_eq!(
                        stored_order,
                        order.amount.u128(),
                        "Test case: {}",
                        test.name
                    );
                }

                // Check tier limits
                for order in &test.orders {
                    let tier = TIERS
                        .load(deps.as_ref().storage, order.level.into())
                        .unwrap();
                    assert_eq!(
                        tier.sold_amount.u128(),
                        order.amount.u128(),
                        "Test case: {}",
                        test.name
                    );
                }
            }
        }
    }

    struct EndCampaignTestCase {
        name: String,
        stage: CampaignStage,
        sender: String,
        current_cap: Uint128,
        soft_cap: Option<Uint128>,
        end_time: MillisecondsExpiration,
        denom: Asset,
        is_discard: bool,
        expected_res: Result<Response, ContractError>,
        expected_stage: CampaignStage,
    }
    #[test]
    fn test_execute_end_campaign() {
        let env = mock_env();
        let test_cases: Vec<EndCampaignTestCase> = vec![
            EndCampaignTestCase {
                name: "Successful campaign using native token".to_string(),
                stage: CampaignStage::ONGOING,
                sender: MOCK_DEFAULT_OWNER.to_string(),
                current_cap: Uint128::new(10000u128),
                soft_cap: Some(Uint128::new(9000u128)),
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds()),
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
                is_discard: false,
                expected_res: Ok(Response::new()
                    .add_attribute("action", "end_campaign")
                    .add_attribute("result", CampaignStage::SUCCESS.to_string())
                    .add_message(BankMsg::Send {
                        to_address: MOCK_WITHDRAWAL_ADDRESS.to_string(),
                        amount: coins(10000, MOCK_NATIVE_DENOM),
                    })
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER.to_string()),
                                action: "EndCampaign".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
                expected_stage: CampaignStage::SUCCESS,
            },
            EndCampaignTestCase {
                name: "Successful campaign using cw20".to_string(),
                stage: CampaignStage::ONGOING,
                sender: MOCK_DEFAULT_OWNER.to_string(),
                current_cap: Uint128::new(10000u128),
                soft_cap: Some(Uint128::new(9000u128)),
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds()),
                denom: Asset::Cw20Token(AndrAddr::from_string(MOCK_CW20_CONTRACT.to_string())),
                is_discard: false,
                expected_res: Ok(Response::new()
                    .add_attribute("action", "end_campaign")
                    .add_attribute("result", CampaignStage::SUCCESS.to_string())
                    .add_message(
                        wasm_execute(
                            MOCK_CW20_CONTRACT.to_string(),
                            &Cw20ExecuteMsg::Transfer {
                                recipient: MOCK_WITHDRAWAL_ADDRESS.to_string(),
                                amount: Uint128::new(10000u128),
                            },
                            vec![],
                        )
                        .unwrap(),
                    )
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER.to_string()),
                                action: "EndCampaign".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
                expected_stage: CampaignStage::SUCCESS,
            },
            EndCampaignTestCase {
                name: "Failed campaign".to_string(),
                stage: CampaignStage::ONGOING,
                sender: MOCK_DEFAULT_OWNER.to_string(),
                current_cap: Uint128::new(10000u128),
                soft_cap: Some(Uint128::new(11000u128)),
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds()),
                denom: Asset::Cw20Token(AndrAddr::from_string(MOCK_CW20_CONTRACT.to_string())),
                is_discard: true,
                expected_res: Ok(Response::new()
                    .add_attribute("action", "end_campaign")
                    .add_attribute("result", CampaignStage::FAILED.to_string())
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER.to_string()),
                                action: "EndCampaign".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
                expected_stage: CampaignStage::FAILED,
            },
            EndCampaignTestCase {
                name: "Discard campaign using native token".to_string(),
                stage: CampaignStage::ONGOING,
                sender: MOCK_DEFAULT_OWNER.to_string(),
                current_cap: Uint128::new(10000u128),
                soft_cap: Some(Uint128::new(9000u128)),
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds()),
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
                is_discard: true,
                expected_res: Ok(Response::new()
                    .add_attribute("action", "end_campaign")
                    .add_attribute("result", CampaignStage::FAILED.to_string())
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER.to_string()),
                                action: "EndCampaign".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
                expected_stage: CampaignStage::FAILED,
            },
            EndCampaignTestCase {
                name: "Pause campaign".to_string(),
                stage: CampaignStage::ONGOING,
                sender: MOCK_DEFAULT_OWNER.to_string(),
                current_cap: Uint128::new(0u128),
                soft_cap: None,
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds() + 1000),
                denom: Asset::Cw20Token(AndrAddr::from_string(MOCK_CW20_CONTRACT.to_string())),
                is_discard: false,
                expected_res: Ok(Response::new()
                    .add_attribute("action", "end_campaign")
                    .add_attribute("result", CampaignStage::READY.to_string())
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER.to_string()),
                                action: "EndCampaign".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
                expected_stage: CampaignStage::READY,
            },
            EndCampaignTestCase {
                name: "End campaign from unauthorized sender".to_string(),
                stage: CampaignStage::ONGOING,
                sender: "sender".to_string(),
                current_cap: Uint128::new(10000u128),
                soft_cap: None,
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds()),
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
                is_discard: false,
                expected_res: Err(ContractError::Unauthorized {}),
                expected_stage: CampaignStage::ONGOING,
            },
            EndCampaignTestCase {
                name: "End campaign on invalid stage".to_string(),
                stage: CampaignStage::READY,
                sender: MOCK_DEFAULT_OWNER.to_string(),
                current_cap: Uint128::new(10000u128),
                soft_cap: None,
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds()),
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
                is_discard: false,
                expected_res: Err(ContractError::InvalidCampaignOperation {
                    operation: "end_campaign".to_string(),
                    stage: CampaignStage::READY.to_string(),
                }),
                expected_stage: CampaignStage::READY,
            },
            EndCampaignTestCase {
                name: "End unexpired campaign".to_string(),
                stage: CampaignStage::ONGOING,
                sender: MOCK_DEFAULT_OWNER.to_string(),
                current_cap: Uint128::new(10000u128),
                soft_cap: Some(Uint128::new(11000u128)),
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds() + 100),
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
                is_discard: false,
                expected_res: Err(ContractError::CampaignNotExpired {}),
                expected_stage: CampaignStage::ONGOING,
            },
        ];
        for test in test_cases {
            let mut deps = mock_dependencies_custom(&[coin(100000, MOCK_NATIVE_DENOM)]);
            let mut mock_config = mock_campaign_config(test.denom.clone());
            let _ = init(deps.as_mut(), mock_config.clone(), vec![]);

            let info = mock_info(&test.sender, &[]);
            set_campaign_stage(deps.as_mut().storage, &test.stage);
            set_current_cap(deps.as_mut().storage, &test.current_cap);

            mock_config.end_time = test.end_time;
            mock_config.soft_cap = test.soft_cap;
            set_campaign_config(deps.as_mut().storage, &mock_config);
            let msg = ExecuteMsg::EndCampaign {
                is_discard: test.is_discard,
            };

            let res = execute(deps.as_mut(), env.clone(), info, msg);
            assert_eq!(res, test.expected_res, "Test case: {}", test.name);
            if res.is_ok() {
                assert_eq!(
                    CAMPAIGN_STAGE
                        .load(&deps.storage)
                        .unwrap_or(CampaignStage::SUCCESS),
                    test.expected_stage,
                    "Test case: {}",
                    test.name
                );
            }
        }
    }
}

// #[test]
// fn test_mint_unauthorized() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);

//     let msg = ExecuteMsg::Mint(vec![CrowdfundMintMsg {
//         token_id: "token_id".to_string(),
//         owner: None,
//         token_uri: None,
//         extension: TokenExtension {
//             publisher: "publisher".to_string(),
//         },
//     }]);
//     let info = mock_info("not_owner", &[]);
//     let res = execute(deps.as_mut(), mock_env(), info, msg);

//     assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
// }

// #[test]
// fn test_mint_owner_not_crowdfund() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);

//     let msg = ExecuteMsg::Mint(vec![CrowdfundMintMsg {
//         token_id: "token_id".to_string(),
//         owner: Some("not_crowdfund".to_string()),
//         token_uri: None,
//         extension: TokenExtension {
//             publisher: "publisher".to_string(),
//         },
//     }]);
//     let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
//     let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     // Since token was minted to owner that is not the contract, it is not available for sale.
//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, "token_id"));
// }

// #[test]
// fn test_mint_sale_started() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);
//     let current_time = mock_env().block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;

//     let msg = ExecuteMsg::StartSale {
//         start_time: None,
//         end_time: Expiry::AtTime(Milliseconds::from_nanos((current_time + 2) * 1_000_000)),
//         price: coin(100, "uusd"),
//         min_tokens_sold: Uint128::from(1u128),
//         max_amount_per_wallet: Some(5),
//         recipient: Recipient::from_string("recipient"),
//     };

//     let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
//     let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     let res = mint(deps.as_mut(), "token_id");

//     assert_eq!(ContractError::SaleStarted {}, res.unwrap_err());
// }

// #[test]
// fn test_mint_sale_conducted_cant_mint_after_sale() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let msg = InstantiateMsg {
//         token_address: AndrAddr::from_string(MOCK_TOKEN_CONTRACT.to_owned()),
//         modules: None,
//         owner: None,
//         can_mint_after_sale: false,
//         kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
//     };

//     let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
//     let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//     SALE_CONDUCTED.save(deps.as_mut().storage, &true).unwrap();

//     let res = mint(deps.as_mut(), "token_id");

//     assert_eq!(
//         ContractError::CannotMintAfterSaleConducted {},
//         res.unwrap_err()
//     );
// }

// #[test]
// fn test_mint_sale_conducted_can_mint_after_sale() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);

//     SALE_CONDUCTED.save(deps.as_mut().storage, &true).unwrap();

//     let _res = mint(deps.as_mut(), "token_id").unwrap();

//     assert!(AVAILABLE_TOKENS.has(deps.as_ref().storage, "token_id"));
// }

// #[test]
// fn test_mint_successful() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);

//     let res = mint(deps.as_mut(), "token_id").unwrap();

//     let mint_msg = Cw721ExecuteMsg::Mint {
//         token_id: "token_id".to_string(),
//         owner: mock_env().contract.address.to_string(),
//         token_uri: None,
//         extension: TokenExtension {
//             publisher: "publisher".to_string(),
//         },
//     };

//     assert_eq!(
//         Response::new()
//             .add_attribute("action", "mint")
//             .add_message(WasmMsg::Execute {
//                 contract_addr: MOCK_TOKEN_CONTRACT.to_owned(),
//                 msg: encode_binary(&mint_msg).unwrap(),
//                 funds: vec![],
//             })
//             .add_submessage(generate_economics_message(MOCK_DEFAULT_OWNER, "Mint")),
//         res
//     );

//     assert!(AVAILABLE_TOKENS.has(deps.as_ref().storage, "token_id"));
// }

// #[test]
// fn test_mint_multiple_successful() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);

//     let mint_msgs = vec![
//         CrowdfundMintMsg {
//             token_id: "token_id1".to_string(),
//             owner: None,
//             token_uri: None,
//             extension: TokenExtension {
//                 publisher: "publisher".to_string(),
//             },
//         },
//         CrowdfundMintMsg {
//             token_id: "token_id2".to_string(),
//             owner: None,
//             token_uri: None,
//             extension: TokenExtension {
//                 publisher: "publisher".to_string(),
//             },
//         },
//     ];

//     let msg = ExecuteMsg::Mint(mint_msgs);
//     let res = execute(deps.as_mut(), mock_env(), mock_info(MOCK_DEFAULT_OWNER, &[]), msg).unwrap();

//     assert_eq!(
//         Response::new()
//             .add_attribute("action", "mint")
//             .add_attribute("action", "mint")
//             .add_message(WasmMsg::Execute {
//                 contract_addr: MOCK_TOKEN_CONTRACT.to_owned(),
//                 msg: encode_binary(&Cw721ExecuteMsg::Mint {
//                     token_id: "token_id1".to_string(),
//                     owner: mock_env().contract.address.to_string(),
//                     token_uri: None,
//                     extension: TokenExtension {
//                         publisher: "publisher".to_string(),
//                     },
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })
//             .add_message(WasmMsg::Execute {
//                 contract_addr: MOCK_TOKEN_CONTRACT.to_owned(),
//                 msg: encode_binary(&Cw721ExecuteMsg::Mint {
//                     token_id: "token_id2".to_string(),
//                     owner: mock_env().contract.address.to_string(),
//                     token_uri: None,
//                     extension: TokenExtension {
//                         publisher: "publisher".to_string(),
//                     },
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             })
//             .add_submessage(generate_economics_message(MOCK_DEFAULT_OWNER, "Mint")),
//         res
//     );

//     assert!(AVAILABLE_TOKENS.has(deps.as_ref().storage, "token_id1"));
//     assert!(AVAILABLE_TOKENS.has(deps.as_ref().storage, "token_id2"));

//     assert_eq!(
//         NUMBER_OF_TOKENS_AVAILABLE
//             .load(deps.as_ref().storage)
//             .unwrap(),
//         Uint128::new(2)
//     );
// }

// #[test]
// fn test_mint_multiple_exceeds_limit() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);

//     let mint_msg = CrowdfundMintMsg {
//         token_id: "token_id1".to_string(),
//         owner: None,
//         token_uri: None,
//         extension: TokenExtension {
//             publisher: "publisher".to_string(),
//         },
//     };

//     let mut mint_msgs: Vec<CrowdfundMintMsg> = vec![];

//     for _ in 0..MAX_MINT_LIMIT + 1 {
//         mint_msgs.push(mint_msg.clone());
//     }

//     let msg = ExecuteMsg::Mint(mint_msgs.clone());
//     let res = execute(deps.as_mut(), mock_env(), mock_info(MOCK_DEFAULT_OWNER, &[]), msg);

//     assert_eq!(
//         ContractError::TooManyMintMessages {
//             limit: MAX_MINT_LIMIT
//         },
//         res.unwrap_err()
//     );
// }

// #[test]
// fn test_start_sale_end_time_zero() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);
//     let one_minute_in_future =
//         mock_env().block.time.plus_minutes(1).nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;

//     let msg = ExecuteMsg::StartSale {
//         start_time: Some(Expiry::AtTime(Milliseconds(one_minute_in_future))),
//         end_time: Expiry::AtTime(Milliseconds::zero()),
//         price: coin(100, "uusd"),
//         min_tokens_sold: Uint128::from(1u128),
//         max_amount_per_wallet: None,
//         recipient: Recipient::from_string("recipient".to_string()),
//     };

//     let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
//     let res = execute(deps.as_mut(), mock_env(), info, msg);
//     assert_eq!(ContractError::StartTimeAfterEndTime {}, res.unwrap_err());
// }

// #[test]
// fn test_start_sale_unauthorized() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);
//     let current_time = mock_env().block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;

//     let msg = ExecuteMsg::StartSale {
//         start_time: None,
//         end_time: Expiry::AtTime(Milliseconds::from_nanos((current_time + 1) * 1_000_000)),
//         price: coin(100, "uusd"),
//         min_tokens_sold: Uint128::from(1u128),
//         max_amount_per_wallet: None,
//         recipient: Recipient::from_string("recipient"),
//     };

//     let info = mock_info("anyone", &[]);
//     let res = execute(deps.as_mut(), mock_env(), info, msg);
//     assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
// }

// #[test]
// fn test_start_sale_start_time_in_past() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();
//     init(deps.as_mut(), None);
//     let current_time = mock_env().block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;

//     let one_minute_in_past = env.block.time.minus_minutes(1).seconds();
//     let msg = ExecuteMsg::StartSale {
//         start_time: Some(Expiry::AtTime(Milliseconds(one_minute_in_past))),
//         end_time: Expiry::AtTime(Milliseconds::from_nanos((current_time + 2) * 1_000_000)),
//         price: coin(100, "uusd"),
//         min_tokens_sold: Uint128::from(1u128),
//         max_amount_per_wallet: None,
//         recipient: Recipient::from_string("recipient"),
//     };

//     let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
//     let res = execute(deps.as_mut(), mock_env(), info, msg);
//     assert_eq!(
//         ContractError::StartTimeInThePast {
//             current_time: env.block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO,
//             current_block: env.block.height,
//         },
//         res.unwrap_err()
//     );
// }

// #[test]
// fn test_start_sale_start_time_in_future() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let env = mock_env();
//     init(deps.as_mut(), None);

//     let one_minute_in_future =
//         env.block.time.plus_minutes(1).nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;
//     let msg = ExecuteMsg::StartSale {
//         start_time: Some(Expiry::AtTime(Milliseconds(one_minute_in_future))),
//         end_time: Expiry::AtTime(Milliseconds::from_nanos(
//             (one_minute_in_future + 2) * 1_000_000,
//         )),
//         price: coin(100, "uusd"),
//         min_tokens_sold: Uint128::from(1u128),
//         max_amount_per_wallet: None,
//         recipient: Recipient::from_string("recipient"),
//     };

//     let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
//     let res = execute(deps.as_mut(), mock_env(), info, msg);
//     assert!(res.is_ok())
// }

// #[test]
// fn test_start_sale_max_default() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);
//     let current_time = mock_env().block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;

//     let msg = ExecuteMsg::StartSale {
//         start_time: None,
//         end_time: Expiry::AtTime(Milliseconds::from_nanos((current_time + 2) * 1_000_000)),
//         price: coin(100, "uusd"),
//         min_tokens_sold: Uint128::from(1u128),
//         max_amount_per_wallet: None,
//         recipient: Recipient::from_string("recipient"),
//     };

//     let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
//     let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
//     // Using current time since start time wasn't provided
//     let current_time = mock_env().block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;
//     let start_expiration = expiration_from_milliseconds(Milliseconds(current_time + 1)).unwrap();
//     let end_expiration = expiration_from_milliseconds(Milliseconds(current_time + 2)).unwrap();

//     assert_eq!(
//         Response::new()
//             .add_attribute("action", "start_sale")
//             .add_attribute("start_time", start_expiration.to_string())
//             .add_attribute("end_time", end_expiration.to_string())
//             .add_attribute("price", "100uusd")
//             .add_attribute("min_tokens_sold", "1")
//             .add_attribute("max_amount_per_wallet", "1")
//             .add_submessage(generate_economics_message(MOCK_DEFAULT_OWNER, "StartSale")),
//         res
//     );

//     assert_eq!(
//         State {
//             end_time: end_expiration,
//             price: coin(100, "uusd"),
//             min_tokens_sold: Uint128::from(1u128),
//             max_amount_per_wallet: 1,
//             amount_sold: Uint128::zero(),
//             amount_to_send: Uint128::zero(),
//             amount_transferred: Uint128::zero(),
//             recipient: Recipient::from_string("recipient"),
//         },
//         STATE.load(deps.as_ref().storage).unwrap()
//     );

//     assert!(SALE_CONDUCTED.load(deps.as_ref().storage).unwrap());

//     let res = execute(deps.as_mut(), mock_env(), info, msg);
//     assert_eq!(ContractError::SaleStarted {}, res.unwrap_err());
// }

// #[test]
// fn test_start_sale_max_modified() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);
//     let current_time = mock_env().block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;

//     let msg = ExecuteMsg::StartSale {
//         start_time: None,
//         end_time: Expiry::AtTime(Milliseconds::from_nanos((current_time + 2) * 1_000_000)),
//         price: coin(100, "uusd"),
//         min_tokens_sold: Uint128::from(1u128),
//         max_amount_per_wallet: Some(5),
//         recipient: Recipient::from_string("recipient"),
//     };
//     // Using current time since start time wasn't provided
//     let current_time = mock_env().block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;
//     let start_expiration = expiration_from_milliseconds(Milliseconds(current_time + 1)).unwrap();
//     let end_expiration = expiration_from_milliseconds(Milliseconds(current_time + 2)).unwrap();

//     let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
//     let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//     assert_eq!(
//         Response::new()
//             .add_attribute("action", "start_sale")
//             .add_attribute("start_time", start_expiration.to_string())
//             .add_attribute("end_time", end_expiration.to_string())
//             .add_attribute("price", "100uusd")
//             .add_attribute("min_tokens_sold", "1")
//             .add_attribute("max_amount_per_wallet", "5")
//             .add_submessage(generate_economics_message(MOCK_DEFAULT_OWNER, "StartSale")),
//         res
//     );

//     assert_eq!(
//         State {
//             end_time: end_expiration,
//             price: coin(100, "uusd"),
//             min_tokens_sold: Uint128::from(1u128),
//             max_amount_per_wallet: 5,
//             amount_sold: Uint128::zero(),
//             amount_to_send: Uint128::zero(),
//             amount_transferred: Uint128::zero(),
//             recipient: Recipient::from_string("recipient"),
//         },
//         STATE.load(deps.as_ref().storage).unwrap()
//     );
// }

// #[test]
// fn test_purchase_sale_not_started() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);

//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: None,
//     };

//     let info = mock_info("sender", &[]);
//     let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
//     assert_eq!(ContractError::NoOngoingSale {}, res.unwrap_err());

//     let msg = ExecuteMsg::PurchaseByTokenId {
//         token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
//     };

//     let res = execute(deps.as_mut(), mock_env(), info, msg);
//     assert_eq!(ContractError::NoOngoingSale {}, res.unwrap_err());
// }

// #[test]
// fn test_purchase_sale_not_ended() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);

//     STATE
//         .save(
//             deps.as_mut().storage,
//             &State {
//                 end_time: Expiration::AtHeight(mock_env().block.height - 1),
//                 price: coin(100, "uusd"),
//                 min_tokens_sold: Uint128::from(1u128),
//                 max_amount_per_wallet: 5,
//                 amount_sold: Uint128::zero(),
//                 amount_to_send: Uint128::zero(),
//                 amount_transferred: Uint128::zero(),
//                 recipient: Recipient::from_string("recipient"),
//             },
//         )
//         .unwrap();

//     let info = mock_info("sender", &[]);

//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: None,
//     };

//     let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
//     assert_eq!(ContractError::NoOngoingSale {}, res.unwrap_err());

//     let msg = ExecuteMsg::PurchaseByTokenId {
//         token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
//     };

//     let res = execute(deps.as_mut(), mock_env(), info, msg);
//     assert_eq!(ContractError::NoOngoingSale {}, res.unwrap_err());
// }

// #[test]
// fn test_purchase_no_funds() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);

//     mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[0]).unwrap();

//     STATE
//         .save(
//             deps.as_mut().storage,
//             &State {
//                 end_time: Expiration::AtHeight(mock_env().block.height + 1),
//                 price: coin(100, "uusd"),
//                 min_tokens_sold: Uint128::from(1u128),
//                 max_amount_per_wallet: 5,
//                 amount_sold: Uint128::zero(),
//                 amount_to_send: Uint128::zero(),
//                 amount_transferred: Uint128::zero(),
//                 recipient: Recipient::from_string("recipient"),
//             },
//         )
//         .unwrap();

//     let info = mock_info("sender", &[]);

//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: None,
//     };
//     let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
//     assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());

//     let msg = ExecuteMsg::PurchaseByTokenId {
//         token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
//     };
//     let res = execute(deps.as_mut(), mock_env(), info, msg);
//     assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());
// }

// #[test]
// fn test_purchase_wrong_denom() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);

//     mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[0]).unwrap();

//     STATE
//         .save(
//             deps.as_mut().storage,
//             &State {
//                 end_time: Expiration::AtHeight(mock_env().block.height + 1),
//                 price: coin(100, "uusd"),
//                 min_tokens_sold: Uint128::from(1u128),
//                 max_amount_per_wallet: 5,
//                 amount_sold: Uint128::zero(),
//                 amount_to_send: Uint128::zero(),
//                 amount_transferred: Uint128::zero(),
//                 recipient: Recipient::from_string("recipient"),
//             },
//         )
//         .unwrap();

//     let info = mock_info("sender", &coins(100, "uluna"));

//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: None,
//     };
//     let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
//     assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());

//     let msg = ExecuteMsg::PurchaseByTokenId {
//         token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
//     };
//     let res = execute(deps.as_mut(), mock_env(), info, msg);
//     assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());
// }

// #[test]
// fn test_purchase_not_enough_for_price() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let modules = vec![Module {
//         name: Some(RATES.to_owned()),
//         address: AndrAddr::from_string(MOCK_RATES_CONTRACT.to_owned()),
//         is_mutable: false,
//     }];
//     init(deps.as_mut(), Some(modules));

//     mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[0]).unwrap();

//     STATE
//         .save(
//             deps.as_mut().storage,
//             &State {
//                 end_time: Expiration::AtHeight(mock_env().block.height + 1),
//                 price: coin(100, "uusd"),
//                 min_tokens_sold: Uint128::from(1u128),
//                 max_amount_per_wallet: 5,
//                 amount_sold: Uint128::zero(),
//                 amount_to_send: Uint128::zero(),
//                 amount_transferred: Uint128::zero(),
//                 recipient: Recipient::from_string("recipient"),
//             },
//         )
//         .unwrap();

//     let info = mock_info("sender", &coins(50u128, "uusd"));

//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: None,
//     };
//     let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
//     assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());

//     let msg = ExecuteMsg::PurchaseByTokenId {
//         token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
//     };
//     let res = execute(deps.as_mut(), mock_env(), info, msg);
//     assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());
// }

// #[test]
// fn test_purchase_not_enough_for_tax() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let modules = vec![Module {
//         name: Some(RATES.to_owned()),
//         address: AndrAddr::from_string(MOCK_RATES_CONTRACT.to_owned()),
//         is_mutable: false,
//     }];
//     init(deps.as_mut(), Some(modules));

//     mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[0]).unwrap();

//     NUMBER_OF_TOKENS_AVAILABLE
//         .save(deps.as_mut().storage, &Uint128::new(1))
//         .unwrap();

//     STATE
//         .save(
//             deps.as_mut().storage,
//             &State {
//                 end_time: Expiration::AtHeight(mock_env().block.height + 1),
//                 price: coin(100, "uusd"),
//                 min_tokens_sold: Uint128::from(1u128),
//                 max_amount_per_wallet: 5,
//                 amount_sold: Uint128::zero(),
//                 amount_to_send: Uint128::zero(),
//                 amount_transferred: Uint128::zero(),
//                 recipient: Recipient::from_string("recipient"),
//             },
//         )
//         .unwrap();

//     let info = mock_info("sender", &coins(100u128, "uusd"));

//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: None,
//     };
//     let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
//     assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());

//     // Reset the state since state does not roll back on failure in tests like it does in prod.
//     AVAILABLE_TOKENS
//         .save(deps.as_mut().storage, MOCK_TOKENS_FOR_SALE[0], &true)
//         .unwrap();
//     NUMBER_OF_TOKENS_AVAILABLE
//         .save(deps.as_mut().storage, &Uint128::new(1))
//         .unwrap();

//     let msg = ExecuteMsg::PurchaseByTokenId {
//         token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
//     };
//     let res = execute(deps.as_mut(), mock_env(), info, msg);
//     assert_eq!(ContractError::InsufficientFunds {}, res.unwrap_err());
// }

// #[test]
// fn test_purchase_by_token_id_not_available() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let modules = vec![Module {
//         name: Some(RATES.to_owned()),
//         address: AndrAddr::from_string(MOCK_RATES_CONTRACT.to_owned()),
//         is_mutable: false,
//     }];
//     init(deps.as_mut(), Some(modules));

//     mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[0]).unwrap();

//     STATE
//         .save(
//             deps.as_mut().storage,
//             &State {
//                 end_time: Expiration::AtHeight(mock_env().block.height + 1),
//                 price: coin(100, "uusd"),
//                 min_tokens_sold: Uint128::from(1u128),
//                 max_amount_per_wallet: 5,
//                 amount_sold: Uint128::zero(),
//                 amount_to_send: Uint128::zero(),
//                 amount_transferred: Uint128::zero(),
//                 recipient: Recipient::from_string("recipient"),
//             },
//         )
//         .unwrap();

//     let info = mock_info("sender", &coins(150, "uusd"));

//     let msg = ExecuteMsg::PurchaseByTokenId {
//         token_id: MOCK_TOKENS_FOR_SALE[1].to_owned(),
//     };
//     let res = execute(deps.as_mut(), mock_env(), info, msg);
//     assert_eq!(ContractError::TokenNotAvailable {}, res.unwrap_err());
// }

// #[test]
// fn test_purchase_by_token_id() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let modules = vec![Module {
//         name: Some(RATES.to_owned()),
//         address: AndrAddr::from_string(MOCK_RATES_CONTRACT.to_owned()),
//         is_mutable: false,
//     }];
//     init(deps.as_mut(), Some(modules));

//     mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[0]).unwrap();
//     mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[1]).unwrap();

//     let mut state = State {
//         end_time: Expiration::AtHeight(mock_env().block.height + 1),
//         price: coin(100, "uusd"),
//         min_tokens_sold: Uint128::from(1u128),
//         max_amount_per_wallet: 1,
//         amount_sold: Uint128::zero(),
//         amount_to_send: Uint128::zero(),
//         amount_transferred: Uint128::zero(),
//         recipient: Recipient::from_string("recipient"),
//     };

//     STATE.save(deps.as_mut().storage, &state).unwrap();

//     let info = mock_info("sender", &coins(150, "uusd"));

//     // Purchase a token.
//     let msg = ExecuteMsg::PurchaseByTokenId {
//         token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
//     };
//     let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
//     assert_eq!(
//         Response::new()
//             .add_attribute("action", "purchase")
//             .add_attribute("token_id", MOCK_TOKENS_FOR_SALE[0])
//             .add_submessage(generate_economics_message("sender", "PurchaseByTokenId")),
//         res
//     );

//     state.amount_to_send += Uint128::from(90u128);
//     state.amount_sold += Uint128::from(1u128);
//     assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[0]));
//     assert_eq!(
//         NUMBER_OF_TOKENS_AVAILABLE
//             .load(deps.as_ref().storage)
//             .unwrap(),
//         Uint128::new(1)
//     );

//     // Purchase a second one.
//     let msg = ExecuteMsg::PurchaseByTokenId {
//         token_id: MOCK_TOKENS_FOR_SALE[1].to_owned(),
//     };
//     let res = execute(deps.as_mut(), mock_env(), info, msg);

//     assert_eq!(ContractError::PurchaseLimitReached {}, res.unwrap_err());
// }

// #[test]
// fn test_multiple_purchases() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let modules = vec![Module {
//         name: Some(RATES.to_owned()),
//         address: AndrAddr::from_string(MOCK_RATES_CONTRACT.to_owned()),
//         is_mutable: false,
//     }];
//     init(deps.as_mut(), Some(modules));

//     // Mint four tokens.
//     mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[0]).unwrap();
//     mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[1]).unwrap();
//     mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[2]).unwrap();
//     mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[3]).unwrap();

//     // Query available tokens.
//     let msg = QueryMsg::AvailableTokens {
//         start_after: None,
//         limit: None,
//     };
//     let res: Vec<String> = from_json(query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
//     assert_eq!(
//         vec![
//             MOCK_TOKENS_FOR_SALE[0],
//             MOCK_TOKENS_FOR_SALE[1],
//             MOCK_TOKENS_FOR_SALE[2],
//             MOCK_TOKENS_FOR_SALE[3]
//         ],
//         res
//     );

//     // Query if individual token is available
//     let msg = QueryMsg::IsTokenAvailable {
//         id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
//     };
//     let res: IsTokenAvailableResponse =
//         from_json(query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
//     assert!(res.is_token_available);

//     // Query if another token is available
//     let msg = QueryMsg::IsTokenAvailable {
//         id: MOCK_TOKENS_FOR_SALE[4].to_owned(),
//     };
//     let res: IsTokenAvailableResponse =
//         from_json(query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
//     assert!(!res.is_token_available);

//     // Purchase 2 tokens
//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: Some(2),
//     };

//     let mut state = State {
//         end_time: Expiration::AtHeight(mock_env().block.height + 1),
//         price: coin(100, "uusd"),
//         min_tokens_sold: Uint128::from(1u128),
//         max_amount_per_wallet: 3,
//         amount_sold: Uint128::zero(),
//         amount_to_send: Uint128::zero(),
//         amount_transferred: Uint128::zero(),
//         recipient: Recipient::from_string("recipient"),
//     };
//     STATE.save(deps.as_mut().storage, &state).unwrap();

//     let info = mock_info("sender", &coins(300u128, "uusd"));
//     let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     assert_eq!(
//         Response::new()
//             .add_attribute("action", "purchase")
//             .add_attribute("number_of_tokens_wanted", "2")
//             .add_attribute("number_of_tokens_purchased", "2")
//             .add_submessage(generate_economics_message("sender", "Purchase")),
//         res
//     );

//     state.amount_to_send += Uint128::from(180u128);
//     state.amount_sold += Uint128::from(2u128);
//     assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[0]));
//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[1]));

//     assert_eq!(
//         vec![
//             get_purchase(MOCK_TOKENS_FOR_SALE[0], "sender"),
//             get_purchase(MOCK_TOKENS_FOR_SALE[1], "sender")
//         ],
//         PURCHASES.load(deps.as_ref().storage, "sender").unwrap()
//     );

//     // Purchase max number of tokens.
//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: None,
//     };

//     let info = mock_info("sender", &coins(300u128, "uusd"));
//     let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//     assert_eq!(
//         Response::new()
//             .add_message(BankMsg::Send {
//                 to_address: "sender".to_string(),
//                 // Refund sent back as they only were able to mint one.
//                 amount: coins(150, "uusd")
//             })
//             .add_attribute("action", "purchase")
//             .add_attribute("number_of_tokens_wanted", "1")
//             .add_attribute("number_of_tokens_purchased", "1")
//             .add_submessage(generate_economics_message("sender", "Purchase")),
//         res
//     );

//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[2]));
//     state.amount_to_send += Uint128::from(90u128);
//     state.amount_sold += Uint128::from(1u128);
//     assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

//     assert_eq!(
//         vec![
//             get_purchase(MOCK_TOKENS_FOR_SALE[0], "sender"),
//             get_purchase(MOCK_TOKENS_FOR_SALE[1], "sender"),
//             get_purchase(MOCK_TOKENS_FOR_SALE[2], "sender")
//         ],
//         PURCHASES.load(deps.as_ref().storage, "sender").unwrap()
//     );

//     // Try to purchase an additional token when limit has already been reached.
//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: Some(1),
//     };

//     let res = execute(deps.as_mut(), mock_env(), info, msg);

//     assert_eq!(ContractError::PurchaseLimitReached {}, res.unwrap_err());

//     // User 2 tries to purchase 2 but only 1 is left.
//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: Some(2),
//     };

//     let info = mock_info("user2", &coins(300, "uusd"));
//     let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     assert_eq!(
//         Response::new()
//             .add_message(BankMsg::Send {
//                 to_address: "user2".to_string(),
//                 // Refund sent back as they only were able to mint one.
//                 amount: coins(150, "uusd")
//             })
//             .add_attribute("action", "purchase")
//             .add_attribute("number_of_tokens_wanted", "2")
//             .add_attribute("number_of_tokens_purchased", "1")
//             .add_submessage(generate_economics_message("user2", "Purchase")),
//         res
//     );

//     assert_eq!(
//         vec![get_purchase(MOCK_TOKENS_FOR_SALE[3], "user2"),],
//         PURCHASES.load(deps.as_ref().storage, "user2").unwrap()
//     );
//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[3]));
//     state.amount_to_send += Uint128::from(90u128);
//     state.amount_sold += Uint128::from(1u128);
//     assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

//     assert_eq!(
//         NUMBER_OF_TOKENS_AVAILABLE
//             .load(deps.as_ref().storage)
//             .unwrap(),
//         Uint128::zero()
//     );

//     // User 2 tries to purchase again.
//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: None,
//     };

//     let info = mock_info("user2", &coins(150, "uusd"));
//     let res = execute(deps.as_mut(), mock_env(), info, msg);

//     assert_eq!(ContractError::AllTokensPurchased {}, res.unwrap_err());
// }

// #[test]
// fn test_purchase_more_than_allowed_per_wallet() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let modules = vec![Module {
//         name: Some(RATES.to_owned()),
//         address: AndrAddr::from_string(MOCK_RATES_CONTRACT.to_owned()),
//         is_mutable: false,
//     }];
//     init(deps.as_mut(), Some(modules));

//     // Mint four tokens.
//     mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[0]).unwrap();
//     mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[1]).unwrap();
//     mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[2]).unwrap();
//     mint(deps.as_mut(), MOCK_TOKENS_FOR_SALE[3]).unwrap();

//     // Try to purchase 4
//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: Some(4),
//     };

//     let state = State {
//         end_time: Expiration::AtHeight(mock_env().block.height + 1),
//         price: coin(100, "uusd"),
//         min_tokens_sold: Uint128::from(1u128),
//         max_amount_per_wallet: 3,
//         amount_sold: Uint128::zero(),
//         amount_to_send: Uint128::zero(),
//         amount_transferred: Uint128::zero(),
//         recipient: Recipient::from_string("recipient"),
//     };
//     STATE.save(deps.as_mut().storage, &state).unwrap();

//     let info = mock_info("sender", &coins(600, "uusd"));
//     let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     assert_eq!(
//         Response::new()
//             .add_message(BankMsg::Send {
//                 to_address: "sender".to_string(),
//                 amount: coins(150, "uusd")
//             })
//             .add_attribute("action", "purchase")
//             // Number got truncated to 3 which is the max possible.
//             .add_attribute("number_of_tokens_wanted", "3")
//             .add_attribute("number_of_tokens_purchased", "3")
//             .add_submessage(generate_economics_message("sender", "Purchase")),
//         res
//     );
// }

// #[test]
// fn test_end_sale_not_expired() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);

//     let state = State {
//         end_time: Expiration::AtHeight(mock_env().block.height + 1),
//         price: coin(100, "uusd"),
//         min_tokens_sold: Uint128::from(1u128),
//         max_amount_per_wallet: 2,
//         amount_sold: Uint128::zero(),
//         amount_to_send: Uint128::zero(),
//         amount_transferred: Uint128::zero(),
//         recipient: Recipient::from_string("recipient"),
//     };
//     STATE.save(deps.as_mut().storage, &state).unwrap();
//     NUMBER_OF_TOKENS_AVAILABLE
//         .save(deps.as_mut().storage, &Uint128::new(1))
//         .unwrap();

//     let msg = ExecuteMsg::EndSale { limit: None };
//     let info = mock_info("anyone", &[]);
//     let res = execute(deps.as_mut(), mock_env(), info, msg);
//     assert_eq!(ContractError::SaleNotEnded {}, res.unwrap_err());
// }

// fn mint(deps: DepsMut, token_id: impl Into<String>) -> Result<Response, ContractError> {
//     let msg = ExecuteMsg::Mint(vec![CrowdfundMintMsg {
//         token_id: token_id.into(),
//         owner: None,
//         token_uri: None,
//         extension: TokenExtension {
//             publisher: "publisher".to_string(),
//         },
//     }]);
//     execute(deps, mock_env(), mock_info(MOCK_DEFAULT_OWNER, &[]), msg)
// }

// #[test]
// fn test_integration_conditions_not_met() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let modules = vec![Module {
//         name: Some(RATES.to_owned()),
//         address: AndrAddr::from_string(MOCK_RATES_CONTRACT.to_owned()),
//         is_mutable: false,
//     }];
//     init(deps.as_mut(), Some(modules));

//     // Mint all tokens.
//     for &token_id in MOCK_TOKENS_FOR_SALE {
//         let _res = mint(deps.as_mut(), token_id).unwrap();
//         assert!(AVAILABLE_TOKENS.has(deps.as_ref().storage, token_id));
//     }

//     assert_eq!(
//         NUMBER_OF_TOKENS_AVAILABLE
//             .load(deps.as_ref().storage)
//             .unwrap(),
//         Uint128::new(7)
//     );
//     let current_time = mock_env().block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;

//     let msg = ExecuteMsg::StartSale {
//         start_time: None,
//         end_time: Expiry::AtTime(Milliseconds::from_nanos((current_time + 2) * 1_000_000)),
//         price: coin(100, "uusd"),
//         min_tokens_sold: Uint128::from(5u128),
//         max_amount_per_wallet: Some(2),
//         recipient: Recipient::from_string("recipient"),
//     };

//     let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
//     let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     // Can't mint once sale started.
//     let res = mint(deps.as_mut(), "token_id");
//     assert_eq!(ContractError::SaleStarted {}, res.unwrap_err());

//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: Some(1),
//     };
//     let info = mock_info("A", &coins(150, "uusd"));
//     let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: Some(1),
//     };
//     let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: Some(1),
//     };
//     let info = mock_info("B", &coins(150, "uusd"));
//     let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: Some(1),
//     };
//     let info = mock_info("C", &coins(150, "uusd"));
//     let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     // Using current time since start time wasn't provided
//     let current_time = mock_env().block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;
//     let end_expiration = expiration_from_milliseconds(Milliseconds(current_time + 2)).unwrap();

//     let state = State {
//         end_time: end_expiration,
//         price: coin(100, "uusd"),
//         min_tokens_sold: Uint128::from(5u128),
//         max_amount_per_wallet: 2,
//         amount_sold: Uint128::from(4u128),
//         amount_to_send: Uint128::from(360u128),
//         amount_transferred: Uint128::zero(),
//         recipient: Recipient::from_string("recipient"),
//     };
//     assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

//     assert_eq!(
//         vec![
//             get_purchase(MOCK_TOKENS_FOR_SALE[0], "A"),
//             get_purchase(MOCK_TOKENS_FOR_SALE[1], "A")
//         ],
//         PURCHASES.load(deps.as_ref().storage, "A").unwrap()
//     );

//     assert_eq!(
//         vec![get_purchase(MOCK_TOKENS_FOR_SALE[2], "B"),],
//         PURCHASES.load(deps.as_ref().storage, "B").unwrap()
//     );

//     assert_eq!(
//         vec![get_purchase(MOCK_TOKENS_FOR_SALE[3], "C"),],
//         PURCHASES.load(deps.as_ref().storage, "C").unwrap()
//     );
//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[0]));
//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[1]));
//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[2]));
//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[3]));

//     assert_eq!(
//         NUMBER_OF_TOKENS_AVAILABLE
//             .load(deps.as_ref().storage)
//             .unwrap(),
//         Uint128::new(3)
//     );

//     let mut env = mock_env();
//     env.block.time = env.block.time.plus_hours(1);

//     // User B claims their own refund.
//     let msg = ExecuteMsg::ClaimRefund {};
//     let info = mock_info("B", &[]);
//     let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
//     assert_eq!(
//         Response::new()
//             .add_attribute("action", "claim_refund")
//             .add_message(CosmosMsg::Bank(BankMsg::Send {
//                 to_address: "B".to_string(),
//                 amount: coins(150, "uusd"),
//             }))
//             .add_submessage(generate_economics_message("B", "ClaimRefund")),
//         res
//     );

//     assert!(!PURCHASES.has(deps.as_ref().storage, "B"));

//     env.contract.address = Addr::unchecked(MOCK_CONDITIONS_NOT_MET_CONTRACT);
//     deps.querier.tokens_left_to_burn = 7;
//     let msg = ExecuteMsg::EndSale { limit: None };
//     let info = mock_info("anyone", &[]);
//     let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
//     let refund_msgs: Vec<CosmosMsg> = vec![
//         // All of A's payments grouped into one message.
//         CosmosMsg::Bank(BankMsg::Send {
//             to_address: "A".to_string(),
//             amount: coins(300, "uusd"),
//         }),
//         CosmosMsg::Bank(BankMsg::Send {
//             to_address: "C".to_string(),
//             amount: coins(150, "uusd"),
//         }),
//     ];
//     let burn_msgs: Vec<CosmosMsg> = vec![
//         get_burn_message(MOCK_TOKENS_FOR_SALE[0]),
//         get_burn_message(MOCK_TOKENS_FOR_SALE[1]),
//         get_burn_message(MOCK_TOKENS_FOR_SALE[2]),
//         get_burn_message(MOCK_TOKENS_FOR_SALE[3]),
//         // Tokens that were not sold.
//         get_burn_message(MOCK_TOKENS_FOR_SALE[4]),
//         get_burn_message(MOCK_TOKENS_FOR_SALE[5]),
//         get_burn_message(MOCK_TOKENS_FOR_SALE[6]),
//     ];

//     assert_eq!(
//         Response::new()
//             .add_attribute("action", "issue_refunds_and_burn_tokens")
//             .add_messages(refund_msgs)
//             .add_messages(burn_msgs)
//             .add_submessage(generate_economics_message("anyone", "EndSale")),
//         res
//     );

//     assert!(!PURCHASES.has(deps.as_ref().storage, "A"));
//     assert!(!PURCHASES.has(deps.as_ref().storage, "C"));

//     // Burned tokens have been removed.
//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[4]));
//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[5]));
//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[6]));

//     deps.querier.tokens_left_to_burn = 0;
//     let _res = execute(deps.as_mut(), env, info, msg).unwrap();
//     assert!(STATE.may_load(deps.as_mut().storage).unwrap().is_none());
//     assert_eq!(
//         NUMBER_OF_TOKENS_AVAILABLE
//             .load(deps.as_ref().storage)
//             .unwrap(),
//         Uint128::zero()
//     );
// }

// #[test]
// fn test_integration_conditions_met() {
//     let mut deps = mock_dependencies_custom(&[]);
//     deps.querier.contract_address = MOCK_CONDITIONS_MET_CONTRACT.to_string();
//     let modules = vec![Module {
//         name: Some(RATES.to_owned()),
//         address: AndrAddr::from_string(MOCK_RATES_CONTRACT.to_owned()),
//         is_mutable: false,
//     }];
//     init(deps.as_mut(), Some(modules));
//     let mut env = mock_env();
//     env.contract.address = Addr::unchecked(MOCK_CONDITIONS_MET_CONTRACT);

//     // Mint all tokens.
//     for &token_id in MOCK_TOKENS_FOR_SALE {
//         let _res = mint(deps.as_mut(), token_id).unwrap();
//         assert!(AVAILABLE_TOKENS.has(deps.as_ref().storage, token_id));
//     }
//     let current_time = mock_env().block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;

//     let msg = ExecuteMsg::StartSale {
//         start_time: None,
//         end_time: Expiry::AtTime(Milliseconds::from_nanos((current_time + 2) * 1_000_000)),
//         price: coin(100, "uusd"),
//         min_tokens_sold: Uint128::from(3u128),
//         max_amount_per_wallet: Some(2),
//         recipient: Recipient::from_string("recipient"),
//     };

//     let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
//     let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: Some(1),
//     };
//     let info = mock_info("A", &coins(150, "uusd"));
//     let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: Some(1),
//     };
//     let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: Some(1),
//     };
//     let info = mock_info("B", &coins(150, "uusd"));
//     let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: Some(1),
//     };
//     let info = mock_info("C", &coins(150, "uusd"));
//     let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: Some(1),
//     };
//     let info = mock_info("D", &coins(150, "uusd"));
//     let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
//     // Using current time since start time wasn't provided
//     let current_time = env.block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;
//     let end_expiration = expiration_from_milliseconds(Milliseconds(current_time + 2)).unwrap();
//     let mut state = State {
//         end_time: end_expiration,
//         price: coin(100, "uusd"),
//         min_tokens_sold: Uint128::from(3u128),
//         max_amount_per_wallet: 2,
//         amount_sold: Uint128::from(5u128),
//         amount_to_send: Uint128::from(450u128),
//         amount_transferred: Uint128::zero(),
//         recipient: Recipient::from_string("recipient"),
//     };
//     assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

//     assert_eq!(
//         vec![
//             get_purchase(MOCK_TOKENS_FOR_SALE[0], "A"),
//             get_purchase(MOCK_TOKENS_FOR_SALE[1], "A")
//         ],
//         PURCHASES.load(deps.as_ref().storage, "A").unwrap()
//     );

//     assert_eq!(
//         vec![get_purchase(MOCK_TOKENS_FOR_SALE[2], "B"),],
//         PURCHASES.load(deps.as_ref().storage, "B").unwrap()
//     );
//     assert_eq!(
//         vec![get_purchase(MOCK_TOKENS_FOR_SALE[3], "C"),],
//         PURCHASES.load(deps.as_ref().storage, "C").unwrap()
//     );
//     assert_eq!(
//         vec![get_purchase(MOCK_TOKENS_FOR_SALE[4], "D"),],
//         PURCHASES.load(deps.as_ref().storage, "D").unwrap()
//     );
//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[0]));
//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[1]));
//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[2]));
//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[3]));
//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[4]));

//     env.block.time = env.block.time.plus_hours(1);
//     env.contract.address = Addr::unchecked(MOCK_CONDITIONS_MET_CONTRACT);

//     let msg = ExecuteMsg::EndSale { limit: Some(1) };
//     let info = mock_info("anyone", &[]);
//     let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

//     assert_eq!(
//         Response::new()
//             .add_attribute("action", "transfer_tokens_and_send_funds")
//             .add_message(get_transfer_message(
//                 MOCK_TOKENS_FOR_SALE[0],
//                 AndrAddr::from_string("A")
//             ))
//             .add_submessages(get_rates_messages())
//             .add_submessage(generate_economics_message("anyone", "EndSale")),
//         res
//     );

//     assert_eq!(
//         vec![get_purchase(MOCK_TOKENS_FOR_SALE[1], "A")],
//         PURCHASES.load(deps.as_ref().storage, "A").unwrap()
//     );

//     state.amount_transferred += Uint128::from(1u128);
//     assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

//     let msg = ExecuteMsg::EndSale { limit: Some(2) };
//     let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

//     assert_eq!(
//         Response::new()
//             .add_attribute("action", "transfer_tokens_and_send_funds")
//             .add_message(get_transfer_message(
//                 MOCK_TOKENS_FOR_SALE[1],
//                 AndrAddr::from_string("A")
//             ))
//             .add_message(get_transfer_message(
//                 MOCK_TOKENS_FOR_SALE[2],
//                 AndrAddr::from_string("B")
//             ))
//             .add_message(CosmosMsg::Bank(BankMsg::Send {
//                 to_address: MOCK_ROYALTY_RECIPIENT.to_owned(),
//                 amount: vec![Coin {
//                     // Royalty of 10% for A and B combined
//                     amount: Uint128::from(20u128),
//                     denom: "uusd".to_string(),
//                 }],
//             }))
//             .add_message(CosmosMsg::Bank(BankMsg::Send {
//                 to_address: MOCK_TAX_RECIPIENT.to_owned(),
//                 amount: vec![Coin {
//                     // Combined tax for both A and B
//                     amount: Uint128::from(100u128),
//                     denom: "uusd".to_string(),
//                 }],
//             }))
//             .add_submessage(generate_economics_message("anyone", "EndSale")),
//         res
//     );

//     assert!(!PURCHASES.has(deps.as_ref().storage, "A"),);
//     assert!(!PURCHASES.has(deps.as_ref().storage, "B"),);
//     assert!(PURCHASES.has(deps.as_ref().storage, "C"),);
//     assert!(PURCHASES.has(deps.as_ref().storage, "D"),);

//     state.amount_transferred += Uint128::from(2u128);
//     assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

//     let msg = ExecuteMsg::EndSale { limit: None };
//     let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

//     assert!(!PURCHASES.has(deps.as_ref().storage, "C"),);
//     assert!(!PURCHASES.has(deps.as_ref().storage, "D"),);

//     assert_eq!(
//         Response::new()
//             .add_attribute("action", "transfer_tokens_and_send_funds")
//             .add_message(get_transfer_message(
//                 MOCK_TOKENS_FOR_SALE[3],
//                 AndrAddr::from_string("C")
//             ))
//             .add_message(get_transfer_message(
//                 MOCK_TOKENS_FOR_SALE[4],
//                 AndrAddr::from_string("D")
//             ))
//             .add_message(CosmosMsg::Bank(BankMsg::Send {
//                 to_address: MOCK_ROYALTY_RECIPIENT.to_owned(),
//                 amount: vec![Coin {
//                     // Royalty of 10% for C and D combined
//                     amount: Uint128::from(20u128),
//                     denom: "uusd".to_string(),
//                 }],
//             }))
//             .add_message(CosmosMsg::Bank(BankMsg::Send {
//                 to_address: MOCK_TAX_RECIPIENT.to_owned(),
//                 amount: vec![Coin {
//                     // Combined tax for both C and D
//                     amount: Uint128::from(100u128),
//                     denom: "uusd".to_string(),
//                 }],
//             }))
//             .add_submessage(generate_economics_message("anyone", "EndSale")),
//         res
//     );

//     state.amount_transferred += Uint128::from(2u128);
//     assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

//     let msg = ExecuteMsg::EndSale { limit: None };
//     let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
//     // Added one for economics message
//     assert_eq!(3 + 1, res.messages.len());

//     // assert_eq!(
//     //     Response::new()
//     //         .add_attribute("action", "transfer_tokens_and_send_funds")
//     //         // Now that all tokens have been transfered, can send the funds to recipient.
//     //         .add_message(CosmosMsg::Bank(BankMsg::Send {
//     //             to_address: "recipient".to_string(),
//     //             amount: coins(450u128, "uusd")
//     //         }))
//     //         // Burn tokens that were not purchased
//     //         .add_message(get_burn_message(MOCK_TOKENS_FOR_SALE[5]))
//     //         .add_message(get_burn_message(MOCK_TOKENS_FOR_SALE[6])),
//     //     res
//     // );

//     state.amount_to_send = Uint128::zero();
//     assert_eq!(state, STATE.load(deps.as_ref().storage).unwrap());

//     // Burned tokens removed.
//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[5]));
//     assert!(!AVAILABLE_TOKENS.has(deps.as_ref().storage, MOCK_TOKENS_FOR_SALE[6]));

//     deps.querier.tokens_left_to_burn = 0;
//     let _res = execute(deps.as_mut(), env, info, msg).unwrap();
//     assert!(STATE.may_load(deps.as_mut().storage).unwrap().is_none());
//     assert_eq!(
//         NUMBER_OF_TOKENS_AVAILABLE
//             .load(deps.as_ref().storage)
//             .unwrap(),
//         Uint128::zero()
//     );
// }

// #[test]
// fn test_end_sale_single_purchase() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);

//     STATE
//         .save(
//             deps.as_mut().storage,
//             &State {
//                 end_time: Expiration::AtHeight(mock_env().block.height - 1),
//                 price: coin(100, "uusd"),
//                 min_tokens_sold: Uint128::from(1u128),
//                 max_amount_per_wallet: 5,
//                 amount_sold: Uint128::from(1u128),
//                 amount_to_send: Uint128::from(100u128),
//                 amount_transferred: Uint128::zero(),
//                 recipient: Recipient::from_string("recipient"),
//             },
//         )
//         .unwrap();

//     PURCHASES
//         .save(
//             deps.as_mut().storage,
//             "A",
//             &vec![Purchase {
//                 token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
//                 purchaser: "A".to_string(),
//                 tax_amount: Uint128::zero(),
//                 msgs: vec![],
//             }],
//         )
//         .unwrap();

//     let msg = ExecuteMsg::EndSale { limit: None };
//     let info = mock_info("anyone", &[]);
//     let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     assert_eq!(
//         Response::new()
//             .add_attribute("action", "transfer_tokens_and_send_funds")
//             // Burn tokens that were not purchased
//             .add_message(get_transfer_message(
//                 MOCK_TOKENS_FOR_SALE[0],
//                 AndrAddr::from_string("A")
//             ))
//             .add_submessage(generate_economics_message("anyone", "EndSale")),
//         res
//     );
// }

// #[test]
// fn test_end_sale_all_tokens_sold() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);

//     STATE
//         .save(
//             deps.as_mut().storage,
//             &State {
//                 // Sale has not expired yet.
//                 end_time: Expiration::AtHeight(mock_env().block.height + 1),
//                 price: coin(100, "uusd"),
//                 min_tokens_sold: Uint128::from(1u128),
//                 max_amount_per_wallet: 5,
//                 amount_sold: Uint128::from(1u128),
//                 amount_to_send: Uint128::from(100u128),
//                 amount_transferred: Uint128::zero(),
//                 recipient: Recipient::from_string("recipient"),
//             },
//         )
//         .unwrap();

//     PURCHASES
//         .save(
//             deps.as_mut().storage,
//             "A",
//             &vec![Purchase {
//                 token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
//                 purchaser: "A".to_string(),
//                 tax_amount: Uint128::zero(),
//                 msgs: vec![],
//             }],
//         )
//         .unwrap();

//     NUMBER_OF_TOKENS_AVAILABLE
//         .save(deps.as_mut().storage, &Uint128::zero())
//         .unwrap();

//     let msg = ExecuteMsg::EndSale { limit: None };
//     let info = mock_info("anyone", &[]);
//     let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     assert_eq!(
//         Response::new()
//             .add_attribute("action", "transfer_tokens_and_send_funds")
//             // Burn tokens that were not purchased
//             .add_message(get_transfer_message(
//                 MOCK_TOKENS_FOR_SALE[0],
//                 AndrAddr::from_string("A")
//             ))
//             .add_submessage(generate_economics_message("anyone", "EndSale")),
//         res
//     );
// }

// #[test]
// fn test_end_sale_some_tokens_sold_threshold_met() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);

//     STATE
//         .save(
//             deps.as_mut().storage,
//             &State {
//                 // Sale has not expired yet.
//                 end_time: Expiration::AtHeight(mock_env().block.height + 1),
//                 price: coin(100, "uusd"),
//                 min_tokens_sold: Uint128::from(1u128),
//                 max_amount_per_wallet: 5,
//                 amount_sold: Uint128::from(2u128),
//                 amount_to_send: Uint128::from(100u128),
//                 amount_transferred: Uint128::zero(),
//                 recipient: Recipient::from_string("recipient"),
//             },
//         )
//         .unwrap();

//     PURCHASES
//         .save(
//             deps.as_mut().storage,
//             "A",
//             &vec![Purchase {
//                 token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
//                 purchaser: "A".to_string(),
//                 tax_amount: Uint128::zero(),
//                 msgs: vec![],
//             }],
//         )
//         .unwrap();

//     NUMBER_OF_TOKENS_AVAILABLE
//         .save(deps.as_mut().storage, &Uint128::one())
//         .unwrap();

//     let msg = ExecuteMsg::EndSale { limit: None };
//     // Only the owner can end the sale if only the minimum token threshold is met.
//     // Anyone can end the sale if it's expired or the remaining number of tokens available is zero.
//     let info = mock_info("anyone", &[]);
//     let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
//     assert_eq!(err, ContractError::SaleNotEnded {});

//     let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
//     let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     assert_eq!(
//         Response::new()
//             .add_attribute("action", "transfer_tokens_and_send_funds")
//             // Burn tokens that were not purchased
//             .add_message(get_transfer_message(
//                 MOCK_TOKENS_FOR_SALE[0],
//                 AndrAddr::from_string("A")
//             ))
//             .add_submessage(generate_economics_message(MOCK_DEFAULT_OWNER, "EndSale")),
//         res
//     );
// }

// #[test]
// fn test_end_sale_some_tokens_sold_threshold_not_met() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);

//     STATE
//         .save(
//             deps.as_mut().storage,
//             &State {
//                 // Sale has not expired yet.
//                 end_time: Expiration::AtHeight(mock_env().block.height + 1),
//                 price: coin(100, "uusd"),
//                 min_tokens_sold: Uint128::from(2u128),
//                 max_amount_per_wallet: 5,
//                 amount_sold: Uint128::from(0u128),
//                 amount_to_send: Uint128::from(100u128),
//                 amount_transferred: Uint128::zero(),
//                 recipient: Recipient::from_string("recipient"),
//             },
//         )
//         .unwrap();

//     PURCHASES
//         .save(
//             deps.as_mut().storage,
//             "A",
//             &vec![Purchase {
//                 token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
//                 purchaser: "A".to_string(),
//                 tax_amount: Uint128::zero(),
//                 msgs: vec![],
//             }],
//         )
//         .unwrap();

//     NUMBER_OF_TOKENS_AVAILABLE
//         .save(deps.as_mut().storage, &Uint128::new(2))
//         .unwrap();

//     let msg = ExecuteMsg::EndSale { limit: None };

//     let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
//     // Minimum sold is 2, actual sold is 0
//     let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
//     assert_eq!(err, ContractError::SaleNotEnded {});
// }

// #[test]
// fn test_end_sale_limit_zero() {
//     let mut deps = mock_dependencies_custom(&[]);
//     init(deps.as_mut(), None);

//     STATE
//         .save(
//             deps.as_mut().storage,
//             &State {
//                 end_time: Expiration::AtHeight(mock_env().block.height - 1),
//                 price: coin(100, "uusd"),
//                 min_tokens_sold: Uint128::from(1u128),
//                 max_amount_per_wallet: 5,
//                 amount_sold: Uint128::from(1u128),
//                 amount_to_send: Uint128::from(100u128),
//                 amount_transferred: Uint128::zero(),
//                 recipient: Recipient::from_string("recipient"),
//             },
//         )
//         .unwrap();
//     NUMBER_OF_TOKENS_AVAILABLE
//         .save(deps.as_mut().storage, &Uint128::new(1))
//         .unwrap();

//     PURCHASES
//         .save(
//             deps.as_mut().storage,
//             "A",
//             &vec![Purchase {
//                 token_id: MOCK_TOKENS_FOR_SALE[0].to_owned(),
//                 purchaser: "A".to_string(),
//                 tax_amount: Uint128::zero(),
//                 msgs: vec![],
//             }],
//         )
//         .unwrap();

//     let msg = ExecuteMsg::EndSale { limit: Some(0) };
//     let info = mock_info("anyone", &[]);
//     let res = execute(deps.as_mut(), mock_env(), info, msg);

//     assert_eq!(ContractError::LimitMustNotBeZero {}, res.unwrap_err());
// }

// #[test]
// fn test_validate_andr_addresses_regular_address() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let msg = InstantiateMsg {
//         token_address: AndrAddr::from_string("terra1asdf1ssdfadf".to_owned()),
//         owner: None,
//         modules: None,
//         can_mint_after_sale: true,
//         kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
//     };

//     let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
//     let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//     let msg = ExecuteMsg::UpdateAppContract {
//         address: MOCK_APP_CONTRACT.to_owned(),
//     };

//     let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     assert_eq!(
//         Response::new()
//             .add_attribute("action", "update_app_contract")
//             .add_attribute("address", MOCK_APP_CONTRACT)
//             .add_submessage(generate_economics_message(MOCK_DEFAULT_OWNER, "UpdateAppContract")),
//         res
//     );
// }

// #[test]
// fn test_addresslist() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let modules = vec![Module {
//         name: Some(ADDRESS_LIST.to_owned()),
//         address: AndrAddr::from_string(MOCK_ADDRESS_LIST_CONTRACT.to_owned()),
//         is_mutable: false,
//     }];
//     let msg = InstantiateMsg {
//         token_address: AndrAddr::from_string(MOCK_TOKEN_CONTRACT.to_owned()),
//         modules: Some(modules),
//         can_mint_after_sale: true,
//         owner: None,
//         kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
//     };

//     let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
//     let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//     // Not whitelisted user
//     let msg = ExecuteMsg::Purchase {
//         number_of_tokens: None,
//     };
//     let info = mock_info("not_whitelisted", &[]);
//     let res = execute(deps.as_mut(), mock_env(), info, msg);

//     assert_eq!(
//         ContractError::Std(StdError::generic_err(
//             "Querier contract error: InvalidAddress"
//         )),
//         res.unwrap_err()
//     );
// }

// #[test]
// fn test_update_token_contract() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let msg = InstantiateMsg {
//         token_address: AndrAddr::from_string(MOCK_TOKEN_CONTRACT.to_owned()),
//         modules: None,
//         can_mint_after_sale: true,
//         owner: None,
//         kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
//     };

//     let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
//     let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//     let msg = ExecuteMsg::UpdateTokenContract {
//         address: AndrAddr::from_string(MOCK_TOKEN_CONTRACT.to_owned()),
//     };

//     let res = execute(deps.as_mut(), mock_env(), info, msg);
//     assert!(res.is_ok())
// }

// #[test]
// fn test_update_token_contract_unauthorized() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let msg = InstantiateMsg {
//         token_address: AndrAddr::from_string(MOCK_TOKEN_CONTRACT.to_owned()),
//         modules: None,
//         can_mint_after_sale: true,
//         owner: None,
//         kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
//     };

//     let info = mock_info("app_contract", &[]);
//     let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//     let msg = ExecuteMsg::UpdateTokenContract {
//         address: AndrAddr::from_string(MOCK_TOKEN_CONTRACT.to_owned()),
//     };

//     let unauth_info = mock_info("attacker", &[]);
//     let res = execute(deps.as_mut(), mock_env(), unauth_info, msg).unwrap_err();
//     assert_eq!(ContractError::Unauthorized {}, res);
// }

// #[test]
// fn test_update_token_contract_post_mint() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let msg = InstantiateMsg {
//         token_address: AndrAddr::from_string(MOCK_TOKEN_CONTRACT.to_owned()),
//         modules: None,
//         can_mint_after_sale: true,
//         owner: None,
//         kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
//     };

//     let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
//     let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//     mint(deps.as_mut(), "1").unwrap();

//     let msg = ExecuteMsg::UpdateTokenContract {
//         address: AndrAddr::from_string(MOCK_TOKEN_CONTRACT.to_owned()),
//     };

//     let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
//     assert_eq!(ContractError::Unauthorized {}, res);
// }

// #[test]
// fn test_update_token_contract_not_cw721() {
//     let mut deps = mock_dependencies_custom(&[]);
//     let msg = InstantiateMsg {
//         token_address: AndrAddr::from_string(MOCK_TOKEN_CONTRACT.to_owned()),
//         modules: None,
//         can_mint_after_sale: true,
//         owner: None,
//         kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
//     };

//     let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
//     let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//     let msg = ExecuteMsg::UpdateTokenContract {
//         address: AndrAddr::from_string("not_a_token_contract".to_owned()),
//     };

//     let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
//     assert_eq!(ContractError::Unauthorized {}, res);
// }
