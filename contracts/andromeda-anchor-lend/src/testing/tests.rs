use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    coins,
    testing::{mock_env, mock_info},
    to_binary, Addr, BankMsg, CosmosMsg, DepsMut, Response, Uint128, WasmMsg,
};

use crate::contract::{execute, instantiate};
use crate::primitive_keys::{
    ANCHOR_BLUNA, ANCHOR_BLUNA_CUSTODY, ANCHOR_BLUNA_HUB, ANCHOR_MARKET, ANCHOR_ORACLE,
    ANCHOR_OVERSEER,
};
use crate::testing::mock_querier::{
    mock_dependencies_custom, MOCK_ANC_TOKEN, MOCK_BLUNA_HUB_CONTRACT, MOCK_BLUNA_TOKEN,
    MOCK_CUSTODY_CONTRACT, MOCK_GOV_CONTRACT, MOCK_MARKET_CONTRACT, MOCK_ORACLE_CONTRACT,
    MOCK_OVERSEER_CONTRACT, MOCK_PRIMITIVE_CONTRACT,
};
use ado_base::ADOContract;
use anchor_token::gov::{Cw20HookMsg as GovCw20HookMsg, ExecuteMsg as GovExecuteMsg};
use andromeda_protocol::anchor_lend::{
    BLunaHubCw20HookMsg, BLunaHubExecuteMsg, Cw20HookMsg, ExecuteMsg, InstantiateMsg,
};
use common::{
    ado_base::{recipient::Recipient, AndromedaMsg},
    error::ContractError,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_asset::AssetInfo;
use moneymarket::{
    custody::{Cw20HookMsg as CustodyCw20HookMsg, ExecuteMsg as CustodyExecuteMsg},
    market::ExecuteMsg as MarketExecuteMsg,
    overseer::ExecuteMsg as OverseerExecuteMsg,
};

fn init(deps: DepsMut) {
    let env = mock_env();
    let owner = "owner";
    let info = mock_info(owner, &[]);
    let msg = InstantiateMsg {
        primitive_contract: MOCK_PRIMITIVE_CONTRACT.to_owned(),
    };
    let res = instantiate(deps, env, info, msg).unwrap();

    assert_eq!(0, res.messages.len());
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let contract = ADOContract::default();

    assert_eq!(
        contract
            .withdrawable_tokens
            .load(deps.as_ref().storage, MOCK_ANC_TOKEN)
            .unwrap(),
        AssetInfo::Cw20(Addr::unchecked(MOCK_ANC_TOKEN))
    );

    assert_eq!(
        MOCK_MARKET_CONTRACT,
        contract
            .get_cached_address(deps.as_ref().storage, ANCHOR_MARKET)
            .unwrap()
    );
    assert_eq!(
        MOCK_OVERSEER_CONTRACT,
        contract
            .get_cached_address(deps.as_ref().storage, ANCHOR_OVERSEER)
            .unwrap()
    );
    assert_eq!(
        MOCK_BLUNA_HUB_CONTRACT,
        contract
            .get_cached_address(deps.as_ref().storage, ANCHOR_BLUNA_HUB)
            .unwrap()
    );
    assert_eq!(
        MOCK_CUSTODY_CONTRACT,
        contract
            .get_cached_address(deps.as_ref().storage, ANCHOR_BLUNA_CUSTODY)
            .unwrap()
    );
    assert_eq!(
        MOCK_ORACLE_CONTRACT,
        contract
            .get_cached_address(deps.as_ref().storage, ANCHOR_ORACLE)
            .unwrap()
    );
    assert_eq!(
        MOCK_BLUNA_TOKEN,
        contract
            .get_cached_address(deps.as_ref().storage, ANCHOR_BLUNA)
            .unwrap()
    );
}

#[test]
fn test_withdraw_tokens_none() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &[]);

    let recipient = "recipient";

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Withdraw {
        recipient: Some(Recipient::Addr(recipient.to_owned())),
        tokens_to_withdraw: None,
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        Response::new()
            .add_attribute("action", "withdraw")
            .add_attribute("recipient", "Addr(\"recipient\")")
            .add_message(WasmMsg::Execute {
                contract_addr: MOCK_ANC_TOKEN.to_owned(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    amount: Uint128::new(100),
                    recipient: recipient.to_owned()
                })
                .unwrap()
            }),
        res
    );
}

