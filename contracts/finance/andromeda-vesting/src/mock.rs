#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_finance::vesting::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::amp::Recipient;
use andromeda_testing::{
    mock::MockApp,
    mock_ado,
    mock_contract::{MockADO, MockContract},
};
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};
use cw_utils::Duration;

pub struct MockVestingContract(Addr);
mock_ado!(MockVestingContract, ExecuteMsg, QueryMsg);

impl MockVestingContract {
    #[allow(clippy::too_many_arguments)]
    pub fn instantiate(
        code_id: u64,
        sender: &Addr,
        app: &mut MockApp,
        unbonding_duration: Duration,
        recipient: Recipient,
        denom: String,
        kernel_address: impl Into<String>,
        owner: Option<String>,
    ) -> MockVestingContract {
        let msg = mock_vesting_instantiate_msg(
            unbonding_duration,
            recipient,
            denom,
            kernel_address,
            owner,
        );
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "App Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockVestingContract(Addr::unchecked(addr))
    }
}

pub fn mock_andromeda_vesting() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_vesting_instantiate_msg(
    unbonding_duration: Duration,
    recipient: Recipient,
    denom: String,
    kernel_address: impl Into<String>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        unbonding_duration,
        recipient,
        denom,
        kernel_address: kernel_address.into(),
        owner,
    }
}
