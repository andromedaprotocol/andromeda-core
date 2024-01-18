#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_non_fungible_tokens::cw721::{
    ExecuteMsg, InstantiateMsg, MintMsg, QueryMsg, TokenExtension, TransferAgreement,
};
use andromeda_std::{ado_base::modules::Module, amp::addresses::AndrAddr};
use andromeda_testing::mock_contract::{MockADO, MockContract};
use cosmwasm_std::{Addr, Binary, Coin, Empty};
use cw721::OwnerOfResponse;
use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};

pub struct MockCW721(Addr);

impl MockContract for MockCW721 {
    fn addr(&self) -> &Addr {
        &self.0
    }
}

impl From<Addr> for MockCW721 {
    fn from(addr: Addr) -> Self {
        Self(addr)
    }
}

impl MockADO for MockCW721 {}

impl MockCW721 {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut App,
        name: impl Into<String>,
        symbol: impl Into<String>,
        minter: impl Into<String>,
        modules: Option<Vec<Module>>,
        kernel_address: impl Into<String>,
        owner: Option<String>,
    ) -> MockCW721 {
        let msg = mock_cw721_instantiate_msg(
            name.into(),
            symbol.into(),
            minter.into(),
            modules,
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
        MockCW721(Addr::unchecked(addr))
    }

    pub fn execute_quick_mint(
        &self,
        app: &mut App,
        sender: Addr,
        amount: u32,
        owner: impl Into<String>,
    ) -> AppResponse {
        let msg = mock_quick_mint_msg(amount, owner.into());
        self.execute(app, msg, sender, &[])
    }

    pub fn query_minter(&self, app: &mut App) -> Addr {
        self.query::<QueryMsg, Addr>(app, mock_cw721_minter_query())
    }

    pub fn query_owner_of(&self, app: &mut App, token_id: impl Into<String>) -> Addr {
        Addr::unchecked(
            self.query::<QueryMsg, OwnerOfResponse>(
                app,
                mock_cw721_owner_of(token_id.into(), None),
            )
            .owner,
        )
    }
}

pub fn mock_andromeda_cw721() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_cw721_instantiate_msg(
    name: String,
    symbol: String,
    minter: impl Into<String>,
    modules: Option<Vec<Module>>,
    kernel_address: String,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        name,
        symbol,
        minter: AndrAddr::from_string(minter.into()),
        modules,
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

pub fn mock_cw721_minter_query() -> QueryMsg {
    QueryMsg::Minter {}
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

pub fn mock_send_nft(contract: String, token_id: String, msg: Binary) -> ExecuteMsg {
    ExecuteMsg::SendNft {
        contract,
        token_id,
        msg,
    }
}

pub fn mock_transfer_nft(recipient: String, token_id: String) -> ExecuteMsg {
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
