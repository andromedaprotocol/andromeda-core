use crate::state::{State, CW721_CONTRACT, LIST, RANDOMNESS_PROVIDER, STATE, STATUS};
use ado_base::ADOContract;
use andromeda_non_fungible_tokens::gumball::{GumballMintMsg, LatestRandomResponse};
use andromeda_non_fungible_tokens::{
    cw721::{ExecuteMsg as Cw721ExecuteMsg, MintMsg, TokenExtension},
    gumball::{ExecuteMsg, InstantiateMsg, NumberOfNftsResponse, QueryMsg, StatusResponse},
};
use common::{
    ado_base::{recipient::Recipient, InstantiateMsg as BaseInstantiateMsg},
    encode_binary,
    error::ContractError,
    require,
};
use cosmwasm_std::{attr, entry_point, Binary, Storage};
use cosmwasm_std::{
    Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QueryRequest, Response, Uint128, WasmMsg,
    WasmQuery,
};
use cw2::set_contract_version;

const CONTRACT_NAME: &str = "crates.io:andromeda_gumball";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const UUSD_DENOM: &str = "uusd";

pub(crate) const MAX_MINT_LIMIT: u32 = 100;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CW721_CONTRACT.save(deps.storage, &msg.andromeda_cw721_contract)?;
    // Set initial status to false since there's nothing to buy upon instantiation
    let new_list: Vec<String> = Vec::new();
    LIST.save(deps.storage, &new_list)?;
    STATUS.save(deps.storage, &false)?;
    RANDOMNESS_PROVIDER.save(deps.storage, &msg.randomness_source)?;
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
        ExecuteMsg::SetSaleDetails {
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
    // in case owner forgot to set the state, can't allow purchasing without the sale details set
    let state = STATE.may_load(deps.storage)?;
    require(state.is_some(), ContractError::PriceNotSet {})?;
    // Automatically switch to opposite status
    status = !status;
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
    require(!status, ContractError::Refilling {})?;
    // Check authority
    require(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    // Check valid amount
    require(!price.amount.is_zero(), ContractError::InvalidZeroAmount {})?;
    // Check valid denomination
    require(
        price.denom == UUSD_DENOM,
        ContractError::InvalidFunds {
            msg: "Only uusd is allowed".to_string(),
        },
    )?;
    // Check valid max amount per wallet
    let max_amount_per_wallet = max_amount_per_wallet.unwrap_or_else(|| Uint128::from(1u128));

    require(
        !max_amount_per_wallet.is_zero(),
        ContractError::InvalidZeroAmount {},
    )?;
    // This is to prevent cloning price.
    let price_str = price.to_string();

    let rec = recipient.get_addr(
        deps.api,
        &deps.querier,
        contract.get_mission_contract(deps.storage)?,
    )?;

    // Set the state
    let state = State {
        price,
        max_amount_per_wallet,
        recipient,
    };
    STATE.save(deps.storage, &state)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "switch status"),
        attr("price", price_str),
        attr("max_amount_per_wallet", max_amount_per_wallet),
        attr("recipient", rec),
    ]))
}

fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mint_msgs: Vec<GumballMintMsg>,
) -> Result<Response, ContractError> {
    require(
        mint_msgs.len() <= MAX_MINT_LIMIT as usize,
        ContractError::TooManyMintMessages {
            limit: MAX_MINT_LIMIT,
        },
    )?;
    let status = STATUS.load(deps.storage)?;
    // Can only mint when in "refill" mode, and that's when status is set to false.
    require(!status, ContractError::NotInRefillMode {})?;
    let contract = ADOContract::default();
    // check authority
    require(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    let mission_contract = contract.get_mission_contract(deps.storage)?;

    let token_contract_andr = CW721_CONTRACT.load(deps.storage)?;
    let token_contract =
        token_contract_andr.get_address(deps.api, &deps.querier, mission_contract)?;
    let gumball_contract = env.contract.address.to_string();

    let mut resp = Response::new();
    for mint_msg in mint_msgs {
        let mint_resp = mint(
            deps.storage,
            &gumball_contract,
            token_contract.clone(),
            mint_msg,
        )?;
        resp = resp
            .add_attributes(mint_resp.attributes)
            .add_submessages(mint_resp.messages);
    }

    Ok(resp)
}

fn mint(
    storage: &mut dyn Storage,
    gumball_contract: &str,
    token_contract: String,
    mint_msg: GumballMintMsg,
) -> Result<Response, ContractError> {
    let mint_msg: MintMsg<TokenExtension> = MintMsg {
        token_id: mint_msg.token_id,
        owner: mint_msg
            .owner
            .unwrap_or_else(|| gumball_contract.to_owned()),
        token_uri: mint_msg.token_uri,
        extension: mint_msg.extension,
    };
    // We allow for owners other than the contract, incase the creator wants to set aside a few
    // tokens for some other use, say airdrop, team allocation, etc.  Only those which have the
    // contract as the owner will be available to sell.
    if mint_msg.owner == gumball_contract {
        // Mark token as available to purchase in next sale.
        let mut list = LIST.load(storage)?;
        list.push(mint_msg.token_id.clone());
        LIST.save(storage, &list)?;
    }
    Ok(Response::new()
        .add_attribute("action", "mint")
        .add_message(WasmMsg::Execute {
            contract_addr: token_contract,
            msg: encode_binary(&Cw721ExecuteMsg::Mint(Box::new(mint_msg)))?,
            funds: vec![],
        }))
}

fn execute_buy(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let status = STATUS.load(deps.storage)?;
    // check gumball's status
    require(status, ContractError::Refilling {})?;
    let mut list = LIST.load(deps.storage)?;
    let n_of_nfts = list.len();
    // check if we still have any NFTs left
    require(n_of_nfts > 0, ContractError::OutOfNFTs {})?;
    // check if more than one type of coin was sent
    require(
        info.funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Only one type of coin is required (uusd).".to_string(),
        },
    )?;
    let sent_funds = &info.funds[0];
    // check for correct denomination
    require(
        sent_funds.denom == UUSD_DENOM,
        ContractError::InvalidFunds {
            msg: "Only uusd is accepted".to_string(),
        },
    )?;

    let state = STATE.load(deps.storage)?;
    // check for correct amount of funds
    require(
        sent_funds.amount == state.price.amount,
        ContractError::InsufficientFunds {},
    )?;
    let randomness_source = RANDOMNESS_PROVIDER.load(deps.storage)?;
    let random_response: LatestRandomResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: randomness_source,
            msg: encode_binary(&terrand::msg::QueryMsg::LatestDrand {})?,
        }))?;
    let randomness = Binary::to_base64(&random_response.randomness);
    let vec = randomness.into_bytes();
    let ran_vec: Vec<u64> = vec.iter().map(|x| *x as u64).collect();
    // Concatinating the elements of the random number would yield an unworkably large number
    // So I opted for the sum, which is still random and large enough to work with modulus of list's length
    let mut random_number: u64 = ran_vec.iter().sum();
    // In case the random number is smaller than the number of NFTs
    while random_number < n_of_nfts as u64 {
        random_number *= 2;
    }
    // Use modulus to get a random index of the NFTs list
    let index = random_number as usize % n_of_nfts;
    // Select NFT & remove it from list at the same time. Used swap_remove since it's more efficient and the ordering doesn't matter
    let random_nft = list.swap_remove(index);
    LIST.save(deps.storage, &list)?;
    let token_contract = CW721_CONTRACT.load(deps.storage)?;
    let mission_contract = ADOContract::default().get_mission_contract(deps.storage)?;
    let contract_addr = token_contract.get_address(deps.api, &deps.querier, mission_contract)?;

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.clone(),
            msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: info.sender.to_string(),
                token_id: random_nft.clone(),
            })?,
            funds: vec![],
        }))
        .add_attribute("action", "claim")
        .add_attribute("token_id", random_nft)
        .add_attribute("token_contract", contract_addr)
        .add_attribute("recipient", info.sender.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::NumberOfNfts {} => encode_binary(&query_number_of_nfts(deps)?),
        QueryMsg::SaleDetails {} => encode_binary(&query_state(deps)?),
        QueryMsg::Status {} => encode_binary(&query_status(deps)?),
    }
}

fn query_status(deps: Deps) -> Result<StatusResponse, ContractError> {
    let status = STATUS.load(deps.storage)?;
    Ok(StatusResponse { status })
}

fn query_number_of_nfts(deps: Deps) -> Result<NumberOfNftsResponse, ContractError> {
    let list = LIST.load(deps.storage)?;
    let number = list.len();
    Ok(NumberOfNftsResponse { number })
}

