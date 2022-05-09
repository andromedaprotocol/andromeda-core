use crate::state::{ANDROMEDA_CW721_ADDR, CAN_UNWRAP};
use ado_base::state::ADOContract;
use andromeda_non_fungible_tokens::{
    cw721::{
        ExecuteMsg as Cw721ExecuteMsg, InstantiateMsg as Cw721InstantiateMsg, MetadataAttribute,
        MetadataType, TokenExtension, TokenMetadata,
    },
    wrapped_cw721::{Cw721HookMsg, ExecuteMsg, InstantiateMsg, InstantiateType, QueryMsg},
};
use common::{
    ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError,
    mission::AndrAddress, require, response::get_reply_address,
};
use cosmwasm_std::{
    entry_point, from_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper,
    QueryRequest, Reply, Response, StdError, SubMsg, WasmMsg, WasmQuery,
};
use cw721::{Cw721QueryMsg, Cw721ReceiveMsg, NftInfoResponse};
use cw721_base::MintMsg;

const ORIGINAL_TOKEN_ID: &str = "original_token_id";
const ORIGINAL_TOKEN_ADDRESS: &str = "original_token_address";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    CAN_UNWRAP.save(deps.storage, &msg.can_unwrap)?;
    let mut msgs: Vec<SubMsg> = vec![];
    let resp = contract.instantiate(
        deps.storage,
        deps.api,
        info.clone(),
        BaseInstantiateMsg {
            ado_type: "wrapped_cw721".to_string(),
            operators: None,
            modules: None,
            primitive_contract: Some(msg.primitive_contract),
        },
    )?;
    match msg.cw721_instantiate_type {
        InstantiateType::Address(addr) => ANDROMEDA_CW721_ADDR.save(deps.storage, &addr)?,
        InstantiateType::New(specification) => {
            let instantiate_msg = Cw721InstantiateMsg {
                name: specification.name,
                symbol: specification.symbol,
                modules: specification.modules,
                minter: AndrAddress {
                    identifier: env.contract.address.to_string(),
                },
            };
            let msg = contract.generate_instantiate_msg(
                deps.storage,
                &deps.querier,
                1,
                encode_binary(&instantiate_msg)?,
                "cw721".to_string(),
                info.sender.to_string(),
            )?;
            msgs.push(msg);
        }
    }
    Ok(resp.add_submessages(msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }
    require(msg.id == 1, ContractError::InvalidReplyId {})?;

    let addr = get_reply_address(&msg)?;
    ANDROMEDA_CW721_ADDR.save(deps.storage, &addr)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ReceiveNft(msg) => handle_receive_cw721(deps, env, info, msg),
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
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
        Cw721HookMsg::Wrap { wrapped_token_id } => execute_wrap(
            deps,
            env,
            msg.sender,
            msg.token_id,
            info.sender,
            wrapped_token_id,
        ),
        Cw721HookMsg::Unwrap {} => execute_unwrap(deps, msg.sender, msg.token_id, info.sender),
    }
}

fn execute_wrap(
    deps: DepsMut,
    env: Env,
    sender: String,
    token_id: String,
    token_address: Addr,
    wrapped_token_id: Option<String>,
) -> Result<Response, ContractError> {
    require(
        ADOContract::default().is_owner_or_operator(deps.storage, &sender)?,
        ContractError::Unauthorized {},
    )?;
    require(
        token_address != env.contract.address,
        ContractError::CannotDoubleWrapToken {},
    )?;

    let wrapped_token_id = wrapped_token_id.unwrap_or_else(|| token_id.to_string());
    let extension = TokenExtension {
        name: wrapped_token_id.clone(),
        publisher: sender.clone(),
        description: None,
        transfer_agreement: None,
        metadata: Some(TokenMetadata {
            data_type: MetadataType::Other,
            external_url: None,
            data_url: None,
            attributes: Some(vec![
                MetadataAttribute {
                    key: ORIGINAL_TOKEN_ID.to_owned(),
                    value: token_id.clone(),
                    display_label: None,
                },
                MetadataAttribute {
                    key: ORIGINAL_TOKEN_ADDRESS.to_owned(),
                    value: token_address.to_string(),
                    display_label: None,
                },
            ]),
        }),
        archived: false,
    };
    let mint_msg = MintMsg {
        token_id: wrapped_token_id.to_string(),
        owner: sender,
        token_uri: None,
        extension,
    };
    let msg = encode_binary(&Cw721ExecuteMsg::Mint(Box::new(mint_msg)))?;
    let cw721_contract_addr = ANDROMEDA_CW721_ADDR.load(deps.storage)?;
    let wasm_msg = WasmMsg::Execute {
        contract_addr: cw721_contract_addr.clone(),
        funds: vec![],
        msg,
    };
    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "wrap")
        .add_attribute("token_id", token_id)
        .add_attribute("token_address", token_address)
        .add_attribute("wrapped_token_id", wrapped_token_id)
        .add_attribute("wrapped_token_address", cw721_contract_addr))
}