#[test]
fn test_withdraw_tokens_empty() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &[]);

    let recipient = "recipient";

    let msg = ExecuteMsg::AndrReceive(AndromedaMsg::Withdraw {
        recipient: Some(Recipient::Addr(recipient.to_owned())),
        tokens_to_withdraw: Some(vec![]),
    });
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(
        ContractError::InvalidFunds {
            msg: "No funds to withdraw".to_string(),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_deposit_collateral() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::DepositCollateral {};

    let info = mock_info("owner", &coins(100, "uluna"));
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "deposit_collateral")
            // Convert luna -> bluna
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_BLUNA_HUB_CONTRACT.to_owned(),
                funds: info.funds,
                msg: to_binary(&BLunaHubExecuteMsg::Bond {}).unwrap(),
            }))
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: mock_env().contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::DepositCollateralToAnchor {
                    collateral_addr: MOCK_BLUNA_TOKEN.to_owned()
                })
                .unwrap(),
                funds: vec![],
            })),
        res
    );
}

#[test]
fn test_deposit_collateral_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::DepositCollateral {};

    let info = mock_info("anyone", &coins(100, "uluna"));
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_deposit_collateral_to_anchor() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::DepositCollateralToAnchor {
        collateral_addr: MOCK_BLUNA_TOKEN.to_owned(),
    };

    let info = mock_info(&mock_env().contract.address.to_string(), &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        Response::new()
            .add_attribute("action", "deposit_collateral_to_anchor")
            // Provide collateral
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_BLUNA_TOKEN.to_owned(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: MOCK_CUSTODY_CONTRACT.to_owned(),
                    msg: to_binary(&CustodyCw20HookMsg::DepositCollateral {}).unwrap(),
                    amount: Uint128::from(100u128),
                })
                .unwrap(),
            }))
            // Lock collateral
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_OVERSEER_CONTRACT.to_owned(),
                msg: to_binary(&OverseerExecuteMsg::LockCollateral {
                    collaterals: vec![(MOCK_BLUNA_TOKEN.to_owned(), Uint128::from(100u128).into())],
                })
                .unwrap(),
                funds: vec![],
            })),
        res
    );
}

#[test]
fn test_deposit_collateral_to_anchor_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::DepositCollateralToAnchor {
        collateral_addr: MOCK_BLUNA_TOKEN.to_owned(),
    };

    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_deposit_collateral_cw20() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "owner".to_string(),
        amount: 100u128.into(),
        msg: to_binary(&Cw20HookMsg::DepositCollateral {}).unwrap(),
    });

    let info = mock_info(MOCK_BLUNA_TOKEN, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "deposit_collateral_to_anchor")
            // Provide collateral
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_BLUNA_TOKEN.to_owned(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: MOCK_CUSTODY_CONTRACT.to_owned(),
                    msg: to_binary(&CustodyCw20HookMsg::DepositCollateral {}).unwrap(),
                    amount: Uint128::from(100u128),
                })
                .unwrap(),
            }))
            // Lock collateral
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_OVERSEER_CONTRACT.to_owned(),
                msg: to_binary(&OverseerExecuteMsg::LockCollateral {
                    collaterals: vec![(MOCK_BLUNA_TOKEN.to_owned(), Uint256::from(100u128))],
                })
                .unwrap(),
                funds: vec![],
            })),
        res
    );
}

#[test]
fn test_deposit_collateral_cw20_invalid_collateral() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "owner".to_string(),
        amount: 100u128.into(),
        msg: to_binary(&Cw20HookMsg::DepositCollateral {}).unwrap(),
    });

    let info = mock_info("invalid_collateral", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidFunds {
            msg: "Only bLuna collateral supported".to_string(),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_deposit_collateral_cw20_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "anyone".to_string(),
        amount: 100u128.into(),
        msg: to_binary(&Cw20HookMsg::DepositCollateral {}).unwrap(),
    });

    let info = mock_info(MOCK_BLUNA_TOKEN, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_borrow_new_loan() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::Borrow {
        desired_ltv_ratio: Decimal256::percent(50),
        recipient: None,
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "borrow")
            .add_attribute("desired_ltv_ratio", "0.5")
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_MARKET_CONTRACT.to_owned(),
                msg: to_binary(&MarketExecuteMsg::BorrowStable {
                    // The current collateral is worth 100
                    borrow_amount: Uint256::from(50u128),
                    to: Some(mock_env().contract.address.to_string()),
                })
                .unwrap(),
                funds: vec![],
            }))
            .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: "owner".to_string(),
                amount: coins(50, "uusd")
            })),
        res
    );
}

