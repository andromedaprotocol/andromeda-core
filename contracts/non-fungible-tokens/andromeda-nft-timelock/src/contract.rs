use ado_base::state::ADOContract;
use andromeda_non_fungible_tokens::nft_timelock::{ExecuteMsg, InstantiateMsg, QueryMsg};
use common::{
    ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError, require,
};
use cosmwasm_std::{
    entry_point, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, QueryRequest,
    Response, WasmMsg, WasmQuery,
};
use cw721::{Cw721ExecuteMsg, Cw721QueryMsg, Expiration, OwnerOfResponse};

use crate::state::{LockDetails, LOCKED_ITEMS};

// 1 day in seconds
const ONE_DAY: u64 = 86_400;
// 1 year in seconds
const ONE_YEAR: u64 = 31_536_000;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    ADOContract::default().instantiate(
        deps.storage,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "nft-timelock".to_string(),
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
        ExecuteMsg::Lock {
            recipient,
            nft_id,
            lock_time,
            andromeda_cw721_contract,
        } => execute_lock(
            deps,
            env,
            info,
            recipient,
            nft_id,
            lock_time,
            andromeda_cw721_contract,
        ),
        ExecuteMsg::Claim { lock_id } => execute_claim(deps, env, info, lock_id),
        ExecuteMsg::UpdateOwner { address } => {
            ADOContract::default().execute_update_owner(deps, info, address)
        }
    }
}

fn execute_lock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Option<String>,
    nft_id: String,
    lock_time: u64,
    andromeda_cw721_contract: String,
) -> Result<Response, ContractError> {
    // Lock time can't be too long
    require(lock_time <= ONE_YEAR, ContractError::LockTimeTooLong {})?;

    // Lock time can't be too short
    require(lock_time >= ONE_DAY, ContractError::LockTimeTooShort {})?;

    // Concatenate NFT's contract address and ID
    let lock_id = format!("{andromeda_cw721_contract}{nft_id}");

    // Make sure NFT isn't already locked
    let lock_id_check = LOCKED_ITEMS.may_load(deps.storage, &lock_id)?;
    require(lock_id_check.is_some(), ContractError::NFTNotFound {})?;

    // Validate recipient's address if given, and set the sender as recipient if none was provided
    let recip = if let Some(recipient) = recipient {
        deps.api.addr_validate(&recipient)?;
        recipient
    } else {
        info.sender.to_string()
    };

    // Get NFT's owner
    let nft_owner = query_owner_of(
        deps.querier,
        andromeda_cw721_contract.clone(),
        nft_id.clone(),
    )?
    .owner;

    // Check if sender is the NFT's owner
    require(info.sender == nft_owner, ContractError::Unauthorized {})?;

    // Add lock time to current block time
    let expiration_time = env.block.time.plus_seconds(lock_time);

    // Set lock details
    let lock_details = LockDetails {
        recipient: recip,
        expiration: Expiration::AtTime(expiration_time),
        nft_id: nft_id.clone(),
        nft_contract: andromeda_cw721_contract.clone(),
    };
    // Get timelock's contract address
    let contract_address = env.contract.address;

    // Save all the details. The key represents the concatenated lock_id & the value represents the lock details
    LOCKED_ITEMS.save(deps.storage, &lock_id, &lock_details)?;

    Ok(Response::new()
        // Send NFT to the timelock contract
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: andromeda_cw721_contract,
            msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: contract_address.to_string(),
                token_id: nft_id,
            })?,
            funds: vec![],
        }))
        .add_attribute("action", "locked_nft")
        // The recipient should keep the lock ID to easily claim the NFT
        .add_attribute("lock_id", lock_id))
}

fn query_owner_of(
    querier: QuerierWrapper,
    token_addr: String,
    token_id: String,
) -> Result<OwnerOfResponse, ContractError> {
    let res: OwnerOfResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: token_addr,
        msg: encode_binary(&Cw721QueryMsg::OwnerOf {
            token_id,
            include_expired: None,
        })?,
    }))?;

    Ok(res)
}

fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lock_id: String,
) -> Result<Response, ContractError> {
    // Check if lock ID exists
    let locked_item = LOCKED_ITEMS.may_load(deps.storage, &lock_id)?;
    require(locked_item.is_some(), ContractError::NFTNotFound {})?;

    let locked_nft = locked_item.unwrap();
    // Check if lock is expired
    let expiration = locked_nft.expiration;
    require(
        expiration.is_expired(&env.block),
        ContractError::LockedNFT {},
    )?;

    // check if sender is recipient
    require(
        info.sender == locked_nft.recipient,
        ContractError::Unauthorized {},
    )?;

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
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::LockedToken { lock_id } => encode_binary(&query_locked_token(deps, lock_id)?),
        QueryMsg::Owner {} => encode_binary(&ADOContract::default().query_contract_owner(deps)?),
    }
}
fn query_locked_token(deps: Deps, lock_id: String) -> Result<LockDetails, ContractError> {
    let nft = LOCKED_ITEMS.load(deps.storage, &lock_id)?;
    Ok(nft)
}
