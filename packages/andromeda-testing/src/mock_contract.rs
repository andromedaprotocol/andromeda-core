use core::fmt;

use andromeda_std::ado_base::{ownership::ContractOwnerResponse, AndromedaQuery};
use cosmwasm_std::{Addr, Coin};
use cw_multi_test::{App, AppResponse, Executor};
use serde::{de::DeserializeOwned, Serialize};

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
        app: &mut App,
        msg: M,
        sender: Addr,
        funds: &[Coin],
    ) -> AppResponse {
        app.execute_contract(sender, self.addr().clone(), &msg, funds)
            .unwrap()
    }

    pub fn query<M: Serialize + fmt::Debug, T: DeserializeOwned>(&self, app: &App, msg: M) -> T {
        app.wrap()
            .query_wasm_smart::<T>(self.addr().clone(), &msg)
            .unwrap()
    }

    pub fn query_owner(&self, app: &App) -> String {
        self.query::<AndromedaQuery, ContractOwnerResponse>(app, AndromedaQuery::Owner {})
            .owner
    }
}

impl From<String> for MockContract {
    fn from(addr: String) -> Self {
        Self(Addr::unchecked(addr))
    }
}
