use cosmwasm_schema::write_api;

use andromeda_non_fungible_tokens::crowdfund::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,

    }
}
