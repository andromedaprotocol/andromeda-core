use crate::state::{CONFIG, CW721_CONTRACT, LIST, STATE};
use ado_base::ADOContract;
use andromeda_protocol::gumball::State;
use andromeda_protocol::{
    cw721::{ExecuteMsg as Cw721ExecuteMsg, MintMsg, QueryMsg as Cw721QueryMsg, TokenExtension},
    gumball::{ExecuteMsg, GetNumberOfNFTsResponse, GetStateResponse, InstantiateMsg, QueryMsg},
    rates::get_tax_amount,
    receipt::Receipt,
};
use cw2::{get_contract_version, set_contract_version};
use std::{collections::btree_set::Union, convert::TryFrom};
use terrand::contract::{self, add_random};
use terrand::msg::GetRandomResponse;

use common::{
    ado_base::{recipient::Recipient, InstantiateMsg as BaseInstantiateMsg},
    encode_binary,
    error::ContractError,
    merge_sub_msgs, require, Funds,
};
use cosmwasm_std::{attr, entry_point, from_binary, to_binary};
use cosmwasm_std::{
    has_coins, Api, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    QuerierWrapper, QueryRequest, Response, Storage, SubMsg, Uint128, WasmMsg, WasmQuery,
};
use cw0::Expiration;
use cw721::{OwnerOfResponse, TokensResponse};
use schemars::_private::NoSerialize;
// terrand-specific
const GENESIS_TIME: u64 = 1595431050;
const PERIOD: u64 = 30;

