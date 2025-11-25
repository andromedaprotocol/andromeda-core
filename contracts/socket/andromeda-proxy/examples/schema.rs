use andromeda_socket::proxy::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_schema::write_api;
use std::env::current_dir;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    };
}
