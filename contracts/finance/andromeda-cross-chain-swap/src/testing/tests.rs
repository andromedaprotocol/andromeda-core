use andromeda_std::{
    ado_base::modules::Module,
    amp::{
        messages::{AMPMsg, AMPPkt},
        recipient::Recipient,
        AndrAddr,
    },
    error::ContractError,
    testing::mock_querier::MOCK_ADDRESS_LIST_CONTRACT,
};

use cosmwasm_std::{
    attr, coin, coins, from_binary,
    testing::{mock_env, mock_info},
    to_binary, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Response, StdError, SubMsg, Timestamp,
};
use cw_utils::Expiration;
pub const OWNER: &str = "creator";

use super::mock_querier::MOCK_KERNEL_CONTRACT;

use crate::{
    contract::{execute, instantiate, query},
    testing::mock_querier::mock_dependencies_custom,
};
use andromeda_finance::cross_chain_swap::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn init(deps: DepsMut, modules: Option<Vec<Module>>) -> Response {
    let msg = InstantiateMsg {
        owner: Some(OWNER.to_owned()),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
    };

    let info = mock_info("owner", &[]);
    instantiate(deps, mock_env(), info, msg).unwrap()
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);
    let res = init(deps.as_mut(), None);
    assert_eq!(0, res.messages.len());
}
