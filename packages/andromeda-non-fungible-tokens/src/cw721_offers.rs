use common::ado_base::hooks::AndromedaHook;
use cosmwasm_std::{Coin, Event, SubMsg, Uint128};
use cw721::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Offer {
    pub denom: String,
    /// What the purchaser offers.
    pub offer_amount: Uint128,
    /// What the owner of the token will get if they accept (royalties deducted).
    pub remaining_amount: Uint128,
    /// The amount of tax the purchaser paid.
    pub tax_amount: Uint128,
    pub expiration: Expiration,
    pub purchaser: String,
    pub msgs: Vec<SubMsg>,
    pub events: Vec<Event>,
}

impl Offer {
    pub fn get_full_amount(&self) -> Coin {
        Coin {
            denom: self.denom.clone(),
            amount: self.offer_amount + self.tax_amount,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub andromeda_cw721_contract: String,
    pub valid_demoms: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    PlaceOffer {
        token_id: String,
        expiration: Expiration,
        offer_amount: Uint128,
    },
    CancelOffer {
        token_id: String,
    },
    /// Restricted to Cw721 contract.
    AcceptOffer {
        token_id: String,
        recipient: String,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OfferResponse {
    pub denom: String,
    pub offer_amount: Uint128,
    pub remaining_amount: Uint128,
    pub tax_amount: Uint128,
    pub expiration: Expiration,
    pub purchaser: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AllOffersResponse {
    pub offers: Vec<OfferResponse>,
}

impl From<Offer> for OfferResponse {
    fn from(offer: Offer) -> OfferResponse {
        OfferResponse {
            denom: offer.denom,
            offer_amount: offer.offer_amount,
            remaining_amount: offer.remaining_amount,
            tax_amount: offer.tax_amount,
            expiration: offer.expiration,
            purchaser: offer.purchaser,
        }
    }
}