#[test]
fn test_borrow_existing_loan() {
    let mut deps = mock_dependencies_custom(&[]);
    deps.querier.loan_amount = Uint256::from(50u128);

    init(deps.as_mut());

    let msg = ExecuteMsg::Borrow {
        desired_ltv_ratio: Decimal256::percent(75),
        recipient: None,
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "borrow")
            .add_attribute("desired_ltv_ratio", "0.75")
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_MARKET_CONTRACT.to_owned(),
                msg: to_binary(&MarketExecuteMsg::BorrowStable {
                    // The current ltv ratio is 0.5, so need to borrow another 25 to get to 0.75
                    // ltv ratio.
                    borrow_amount: Uint256::from(25u128),
                    to: Some(mock_env().contract.address.to_string()),
                })
                .unwrap(),
                funds: vec![],
            }))
            .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: "owner".to_string(),
                amount: coins(25, "uusd")
            })),
        res
    );
}

#[test]
fn test_borrow_existing_loan_lower_ltv() {
    let mut deps = mock_dependencies_custom(&[]);
    deps.querier.loan_amount = Uint256::from(50u128);

    init(deps.as_mut());

    let msg = ExecuteMsg::Borrow {
        desired_ltv_ratio: Decimal256::percent(20),
        recipient: None,
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidLtvRatio {
            msg: "Desired LTV ratio lower than current".to_string(),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_borrow_ltv_too_high() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::Borrow {
        desired_ltv_ratio: Decimal256::one(),
        recipient: None,
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidLtvRatio {
            msg: "Desired LTV ratio must be less than 1".to_string(),
        },
        res.unwrap_err()
    );
}

#[test]
fn test_borrow_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::Borrow {
        desired_ltv_ratio: Decimal256::percent(50),
        recipient: None,
    };

    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_repay_loan() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    deps.querier.loan_amount = Uint256::from(100u128);

    let msg = ExecuteMsg::RepayLoan {};

    let info = mock_info("owner", &coins(100, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "repay_loan")
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_MARKET_CONTRACT.to_owned(),
                msg: to_binary(&MarketExecuteMsg::RepayStable {}).unwrap(),
                funds: info.funds,
            })),
        res
    );
}

#[test]
fn test_repay_loan_overpay() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    deps.querier.loan_amount = Uint256::from(100u128);

    let msg = ExecuteMsg::RepayLoan {};

    let info = mock_info("owner", &coins(150, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "repay_loan")
            .add_message(WasmMsg::Execute {
                contract_addr: MOCK_MARKET_CONTRACT.to_owned(),
                msg: to_binary(&MarketExecuteMsg::RepayStable {}).unwrap(),
                funds: info.funds,
            })
            .add_message(BankMsg::Send {
                to_address: "owner".to_string(),
                amount: coins(50, "uusd")
            }),
        res
    );
}

#[test]
fn test_repay_loan_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::RepayLoan {};

    let info = mock_info("anyone", &coins(100, "uusd"));
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_withdraw_collateral() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::WithdrawCollateral {
        collateral_addr: MOCK_BLUNA_TOKEN.to_owned(),
        amount: None,
        unbond: None,
        recipient: None,
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "withdraw_collateral")
            // Unlock collateral
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_OVERSEER_CONTRACT.to_owned(),
                msg: to_binary(&OverseerExecuteMsg::UnlockCollateral {
                    collaterals: vec![(MOCK_BLUNA_TOKEN.to_owned(), 100u128.into())],
                })
                .unwrap(),
                funds: vec![],
            }))
            // Withdraw collateral
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CUSTODY_CONTRACT.to_owned(),
                funds: vec![],
                msg: to_binary(&CustodyExecuteMsg::WithdrawCollateral {
                    amount: Some(100u128.into()),
                })
                .unwrap(),
            }))
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_BLUNA_TOKEN.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "owner".to_string(),
                    amount: 100u128.into(),
                })
                .unwrap(),
                funds: vec![],
            })),
        res
    );
}

#[test]
fn test_withdraw_collateral_unbond() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::WithdrawCollateral {
        collateral_addr: MOCK_BLUNA_TOKEN.to_owned(),
        amount: None,
        unbond: Some(true),
        recipient: None,
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "withdraw_collateral")
            // Unlock collateral
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_OVERSEER_CONTRACT.to_owned(),
                msg: to_binary(&OverseerExecuteMsg::UnlockCollateral {
                    collaterals: vec![(MOCK_BLUNA_TOKEN.to_owned(), 100u128.into())],
                })
                .unwrap(),
                funds: vec![],
            }))
            // Withdraw collateral
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CUSTODY_CONTRACT.to_owned(),
                funds: vec![],
                msg: to_binary(&CustodyExecuteMsg::WithdrawCollateral {
                    amount: Some(100u128.into()),
                })
                .unwrap(),
            }))
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_BLUNA_TOKEN.to_owned(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: MOCK_BLUNA_HUB_CONTRACT.to_owned(),
                    amount: 100u128.into(),
                    msg: to_binary(&BLunaHubCw20HookMsg::Unbond {}).unwrap(),
                })
                .unwrap(),
            })),
        res
    );
}

