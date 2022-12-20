use andromeda_ecosystem::anchor_earn::{ExecuteMsg, InstantiateMsg, PositionResponse, QueryMsg};
use cosmwasm_schema::write_api;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,

    }
}
