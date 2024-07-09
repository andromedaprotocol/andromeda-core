use andromeda_data_storage::string_storage::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_schema::write_api;
fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,

    }
}