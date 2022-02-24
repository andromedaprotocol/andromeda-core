use crate::contract::{execute_unwrap_nft, execute_wrap_nft, instantiate, REPLY_CREATE_TOKEN};
use crate::state::CONFIG;
use andromeda_protocol::{
    factory::ExecuteMsg as FactoryExecuteMsg,
    token::{ExecuteMsg as TokenExecuteMsg, MintMsg as TokenMintMsg},
    wrapped_nft::InstantiateMsg,
};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{attr, to_binary, Addr, Api, CosmosMsg, ReplyOn, Response, SubMsg, WasmMsg};

#[test]
fn test_instantiate() {
    let owner = "creator";
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let info = mock_info(owner, &[]);
    let msg = InstantiateMsg {
        andromeda_factory_addr: Addr::unchecked("FACTORY_ADDR"),
        name: "WRAPP_NFT".to_string(),
        symbol: "WNFT".to_string(),
        modules: vec![],
    };
    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res,
        Response::new()
            .add_submessage(SubMsg {
                msg: CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "FACTORY_ADDR".to_string(),
                    msg: to_binary(&FactoryExecuteMsg::Create {
                        name: "WRAPP_NFT".to_string(),
                        symbol: "WNFT".to_string(),
                        modules: vec![],
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                gas_limit: None,
                id: REPLY_CREATE_TOKEN,
                reply_on: ReplyOn::Always
            })
            .add_attributes(vec![attr("action", "instantiate")])
    );
}

#[test]
fn test_execute_wrap_nft() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let info = mock_info("OWNER", &[]);

    let msg = InstantiateMsg {
        andromeda_factory_addr: Addr::unchecked("FACTORY_ADDR"),
        name: "WRAPP_NFT".to_string(),
        symbol: "WNFT".to_string(),
        modules: vec![],
    };
    let _ = instantiate(deps.as_mut(), env, info, msg).unwrap();
    let mut config = CONFIG.load(&deps.storage).unwrap();
    config.token_addr = deps.api.addr_canonicalize("WRAP_TOKEN_ADDR").unwrap();

    CONFIG.save(deps.as_mut().storage, &config).unwrap();

    let sender = "NFT_OWNER1".to_string();
    let nft_addr = "NFT_ADDR1".to_string();
    let token_id = "NFT_TOKEN_ID_1".to_string();

    let res = execute_wrap_nft(deps.as_mut(), sender, nft_addr, token_id).unwrap();
    assert_eq!(
        res,
        Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "WRAP_TOKEN_ADDR".to_string(),
                msg: to_binary(&TokenExecuteMsg::Mint(TokenMintMsg {
                    token_id: "0".to_string(),
                    owner: "NFT_OWNER1".to_string(),
                    name: "WRAPP_NFT".to_string(),
                    token_uri: None,
                    description: None,
                    metadata: None,
                    pricing: None
                }))
                .unwrap(),
                funds: vec![]
            }))
            .add_attributes(vec![
                attr("action", "mint_wrap_nft"),
                attr("new_token_id", "0".to_string())
            ])
    );
}

#[test]
fn test_execute_unwrap_nft() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let info = mock_info("OWNER", &[]);

    let msg = InstantiateMsg {
        andromeda_factory_addr: Addr::unchecked("FACTORY_ADDR"),
        name: "WRAPP_NFT".to_string(),
        symbol: "WNFT".to_string(),
        modules: vec![],
    };
    let _ = instantiate(deps.as_mut(), env, info, msg).unwrap();
    let mut config = CONFIG.load(&deps.storage).unwrap();
    config.token_addr = deps.api.addr_canonicalize("WRAP_TOKEN_ADDR").unwrap();

    CONFIG.save(deps.as_mut().storage, &config).unwrap();

    let sender = "NFT_OWNER1".to_string();
    let nft_addr = "NFT_ADDR1".to_string();
    let token_id = "NFT_TOKEN_ID_1".to_string();

    let _res = execute_wrap_nft(
        deps.as_mut(),
        sender.clone(),
        nft_addr.clone(),
        token_id.clone(),
    )
    .unwrap();

    let res = execute_unwrap_nft(
        deps.as_mut(),
        sender,
        "WRAP_TOKEN_ADDR".to_string(),
        "0".to_string(),
    )
    .unwrap();
    assert_eq!(
        res,
        Response::new()
            .add_messages(vec![
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "WRAP_TOKEN_ADDR".to_string(),
                    msg: to_binary(&TokenExecuteMsg::Burn {
                        token_id: "0".to_string(),
                    })
                    .unwrap(),
                    funds: vec![],
                }),
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "NFT_ADDR1".to_string(),
                    msg: to_binary(&TokenExecuteMsg::TransferNft {
                        recipient: "NFT_OWNER1".to_string(),
                        token_id: "NFT_TOKEN_ID_1".to_string(),
                    })
                    .unwrap(),
                    funds: vec![],
                }),
            ])
            .add_attributes(vec![
                attr("action", "unwrap_nft"),
                attr("burn_token_id", "0"),
                attr("receiver", "NFT_OWNER1"),
                attr("token_id", "NFT_TOKEN_ID_1"),
            ])
    )
}