fn execute_unwrap(
    deps: DepsMut,
    sender: String,
    token_id: String,
    token_address: Addr,
) -> Result<Response, ContractError> {
    let can_unwrap = CAN_UNWRAP.load(deps.storage)?;
    let cw721_contract_addr = ANDROMEDA_CW721_ADDR.load(deps.storage)?;
    require(can_unwrap, ContractError::UnwrappingDisabled {})?;
    require(
        token_address == cw721_contract_addr,
        ContractError::TokenNotWrappedByThisContract {},
    )?;
    let (original_token_id, original_token_address) =
        get_original_nft_data(&deps.querier, token_id.clone(), token_address.to_string())?;

    let burn_msg = Cw721ExecuteMsg::Burn { token_id };
    let transfer_msg = Cw721ExecuteMsg::TransferNft {
        recipient: sender,
        token_id: original_token_id,
    };
    Ok(Response::new()
        .add_message(WasmMsg::Execute {
            contract_addr: cw721_contract_addr,
            funds: vec![],
            msg: encode_binary(&burn_msg)?,
        })
        .add_message(WasmMsg::Execute {
            contract_addr: original_token_address,
            funds: vec![],
            msg: encode_binary(&transfer_msg)?,
        })
        .add_attribute("action", "unwrap"))
}

