use crate::state::{ALLOWED_CONTRACTS, REWARD, STAKED_NFTS, UNBONDING_PERIOD};
use ado_base::state::ADOContract;
use andromeda_non_fungible_tokens::cw721_staking::{
    Cw721HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, StakedNft,
};
use common::{ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError};
use cosmwasm_std::{
    attr, ensure, entry_point, from_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, Uint128, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw721::{Cw721ExecuteMsg, Cw721ReceiveMsg};
use cw_utils::nonpayable;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-cw721-staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

use semver::Version;

// One day in seconds
pub const ONE_DAY: u64 = 86400;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    ALLOWED_CONTRACTS.save(deps.storage, &msg.nft_contract)?;
    UNBONDING_PERIOD.save(deps.storage, &msg.unbonding_period)?;
    REWARD.save(deps.storage, &msg.reward)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "nft-staking".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
            primitive_contract: None,
            kernel_address: msg.kernel_address,
        },
    )
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
        }
        ExecuteMsg::ReceiveNft(msg) => handle_receive_cw721(deps, env, info, msg),
        ExecuteMsg::Claim { key } => execute_claim(deps, env, info, key),
        ExecuteMsg::Unstake { key } => execute_unstake(deps, env, info, key),
        ExecuteMsg::UpdateAllowedContracts { contracts } => {
            execute_update_allowed_contracts(deps, info, contracts)
        }
        ExecuteMsg::AddAllowedContract { new_contract } => {
            execute_add_allowed_contract(deps, info, new_contract)
        }
        ExecuteMsg::RemoveAllowedContract { old_contract } => {
            execute_remove_allowed_contract(deps, info, old_contract)
        }
        ExecuteMsg::UpdateUnbondingPeriod { new_period } => {
            execute_update_unbonding_period(deps, info, new_period)
        }
    }
}

