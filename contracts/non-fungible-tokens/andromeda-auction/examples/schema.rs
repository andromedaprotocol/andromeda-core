use std::env::current_dir;

use cosmwasm_schema::{write_api, schema_for, export_schema_with_title};

use andromeda_non_fungible_tokens::auction::{ExecuteMsg, InstantiateMsg, QueryMsg, Cw721HookMsg};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    };
    export_schema_with_title(&schema_for!(Cw721HookMsg), &out_dir, "cw721receive");
}
