use std::env::current_dir;

use andromeda_socket::osmosis::{ExecuteMsg, InstantiateMsg, QueryMsg};

use cosmwasm_schema::write_api;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    };
    // Add this if there is a cw20 receive integration
    // export_schema_with_title(&schema_for!(Cw20HookMsg), &out_dir, "cw20receive");
}
