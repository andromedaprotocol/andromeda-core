use std::env::current_dir;
use std::fs::create_dir_all;

use andromeda_gumball::state::State;
use andromeda_non_fungible_tokens::gumball::{
    ExecuteMsg, InstantiateMsg, NumberOfNftsResponse, QueryMsg, StatusResponse,
};
use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);

    export_schema(&schema_for!(StatusResponse), &out_dir);
    export_schema(&schema_for!(NumberOfNftsResponse), &out_dir);
    export_schema(&schema_for!(State), &out_dir);
}