fn execute_update_unbonding_period(
    deps: DepsMut,
    info: MessageInfo,
    new_period: u64,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let contract = ADOContract::default();

    // Only owner or operator can use this function
    ensure!(
        contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    // Save new unbonding period
    UNBONDING_PERIOD.save(deps.storage, &new_period)?;

    Ok(Response::new().add_attribute("action", "updated_unbonding_time"))
}

fn execute_update_allowed_contracts(
    deps: DepsMut,
    info: MessageInfo,
    contracts: Vec<String>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let contract = ADOContract::default();

    // Only owner or operator can use this function
    ensure!(
        contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    ALLOWED_CONTRACTS.save(deps.storage, &contracts)?;
    Ok(Response::new().add_attribute("action", "updated_allowed_contracts"))
}

fn execute_add_allowed_contract(
    deps: DepsMut,
    info: MessageInfo,
    new_contract: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let contract = ADOContract::default();

    // Only owner or operator can use this function
    ensure!(
        contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    let mut new_contracts = ALLOWED_CONTRACTS.load(deps.storage)?;

    // Prevent duplicate contracts
    ensure!(
        !new_contracts.contains(&new_contract),
        ContractError::DuplicateContract {}
    );

    new_contracts.push(new_contract);

    ALLOWED_CONTRACTS.save(deps.storage, &new_contracts)?;
    Ok(Response::new().add_attribute("action", "updated_allowed_contracts"))
}

fn execute_remove_allowed_contract(
    deps: DepsMut,
    info: MessageInfo,
    old_contract: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let contract = ADOContract::default();

    // Only owner or operator can use this function
    ensure!(
        contract.is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    let mut new_contracts = ALLOWED_CONTRACTS.load(deps.storage)?;
    let index = new_contracts.iter().position(|x| x == &old_contract);
    if let Some(index) = index {
        new_contracts.swap_remove(index);
        ALLOWED_CONTRACTS.save(deps.storage, &new_contracts)?;
        Ok(Response::new().add_attribute("action", "updated_allowed_contracts"))
    } else {
        Err(ContractError::ContractAddressNotInAddressList {})
    }
}

fn handle_receive_cw721(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&msg.msg)? {
        Cw721HookMsg::Stake {} => {
            execute_stake(deps, env, msg.sender, msg.token_id, info.sender.to_string())
        }
    }
}

fn execute_stake(
    deps: DepsMut,
    env: Env,
    sender: String,
    token_id: String,
    token_address: String,
) -> Result<Response, ContractError> {
    let allowed_contracts = ALLOWED_CONTRACTS.load(deps.storage)?;

    // NFT has to be sent from an allowed contract
    ensure!(
        allowed_contracts.contains(&token_address),
        ContractError::UnsupportedNFT {}
    );
    // Concatenate the token's address and ID to form a unique key
    let key = format!("{token_address}{token_id}");

    let reward = REWARD.load(deps.storage)?;

    let data = StakedNft {
        owner: sender,
        id: token_id,
        contract_address: token_address,
        time_of_staking: env.block.time,
        time_of_unbonding: None,
        reward,
        accrued_reward: None,
    };
    STAKED_NFTS.save(deps.storage, key, &data)?;
    Ok(Response::new().add_attributes(vec![attr("action", "staked_nft")]))
}

fn execute_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    key: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let nft = STAKED_NFTS.may_load(deps.storage, key.clone())?;
    if let Some(nft) = nft {
        // Only owner can unstake the NFT
        ensure!(info.sender == nft.owner, ContractError::Unauthorized {});

        // Can't unbond twice
        ensure!(
            nft.time_of_unbonding.is_none(),
            ContractError::AlreadyUnbonded {}
        );

        let current_time = env.block.time;

        let time_spent_bonded = current_time.seconds() - nft.time_of_staking.seconds();

        // Time spent bonded should be at least a day
        ensure!(
            time_spent_bonded >= ONE_DAY,
            ContractError::InsufficientBondedTime {}
        );

        // We use the reward that was set at the time of staking
        let reward = nft.reward;

        let payment = reward.amount * Uint128::from(time_spent_bonded);

        let accrued_reward = Coin {
            denom: reward.clone().denom,
            amount: payment,
        };

        let new_data = StakedNft {
            owner: nft.owner,
            id: nft.id,
            contract_address: nft.contract_address,
            time_of_staking: nft.time_of_staking,
            time_of_unbonding: Some(env.block.time),
            reward,
            accrued_reward: Some(accrued_reward),
        };

        STAKED_NFTS.save(deps.storage, key, &new_data)?;

        Ok(Response::new().add_attribute("action", "unbonded"))
    } else {
        Err(ContractError::NFTNotFound {})
    }
}

fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    key: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let nft = STAKED_NFTS.may_load(deps.storage, key.clone())?;
    if let Some(nft) = nft {
        // Only owner can claim the NFT
        ensure!(info.sender == nft.owner, ContractError::Unauthorized {});

        // NFT should be unbonded
        if let Some(time_of_unbonding) = nft.time_of_unbonding {
            let unbonding_period = UNBONDING_PERIOD.load(deps.storage)?;

            // Calculate the time passed since unbonding
            let time_spent_unbonded = env.block.time.seconds() - time_of_unbonding.seconds();

            // time spent unbonded should equal or exceed the unbonding period
            ensure!(
                time_spent_unbonded >= unbonding_period,
                ContractError::IncompleteUnbondingPeriod {}
            );

            // Remove NFT from list of staked NFTs
            STAKED_NFTS.remove(deps.storage, key);

            // payout rewards and send back NFT
            Ok(Response::new()
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: nft.owner.clone(),
                    amount: vec![nft.accrued_reward.unwrap_or_default()],
                }))
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: nft.contract_address,
                    msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                        recipient: nft.owner,
                        token_id: nft.id,
                    })?,
                    funds: vec![],
                }))
                .add_attribute("action", "claimed_nft_and_reward"))
        } else {
            Err(ContractError::StillBonded {})
        }
    } else {
        Err(ContractError::NFTNotFound {})
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::StakedNft { key } => encode_binary(&query_staked_nft(deps, env, key)?),
        QueryMsg::AllowedContracts {} => encode_binary(&query_allowed_contracts(deps)?),
        QueryMsg::UnbondingPeriod {} => encode_binary(&query_unbonding_period(deps)?),
        QueryMsg::Reward {} => encode_binary(&query_reward(deps)?),
    }
}

