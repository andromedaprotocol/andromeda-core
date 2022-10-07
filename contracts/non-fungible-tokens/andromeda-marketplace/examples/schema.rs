use std::env::current_dir;

use andromeda_non_fungible_tokens::marketplace::{ExecuteMsg, InstantiateMsg, QueryMsg, Cw721HookMsg};
use cosmwasm_schema::{write_api, schema_for, export_schema_with_title};

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
