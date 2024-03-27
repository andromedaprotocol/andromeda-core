use core::fmt;

use andromeda_std::ado_base::{
    ownership::{ContractOwnerResponse, OwnershipMessage},
    AndromedaMsg, AndromedaQuery,
};
use cosmwasm_std::{Addr, Coin};
use cw_multi_test::{AppResponse, Executor};
use serde::{de::DeserializeOwned, Serialize};

pub use anyhow::Result as AnyResult;

use crate::mock::MockApp;

pub struct MockContract(Addr);

impl MockContract {
    pub fn new(addr: Addr) -> Self {
        Self(addr)
    }

    pub fn addr(&self) -> &Addr {
        &self.0
    }

    pub fn execute<M: Serialize + fmt::Debug>(
        &self,
        app: &mut MockApp,
        msg: M,
        sender: Addr,
        funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        app.execute_contract(sender, self.addr().clone(), &msg, funds)
    }

    pub fn query<M: Serialize + fmt::Debug, T: DeserializeOwned>(
        &self,
        app: &MockApp,
        msg: M,
    ) -> T {
        app.wrap()
            .query_wasm_smart::<T>(self.addr().clone(), &msg)
            .unwrap()
    }

    pub fn query_owner(&self, app: &MockApp) -> String {
        self.query::<AndromedaQuery, ContractOwnerResponse>(app, AndromedaQuery::Owner {})
            .owner
    }

    pub fn accept_ownership(&self, app: &mut MockApp, sender: Addr) -> AnyResult<AppResponse> {
        self.execute(
            app,
            AndromedaMsg::Ownership(OwnershipMessage::AcceptOwnership {}),
            sender,
            &[],
        )
    }
}

impl From<String> for MockContract {
    fn from(addr: String) -> Self {
        Self(Addr::unchecked(addr))
    }
}
