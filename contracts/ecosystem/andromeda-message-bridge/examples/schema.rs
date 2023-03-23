use andromeda_ibc::message_bridge::{ExecuteMsg, InstantiateMsg};
use cosmwasm_schema::write_api;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,

    }
}
