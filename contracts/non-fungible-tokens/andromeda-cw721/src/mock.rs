#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_non_fungible_tokens::cw721::{
    ExecuteMsg, InstantiateMsg, MintMsg, QueryMsg, TokenExtension, TransferAgreement,
};
use andromeda_std::amp::addresses::AndrAddr;
use andromeda_testing::{
    mock::MockApp,
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::{to_json_binary, Addr, Binary, Coin, Empty};
use cw721::OwnerOfResponse;
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockCW721(Addr);
mock_ado!(MockCW721, ExecuteMsg, QueryMsg);

impl MockCW721 {
    #[allow(clippy::too_many_arguments)]
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        name: impl Into<String>,
        symbol: impl Into<String>,
        minter: impl Into<String>,
        kernel_address: impl Into<String>,
        owner: Option<String>,
    ) -> MockCW721 {
        let msg = mock_cw721_instantiate_msg(
            name.into(),
            symbol.into(),
            minter.into(),
            kernel_address.into(),
            owner,
        );
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "CW721 Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockCW721(addr)
    }

    pub fn execute_quick_mint(
        &self,
        app: &mut MockApp,
        sender: Addr,
        amount: u32,
        owner: impl Into<String>,
    ) -> ExecuteResult {
        let msg = mock_quick_mint_msg(amount, owner.into());
        self.execute(app, &msg, sender, &[])
    }

    pub fn execute_send_nft(
        &self,
        app: &mut MockApp,
        sender: Addr,
        contract: impl Into<String>,
        token_id: impl Into<String>,
        msg: &impl Serialize,
    ) -> ExecuteResult {
        let msg = mock_send_nft(
            AndrAddr::from_string(contract.into()),
            token_id.into(),
            to_json_binary(msg).unwrap(),
        );
        self.execute(app, &msg, sender, &[])
    }

    pub fn query_minter(&self, app: &MockApp) -> Addr {
        self.query::<Addr>(app, mock_cw721_minter_query())
    }

    pub fn query_owner_of(&self, app: &MockApp, token_id: impl Into<String>) -> Addr {
        Addr::unchecked(
            self.query::<OwnerOfResponse>(app, mock_cw721_owner_of(token_id.into(), None))
                .owner,
        )
    }
}

pub fn mock_cw721_minter_query() -> QueryMsg {
    QueryMsg::Minter {}
}

pub fn mock_andromeda_cw721() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_cw721_instantiate_msg(
    name: String,
    symbol: String,
    minter: impl Into<String>,
    kernel_address: String,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        name,
        symbol,
        minter: AndrAddr::from_string(minter.into()),
        kernel_address,
        owner,
    }
}

pub fn mock_cw721_owner_of(token_id: String, include_expired: Option<bool>) -> QueryMsg {
    QueryMsg::OwnerOf {
        token_id,
        include_expired,
    }
}

pub fn mock_mint_msg(
    token_id: String,
    extension: TokenExtension,
    token_uri: Option<String>,
    owner: String,
) -> MintMsg {
    MintMsg {
        token_id,
        owner,
        token_uri,
        extension,
    }
}

pub fn mock_quick_mint_msg(amount: u32, owner: String) -> ExecuteMsg {
    let mut mint_msgs: Vec<MintMsg> = Vec::new();
    for i in 0..amount {
        let extension = TokenExtension {
            publisher: owner.clone(),
        };

        let msg = mock_mint_msg(i.to_string(), extension, None, owner.clone());
        mint_msgs.push(msg);
    }

    ExecuteMsg::BatchMint { tokens: mint_msgs }
}

pub fn mock_send_nft(contract: AndrAddr, token_id: String, msg: Binary) -> ExecuteMsg {
    ExecuteMsg::SendNft {
        contract,
        token_id,
        msg,
    }
}

pub fn mock_transfer_nft(recipient: AndrAddr, token_id: String) -> ExecuteMsg {
    ExecuteMsg::TransferNft {
        recipient,
        token_id,
    }
}

pub fn mock_transfer_agreement(amount: Coin, purchaser: String) -> TransferAgreement {
    TransferAgreement { amount, purchaser }
}

pub fn mock_create_transfer_agreement_msg(
    token_id: String,
    agreement: Option<TransferAgreement>,
) -> ExecuteMsg {
    ExecuteMsg::TransferAgreement {
        token_id,
        agreement,
    }
}