/// Retrieves the original token id and contract address of the wrapped token.
fn get_original_nft_data(
    querier: &QuerierWrapper,
    wrapped_token_id: String,
    wrapped_token_address: String,
) -> Result<(String, String), ContractError> {
    let token_info = querier.query::<NftInfoResponse<TokenExtension>>(&QueryRequest::Wasm(
        WasmQuery::Smart {
            contract_addr: wrapped_token_address,
            msg: encode_binary(&Cw721QueryMsg::NftInfo {
                token_id: wrapped_token_id,
            })?,
        },
    ))?;
    if let Some(metadata) = token_info.extension.metadata {
        if let Some(attributes) = metadata.attributes {
            require(attributes.len() == 2, ContractError::InvalidMetadata {})?;
            let original_token_id = attributes[0].value.clone();
            let original_token_address = attributes[1].value.clone();
            return Ok((original_token_id, original_token_address));
        }
        return Err(ContractError::InvalidMetadata {});
    }
    Err(ContractError::InvalidMetadata {})
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_non_fungible_tokens::wrapped_cw721::Cw721Specification;
    use andromeda_testing::testing::mock_querier::{
        mock_dependencies_custom, MOCK_CW721_CONTRACT, MOCK_PRIMITIVE_CONTRACT,
    };
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        CosmosMsg, ReplyOn,
    };

    fn init(deps: DepsMut) {
        instantiate(
            deps,
            mock_env(),
            mock_info("owner", &[]),
            InstantiateMsg {
                primitive_contract: MOCK_PRIMITIVE_CONTRACT.to_owned(),
                cw721_instantiate_type: InstantiateType::Address(MOCK_CW721_CONTRACT.to_owned()),
                can_unwrap: true,
            },
        )
        .unwrap();
    }

    #[test]
    fn test_instantiate_address() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("sender", &[]);

        let msg = InstantiateMsg {
            primitive_contract: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            can_unwrap: true,
            cw721_instantiate_type: InstantiateType::Address(MOCK_CW721_CONTRACT.to_owned()),
        };

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            Response::new()
                .add_attribute("method", "instantiate")
                .add_attribute("type", "wrapped_cw721"),
            res
        );
        assert!(ADOContract::default()
            .is_contract_owner(deps.as_ref().storage, "sender")
            .unwrap());
        assert_eq!(
            MOCK_CW721_CONTRACT.to_owned(),
            ANDROMEDA_CW721_ADDR.load(deps.as_ref().storage).unwrap()
        );
        assert!(CAN_UNWRAP.load(deps.as_ref().storage).unwrap());
    }

    #[test]
    fn test_instantiate_new() {
        let mut deps = mock_dependencies_custom(&[]);
        let info = mock_info("sender", &[]);

        let msg = InstantiateMsg {
            primitive_contract: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            can_unwrap: true,
            cw721_instantiate_type: InstantiateType::New(Cw721Specification {
                name: "name".to_string(),
                symbol: "symbol".to_string(),
                modules: None,
            }),
        };

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        let cw721_insantiate_msg = Cw721InstantiateMsg {
            name: "name".to_string(),
            symbol: "symbol".to_string(),
            modules: None,
            minter: AndrAddress {
                identifier: mock_env().contract.address.to_string(),
            },
        };
        let msg: SubMsg = SubMsg {
            id: 1,
            reply_on: ReplyOn::Always,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some("sender".to_string()),
                code_id: 4,
                msg: encode_binary(&cw721_insantiate_msg).unwrap(),
                funds: vec![],
                label: "Instantiate: cw721".to_string(),
            }),
            gas_limit: None,
        };
        assert_eq!(
            Response::new()
                .add_submessage(msg)
                .add_attribute("method", "instantiate")
                .add_attribute("type", "wrapped_cw721"),
            res
        );
        assert!(ADOContract::default()
            .is_contract_owner(deps.as_ref().storage, "sender")
            .unwrap());
        assert!(CAN_UNWRAP.load(deps.as_ref().storage).unwrap());
    }

    #[test]
    fn test_wrap() {
        let mut deps = mock_dependencies(&[]);

        let token_id = String::from("token_id");
        let owner = String::from("owner");
        let token_address = String::from("token_address");

        init(deps.as_mut());

        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: "not_owner".to_string(),
            token_id: token_id.clone(),
            msg: encode_binary(&Cw721HookMsg::Wrap {
                wrapped_token_id: None,
            })
            .unwrap(),
        });

        let info = mock_info(&token_address, &[]);
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: owner.clone(),
            token_id: token_id.clone(),
            msg: encode_binary(&Cw721HookMsg::Wrap {
                wrapped_token_id: None,
            })
            .unwrap(),
        });

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let extension = TokenExtension {
            name: token_id.clone(),
            publisher: owner.clone(),
            description: None,
            transfer_agreement: None,
            metadata: Some(TokenMetadata {
                data_type: MetadataType::Other,
                external_url: None,
                data_url: None,
                attributes: Some(vec![
                    MetadataAttribute {
                        key: ORIGINAL_TOKEN_ID.to_owned(),
                        value: token_id.clone(),
                        display_label: None,
                    },
                    MetadataAttribute {
                        key: ORIGINAL_TOKEN_ADDRESS.to_owned(),
                        value: token_address.to_string(),
                        display_label: None,
                    },
                ]),
            }),
            archived: false,
        };
        let mint_msg = MintMsg {
            token_id: token_id.clone(),
            owner,
            token_uri: None,
            extension,
        };
        let msg = encode_binary(&Cw721ExecuteMsg::Mint(Box::new(mint_msg))).unwrap();
        let wasm_msg = WasmMsg::Execute {
            contract_addr: MOCK_CW721_CONTRACT.to_owned(),
            funds: vec![],
            msg,
        };

        assert_eq!(
            Response::new()
                .add_message(wasm_msg)
                .add_attribute("action", "wrap")
                .add_attribute("token_id", &token_id)
                .add_attribute("token_address", token_address)
                .add_attribute("wrapped_token_id", &token_id)
                .add_attribute("wrapped_token_address", MOCK_CW721_CONTRACT),
            res
        );
    }

    #[test]
    fn test_wrap_new_wrapped_token_id() {
        let mut deps = mock_dependencies(&[]);

        let token_id = String::from("token_id");
        let wrapped_token_id = String::from("wrapped_token_id");
        let owner = String::from("owner");
        let token_address = String::from("token_address");

        init(deps.as_mut());

        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: owner.clone(),
            token_id: token_id.clone(),
            msg: encode_binary(&Cw721HookMsg::Wrap {
                wrapped_token_id: Some(wrapped_token_id.clone()),
            })
            .unwrap(),
        });

        let info = mock_info(&token_address, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let extension = TokenExtension {
            name: wrapped_token_id.clone(),
            publisher: owner.clone(),
            description: None,
            transfer_agreement: None,
            metadata: Some(TokenMetadata {
                data_type: MetadataType::Other,
                external_url: None,
                data_url: None,
                attributes: Some(vec![
                    MetadataAttribute {
                        key: ORIGINAL_TOKEN_ID.to_owned(),
                        value: token_id.clone(),
                        display_label: None,
                    },
                    MetadataAttribute {
                        key: ORIGINAL_TOKEN_ADDRESS.to_owned(),
                        value: token_address.to_string(),
                        display_label: None,
                    },
                ]),
            }),
            archived: false,
        };
        let mint_msg = MintMsg {
            token_id: wrapped_token_id.to_owned(),
            owner,
            token_uri: None,
            extension,
        };
        let msg = encode_binary(&Cw721ExecuteMsg::Mint(Box::new(mint_msg))).unwrap();
        let wasm_msg = WasmMsg::Execute {
            contract_addr: MOCK_CW721_CONTRACT.to_owned(),
            funds: vec![],
            msg,
        };

        assert_eq!(
            Response::new()
                .add_message(wasm_msg)
                .add_attribute("action", "wrap")
                .add_attribute("token_id", &token_id)
                .add_attribute("token_address", token_address)
                .add_attribute("wrapped_token_id", &wrapped_token_id)
                .add_attribute("wrapped_token_address", MOCK_CW721_CONTRACT),
            res
        );
    }

    #[test]
    fn test_wrap_operator() {
        let mut deps = mock_dependencies(&[]);

        let token_id = String::from("token_id");
        let operator = String::from("operator");
        let token_address = String::from("token_address");

        init(deps.as_mut());
        ADOContract::default()
            .execute_update_operators(
                deps.as_mut(),
                mock_info("owner", &[]),
                vec![operator.to_owned()],
            )
            .unwrap();

        let info = mock_info(&token_address, &[]);

        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: operator,
            token_id,
            msg: encode_binary(&Cw721HookMsg::Wrap {
                wrapped_token_id: None,
            })
            .unwrap(),
        });

        // Make sure call does not error by unwrapping.
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    #[test]
    fn test_unwrap_unwrap_disabled() {
        let mut deps = mock_dependencies(&[]);

        let token_id = String::from("token_id");
        let owner = String::from("owner");

        init(deps.as_mut());
        CAN_UNWRAP.save(deps.as_mut().storage, &false).unwrap();

        let info = mock_info(MOCK_CW721_CONTRACT, &[]);

        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: owner,
            token_id,
            msg: encode_binary(&Cw721HookMsg::Unwrap {}).unwrap(),
        });

        let res = execute(deps.as_mut(), mock_env(), info, msg);

        assert_eq!(ContractError::UnwrappingDisabled {}, res.unwrap_err());
    }

    #[test]
    fn test_unwrap_invalid_token_address() {
        let mut deps = mock_dependencies(&[]);

        let token_id = String::from("token_id");
        let owner = String::from("owner");

        init(deps.as_mut());
        CAN_UNWRAP.save(deps.as_mut().storage, &true).unwrap();

        let info = mock_info("invalid_address", &[]);

        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: owner,
            token_id,
            msg: encode_binary(&Cw721HookMsg::Unwrap {}).unwrap(),
        });

        let res = execute(deps.as_mut(), mock_env(), info, msg);

        assert_eq!(
            ContractError::TokenNotWrappedByThisContract {},
            res.unwrap_err()
        );
    }

    #[test]
    fn test_unwrap() {
        let mut deps = mock_dependencies_custom(&[]);

        let token_id = String::from("original_token_id");
        let owner = String::from("owner");

        init(deps.as_mut());
        CAN_UNWRAP.save(deps.as_mut().storage, &true).unwrap();

        let info = mock_info(MOCK_CW721_CONTRACT, &[]);

        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: owner.clone(),
            token_id: token_id.clone(),
            msg: encode_binary(&Cw721HookMsg::Unwrap {}).unwrap(),
        });

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let burn_msg = Cw721ExecuteMsg::Burn { token_id };
        let transfer_msg = Cw721ExecuteMsg::TransferNft {
            recipient: owner,
            token_id: "original_token_id".to_owned(),
        };
        assert_eq!(
            Response::new()
                .add_message(WasmMsg::Execute {
                    contract_addr: MOCK_CW721_CONTRACT.to_owned(),
                    funds: vec![],
                    msg: encode_binary(&burn_msg).unwrap(),
                })
                .add_message(WasmMsg::Execute {
                    contract_addr: "original_token_address".to_owned(),
                    funds: vec![],
                    msg: encode_binary(&transfer_msg).unwrap(),
                })
                .add_attribute("action", "unwrap"),
            res
        );
    }
}
