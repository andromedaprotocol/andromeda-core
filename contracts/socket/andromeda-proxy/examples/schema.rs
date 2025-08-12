use andromeda_socket::proxy::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_schema::{export_schema_with_title, schema_for, write_api};
use std::env::current_dir;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    };
    // Add this if there is a cw20 receive integration
    export_schema_with_title(&schema_for!(Cw20HookMsg), &out_dir, "cw20receive");
}