fn query_reward(deps: Deps) -> Result<Coin, ContractError> {
    let reward = REWARD.load(deps.storage)?;
    Ok(reward)
}

fn query_unbonding_period(deps: Deps) -> Result<u64, ContractError> {
    let period = UNBONDING_PERIOD.load(deps.storage)?;
    Ok(period)
}

fn query_allowed_contracts(deps: Deps) -> Result<Vec<String>, ContractError> {
    let allowed_contracts = ALLOWED_CONTRACTS.load(deps.storage)?;
    Ok(allowed_contracts)
}

fn query_staked_nft(deps: Deps, env: Env, key: String) -> Result<StakedNft, ContractError> {
    let nft = STAKED_NFTS.may_load(deps.storage, key)?;
    if let Some(nft) = nft {
        let current_time = env.block.time;

        let time_spent_bonded = current_time.seconds() - nft.time_of_staking.seconds();

        // We use the reward that was set at the time of staking
        let reward = nft.reward;

        let payment = reward.amount * Uint128::from(time_spent_bonded);

        let accrued_reward = Coin {
            denom: reward.clone().denom,
            amount: payment,
        };

        let new_data = StakedNft {
            owner: nft.owner,
            id: nft.id,
            contract_address: nft.contract_address,
            time_of_staking: nft.time_of_staking,
            time_of_unbonding: nft.time_of_unbonding,
            reward,
            accrued_reward: Some(accrued_reward),
        };

        Ok(new_data)
    } else {
        Err(ContractError::NFTNotFound {})
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    let contract = ADOContract::default();

    ensure!(
        stored.contract == CONTRACT_NAME,
        ContractError::CannotMigrate {
            previous_contract: stored.contract,
        }
    );

    // New version has to be newer/greater than the old version
    ensure!(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        }
    );

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Update the ADOContract's version
    contract.execute_update_version(deps)?;

    Ok(Response::default())
}

fn from_semver(err: semver::Error) -> StdError {
    StdError::generic_err(format!("Semver: {err}"))
}

#[cfg(test)]
mod tests {

    use super::*;

    use andromeda_non_fungible_tokens::cw721_staking::InstantiateMsg;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{BlockInfo, ContractInfo};

