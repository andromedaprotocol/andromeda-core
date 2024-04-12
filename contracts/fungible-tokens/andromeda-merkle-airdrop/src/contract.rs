// The majority of the code was taken unchanged from
// https://github.com/CosmWasm/cw-tokens/blob/main/contracts/cw20-merkle-airdrop/src/contract.rs
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, ensure, to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty,
    Env, MessageInfo, Response, StdResult, Timestamp, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use cw_asset::AssetInfoBase;
use cw_utils::{nonpayable, Expiration};
use sha2::Digest;
use std::convert::TryInto;

use crate::state::{
    Config, CLAIM, CONFIG, LATEST_STAGE, MERKLE_ROOT, STAGE_AMOUNT, STAGE_AMOUNT_CLAIMED,
    STAGE_EXPIRATION,
};
use andromeda_fungible_tokens::airdrop::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, IsClaimedResponse, LatestStageResponse,
    MerkleRootResponse, QueryMsg, TotalClaimedResponse,
};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    common::{
        actions::call_action, context::ExecuteContext, encode_binary, MillisecondsExpiration,
    },
    error::ContractError,
};

// Version info, for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-merkle-airdrop";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        asset_info: msg.asset_info.check(deps.api, None)?,
    };
    CONFIG.save(deps.storage, &config)?;

    let stage = 0;
    LATEST_STAGE.save(deps.storage, &stage)?;

    let contract = ADOContract::default();
    let resp = contract.instantiate(
        deps.storage,
        env,
        deps.api,
        &deps.querier,
        info,
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let ctx = ExecuteContext::new(deps, info, env);

    match msg {
        ExecuteMsg::AMPReceive(pkt) => {
            ADOContract::default().execute_amp_receive(ctx, pkt, handle_execute)
        }
        _ => handle_execute(ctx, msg),
    }
}

pub fn handle_execute(mut ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    let action_response = call_action(
        &mut ctx.deps,
        &ctx.info,
        &ctx.env,
        &ctx.amp_ctx,
        msg.as_ref(),
    )?;
    let res = match msg {
        ExecuteMsg::RegisterMerkleRoot {
            merkle_root,
            expiration,
            total_amount,
        } => execute_register_merkle_root(ctx, merkle_root, expiration, total_amount),
        ExecuteMsg::Claim {
            stage,
            amount,
            proof,
        } => execute_claim(ctx, stage, amount, proof),
        ExecuteMsg::Burn { stage } => execute_burn(ctx, stage),
        _ => ADOContract::default().execute(ctx, msg),
    }?;
    Ok(res
        .add_submessages(action_response.messages)
        .add_attributes(action_response.attributes)
        .add_events(action_response.events))
}

pub fn execute_register_merkle_root(
    ctx: ExecuteContext,
    merkle_root: String,
    expiration: Option<MillisecondsExpiration>,
    total_amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;
    nonpayable(&info)?;

    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    // check merkle root length
    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(&merkle_root, &mut root_buf)?;

    let stage = LATEST_STAGE.update(deps.storage, |stage| -> StdResult<_> { Ok(stage + 1) })?;

    MERKLE_ROOT.save(deps.storage, stage, &merkle_root)?;
    LATEST_STAGE.save(deps.storage, &stage)?;

    // save expiration
    STAGE_EXPIRATION.save(deps.storage, stage, &expiration)?;

    // save total airdropped amount
    let amount = total_amount.unwrap_or_else(Uint128::zero);
    STAGE_AMOUNT.save(deps.storage, stage, &amount)?;
    STAGE_AMOUNT_CLAIMED.save(deps.storage, stage, &Uint128::zero())?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "register_merkle_root"),
        attr("stage", stage.to_string()),
        attr("merkle_root", merkle_root),
        attr("total_amount", amount),
    ]))
}

