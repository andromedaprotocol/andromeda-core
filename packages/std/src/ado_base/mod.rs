pub mod ado_type;
pub mod block_height;
#[cfg(any(feature = "module_hooks", feature = "modules"))]
pub mod hooks;
pub mod kernel_address;
pub mod modules;
pub mod operators;
pub mod ownership;
pub mod permissioning;
pub mod version;

pub mod withdraw;
#[cfg(feature = "withdraw")]
use crate::ado_base::withdraw::Withdrawal;
#[cfg(feature = "withdraw")]
use crate::amp::recipient::Recipient;
use crate::{
    ado_base::permissioning::Permission,
    amp::{messages::AMPPkt, AndrAddr},
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary};
pub use modules::Module;

#[cfg(feature = "modules")]
use cosmwasm_std::Uint64;

use self::ownership::OwnershipMessage;

#[cw_serde]
pub struct InstantiateMsg {
    pub ado_type: String,
    pub ado_version: String,
    pub operators: Option<Vec<String>>,
    pub kernel_address: String,
    pub owner: Option<String>,
}

#[cw_serde]
pub enum AndromedaMsg {
    Ownership(OwnershipMessage),
    UpdateAppContract {
        address: String,
    },
    UpdateKernelAddress {
        address: Addr,
    },
    #[cfg(feature = "withdraw")]
    Withdraw {
        recipient: Option<Recipient>,
        tokens_to_withdraw: Option<Vec<Withdrawal>>,
    },
    #[cfg(feature = "modules")]
    RegisterModule {
        module: Module,
    },
    #[cfg(feature = "modules")]
    DeregisterModule {
        module_idx: Uint64,
    },
    #[cfg(feature = "modules")]
    AlterModule {
        module_idx: Uint64,
        module: Module,
    },
    Deposit {
        recipient: Option<AndrAddr>,
        msg: Option<Binary>,
    },
    #[serde(rename = "amp_receive")]
    AMPReceive(AMPPkt),
    SetPermission {
        actor: AndrAddr,
        action: String,
        permission: Permission,
    },
    RemovePermission {
        action: String,
        actor: AndrAddr,
    },
    PermissionAction {
        action: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum AndromedaQuery {
    #[returns(self::ownership::ContractOwnerResponse)]
    Owner {},
    #[returns(self::ownership::ContractPotentialOwnerResponse)]
    OwnershipRequest {},
    #[returns(self::operators::OperatorsResponse)]
    Operators {},
    #[returns(self::ado_type::TypeResponse)]
    Type {},
    #[returns(self::kernel_address::KernelAddressResponse)]
    KernelAddress {},
    #[returns(self::ownership::PublisherResponse)]
    OriginalPublisher {},
    #[returns(self::block_height::BlockHeightResponse)]
    BlockHeightUponCreation {},
    #[returns(self::operators::IsOperatorResponse)]
    IsOperator { address: String },
    #[returns(self::version::VersionResponse)]
    Version {},
    #[returns(Option<::cosmwasm_std::Addr>)]
    AppContract {},
    #[cfg(feature = "modules")]
    #[returns(Module)]
    Module { id: Uint64 },
    #[cfg(feature = "modules")]
    #[returns(Vec<String>)]
    ModuleIds {},
    #[cfg(feature = "withdraw")]
    #[returns(::cosmwasm_std::BalanceResponse)]
    WithdrawableBalance { address: AndrAddr },
    #[returns(Vec<self::permissioning::PermissionInfo>)]
    Permissions {
        actor: AndrAddr,
        limit: Option<u32>,
        start_after: Option<String>,
    },
    #[returns(Vec<String>)]
    PermissionedActions {},
}
