pub mod ado_type;
pub mod app_contract;
pub mod block_height;
#[cfg(any(feature = "module_hooks", feature = "modules"))]
pub mod hooks;
pub mod kernel_address;
pub mod modules;
pub mod ownership;
pub mod permissioning;
pub mod version;

pub mod withdraw;
use crate::amp::{messages::AMPPkt, AndrAddr};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
pub use modules::Module;

#[cfg(feature = "modules")]
use cosmwasm_std::Uint64;

use self::ownership::OwnershipMessage;
use self::permissioning::PermissioningMessage;

#[cw_serde]
pub struct InstantiateMsg {
    pub ado_type: String,
    pub ado_version: String,
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
    #[serde(rename = "amp_receive")]
    AMPReceive(AMPPkt),
    Permissioning(PermissioningMessage),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum AndromedaQuery {
    #[returns(self::ownership::ContractOwnerResponse)]
    Owner {},
    #[returns(self::ado_type::TypeResponse)]
    Type {},
    #[returns(self::kernel_address::KernelAddressResponse)]
    KernelAddress {},
    #[returns(self::ownership::PublisherResponse)]
    OriginalPublisher {},
    #[returns(self::block_height::BlockHeightResponse)]
    BlockHeightUponCreation {},
    #[returns(self::version::VersionResponse)]
    Version {},
    #[returns(self::app_contract::AppContractResponse)]
    AppContract {},
    #[cfg(feature = "modules")]
    #[returns(Module)]
    Module { id: Uint64 },
    #[cfg(feature = "modules")]
    #[returns(Vec<String>)]
    ModuleIds {},
    #[returns(::cosmwasm_std::BalanceResponse)]
    Balance { address: AndrAddr },
    #[returns(Vec<self::permissioning::PermissionInfo>)]
    Permissions {
        actor: AndrAddr,
        limit: Option<u32>,
        start_after: Option<String>,
    },
    #[returns(Vec<String>)]
    PermissionedActions {},
}