pub fn execute_claim(
    ctx: ExecuteContext,
    stage: u8,
    amount: Uint128,
    proof: Vec<String>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;
    nonpayable(&info)?;

    // not expired
    let expiration = STAGE_EXPIRATION.load(deps.storage, stage)?;
    let expiration = if let Some(expiration) = expiration {
        Expiration::AtTime(Timestamp::from_nanos(expiration.nanos()))
    } else {
        Expiration::Never {}
    };
    ensure!(
        !expiration.is_expired(&env.block),
        ContractError::StageExpired { stage, expiration }
    );

    // verify not claimed
    ensure!(
        !CLAIM.has(deps.storage, (&info.sender, stage)),
        ContractError::Claimed {}
    );

    let config = CONFIG.load(deps.storage)?;
    let merkle_root = MERKLE_ROOT.load(deps.storage, stage)?;

    let user_input = format!("{}{}", info.sender, amount);
    let hash = sha2::Sha256::digest(user_input.as_bytes())
        .as_slice()
        .try_into()
        .map_err(|_| ContractError::WrongLength {})?;

    let hash = proof.into_iter().try_fold(hash, |hash, p| {
        let mut proof_buf = [0; 32];
        hex::decode_to_slice(p, &mut proof_buf)?;
        let mut hashes = [hash, proof_buf];
        hashes.sort_unstable();
        sha2::Sha256::digest(hashes.concat())
            .as_slice()
            .try_into()
            .map_err(|_| ContractError::WrongLength {})
    })?;

    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(merkle_root, &mut root_buf)?;
    ensure!(root_buf == hash, ContractError::VerificationFailed {});

    // Update claim index to the current stage
    CLAIM.save(deps.storage, (&info.sender, stage), &true)?;

    // Update total claimed to reflect
    let mut claimed_amount = STAGE_AMOUNT_CLAIMED.load(deps.storage, stage)?;
    claimed_amount = claimed_amount.checked_add(amount)?;
    STAGE_AMOUNT_CLAIMED.save(deps.storage, stage, &claimed_amount)?;

    let transfer_msg: CosmosMsg = match config.asset_info {
        AssetInfoBase::Native(denom) => CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin { amount, denom }],
        }),
        AssetInfoBase::Cw20(address) => CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: address.to_string(),
            funds: vec![],
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount,
            })?,
        }),
        _ => CosmosMsg::Custom(Empty {}),
    };

    let res = Response::new()
        .add_message(transfer_msg)
        .add_attributes(vec![
            attr("action", "claim"),
            attr("stage", stage.to_string()),
            attr("address", info.sender),
            attr("amount", amount),
        ]);
    Ok(res)
}

pub fn execute_burn(ctx: ExecuteContext, stage: u8) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;
    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    // make sure is expired
    let expiration = STAGE_EXPIRATION.load(deps.storage, stage)?;
    let expiration = if let Some(expiration) = expiration {
        Expiration::AtTime(Timestamp::from_nanos(expiration.nanos()))
    } else {
        Expiration::Never {}
    };
    ensure!(
        expiration.is_expired(&env.block),
        ContractError::StageNotExpired { stage, expiration }
    );

    // Get total amount per stage and total claimed
    let total_amount = STAGE_AMOUNT.load(deps.storage, stage)?;
    let claimed_amount = STAGE_AMOUNT_CLAIMED.load(deps.storage, stage)?;

    // impossible but who knows
    ensure!(
        claimed_amount <= total_amount,
        ContractError::Unauthorized {}
    );

    // Get balance
    let balance_to_burn = total_amount - claimed_amount;

    let config = CONFIG.load(deps.storage)?;
    let burn_msg = match config.asset_info {
        AssetInfoBase::Native(denom) => CosmosMsg::Bank(BankMsg::Burn {
            amount: vec![Coin {
                amount: balance_to_burn,
                denom,
            }],
        }),
        AssetInfoBase::Cw20(address) => CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: address.to_string(),
            funds: vec![],
            msg: to_json_binary(&Cw20ExecuteMsg::Burn {
                amount: balance_to_burn,
            })?,
        }),
        _ => CosmosMsg::Custom(Empty {}),
    };

    // Burn the tokens and response
    let res = Response::new().add_message(burn_msg).add_attributes(vec![
        attr("action", "burn"),
        attr("stage", stage.to_string()),
        attr("address", info.sender),
        attr("amount", balance_to_burn),
    ]);
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => encode_binary(&query_config(deps)?),
        QueryMsg::MerkleRoot { stage } => encode_binary(&query_merkle_root(deps, stage)?),
        QueryMsg::LatestStage {} => encode_binary(&query_latest_stage(deps)?),
        QueryMsg::IsClaimed { stage, address } => {
            encode_binary(&query_is_claimed(deps, stage, address)?)
        }
        QueryMsg::TotalClaimed { stage } => encode_binary(&query_total_claimed(deps, stage)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        asset_info: config.asset_info,
    })
}

pub fn query_merkle_root(deps: Deps, stage: u8) -> Result<MerkleRootResponse, ContractError> {
    let merkle_root = MERKLE_ROOT.load(deps.storage, stage)?;
    let expiration = STAGE_EXPIRATION.load(deps.storage, stage)?;
    let total_amount = STAGE_AMOUNT.load(deps.storage, stage)?;

    let resp = MerkleRootResponse {
        stage,
        merkle_root,
        expiration,
        total_amount,
    };

    Ok(resp)
}

pub fn query_latest_stage(deps: Deps) -> Result<LatestStageResponse, ContractError> {
    let latest_stage = LATEST_STAGE.load(deps.storage)?;
    let resp = LatestStageResponse { latest_stage };

    Ok(resp)
}

pub fn query_is_claimed(
    deps: Deps,
    stage: u8,
    address: String,
) -> Result<IsClaimedResponse, ContractError> {
    let key: (&Addr, u8) = (&deps.api.addr_validate(&address)?, stage);
    let is_claimed = CLAIM.may_load(deps.storage, key)?.unwrap_or(false);
    let resp = IsClaimedResponse { is_claimed };

    Ok(resp)
}

pub fn query_total_claimed(deps: Deps, stage: u8) -> Result<TotalClaimedResponse, ContractError> {
    let total_claimed = STAGE_AMOUNT_CLAIMED.load(deps.storage, stage)?;
    let resp = TotalClaimedResponse { total_claimed };

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}
