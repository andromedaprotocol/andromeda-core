use andromeda_finance::validator_staking::InstantiateMsg;
use cosmwasm_schema::write_api;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
    }
}
