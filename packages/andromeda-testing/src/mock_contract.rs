use core::fmt;

use andromeda_std::{
    ado_base::{
        ownership::{ContractOwnerResponse, OwnershipMessage},
        permissioning::{Permission, PermissioningMessage},
        AndromedaMsg, AndromedaQuery,
    },
    amp::AndrAddr,
};

use cosmwasm_std::{Addr, Coin};
use cw_multi_test::{AppResponse, Executor};
use serde::{de::DeserializeOwned, Serialize};

pub use anyhow::Result as AnyResult;

use crate::mock::MockApp;

pub type ExecuteResult = AnyResult<AppResponse>;

pub trait MockContract<E: Serialize + fmt::Debug, Q: Serialize + fmt::Debug> {
    fn addr(&self) -> &Addr;

    fn execute(
        &self,
        app: &mut MockApp,
        msg: &E,
        sender: Addr,
        funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        app.execute_contract(sender, self.addr().clone(), &msg, funds)
    }

    fn query<T: DeserializeOwned>(&self, app: &MockApp, msg: Q) -> T {
        app.wrap()
            .query_wasm_smart::<T>(self.addr().clone(), &msg)
            .unwrap()
    }
}

pub trait MockADO<E: Serialize + fmt::Debug, Q: Serialize + fmt::Debug>:
    MockContract<E, Q>
{
    fn query_owner(&self, app: &MockApp) -> String {
        app.wrap()
            .query_wasm_smart::<ContractOwnerResponse>(self.addr(), &AndromedaQuery::Owner {})
            .unwrap()
            .owner
    }

    fn accept_ownership(&self, app: &mut MockApp, sender: Addr) -> AnyResult<AppResponse> {
        app.execute_contract(
            sender,
            self.addr().clone(),
            &AndromedaMsg::Ownership(OwnershipMessage::AcceptOwnership {}),
            &[],
        )
    }

    fn execute_set_permissions(
        &self,
        app: &mut MockApp,
        sender: Addr,
        actors: Vec<AndrAddr>,
        action: impl Into<String>,
        permission: Permission,
    ) -> ExecuteResult {
        app.execute_contract(
            sender,
            self.addr().clone(),
            &AndromedaMsg::Permissioning(PermissioningMessage::SetPermission {
                actors,
                action: action.into(),
                permission,
            }),
            &[],
        )
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
