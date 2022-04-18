use crate::state::{CW721_CONTRACT, LIST, STATE, STATUS};
use ado_base::ADOContract;
use andromeda_protocol::gumball::{LatestRandomResponse, State};
use andromeda_protocol::{
    cw721::{ExecuteMsg as Cw721ExecuteMsg, MintMsg, QueryMsg as Cw721QueryMsg, TokenExtension},
    gumball::{
        ExecuteMsg, InstantiateMsg, NumberOfNFTsResponse, QueryMsg, RandQueryMsg, StateResponse,
    },
    rates::get_tax_amount,
    receipt::Receipt,
};
use base64::decode;
use common::parse_message;
use convert_base::Convert;
use cw2::{get_contract_version, set_contract_version};
use std::{collections::btree_set::Union, convert::TryFrom};
use terrand;

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
const CONTRACT_NAME: &str = "crates.io:andromeda_gumball";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const TERRAND_ADDRESS_MAINNET: &str = "terra1s90fm6hmh5n9drvucvv076ldemlqhe032qtjdq";
const TERRAND_ADDRESS_TESTNET: &str = "terra1a62jxn3hh54fa5slan4dkd7u6v4nzgz3pjhygm";

pub const MOCK_TOKEN_CONTRACT: &str = "token_contract";
pub const MOCK_PRIMITIVE_CONTRACT: &str = "primitive_contract";
pub const MOCK_RATES_CONTRACT: &str = "rates_contract";

pub const GENESIS_TIME: u64 = 1595431050;
pub const PERIOD: u64 = 30;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CW721_CONTRACT.save(deps.storage, &msg.andromeda_cw721_contract)?;
    // Set initial status to false since there's nothing to buy upon instantiation
    STATUS.save(deps.storage, &false)?;
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
        ExecuteMsg::SaleDetails {
            price,
            max_amount_per_wallet,
            recipient,
        } => execute_sale_details(deps, env, info, price, max_amount_per_wallet, recipient),
        ExecuteMsg::SwitchStatus {} => execute_switch_status(deps, info),
    }
}
fn execute_switch_status(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let mut status = STATUS.load(deps.storage)?;
    require(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    // Automatically switch to opposite status
    if status == true {
        status = false;
    } else {
        status = true;
    }
    STATUS.save(deps.storage, &status)?;
    Ok(Response::new().add_attribute("action", "Switched Status"))
}
fn execute_sale_details(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    price: Coin,
    max_amount_per_wallet: Option<Uint128>,
    recipient: Recipient,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let status = STATUS.load(deps.storage)?;
    // Check status, can't change sale details while buying is allowed
    require(status == false, ContractError::Refilling {})?;
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

    // Set the state
    let state = State {
        price,
        max_amount_per_wallet,
        recipient: recipient.clone(),
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
    ]))
}
fn execute_mint(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    mint_msg: Box<MintMsg<TokenExtension>>,
) -> Result<Response, ContractError> {
    let mut list = LIST.load(deps.storage)?;
    let status = STATUS.load(deps.storage)?;
    let contract = ADOContract::default();
    // check authority
    require(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    // Can only mint when in "refill" mode, and that's when status is set to false.
    require(status == false, ContractError::NotInRefillMode {})?;
    let config = CW721_CONTRACT.load(deps.storage)?;
    println!("1");
    // Add to list of NFTs
    list.push(mint_msg.clone().token_id);

    let mission_contract = contract.get_mission_contract(deps.storage)?;
    let contract_addr = config.get_address(deps.api, &deps.querier, mission_contract)?;
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
    let status = STATUS.load(deps.storage)?;
    // check gumball's status
    require(status == true, ContractError::Refilling {})?;
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
    // get random number, current form: Binary
    let rand_res =
        terrand::contract::query(deps.as_ref(), env, terrand::msg::QueryMsg::LatestDrand {})?;
    // Binary --> Base64
    let random_hex: LatestRandomResponse = from_binary(&rand_res)?;
    // Base64 --> Vec<u8> using the base64 decode function
    let random_vector = decode(random_hex.randomness).unwrap();
    // Vec<u8> --> Vec<u64> to be able to fit the sum of all the elements
    let ran_vec: Vec<u64> = random_vector.iter().map(|x| *x as u64).collect();
    // Concatinating the elements of the random number would yield an unworkably large number
    // So I opted for the sum. Still random and large enough to work with modulus of list's length
    let random_number: u64 = ran_vec.iter().sum();
    // Use modulus to get a random index of the NFTs list
    let index = random_number as usize % n_of_nfts;
    // Select NFT using index
    let random_nft = list[index].clone();
    // Remove NFT from list using index
    list.remove(index);

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract.clone().identifier,
            msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: info.sender.to_string(),
                token_id: random_nft.clone(),
            })?,
            funds: vec![],
        }))
        .add_attribute("action", "claim")
        .add_attribute("token_id", random_nft)
        .add_attribute("token_contract", contract.identifier)
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
fn query_number_of_nfts(deps: Deps) -> Result<NumberOfNFTsResponse, ContractError> {
    let list = LIST.load(deps.storage)?;
    let number = list.len();
    Ok(NumberOfNFTsResponse { number })
}
fn query_state(deps: Deps) -> Result<StateResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    Ok(StateResponse { state })
}

