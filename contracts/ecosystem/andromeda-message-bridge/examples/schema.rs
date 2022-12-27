use andromeda_ibc::message_bridge::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_schema::write_api;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,

    }
}
