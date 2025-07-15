use std::env::current_dir;

use andromeda_socket::osmosis_token_factory::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveHook};
use cosmwasm_schema::write_api;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    };
    // Export CW20 receive hook schema
    cosmwasm_schema::export_schema_with_title(&cosmwasm_schema::schema_for!(ReceiveHook), &out_dir, "cw20receive");
} 