use std::env::current_dir;

use cosmwasm_schema::{export_schema_with_title, schema_for, write_api};

use andromeda_fungible_tokens::cw20_exchange::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    };
    export_schema_with_title(&schema_for!(Cw20HookMsg), &out_dir, "cw20receive");
}