#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_protocol::testing::mock_querier::mock_dependencies_custom;
    use common::ado_base::recipient::Recipient;
    use common::mission::AndrAddress;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, from_binary, Coin, Decimal, Timestamp};

    fn get_rand(deps: Deps, env: Env) -> Result<Response, ContractError> {
        // CosmWasm 0.16
        let timestamp_now = env.block.time.seconds() + 100000000;

        // Get the current block time from genesis time
        let from_genesis = timestamp_now - GENESIS_TIME;

        // Get the current round
        let current_round = from_genesis / PERIOD;
        // Get the next round
        let next_round = current_round + 1;
        // get random number
        let rand_res = terrand::contract::query(
            deps,
            env,
            terrand::msg::QueryMsg::GetRandomness { round: next_round },
        )?;
        let random_hex: String = from_binary(&rand_res)?;
        println!("{}", random_hex);
        Ok(Response::new().add_attribute("action", "fetched random number"))
    }

    fn mint(deps: DepsMut, token_id: impl Into<String>) -> Result<Response, ContractError> {
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
        execute(deps, mock_env(), mock_info("owner", &[]), msg)
    }

    #[test]
    fn test_mint_successful() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: AndrAddress {
                identifier: "cw721_contract".to_string(),
            },
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();

        let res = mint(deps.as_mut(), "token_id").unwrap();

        let mint_msg = Box::new(MintMsg {
            token_id: "token_id".to_string(),
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
        });

        assert_eq!(
            Response::new()
                .add_attribute("action", "mint")
                .add_message(WasmMsg::Execute {
                    contract_addr: MOCK_TOKEN_CONTRACT.to_owned(),
                    msg: encode_binary(&Cw721ExecuteMsg::Mint(mint_msg)).unwrap(),
                    funds: vec![],
                }),
            res
        );
        let list = LIST.load(&deps.storage).unwrap();

        assert_eq!(list.contains(&"token_id".to_string()), true);
    }

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: AndrAddress {
                identifier: "cw721_contract".to_string(),
            },
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        let status = STATUS.load(&deps.storage).unwrap();
        assert_eq!(status, false);
    }
    #[test]
    fn test_sale_details_unauthorized() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: AndrAddress {
                identifier: "cw721_contract".to_string(),
            },
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::SaleDetails {
            price: coin(5, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(1 as u64)),
            recipient: Recipient::Addr("me".to_string()),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(res, ContractError::Unauthorized {});
    }
    #[test]
    fn test_sale_details_invalid_price() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: AndrAddress {
                identifier: "cw721_contract".to_string(),
            },
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SaleDetails {
            price: coin(0, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(1 as u64)),
            recipient: Recipient::Addr("me".to_string()),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(res, ContractError::InvalidZeroAmount {});
    }
    #[test]
    fn test_sale_details_invalid_denomination() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: AndrAddress {
                identifier: "cw721_contract".to_string(),
            },
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SaleDetails {
            price: coin(10, "LUNA"),
            max_amount_per_wallet: Some(Uint128::from(1 as u64)),
            recipient: Recipient::Addr("me".to_string()),
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
    fn test_sale_details_max_amount_per_wallet() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: AndrAddress {
                identifier: "cw721_contract".to_string(),
            },
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SaleDetails {
            price: coin(10, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(0 as u64)),
            recipient: Recipient::Addr("me".to_string()),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(res, ContractError::InvalidZeroAmount {});
    }
    #[test]
    fn test_sale_details() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: AndrAddress {
                identifier: "cw721_contract".to_string(),
            },
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SaleDetails {
            price: coin(10, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(1 as u64)),
            recipient: Recipient::Addr("me".to_string()),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            res,
            Response::new().add_attributes(vec![
                attr("action", "switch status"),
                attr("price", coin(10, "uusd").to_string()),
                attr("max_amount_per_wallet", Uint128::from(1 as u64)),
                attr("recipient", "me".to_string(),),
            ])
        );
    }
    #[test]
    fn test_switch_status() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: AndrAddress {
                identifier: "cw721_contract".to_string(),
            },
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let status = STATUS.load(&deps.storage).unwrap();
        assert_eq!(status, false);
        let info = mock_info("owner", &[]);
        execute_switch_status(deps.as_mut(), info).unwrap();
        let status = STATUS.load(&deps.storage).unwrap();
        assert_eq!(status, true);
        let info = mock_info("owner", &[]);
        execute_switch_status(deps.as_mut(), info).unwrap();
        let status = STATUS.load(&deps.storage).unwrap();
        assert_eq!(status, false);
        let info = mock_info("anyone", &[]);
        let err = execute_switch_status(deps.as_mut(), info).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }
    #[test]
    fn test_mint_unauthorized() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: AndrAddress {
                identifier: "cw721_contract".to_string(),
            },
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::Mint(Box::new(MintMsg {
            token_id: "token_id".to_string(),
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
        let info = mock_info("not_owner", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        assert_eq!(ContractError::Unauthorized {}, res);
    }

    #[test]
    fn test_buy_refill() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: AndrAddress {
                identifier: "cw721_contract".to_string(),
            },
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SaleDetails {
            price: coin(10, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(1 as u64)),
            recipient: Recipient::Addr("me".to_string()),
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        println!("works here");
        let info = mock_info("buyer", &[]);

        let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
            token_id: "token_id".to_string(),
            owner: "not_crowdfund".to_string(),
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
        execute(deps.as_mut(), mock_env(), info, mint_msg).unwrap();

        let info = mock_info("anyone", &[coin(10, "uusd")]);
        let msg = ExecuteMsg::Buy {};
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Refilling {});
    }
}
