use core::fmt;

use andromeda_std::ado_base::{ownership::ContractOwnerResponse, AndromedaQuery};
use cosmwasm_std::{Addr, Coin};
use cw_multi_test::{App, AppResponse, Executor};
use serde::{de::DeserializeOwned, Serialize};

pub use anyhow::Result as AnyResult;
pub struct MockContract(Addr);

pub type ExecuteResult = AnyResult<AppResponse>;

pub trait MockContract<E: Serialize + fmt::Debug, Q: Serialize + fmt::Debug> {
    fn addr(&self) -> &Addr;

    fn execute(
        &self,
        app: &mut App,
        msg: &E,
        sender: Addr,
        funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        app.execute_contract(sender, self.addr().clone(), &msg, funds)
    }

    fn query<T: DeserializeOwned>(&self, app: &App, msg: Q) -> T {
        app.wrap()
            .query_wasm_smart::<T>(self.addr().clone(), &msg)
            .unwrap()
    }
}

pub trait MockADO<E: Serialize + fmt::Debug, Q: Serialize + fmt::Debug>:
    MockContract<E, Q>
{
    fn query_owner(&self, app: &App) -> String {
        app.wrap()
            .query_wasm_smart::<ContractOwnerResponse>(self.addr(), &AndromedaQuery::Owner {})
            .unwrap()
            .owner
    }
}

#[macro_export]
macro_rules! mock_ado {
    ($t:ident, $e:ident, $q:ident) => {
        impl MockContract<$e, $q> for $t {
            fn addr(&self) -> &Addr {
                &self.0
            }
        }

        impl From<Addr> for $t {
            fn from(addr: Addr) -> Self {
                Self(addr)
            }
        }

        impl MockADO<$e, $q> for $t {}
    };
}
