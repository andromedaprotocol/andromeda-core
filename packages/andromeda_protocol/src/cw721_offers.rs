use crate::communication::hooks::AndromedaHook;
use cosmwasm_std::Uint128;
use cw721::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub andromeda_cw721_contract: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    PlaceOffer {
        token_id: String,
        expiration: Expiration,
        offer_amount: Uint128,
    },
    AcceptOffer {
        token_id: String,
    },
    CancelOffer {
        token_id: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrHook(AndromedaHook),
    Offer {
        token_id: String,
    },
    AllOffers {
        purchaser: String,
        limit: Option<u32>,
        start_after: Option<String>,
    },
}
