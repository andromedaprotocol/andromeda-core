use std::env::current_dir;

use andromeda_non_fungible_tokens::marketplace::{
    Cw20HookMsg, Cw721HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
};
use cosmwasm_schema::{export_schema_with_title, schema_for, write_api};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    };
    export_schema_with_title(&schema_for!(Cw721HookMsg), &out_dir, "cw721receive");
    export_schema_with_title(&schema_for!(Cw20HookMsg), &out_dir, "cw20receive");
}
