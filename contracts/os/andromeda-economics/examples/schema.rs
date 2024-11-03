use andromeda_std::os::economics::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_schema::{export_schema_with_title, schema_for, write_api};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    }
    export_schema_with_title(&schema_for!(Cw20HookMsg), &out_dir, "cw20receive");
}
