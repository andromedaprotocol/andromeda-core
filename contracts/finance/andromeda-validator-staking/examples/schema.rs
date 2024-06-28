use andromeda_finance::validator_staking::{InstantiateMsg, ExecuteMsg, QueryMsg};
use cosmwasm_schema::write_api;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute:ExecuteMsg,
        query:QueryMsg
    }
}
