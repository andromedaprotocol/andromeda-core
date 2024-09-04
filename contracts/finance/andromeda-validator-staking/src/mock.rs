use andromeda_finance::validator_staking::{ExecuteMsg, InstantiateMsg, QueryMsg, UnstakingTokens};
use cosmwasm_std::{Addr, Coin, Delegation, Empty, Uint128};

use crate::contract::{execute, instantiate, query, reply};
use andromeda_testing::{
    mock::MockApp,
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cw_multi_test::{Contract, ContractWrapper};

use andromeda_std::{amp::AndrAddr, error::ContractError};

pub struct MockValidatorStaking(Addr);
mock_ado!(MockValidatorStaking, ExecuteMsg, QueryMsg);

impl MockValidatorStaking {
    pub fn execute_stake(
        &self,
        app: &mut MockApp,
        sender: Addr,
        validator: Option<Addr>,
        funds: Vec<Coin>,
    ) -> ExecuteResult {
        let msg = mock_execute_stake(validator);
        self.execute(app, &msg, sender, &funds)
    }

    pub fn execute_unstake(
        &self,
        app: &mut MockApp,
        sender: Addr,
        validator: Option<Addr>,
        amount: Option<Uint128>,
    ) -> ExecuteResult {
        let msg = mock_execute_unstake(validator, amount);
        self.execute(app, &msg, sender, &[])
    }

    pub fn execute_claim_reward(
        &self,
        app: &mut MockApp,
        sender: Addr,
        validator: Option<Addr>,
        recipient: Option<AndrAddr>,
    ) -> ExecuteResult {
        let msg = mock_execute_claim_reward(validator, recipient);
        self.execute(app, &msg, sender, &[])
    }

    pub fn execute_withdraw_fund(&self, app: &mut MockApp, sender: Addr) -> ExecuteResult {
        let msg = mock_execute_withdraw_fund(None, None);
        self.execute(app, &msg, sender, &[])
    }

    pub fn query_staked_tokens(
        &self,
        app: &MockApp,
        validator: Option<Addr>,
    ) -> Result<Delegation, ContractError> {
        let msg = mock_get_staked_tokens(validator);
        Ok(app
            .wrap()
            .query_wasm_smart::<Delegation>(self.addr().clone(), &msg)?)
    }
    pub fn query_unstaked_tokens(
        &self,
        app: &MockApp,
    ) -> Result<Vec<UnstakingTokens>, ContractError> {
        let msg = mock_get_unstaked_tokens();
        Ok(app
            .wrap()
            .query_wasm_smart::<Vec<UnstakingTokens>>(self.addr().clone(), &msg)?)
    }
}

pub fn mock_andromeda_validator_staking() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query).with_reply(reply);
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

pub fn mock_execute_unstake(validator: Option<Addr>, amount: Option<Uint128>) -> ExecuteMsg {
    ExecuteMsg::Unstake { validator, amount }
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

pub fn mock_execute_withdraw_fund(
    denom: Option<String>,
    recipient: Option<AndrAddr>,
) -> ExecuteMsg {
    ExecuteMsg::WithdrawFunds { denom, recipient }
}

pub fn mock_get_staked_tokens(validator: Option<Addr>) -> QueryMsg {
    QueryMsg::StakedTokens { validator }
}

pub fn mock_get_unstaked_tokens() -> QueryMsg {
    QueryMsg::UnstakedTokens {}
}
