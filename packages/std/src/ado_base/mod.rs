pub mod ado_type;
pub mod app_contract;
pub mod block_height;
pub mod kernel_address;
pub mod modules;
pub mod ownership;
pub mod permissioning;
#[cfg(feature = "rates")]
pub mod rates;
pub mod version;

pub mod withdraw;
use crate::amp::{messages::AMPPkt, AndrAddr};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;



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
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[cw_serde]
pub enum AndromedaMsg {
    Ownership(OwnershipMessage),
    UpdateAppContract {
        address: String,
    },
    UpdateKernelAddress {
        address: Addr,
    },
    #[cfg(feature = "rates")]
    Rates(self::rates::RatesMessage),
    #[serde(rename = "amp_receive")]
    AMPReceive(AMPPkt),
    Permissioning(PermissioningMessage),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum AndromedaQuery {
    #[returns(self::ownership::ContractOwnerResponse)]
    Owner {},
    #[returns(self::ownership::ContractPotentialOwnerResponse)]
    OwnershipRequest {},
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
    #[returns(self::version::ADOBaseVersionResponse)]
    ADOBaseVersion {},
    #[returns(self::app_contract::AppContractResponse)]
    AppContract {},
    #[returns(Vec<self::permissioning::PermissionInfo>)]
    Permissions {
        actor: AndrAddr,
        limit: Option<u32>,
        start_after: Option<String>,
    },
    #[returns(Vec<self::permissioning::PermissionedActionsResponse>)]
    PermissionedActions {},

    #[cfg(feature = "rates")]
    #[returns(Option<self::rates::Rate>)]
    GetRate { action: String },
}
