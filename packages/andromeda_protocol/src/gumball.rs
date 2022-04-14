use common::ado_base::{recipient::Recipient, AndromedaMsg, AndromedaQuery};
use cosmwasm_std::{attr, BankMsg, Binary, Coin, Event, Uint128};
use cw721_base::MintMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::cw721::TokenExtension;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub andromeda_cw721_contract: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    Mint(Box<MintMsg<TokenExtension>>),
    Buy {},
    // Allows buying and suspends mintung
    SwitchState {
        /// The price per token.
        price: Coin,
        /// The amount of tokens a wallet can purchase, default is 1.
        max_amount_per_wallet: Option<Uint128>,
        /// The recipient of the funds if the sale met the minimum sold.
        recipient: Recipient,
        /// The status of the gumball. True for "Available", False for "Refilling"
        status: bool,
    },
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    NumberOfNFTs {},
    State {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetNumberOfNFTsResponse {
    pub number: usize,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    /// The price of each token.
    pub price: Coin,
    /// The max number of tokens allowed per wallet.
    pub max_amount_per_wallet: Uint128,
    /// The recipient of the funds upon a sale.
    /// Most likely the contract, but could also be the splitter contract for example.
    pub recipient: Recipient,
    // true for available, false for refill
    pub status: bool,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetStateResponse {
    pub state: State,
}