#[test]
fn test_withdraw_collateral_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::WithdrawCollateral {
        collateral_addr: MOCK_BLUNA_TOKEN.to_owned(),
        amount: None,
        unbond: None,
        recipient: None,
    };

    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_claim_anc() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::ClaimAncRewards { auto_stake: None };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "claim_anc_rewards")
            .add_message(WasmMsg::Execute {
                contract_addr: MOCK_MARKET_CONTRACT.to_owned(),
                msg: to_binary(&MarketExecuteMsg::ClaimRewards { to: None }).unwrap(),
                funds: vec![],
            }),
        res
    );
}

#[test]
fn test_claim_anc_auto_stake() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::ClaimAncRewards {
        auto_stake: Some(true),
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "claim_anc_rewards")
            .add_attribute("action", "stake_anc")
            .add_attribute("amount", "200")
            .add_message(WasmMsg::Execute {
                contract_addr: MOCK_MARKET_CONTRACT.to_owned(),
                msg: to_binary(&MarketExecuteMsg::ClaimRewards { to: None }).unwrap(),
                funds: vec![],
            })
            .add_message(WasmMsg::Execute {
                contract_addr: MOCK_ANC_TOKEN.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: MOCK_GOV_CONTRACT.to_owned(),
                    msg: to_binary(&GovCw20HookMsg::StakeVotingTokens {}).unwrap(),
                    amount: Uint128::new(200),
                })
                .unwrap(),
                funds: vec![],
            }),
        res
    );
}

#[test]
fn test_claim_anc_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::ClaimAncRewards { auto_stake: None };

    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_stake_anc() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::StakeAnc { amount: None };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "stake_anc")
            .add_attribute("amount", "100")
            .add_message(WasmMsg::Execute {
                contract_addr: MOCK_ANC_TOKEN.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: MOCK_GOV_CONTRACT.to_owned(),
                    msg: to_binary(&GovCw20HookMsg::StakeVotingTokens {}).unwrap(),
                    amount: Uint128::new(100),
                })
                .unwrap(),
                funds: vec![],
            }),
        res
    );
}

#[test]
fn test_stake_anc_amount_specified() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::StakeAnc {
        amount: Some(Uint128::new(10)),
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "stake_anc")
            .add_attribute("amount", "10")
            .add_message(WasmMsg::Execute {
                contract_addr: MOCK_ANC_TOKEN.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: MOCK_GOV_CONTRACT.to_owned(),
                    msg: to_binary(&GovCw20HookMsg::StakeVotingTokens {}).unwrap(),
                    amount: Uint128::new(10),
                })
                .unwrap(),
                funds: vec![],
            }),
        res
    );
}

#[test]
fn test_stake_anc_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::StakeAnc { amount: None };

    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_unstake_anc() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::UnstakeAnc { amount: None };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "unstake_anc")
            .add_attribute("amount", "100")
            .add_message(WasmMsg::Execute {
                contract_addr: MOCK_GOV_CONTRACT.to_owned(),
                msg: to_binary(&GovExecuteMsg::WithdrawVotingTokens {
                    amount: Some(Uint128::new(100)),
                })
                .unwrap(),
                funds: vec![],
            }),
        res
    );
}

#[test]
fn test_unstake_anc_amount_specified() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::UnstakeAnc {
        amount: Some(Uint128::new(10)),
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "unstake_anc")
            .add_attribute("amount", "10")
            .add_message(WasmMsg::Execute {
                contract_addr: MOCK_GOV_CONTRACT.to_owned(),
                msg: to_binary(&GovExecuteMsg::WithdrawVotingTokens {
                    amount: Some(Uint128::new(10)),
                })
                .unwrap(),
                funds: vec![],
            }),
        res
    );
}

#[test]
fn test_unstake_anc_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::UnstakeAnc { amount: None };

    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_receive_cw20_zero_amount() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "sender".to_string(),
        amount: Uint128::zero(),
        msg: to_binary(&"").unwrap(),
    });

    let info = mock_info(MOCK_BLUNA_TOKEN, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidFunds {
            msg: "Amount must be non-zero".to_string()
        },
        res.unwrap_err()
    );
}
