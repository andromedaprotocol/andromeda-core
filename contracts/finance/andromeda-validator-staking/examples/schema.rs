use andromeda_finance::validator_staking::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_schema::write_api;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute:ExecuteMsg,
        query:QueryMsg
    }
}