fn query_state(deps: Deps) -> Result<State, ContractError> {
    let state = STATE.load(deps.storage)?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::ado_base::recipient::Recipient;
    use common::mission::AndrAddress;
    use cosmwasm_std::coin;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    pub const MOCK_TOKEN_CONTRACT: &str = "cw721_contract";

    fn mint(deps: DepsMut, token_id: impl Into<String>) -> Result<Response, ContractError> {
        let msg = ExecuteMsg::Mint(vec![
            (GumballMintMsg {
                token_id: token_id.into(),
                owner: None,
                token_uri: None,
                extension: TokenExtension {
                    name: "name".to_string(),
                    publisher: "publisher".to_string(),
                    description: None,
                    attributes: vec![],
                    image: String::from(""),
                    image_data: None,
                    external_url: None,
                    animation_url: None,
                    youtube_url: None,
                },
            }),
        ]);

        execute(deps, mock_env(), mock_info("owner", &[]), msg)
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
            randomness_source: "terrand".to_string(),
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        let status = STATUS.load(&deps.storage).unwrap();
        assert!(!status);
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
            randomness_source: "terrand".to_string(),
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::SetSaleDetails {
            price: coin(5, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(1_u64)),
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
            randomness_source: "terrand".to_string(),
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SetSaleDetails {
            price: coin(0, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(1_u64)),
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
            randomness_source: "terrand".to_string(),
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SetSaleDetails {
            price: coin(10, "LUNA"),
            max_amount_per_wallet: Some(Uint128::from(1_u64)),
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
            randomness_source: "terrand".to_string(),
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SetSaleDetails {
            price: coin(10, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(0_u64)),
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
            randomness_source: "terrand".to_string(),
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SetSaleDetails {
            price: coin(10, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(1_u64)),
            recipient: Recipient::Addr("me".to_string()),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            res,
            Response::new().add_attributes(vec![
                attr("action", "switch status"),
                attr("price", coin(10, "uusd").to_string()),
                attr("max_amount_per_wallet", Uint128::from(1_u64)),
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
            randomness_source: "terrand".to_string(),
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SetSaleDetails {
            price: coin(10, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(1_u64)),
            recipient: Recipient::Addr("me".to_string()),
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let status = STATUS.load(&deps.storage).unwrap();
        assert!(!status);
        let info = mock_info("owner", &[]);
        execute_switch_status(deps.as_mut(), info).unwrap();
        let status = STATUS.load(&deps.storage).unwrap();
        assert!(status);
        let info = mock_info("owner", &[]);
        execute_switch_status(deps.as_mut(), info).unwrap();
        let status = STATUS.load(&deps.storage).unwrap();
        assert!(!status);
        let info = mock_info("anyone", &[]);
        let err = execute_switch_status(deps.as_mut(), info).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn test_mint_successful() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: AndrAddress {
                identifier: "cw721_contract".to_string(),
            },
            randomness_source: "terrand".to_string(),
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
                attributes: vec![],
                image: String::from(""),
                image_data: None,
                external_url: None,
                animation_url: None,
                youtube_url: None,
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

        assert!(list.contains(&"token_id".to_string()));
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
            randomness_source: "terrand".to_string(),
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::Mint(vec![
            (GumballMintMsg {
                token_id: "token_id".to_string(),
                owner: None,
                token_uri: None,
                extension: TokenExtension {
                    name: "name".to_string(),
                    publisher: "publisher".to_string(),
                    description: None,
                    attributes: vec![],
                    image: String::from(""),
                    image_data: None,
                    external_url: None,
                    animation_url: None,
                    youtube_url: None,
                },
            }),
        ]);
        let info = mock_info("not_owner", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        assert_eq!(ContractError::Unauthorized {}, res);
    }

    #[test]
    fn test_mint_wrong_status() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: AndrAddress {
                identifier: "cw721_contract".to_string(),
            },
            randomness_source: "terrand".to_string(),
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SetSaleDetails {
            price: coin(10, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(1_u64)),
            recipient: Recipient::Addr("me".to_string()),
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let info = mock_info("owner", &[]);
        execute_switch_status(deps.as_mut(), info).unwrap();
        let msg = ExecuteMsg::Mint(vec![
            (GumballMintMsg {
                token_id: "token_id".to_string(),
                owner: None,
                token_uri: None,
                extension: TokenExtension {
                    name: "name".to_string(),
                    publisher: "publisher".to_string(),
                    description: None,
                    attributes: vec![],
                    image: String::from(""),
                    image_data: None,
                    external_url: None,
                    animation_url: None,
                    youtube_url: None,
                },
            }),
        ]);
        let info = mock_info("owner", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        assert_eq!(ContractError::NotInRefillMode {}, res);
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
            randomness_source: "terrand".to_string(),
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SetSaleDetails {
            price: coin(10, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(1_u64)),
            recipient: Recipient::Addr("me".to_string()),
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let info = mock_info("owner", &[]);

        let msg = ExecuteMsg::Mint(vec![
            (GumballMintMsg {
                token_id: "token_id".to_string(),
                owner: None,
                token_uri: None,
                extension: TokenExtension {
                    name: "name".to_string(),
                    publisher: "publisher".to_string(),
                    description: None,
                    attributes: vec![],
                    image: String::from(""),
                    image_data: None,
                    external_url: None,
                    animation_url: None,
                    youtube_url: None,
                },
            }),
        ]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("anyone", &[coin(10, "uusd")]);
        let msg = ExecuteMsg::Buy {};
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Refilling {});
    }

    #[test]
    fn test_buy_insufficient_funds() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: AndrAddress {
                identifier: "cw721_contract".to_string(),
            },
            randomness_source: "terrand".to_string(),
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SetSaleDetails {
            price: coin(10, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(1_u64)),
            recipient: Recipient::Addr("me".to_string()),
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let info = mock_info("owner", &[]);

        let msg = ExecuteMsg::Mint(vec![
            (GumballMintMsg {
                token_id: "token_id".to_string(),
                owner: None,
                token_uri: None,
                extension: TokenExtension {
                    name: "name".to_string(),
                    publisher: "publisher".to_string(),
                    description: None,
                    attributes: vec![],
                    image: String::from(""),
                    image_data: None,
                    external_url: None,
                    animation_url: None,
                    youtube_url: None,
                },
            }),
        ]);

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        // Sets status to true, allowing purchasing
        let info = mock_info("owner", &[]);
        execute_switch_status(deps.as_mut(), info).unwrap();

        let info = mock_info("anyone", &[coin(9, "uusd")]);
        let msg = ExecuteMsg::Buy {};
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::InsufficientFunds {});
    }

    #[test]
    fn test_buy_wrong_denom() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: AndrAddress {
                identifier: "cw721_contract".to_string(),
            },
            randomness_source: "terrand".to_string(),
        };

        instantiate(deps.as_mut(), env, info, msg).unwrap();

        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SetSaleDetails {
            price: coin(10, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(1_u64)),
            recipient: Recipient::Addr("me".to_string()),
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let info = mock_info("owner", &[]);

        let msg = ExecuteMsg::Mint(vec![
            (GumballMintMsg {
                token_id: "token_id".to_string(),
                owner: None,
                token_uri: None,
                extension: TokenExtension {
                    name: "name".to_string(),
                    publisher: "publisher".to_string(),
                    description: None,
                    attributes: vec![],
                    image: String::from(""),
                    image_data: None,
                    external_url: None,
                    animation_url: None,
                    youtube_url: None,
                },
            }),
        ]);

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        // Sets status to true, allowing purchasing
        let info = mock_info("owner", &[]);
        execute_switch_status(deps.as_mut(), info).unwrap();

        let info = mock_info("anyone", &[coin(10, "euro")]);
        let msg = ExecuteMsg::Buy {};
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::InvalidFunds {
                msg: "Only uusd is accepted".to_string(),
            }
        );
    }

    #[test]
    fn test_buy_no_nfts() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            andromeda_cw721_contract: AndrAddress {
                identifier: "cw721_contract".to_string(),
            },
            randomness_source: "terrand".to_string(),
        };
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SetSaleDetails {
            price: coin(10, "uusd"),
            max_amount_per_wallet: Some(Uint128::from(1_u64)),
            recipient: Recipient::Addr("me".to_string()),
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Sets status to true, allowing purchasing
        let info = mock_info("owner", &[]);
        execute_switch_status(deps.as_mut(), info).unwrap();

        let info = mock_info("anyone", &[coin(10, "euro")]);
        let msg = ExecuteMsg::Buy {};
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::OutOfNFTs {});
    }
}
