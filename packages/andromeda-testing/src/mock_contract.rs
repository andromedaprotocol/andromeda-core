use core::fmt;

use andromeda_std::ado_base::{ownership::ContractOwnerResponse, AndromedaQuery};
use cosmwasm_std::{Addr, Coin};
use cw_multi_test::{App, AppResponse, Executor};
use serde::{de::DeserializeOwned, Serialize};

pub struct BaseMockContract(Addr);

impl BaseMockContract {
    pub fn addr(&self) -> &Addr {
        &self.0
    }
}

impl MockContract for BaseMockContract {
    fn addr(&self) -> &Addr {
        &self.0
    }
}

impl From<String> for BaseMockContract {
    fn from(addr: String) -> Self {
        Self(Addr::unchecked(addr))
    }
}

impl MockADO for BaseMockContract {}

pub trait MockContract {
    fn addr(&self) -> &Addr;

    fn execute<M: Serialize + fmt::Debug>(
        &self,
        app: &mut App,
        msg: M,
        sender: Addr,
        funds: &[Coin],
    ) -> AppResponse {
        app.execute_contract(sender, self.addr().clone(), &msg, funds)
            .unwrap()
    }

    fn query<M: Serialize + fmt::Debug, T: DeserializeOwned>(&self, app: &App, msg: M) -> T {
        app.wrap()
            .query_wasm_smart::<T>(self.addr().clone(), &msg)
            .unwrap()
    }
}

pub trait MockADO: MockContract {
    fn query_owner(&self, app: &App) -> String {
        self.query::<AndromedaQuery, ContractOwnerResponse>(app, AndromedaQuery::Owner {})
            .owner
    }
}