    #[test]
    fn execute_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },

            kernel_address: None,
        };

        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    }

    #[test]
    fn execute_update_bonding_time_unauthorized() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },

            kernel_address: None,
        };

        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let info = mock_info("random", &[]);
        let new_period = 10000;

        let err = execute_update_unbonding_period(deps.as_mut(), info, new_period).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn execute_update_bonding_time_works() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },

            kernel_address: None,
        };

        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let info = mock_info("me", &[]);
        let new_period = 10000;

        let _res = execute_update_unbonding_period(deps.as_mut(), info, new_period).unwrap();
        let expected_period = 10000;
        let actual_period = UNBONDING_PERIOD.load(&deps.storage).unwrap();
        assert_eq!(expected_period, actual_period);
    }

    #[test]
    fn execute_stake_unauthorized_contract() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },

            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let sender = "someone".to_string();
        let token_id = "1".to_string();
        let token_address = "invalid".to_string();

        let err = execute_stake(deps.as_mut(), env, sender, token_id, token_address).unwrap_err();
        assert_eq!(err, ContractError::UnsupportedNFT {});
    }

    #[test]
    fn execute_stake_works() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },

            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let sender = "someone".to_string();
        let token_id = "1".to_string();
        let token_address = "valid".to_string();

        let _res = execute_stake(
            deps.as_mut(),
            env.clone(),
            sender.clone(),
            token_id.clone(),
            token_address.clone(),
        )
        .unwrap();

        let expected_details = StakedNft {
            owner: sender,
            id: token_id,
            contract_address: token_address,
            time_of_staking: env.block.time,
            time_of_unbonding: None,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },

            accrued_reward: None,
        };
        let details = STAKED_NFTS
            .load(&deps.storage, "valid1".to_string())
            .unwrap();

        assert_eq!(expected_details, details);
    }

    #[test]
    fn execute_unstake_nft_not_found() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let sender = "someone".to_string();
        let token_id = "1".to_string();
        let token_address = "valid".to_string();

        let _res =
            execute_stake(deps.as_mut(), env.clone(), sender, token_id, token_address).unwrap();
        let key = String::from("valid2");
        let err = execute_unstake(deps.as_mut(), env, info, key).unwrap_err();
        assert_eq!(err, ContractError::NFTNotFound {});
    }

    #[test]
    fn execute_unstake_unauthorized() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let sender = "someone".to_string();
        let token_id = "1".to_string();
        let token_address = "valid".to_string();

        let _res =
            execute_stake(deps.as_mut(), env.clone(), sender, token_id, token_address).unwrap();
        let key = String::from("valid1");

        let info = mock_info("random", &[]);
        let err = execute_unstake(deps.as_mut(), env, info, key).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn execute_unstake_not_long_enough() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let sender = "someone".to_string();
        let token_id = "1".to_string();
        let token_address = "valid".to_string();

        let _res =
            execute_stake(deps.as_mut(), env.clone(), sender, token_id, token_address).unwrap();
        let key = String::from("valid1");

        let info = mock_info("someone", &[]);

        let err = execute_unstake(deps.as_mut(), env, info, key).unwrap_err();
        assert_eq!(err, ContractError::InsufficientBondedTime {});
    }

    #[test]
    fn execute_unstake_works() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let sender = "someone".to_string();
        let token_id = "1".to_string();
        let token_address = "valid".to_string();

        let _res = execute_stake(
            deps.as_mut(),
            env.clone(),
            sender.clone(),
            token_id.clone(),
            token_address.clone(),
        )
        .unwrap();
        let key = String::from("valid1");

        let info = mock_info("someone", &[]);
        let block_info = BlockInfo {
            height: 12345,
            time: env.block.time.plus_seconds(300_000),
            chain_id: "cosmos-testnet-14002".to_string(),
        };
        let env = Env {
            block: block_info,
            transaction: None,
            contract: ContractInfo {
                address: env.contract.address,
            },
        };
        let _res = execute_unstake(deps.as_mut(), env.clone(), info, key).unwrap();

        let details = STAKED_NFTS
            .load(&deps.storage, "valid1".to_string())
            .unwrap();
        println!("{:?}", details.time_of_staking.seconds());
        println!("{:?}", env.block.time.clone().seconds());
        let time_spent = env.block.time.clone().seconds() - details.time_of_staking.seconds();

        println!("{time_spent:?}");
        let set_reward = REWARD.load(&deps.storage).unwrap();
        let expected_reward = set_reward.amount * Uint128::from(time_spent);

        let expected_details = StakedNft {
            owner: sender,
            id: token_id,
            contract_address: token_address,
            time_of_staking: details.time_of_staking,
            time_of_unbonding: Some(env.block.time),
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            accrued_reward: Some(Coin {
                denom: "ujuno".to_string(),
                amount: expected_reward,
            }),
        };

        assert_eq!(expected_details, details);
    }

    #[test]
    fn execute_unstake_twice() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let sender = "someone".to_string();
        let token_id = "1".to_string();
        let token_address = "valid".to_string();

        let _res =
            execute_stake(deps.as_mut(), env.clone(), sender, token_id, token_address).unwrap();
        let key = String::from("valid1");

        let info = mock_info("someone", &[]);
        let block_info = BlockInfo {
            height: 12345,
            time: env.block.time.plus_seconds(300_000),
            chain_id: "cosmos-testnet-14002".to_string(),
        };
        let env = Env {
            block: block_info,
            transaction: None,
            contract: ContractInfo {
                address: env.contract.address,
            },
        };
        let _res = execute_unstake(deps.as_mut(), env.clone(), info.clone(), key.clone()).unwrap();

        let err = execute_unstake(deps.as_mut(), env, info, key).unwrap_err();
        assert_eq!(err, ContractError::AlreadyUnbonded {});
    }

    #[test]
    fn test_claim_unauthorized() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let sender = "someone".to_string();
        let token_id = "1".to_string();
        let token_address = "valid".to_string();

        let _res =
            execute_stake(deps.as_mut(), env.clone(), sender, token_id, token_address).unwrap();
        let key = String::from("valid1");

        let info = mock_info("someone", &[]);
        let block_info = BlockInfo {
            height: 12345,
            time: env.block.time.plus_seconds(300_000),
            chain_id: "cosmos-testnet-14002".to_string(),
        };
        let env = Env {
            block: block_info,
            transaction: None,
            contract: ContractInfo {
                address: env.contract.address,
            },
        };
        let _res = execute_unstake(deps.as_mut(), env.clone(), info, key.clone()).unwrap();
        let info = mock_info("random", &[]);
        let err = execute_claim(deps.as_mut(), env, info, key).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn test_claim_unbonding_time_not_reached() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let sender = "someone".to_string();
        let token_id = "1".to_string();
        let token_address = "valid".to_string();

        let _res =
            execute_stake(deps.as_mut(), env.clone(), sender, token_id, token_address).unwrap();
        let key = String::from("valid1");

        let info = mock_info("someone", &[]);
        let block_info = BlockInfo {
            height: 12345,
            time: env.block.time.plus_seconds(300_000),
            chain_id: "cosmos-testnet-14002".to_string(),
        };
        let env = Env {
            block: block_info,
            transaction: None,
            contract: ContractInfo {
                address: env.contract.address,
            },
        };
        let _res = execute_unstake(deps.as_mut(), env.clone(), info.clone(), key.clone()).unwrap();

        let err = execute_claim(deps.as_mut(), env, info, key).unwrap_err();
        assert_eq!(err, ContractError::IncompleteUnbondingPeriod {});
    }

    #[test]
    fn test_claim_unbonding_time_nft_not_found() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let sender = "someone".to_string();
        let token_id = "1".to_string();
        let token_address = "valid".to_string();

        let _res =
            execute_stake(deps.as_mut(), env.clone(), sender, token_id, token_address).unwrap();
        let key = String::from("valid1");

        let info = mock_info("someone", &[]);
        let block_info = BlockInfo {
            height: 12345,
            time: env.block.time.plus_seconds(300_000),
            chain_id: "cosmos-testnet-14002".to_string(),
        };
        let env = Env {
            block: block_info,
            transaction: None,
            contract: ContractInfo {
                address: env.contract.address,
            },
        };
        let _res = execute_unstake(deps.as_mut(), env.clone(), info.clone(), key).unwrap();
        let key = "random".to_string();
        let err = execute_claim(deps.as_mut(), env, info, key).unwrap_err();
        assert_eq!(err, ContractError::NFTNotFound {});
    }

    #[test]
    fn test_claim_still_bonded() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let sender = "someone".to_string();
        let token_id = "1".to_string();
        let token_address = "valid".to_string();

        let _res =
            execute_stake(deps.as_mut(), env.clone(), sender, token_id, token_address).unwrap();
        let key = String::from("valid1");

        let info = mock_info("someone", &[]);
        let block_info = BlockInfo {
            height: 12345,
            time: env.block.time.plus_seconds(300_000),
            chain_id: "cosmos-testnet-14002".to_string(),
        };
        let env = Env {
            block: block_info,
            transaction: None,
            contract: ContractInfo {
                address: env.contract.address,
            },
        };
        let err = execute_claim(deps.as_mut(), env, info, key).unwrap_err();
        assert_eq!(err, ContractError::StillBonded {});
    }

    #[test]
    fn test_claim_works() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let sender = "someone".to_string();
        let token_id = "1".to_string();
        let token_address = "valid".to_string();

        let _res =
            execute_stake(deps.as_mut(), env.clone(), sender, token_id, token_address).unwrap();
        let key = String::from("valid1");

        let info = mock_info("someone", &[]);
        let block_info = BlockInfo {
            height: 12345,
            time: env.block.time.plus_seconds(300_000),
            chain_id: "cosmos-testnet-14002".to_string(),
        };
        let env = Env {
            block: block_info,
            transaction: None,
            contract: ContractInfo {
                address: env.contract.address,
            },
        };
        let _res = execute_unstake(deps.as_mut(), env.clone(), info.clone(), key.clone()).unwrap();

        let err = execute_claim(deps.as_mut(), env, info, key).unwrap_err();
        assert_eq!(err, ContractError::IncompleteUnbondingPeriod {});
    }

    #[test]
    fn test_update_allowed_contracts_unauthorized() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let info = mock_info("random", &[]);
        let contracts = vec!["1".to_string(), "2".to_string()];

        let err = execute_update_allowed_contracts(deps.as_mut(), info, contracts).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn test_update_allowed_contracts_works() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let info = mock_info("me", &[]);
        let contracts = vec!["1".to_string(), "2".to_string()];

        let _res = execute_update_allowed_contracts(deps.as_mut(), info, contracts).unwrap();
        let expected_contracts = vec!["1".to_string(), "2".to_string()];
        let actual_contracts = ALLOWED_CONTRACTS.load(&deps.storage).unwrap();
        assert_eq!(expected_contracts, actual_contracts);
    }

    #[test]
    fn test_add_allowed_contract_unauthorized() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let info = mock_info("random", &[]);
        let new_contract = "1".to_string();

        let err = execute_add_allowed_contract(deps.as_mut(), info, new_contract).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn test_add_allowed_contract_duplicate_contract() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let info = mock_info("me", &[]);
        let new_contract = "valid".to_string();

        let err = execute_add_allowed_contract(deps.as_mut(), info, new_contract).unwrap_err();

        assert_eq!(err, ContractError::DuplicateContract {});
    }

    #[test]
    fn test_add_allowed_contract_works() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let info = mock_info("me", &[]);
        let new_contract = "1".to_string();

        let _res = execute_add_allowed_contract(deps.as_mut(), info, new_contract).unwrap();

        let expected_contracts = vec!["valid".to_string(), "1".to_string()];
        let actual_contracts = ALLOWED_CONTRACTS.load(&deps.storage).unwrap();
        assert_eq!(expected_contracts, actual_contracts);
    }

    #[test]
    fn test_remove_allowed_contract_unauthorized() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let info = mock_info("random", &[]);
        let new_contract = "1".to_string();

        let err = execute_remove_allowed_contract(deps.as_mut(), info, new_contract).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn test_remove_allowed_contract_not_found() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let info = mock_info("me", &[]);
        let new_contract = "1".to_string();

        let err = execute_remove_allowed_contract(deps.as_mut(), info, new_contract).unwrap_err();
        assert_eq!(err, ContractError::ContractAddressNotInAddressList {});
    }

    #[test]
    fn test_remove_allowed_contract_works() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("me", &[]);

        let msg = InstantiateMsg {
            nft_contract: vec!["valid".to_string()],
            unbonding_period: 200000,
            reward: Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(10_u16),
            },
            kernel_address: None,
        };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let info = mock_info("me", &[]);
        let new_contract = "valid".to_string();

        let _res = execute_remove_allowed_contract(deps.as_mut(), info, new_contract).unwrap();

        let expected_contracts: Vec<String> = vec![];
        let actual_contracts = ALLOWED_CONTRACTS.load(&deps.storage).unwrap();
        assert_eq!(expected_contracts, actual_contracts);
    }
}
