use andromeda_modules::curve::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_schema::write_api;
fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,

    }
}
