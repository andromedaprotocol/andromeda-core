use ado_base::state::ADOContract;
use andromeda_non_fungible_tokens::cw721_timelock::{
    Cw721HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use common::{ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError};
use cosmwasm_std::{
    ensure, entry_point, from_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw721::{Cw721ExecuteMsg, Cw721ReceiveMsg, Expiration};
use cw_utils::nonpayable;
use semver::Version;

use crate::state::{LockDetails, LOCKED_ITEMS};

// 1 day in seconds
const ONE_DAY: u64 = 86_400;
// 1 year in seconds
const ONE_YEAR: u64 = 31_536_000;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-cw721-timelock";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "cw721-timelock".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
            primitive_contract: None,
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
        ExecuteMsg::Claim { lock_id } => execute_claim(deps, env, info, lock_id),
    }
}

fn handle_receive_cw721(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&msg.msg)? {
        Cw721HookMsg::StartLock {
            recipient,
            lock_time,
        } => execute_lock(
            deps,
            env,
            recipient,
            msg.sender,
            msg.token_id,
            lock_time,
            info.sender.to_string(),
        ),
    }
}
fn execute_lock(
    deps: DepsMut,
    env: Env,
    recipient: Option<String>,
    sender: String,
    nft_id: String,
    lock_time: u64,
    andromeda_cw721_contract: String,
) -> Result<Response, ContractError> {
    // Lock time can't be too long
    ensure!(lock_time <= ONE_YEAR, ContractError::LockTimeTooLong {});

    // Lock time can't be too short
    ensure!(lock_time >= ONE_DAY, ContractError::LockTimeTooShort {});

    // Concatenate NFT's contract address and ID to form a unique ID for each NFT
    let lock_id = format!("{andromeda_cw721_contract}{nft_id}");

    // Make sure NFT isn't already locked in this contract
    let lock_id_check = LOCKED_ITEMS.may_load(deps.storage, &lock_id)?;
    ensure!(lock_id_check.is_none(), ContractError::LockedNFT {});

    // Validate recipient's address if given, and set the sender as recipient if none was provided
    let recip = if let Some(recipient) = recipient {
        deps.api.addr_validate(&recipient)?;
        recipient
    } else {
        sender
    };

    // Add lock time to current block time
    let expiration_time = env.block.time.plus_seconds(lock_time);

    // Set lock details
    let lock_details = LockDetails {
        recipient: recip,
        expiration: Expiration::AtTime(expiration_time),
        nft_id,
        nft_contract: andromeda_cw721_contract,
    };

    // Save all the details. The key represents the concatenated lock_id & the value represents the lock details
    LOCKED_ITEMS.save(deps.storage, &lock_id, &lock_details)?;

    Ok(Response::new()
        .add_attribute("action", "locked_nft")
        // The recipient should keep the lock ID to easily claim the NFT
        .add_attribute("lock_id", lock_id))
}

fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lock_id: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    // Check if lock ID exists
    let locked_item = LOCKED_ITEMS.may_load(deps.storage, &lock_id)?;

    if let Some(locked_nft) = locked_item {
        // Check if lock is expired
        let expiration = locked_nft.expiration;
        ensure!(
            expiration.is_expired(&env.block),
            ContractError::LockedNFT {}
        );

        // Remove NFT from the list of locked items
        LOCKED_ITEMS.remove(deps.storage, &lock_id);

        Ok(Response::new()
            // Send NFT to the recipient
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: locked_nft.nft_contract,
                msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: locked_nft.recipient,
                    token_id: locked_nft.nft_id,
                })?,
                funds: vec![],
            }))
            .add_attribute("action", "claimed_nft"))
    } else {
        Err(ContractError::NFTNotFound {})
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::LockedToken { lock_id } => encode_binary(&query_locked_token(deps, lock_id)?),
    }
}
fn query_locked_token(deps: Deps, lock_id: String) -> Result<LockDetails, ContractError> {
    let nft = LOCKED_ITEMS.may_load(deps.storage, &lock_id)?;
    if let Some(nft) = nft {
        Ok(nft)
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
    StdError::generic_err(format!("Semver: {}", err))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::mock_querier::{mock_dependencies_custom, MOCK_TOKEN_ADDR, MOCK_TOKEN_OWNER};
    use andromeda_non_fungible_tokens::cw721_timelock::{ExecuteMsg, InstantiateMsg};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{Addr, BlockInfo, ContractInfo, TransactionInfo};
    use cw721::Expiration;

    #[test]
    fn test_instantiate() {
        let owner = "creator";
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {};
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_lock_too_long() {
        let owner = "creator";
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {};
        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let nft_id = "ape".to_string();
        let lock_time = 1_000_000_000u64;
        let andromeda_cw721_contract = "contract".to_string();

        let err = execute_lock(
            deps.as_mut(),
            env,
            None,
            "me".to_string(),
            nft_id,
            lock_time,
            andromeda_cw721_contract,
        )
        .unwrap_err();
        assert_eq!(err, ContractError::LockTimeTooLong {});
    }

    #[test]
    fn test_lock_too_short() {
        let owner = "creator";
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {};
        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let nft_id = "ape".to_string();
        let lock_time = 100u64;
        let andromeda_cw721_contract = "contract".to_string();

        let err = execute_lock(
            deps.as_mut(),
            env,
            None,
            "me".to_string(),
            nft_id,
            lock_time,
            andromeda_cw721_contract,
        )
        .unwrap_err();
        assert_eq!(err, ContractError::LockTimeTooShort {});
    }

    #[test]
    fn test_lock_works() {
        let owner = "creator";
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {};
        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let nft_id = "ape1".to_string();
        let lock_time = 100_000u64;
        let andromeda_cw721_contract = MOCK_TOKEN_ADDR.to_string();

        let _res = execute_lock(
            deps.as_mut(),
            env.clone(),
            None,
            "me".to_string(),
            nft_id,
            lock_time,
            andromeda_cw721_contract,
        )
        .unwrap();
        let mock_time = env.block.time;
        let nft = LOCKED_ITEMS.load(&deps.storage, "token0001ape1").unwrap();
        let expected_nft = LockDetails {
            recipient: "me".to_string(),
            expiration: Expiration::AtTime(mock_time.plus_seconds(100_000u64)),
            nft_id: "ape1".to_string(),
            nft_contract: MOCK_TOKEN_ADDR.to_string(),
        };
        assert_eq!(nft, expected_nft);
    }

    #[test]
    fn test_lock_already_locked() {
        let owner = "creator";
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {};
        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let nft_id = "ape1".to_string();
        let lock_time = 100_000u64;
        let andromeda_cw721_contract = MOCK_TOKEN_ADDR.to_string();

        let _res = execute_lock(
            deps.as_mut(),
            env.clone(),
            None,
            "me".to_string(),
            nft_id.clone(),
            lock_time,
            andromeda_cw721_contract.clone(),
        )
        .unwrap();
        let err = execute_lock(
            deps.as_mut(),
            env,
            None,
            "me".to_string(),
            nft_id,
            lock_time,
            andromeda_cw721_contract,
        )
        .unwrap_err();
        assert_eq!(err, ContractError::LockedNFT {});
    }

    #[test]
    fn test_claim_nft_not_found() {
        let owner = "creator";
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {};
        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let nft_id = "ape1".to_string();
        let lock_time = 100_000u64;
        let andromeda_cw721_contract = MOCK_TOKEN_ADDR.to_string();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let _res = execute_lock(
            deps.as_mut(),
            env,
            None,
            "me".to_string(),
            nft_id,
            lock_time,
            andromeda_cw721_contract,
        )
        .unwrap();

        let msg = ExecuteMsg::Claim {
            lock_id: "token0001ape2".to_string(),
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::NFTNotFound {});
    }

    #[test]
    fn test_claim_nft_locked() {
        let owner = "creator";
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {};
        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let nft_id = "ape1".to_string();
        let lock_time = 100_000u64;
        let andromeda_cw721_contract = MOCK_TOKEN_ADDR.to_string();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let _res = execute_lock(
            deps.as_mut(),
            env,
            None,
            "me".to_string(),
            nft_id,
            lock_time,
            andromeda_cw721_contract,
        )
        .unwrap();

        let msg = ExecuteMsg::Claim {
            lock_id: "token0001ape1".to_string(),
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::LockedNFT {});
    }

    #[test]
    fn test_claim_works() {
        let owner = "creator";
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {};
        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let nft_id = "ape1".to_string();
        let lock_time = 100_000u64;
        let andromeda_cw721_contract = MOCK_TOKEN_ADDR.to_string();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let _res = execute_lock(
            deps.as_mut(),
            env,
            None,
            "me".to_string(),
            nft_id,
            lock_time,
            andromeda_cw721_contract,
        )
        .unwrap();

        let msg = ExecuteMsg::Claim {
            lock_id: "token0001ape1".to_string(),
        };
        mock_env().block.time = mock_env().block.time.plus_seconds(200_000);

        let env: Env = Env {
            block: BlockInfo {
                height: 12_345,
                time: mock_env().block.time.plus_seconds(200_000),
                chain_id: "cosmos-testnet-14002".to_string(),
            },
            transaction: Some(TransactionInfo { index: 3 }),
            contract: ContractInfo {
                address: Addr::unchecked(MOCK_CONTRACT_ADDR),
            },
        };
        let _res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

        // searching for the same token shouldn't work if it was successfully  claimed

        let msg = ExecuteMsg::Claim {
            lock_id: "token0001ape1".to_string(),
        };
        mock_env().block.time = mock_env().block.time.plus_seconds(200_000);

        let env: Env = Env {
            block: BlockInfo {
                height: 12_345,
                time: mock_env().block.time.plus_seconds(200_000),
                chain_id: "cosmos-testnet-14002".to_string(),
            },
            transaction: Some(TransactionInfo { index: 3 }),
            contract: ContractInfo {
                address: Addr::unchecked(MOCK_CONTRACT_ADDR),
            },
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::NFTNotFound {});
    }
}
