use andromeda_finance::validator_staking::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_std::{Addr, Coin, Delegation, Empty};

use crate::contract::{execute, instantiate, query};
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cw_multi_test::{App, Contract, ContractWrapper};

use andromeda_std::{amp::AndrAddr, error::ContractError};

pub struct MockValidatorStaking(Addr);
mock_ado!(MockValidatorStaking, ExecuteMsg, QueryMsg);

impl MockValidatorStaking {
    pub fn execute_stake(
        &self,
        app: &mut App,
        sender: Addr,
        validator: Option<Addr>,
        funds: Vec<Coin>,
    ) -> ExecuteResult {
        let msg = mock_execute_stake(validator);
        self.execute(app, &msg, sender, &funds)
    }

    pub fn execute_unstake(
        &self,
        app: &mut App,
        sender: Addr,
        validator: Option<Addr>,
    ) -> ExecuteResult {
        let msg = mock_execute_unstake(validator);
        self.execute(app, &msg, sender, &[])
    }

    pub fn execute_claim_reward(
        &self,
        app: &mut App,
        sender: Addr,
        validator: Option<Addr>,
        recipient: Option<AndrAddr>,
    ) -> ExecuteResult {
        let msg = mock_execute_claim_reward(validator, recipient);
        self.execute(app, &msg, sender, &[])
    }

    pub fn query_staked_tokens(
        &self,
        app: &App,
        validator: Option<Addr>,
    ) -> Result<Delegation, ContractError> {
        let msg = mock_get_staked_tokens(validator);
        Ok(app
            .wrap()
            .query_wasm_smart::<Delegation>(self.addr().clone(), &msg)?)
    }
}

pub fn mock_andromeda_validator_staking() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_validator_staking_instantiate_msg(
    default_validator: Addr,
    owner: Option<String>,
    kernel_address: String,
) -> InstantiateMsg {
    InstantiateMsg {
        default_validator,
        owner,
        kernel_address,
    }
}

pub fn mock_execute_stake(validator: Option<Addr>) -> ExecuteMsg {
    ExecuteMsg::Stake { validator }
}

pub fn mock_execute_unstake(validator: Option<Addr>) -> ExecuteMsg {
    ExecuteMsg::Unstake { validator }
}

pub fn mock_execute_claim_reward(
    validator: Option<Addr>,
    recipient: Option<AndrAddr>,
) -> ExecuteMsg {
    ExecuteMsg::Claim {
        validator,
        recipient,
    }
}

pub fn mock_get_staked_tokens(validator: Option<Addr>) -> QueryMsg {
    QueryMsg::StakedTokens { validator }
}
