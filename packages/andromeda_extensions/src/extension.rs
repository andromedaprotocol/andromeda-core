use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub type Fee = i64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Extension {
    WhiteListExtension { moderators: Vec<HumanAddr> },
    TaxableExtension { tax: Fee, receivers: Vec<HumanAddr> },
    RoyaltiesExtension { fee: Fee, receivers: Vec<HumanAddr> },
}
