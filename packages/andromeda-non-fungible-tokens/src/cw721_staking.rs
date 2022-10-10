use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Timestamp};
use cw721::Cw721ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {
    // The cw721 contract(s) that you want to allow NFTs from
    pub nft_contract: Vec<String>,
    pub unbonding_period: u64,
    pub reward: Coin,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    ReceiveNft(Cw721ReceiveMsg),

    /// Assigns reward, and saves the unbonding time for later claim.
    Unstake {
        key: String,
    },
    /// Sends back the NFT to its original owner alongside the accrued rewards
    Claim {
        key: String,
    },
    UpdateAllowedContracts {
        contracts: Vec<String>,
    },
    AddAllowedContract {
        new_contract: String,
    },
    RemoveAllowedContract {
        old_contract: String,
    },
    UpdateUnbondingPeriod {
        new_period: u64,
    },
}

#[cw_serde]
pub enum Cw721HookMsg {
    /// Stakes NFT
    Stake {},
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    #[returns(StakedNft)]
    StakedNft { key: String },
    #[returns(Vec<String>)]
    AllowedContracts {},
    #[returns(u64)]
    UnbondingPeriod {},
    #[returns(Coin)]
    Reward {},
}

#[cw_serde]
pub struct StakedNft {
    pub owner: String,
    pub id: String,
    pub contract_address: String,
    pub time_of_staking: Timestamp,
    pub time_of_unbonding: Option<Timestamp>,
    pub reward: Coin,
    pub accrued_reward: Option<Coin>,
}
