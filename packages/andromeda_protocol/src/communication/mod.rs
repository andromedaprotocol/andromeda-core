use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod msg;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum AndromedaMsg {
    Receive(Option<String>),
}

pub enum AndromedaQuery {
    Get(Option<String>),
    Owner {},
    Operators {},
}
