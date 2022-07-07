use std::fmt::format;

use crate::state::{
    read_auction_infos, read_bids, StakedNft, ALLOWED_CONTRACTS, REWARD, STAKED_NFTS,
    UNBONDING_PERIOD,
};
use ado_base::state::ADOContract;
use andromeda_non_fungible_tokens::cw721_staking::{
    AuctionIdsResponse, AuctionStateResponse, Bid, BidsResponse, Cw721HookMsg, ExecuteMsg,
    InstantiateMsg, QueryMsg,
};
use common::{
    ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError, require,
    OrderBy,
};
use cosmwasm_std::{
    attr, coins, entry_point, from_binary, Addr, BankMsg, Binary, BlockInfo, Coin, CosmosMsg, Deps,
    DepsMut, Env, MessageInfo, QuerierWrapper, QueryRequest, Response, Storage, Uint128, WasmMsg,
    WasmQuery,
};
use cw721::{Cw721ExecuteMsg, Cw721QueryMsg, Cw721ReceiveMsg, Expiration, OwnerOfResponse};
// One day in seconds
pub const ONE_DAY: u64 = 86400;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    ALLOWED_CONTRACTS.save(deps.storage, &vec![msg.nft_contract])?;
    UNBONDING_PERIOD.save(deps.storage, &msg.unbonding_period)?;
    REWARD.save(deps.storage, &msg.reward)?;
    ADOContract::default().instantiate(
        deps.storage,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "nft-staking".to_string(),
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
        ExecuteMsg::Claim { key } => execute_claim(deps, env, info, key),
        ExecuteMsg::Unstake { key } => execute_unstake(deps, env, info, key),
        ExecuteMsg::UpdateOwner { address } => {
            ADOContract::default().execute_update_owner(deps, info, address)
        }
    }
}

fn handle_receive_cw721(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&msg.msg)? {
        Cw721HookMsg::Stake {} => execute_stake(
            deps,
            env,
            info,
            msg.sender,
            msg.token_id,
            info.sender.to_string(),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: String,
    token_id: String,
    token_address: String,
) -> Result<Response, ContractError> {
    let allowed_contracts = ALLOWED_CONTRACTS.load(deps.storage)?;
    // NFT has to be sent from an allowed contract
    require(
        allowed_contracts.contains(&token_address),
        ContractError::UnsupportedNFT {},
    )?;

    let key = format!("{:?}{:?}", token_address, token_id);
    let data = StakedNft {
        owner: sender,
        id: token_id,
        contract_address: token_address,
        time_of_staking: env.block.time,
        time_of_unbonding: None,
        reward: None,
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
    let nft = STAKED_NFTS.may_load(deps.storage, key)?;
    if let Some(nft) = nft {
        // Only owner can claim the NFT
        require(info.sender == nft.owner, ContractError::Unauthorized {})?;

        // Can't unbond twice
        require(
            nft.time_of_unbonding.is_none(),
            ContractError::AlreadyUnbonded {},
        )?;

        let current_time = env.block.time;

        let time_spent_bonded = current_time.seconds() - nft.time_of_staking.seconds();

        // Time spent bonded should be at least a day
        require(
            time_spent_bonded >= ONE_DAY,
            ContractError::InsufficientBondedTime {},
        )?;

        let reward = REWARD.load(deps.storage)?;

        let payment = reward.amount * Uint128::from(time_spent_bonded);

        let new_reward = Coin {
            denom: reward.denom,
            amount: payment,
        };

        let new_data = StakedNft {
            owner: nft.owner,
            id: nft.id,
            contract_address: nft.contract_address,
            time_of_staking: nft.time_of_staking,
            time_of_unbonding: Some(env.block.time),
            reward: Some(new_reward),
        };

        STAKED_NFTS.save(deps.storage, key, &new_data)?;

        Ok(Response::new().add_attribute("action", "unbonded"))
    } else {
        Err(ContractError::OutOfNFTs {})
    }
}

fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    key: String,
) -> Result<Response, ContractError> {
    let nft = STAKED_NFTS.may_load(deps.storage, key)?;
    if let Some(nft) = nft {
        // Only owner can claim the NFT
        require(info.sender == nft.owner, ContractError::Unauthorized {})?;

        // NFT should be unbonded
        if let Some(time_of_unbonding) = nft.time_of_unbonding {
            let unbonding_period = UNBONDING_PERIOD.load(deps.storage)?;

            // Calculate the time passed since unbonding
            let time_spent_unbonded = env.block.time.seconds() - time_of_unbonding.seconds();

            // time spent unbonded should equal or exceed the unbonding period
            require(
                time_spent_unbonded >= unbonding_period,
                ContractError::IncompleteUnbondingPeriod {},
            )?;

            // Remove NFT from list of staked NFTs
            STAKED_NFTS.remove(deps.storage, key);

            // payout rewards and send back NFT
            Ok(Response::new()
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: env.contract.address.to_string(),
                    msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                        recipient: nft.owner,
                        token_id: nft.id,
                    })?,
                    funds: vec![nft.reward.unwrap_or_default()],
                }))
                .add_attribute("action", "claimed_nft"))
        } else {
            Err(ContractError::StillBonded {})
        }
    } else {
        Err(ContractError::OutOfNFTs {})
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::StakedNft { key } => encode_binary(&query_staked_nft(deps, key)?),
        QueryMsg::Owner {} => encode_binary(&ADOContract::default().query_contract_owner(deps)?),
    }
}

fn query_staked_nft(deps: Deps, key: String) -> Result<StakedNft, ContractError> {
    let nft = STAKED_NFTS.may_load(deps.storage, key)?;
    if let Some(nft) = nft {
        Ok(nft)
    } else {
        Err(ContractError::OutOfNFTs {})
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_querier::{
        mock_dependencies_custom, MOCK_TOKEN_ADDR, MOCK_TOKEN_OWNER, MOCK_UNCLAIMED_TOKEN,
    };
    use crate::state::AuctionInfo;
    use andromeda_non_fungible_tokens::auction::{Cw721HookMsg, ExecuteMsg, InstantiateMsg};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, coin, coins, from_binary, BankMsg, CosmosMsg, Response, Timestamp};
    use cw721::Expiration;
}
