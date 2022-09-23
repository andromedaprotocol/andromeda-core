use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_std::Coin;
use cw721::Cw721ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    // The cw721 contract(s) that you want to allow NFTs from
    pub nft_contract: Vec<String>,
    pub unbonding_period: u64,
    pub reward: Coin,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw721HookMsg {
    /// Stakes NFT
    Stake {},
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    StakedNft { key: String },
    AllowedContracts {},
    UnbondingPeriod {},
    Reward {},
}
