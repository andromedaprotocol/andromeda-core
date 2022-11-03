#![cfg(not(target_arch = "wasm32"))]

use crate::contract::{execute, instantiate, query, reply};
use andromeda_non_fungible_tokens::{
    crowdfund::{CrowdfundMintMsg, ExecuteMsg, InstantiateMsg},
    cw721::TokenExtension,
};
use common::{
    ado_base::{modules::Module, recipient::Recipient},
    app::AndrAddress,
};
use cosmwasm_std::{Coin, Empty, Uint128};
use cw_multi_test::{Contract, ContractWrapper};
use cw_utils::Expiration;

pub fn mock_andromeda_crowdfund() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query).with_reply(reply);
    Box::new(contract)
}

pub fn mock_crowdfund_instantiate_msg(
    token_address: String,
    can_mint_after_sale: bool,
    modules: Option<Vec<Module>>,
) -> InstantiateMsg {
    InstantiateMsg {
        token_address: AndrAddress {
            identifier: token_address,
        },
        can_mint_after_sale,
        modules,
    }
}

pub fn mock_start_crowdfund_msg(
    expiration: Expiration,
    price: Coin,
    min_tokens_sold: Uint128,
    max_amount_per_wallet: Option<u32>,
    recipient: Recipient,
) -> ExecuteMsg {
    ExecuteMsg::StartSale {
        expiration,
        price,
        min_tokens_sold,
        max_amount_per_wallet,
        recipient,
    }
}

pub fn mock_end_crowdfund_msg(limit: Option<u32>) -> ExecuteMsg {
    ExecuteMsg::EndSale { limit }
}

pub fn mock_crowdfund_mint_msg(
    token_id: String,
    extension: TokenExtension,
    token_uri: Option<String>,
    owner: Option<String>,
) -> CrowdfundMintMsg {
    CrowdfundMintMsg {
        token_id,
        owner,
        token_uri,
        extension,
    }
}

pub fn mock_crowdfund_quick_mint_msg(amount: u32, publisher: String) -> ExecuteMsg {
    let mut mint_msgs: Vec<CrowdfundMintMsg> = Vec::new();
    for i in 0..amount {
        let extension = TokenExtension {
            name: i.to_string(),
            publisher: publisher.clone(),
            description: None,
            attributes: vec![],
            image: i.to_string(),
            image_data: None,
            external_url: None,
            animation_url: None,
            youtube_url: None,
        };

        let msg = mock_crowdfund_mint_msg(i.to_string(), extension, None, None);
        mint_msgs.push(msg);
    }

    ExecuteMsg::Mint(mint_msgs)
}

pub fn mock_purchase_msg(number_of_tokens: Option<u32>) -> ExecuteMsg {
    ExecuteMsg::Purchase { number_of_tokens }
}
