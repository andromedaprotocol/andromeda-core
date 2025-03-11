use cosmwasm_schema::cw_serde;
use osmosis_std_derive::CosmwasmExt;

#[cw_serde]
// #[proto_message(type_url = "/andromeda.distribution.v1beta1.MsgSetWithdrawAddress")]
pub struct MsgSetWithdrawAddress {
    // #[prost(string, tag = "1")]
    pub delegator_address: ::prost::alloc::string::String,
    // #[prost(string, tag = "2")]
    pub withdraw_address: ::prost::alloc::string::String,
}

#[cw_serde]
// #[proto_message(type_url = "/andromeda.distribution.v1beta1.MsgSetWithdrawAddressResponse")]
pub struct MsgSetWithdrawAddressResponse {}

#[cw_serde]
// #[proto_message(type_url = "/andromeda.distribution.v1beta1.MsgWithdrawDelegatorReward")]
pub struct MsgWithdrawDelegatorReward {
    // #[prost(string, tag = "1")]
    pub delegator_address: ::prost::alloc::string::String,
    // #[prost(string, tag = "2")]
    pub validator_address: ::prost::alloc::string::String,
}

#[cw_serde]
// #[proto_message(type_url = "/andromeda.distribution.v1beta1.MsgWithdrawDelegatorRewardResponse")]
pub struct MsgWithdrawDelegatorRewardResponse {}

#[cw_serde]
// #[proto_message(type_url = "/cosmos.base.v1beta1.Coin")]
pub struct Coin {
    // #[prost(string, tag = "1")]
    pub denom: ::prost::alloc::string::String,
    // #[prost(string, tag = "2")]
    pub amount: ::prost::alloc::string::String,
}

#[cw_serde]
// #[proto_message(type_url = "/cosmos.staking.v1beta1.MsgCancelUnbondingDelegation")]
pub struct MsgCancelUnbondingDelegation {
    // #[prost(string, tag = "1")]
    pub delegator_address: ::prost::alloc::string::String,
    // #[prost(string, tag = "2")]
    pub validator_address: ::prost::alloc::string::String,
    // #[prost(message, tag = "3")]
    pub amount: ::core::option::Option<Coin>,
    // #[prost(uint64, tag = "4")]
    pub creation_height: u64,
}