const CONTRACT_NAME: &str = "crates.io:andromeda_gumball";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CW721_CONTRACT.save(deps.storage, &msg.andromeda_cw721_contract)?;

    ADOContract::default().instantiate(
        deps.storage,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "gumball".to_string(),
            operators: None,
            modules: None,
            primitive_contract: None,
        },
    )
}
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    match msg {
        ExecuteMsg::AndrReceive(msg) => contract.execute(deps, env, info, msg, execute),
        ExecuteMsg::Mint(mint_msg) => execute_mint(deps, env, info, mint_msg),
        ExecuteMsg::Buy {} => execute_buy(deps, env, info),
        ExecuteMsg::SwitchState {
            price,
            max_amount_per_wallet,
            recipient,
            status,
        } => execute_switch_state(
            deps,
            env,
            info,
            price,
            max_amount_per_wallet,
            recipient,
            status,
        ),
    }
}
fn execute_switch_state(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    price: Coin,
    max_amount_per_wallet: Option<Uint128>,
    recipient: Recipient,
    status: bool,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    // Check authority
    require(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    // Check valid amount
    require(
        price.amount > Uint128::from(0 as u64),
        ContractError::InvalidZeroAmount {},
    )?;
    // Check valid denomination
    require(
        price.denom == "uusd".to_string(),
        ContractError::InvalidFunds {
            msg: "Only uusd is allowed".to_string(),
        },
    )?;
    // Check valid max amount per wallet
    require(
        max_amount_per_wallet > Some(Uint128::from(0 as u64)),
        ContractError::InvalidZeroAmount {},
    )?;
    let max_amount_per_wallet = max_amount_per_wallet.unwrap_or_else(|| Uint128::from(1u128));
    // This is to prevent cloning price.
    let price_str = price.to_string();
    let state = State {
        price,
        max_amount_per_wallet,
        recipient: recipient.clone(),
        status,
    };
    STATE.save(deps.storage, &state)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "switch status"),
        attr("price", price_str),
        attr("max_amount_per_wallet", max_amount_per_wallet),
        attr(
            "recipient",
            recipient.get_addr(
                deps.api,
                &deps.querier,
                contract.get_mission_contract(deps.storage)?,
            )?,
        ),
        attr("status", status.to_string()),
    ]))
}
fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mint_msg: Box<MintMsg<TokenExtension>>,
) -> Result<Response, ContractError> {
    let mut list = LIST.load(deps.storage)?;
    let status = STATE.load(deps.storage)?;
    let contract = ADOContract::default();

    require(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    // Can only mint when in "refill" mode, and that's when status is set to false.
    require(status.status == false, ContractError::NotInRefillMode {})?;
    let config = CONFIG.load(deps.storage)?;
    // Add to list of NFTs
    list.push(mint_msg.clone().token_id);

    let mission_contract = contract.get_mission_contract(deps.storage)?;
    let contract_addr =
        config
            .token_address
            .get_address(deps.api, &deps.querier, mission_contract)?;
    Ok(Response::new()
        .add_attribute("action", "mint")
        .add_message(WasmMsg::Execute {
            contract_addr,
            msg: encode_binary(&Cw721ExecuteMsg::Mint(mint_msg))?,
            funds: vec![],
        }))
}

fn execute_buy(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let mut list = LIST.load(deps.storage)?;
    let n_of_nfts = list.len();
    let sent_funds = &info.funds[0];
    let state = STATE.load(deps.storage)?;
    let contract = CW721_CONTRACT.load(deps.storage)?;
    // check gumball's status
    require(state.status == true, ContractError::Refilling {})?;
    // check if we still have any NFTs left
    require(n_of_nfts > 0, ContractError::OutOfNFTs {})?;
    // check for correct denomination
    require(
        sent_funds.denom == "uusd".to_string(),
        ContractError::InvalidFunds {
            msg: "Only uusd is accepted".to_string(),
        },
    )?;

    // check if more than one type of coin was sent
    require(
        info.funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Only one type of coin is required (uusd).".to_string(),
        },
    )?;

    // check for correct amount of funds
    require(
        sent_funds.amount == state.price.amount,
        ContractError::InsufficientFunds {},
    )?;
    // get random number in range of [0, list.len() - 1]
    let timestamp_now = env.block.time.seconds();

    // Get the current block time from genesis time
    let from_genesis = timestamp_now - GENESIS_TIME;

    // Get the current round
    let current_round = from_genesis / PERIOD;
    // Get the next round
    let next_round = current_round + 1;
    // let rand = add_random(deps, env, info, current_round, encode_binary("o/xvA7KrfsthQu6gOqnjiLRwCCUbYA6sDnhF2Vl+95Jh6XsPkZrx93wLn9ukNNyXGQPErcLqWbM8iR2MnZhMWeSuciJ1OvtDZA2ayLoAoOSjrI8ZV8ZP6ekVIIgXBRbK")?, encode_binary("j9GtATWs9bZ7By6qxQTkA+9AaD+YT2zaw1qPHghmBeRhhu9A3FRHdphT7aMF2WAdDGXmSqf6alA7n2P6GOXtRz6ctns3Kkq7jl2zzpzGSgguxvAli5rFMQqSK3iaBSJa")?)?;
    let random_number = 0;
    // select random NFT from list by using that random number as index of vector
    let random_nft = &list.clone()[random_number];
    // remove the selected token from the vector
    list.remove(random_number);
    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract.clone(),
            msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: info.sender.to_string(),
                token_id: random_nft.to_owned(),
            })?,
            funds: vec![],
        }))
        .add_attribute("action", "claim")
        .add_attribute("token_id", random_nft)
        .add_attribute("token_contract", contract)
        .add_attribute("recipient", info.sender.to_string().clone()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::NumberOfNFTs {} => encode_binary(&query_number_of_nfts(deps)?),
        QueryMsg::State {} => encode_binary(&query_state(deps)?),
    }
}
fn query_number_of_nfts(deps: Deps) -> Result<GetNumberOfNFTsResponse, ContractError> {
    let list = LIST.load(deps.storage)?;
    let number = list.len();
    Ok(GetNumberOfNFTsResponse { number })
}
fn query_state(deps: Deps) -> Result<GetStateResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    Ok(GetStateResponse { state })
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::ado_base::recipient::Recipient;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, from_binary, Coin, Decimal};

    fn mint(deps: DepsMut, token_id: impl Into<String>) -> Result<Response, ContractError> {
        println!("works here 2");
        let msg = ExecuteMsg::Mint(Box::new(MintMsg {
            token_id: token_id.into(),
            owner: mock_env().contract.address.to_string(),
            token_uri: None,
            extension: TokenExtension {
                name: "name".to_string(),
                publisher: "publisher".to_string(),
                description: None,
                transfer_agreement: None,
                metadata: None,
                archived: false,
                pricing: None,
            },
        }));
        println!("works hre 3");
        execute(deps, mock_env(), mock_info("owner", &[]), msg)
    }

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: "cw721_contract".to_string(),
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }
    #[test]
    fn test_switch_state_unauthorized() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: "cw721_contract".to_string(),
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::SwitchState {
            price: coin(5, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(1 as u64)),
            recipient: Recipient::Addr("me".to_string()),
            status: false,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(res, ContractError::Unauthorized {});
    }
    #[test]
    fn test_switch_state_invalid_price() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: "cw721_contract".to_string(),
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SwitchState {
            price: coin(0, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(1 as u64)),
            recipient: Recipient::Addr("me".to_string()),
            status: false,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(res, ContractError::InvalidZeroAmount {});
    }
    #[test]
    fn test_switch_state_invalid_denomination() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: "cw721_contract".to_string(),
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SwitchState {
            price: coin(10, "LUNA"),
            max_amount_per_wallet: Some(Uint128::from(1 as u64)),
            recipient: Recipient::Addr("me".to_string()),
            status: false,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            res,
            ContractError::InvalidFunds {
                msg: "Only uusd is allowed".to_string(),
            }
        );
    }
    #[test]
    fn test_switch_state_max_amount_per_wallet() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: "cw721_contract".to_string(),
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SwitchState {
            price: coin(10, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(0 as u64)),
            recipient: Recipient::Addr("me".to_string()),
            status: false,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(res, ContractError::InvalidZeroAmount {});
    }
    #[test]
    fn test_switch_state() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: "cw721_contract".to_string(),
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SwitchState {
            price: coin(10, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(1 as u64)),
            recipient: Recipient::Addr("me".to_string()),
            status: false,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            res,
            Response::new().add_attributes(vec![
                attr("action", "switch status"),
                attr("price", coin(10, "uusd").to_string()),
                attr("max_amount_per_wallet", Uint128::from(1 as u64)),
                attr("recipient", "me".to_string(),),
                attr("status", false.to_string()),
            ])
        );
    }
    #[test]
    fn test_buy_refill() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: "cw721_contract".to_string(),
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SwitchState {
            price: coin(10, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(1 as u64)),
            recipient: Recipient::Addr("me".to_string()),
            status: false,
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        println!("works here");

        mint(deps.as_mut(), "token_id".to_string()).unwrap();

        let info = mock_info("anyone", &[coin(10, "uusd")]);
        let msg = ExecuteMsg::Buy {};
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Refilling {});
    }
}
