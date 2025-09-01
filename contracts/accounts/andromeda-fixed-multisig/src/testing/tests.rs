use cosmwasm_std::testing::{message_info, mock_env};
use cosmwasm_std::{
    coin, from_json, Addr, BankMsg, CosmosMsg, Decimal, Deps, Empty, Env, MessageInfo, Response,
};

use cw2::{get_contract_version, ContractVersion};
use cw3::{ProposalResponse, Status, Vote, VoteListResponse};
use cw_utils::{Duration, Expiration, Threshold};

use andromeda_accounts::fixed_multisig::Voter;
use andromeda_accounts::fixed_multisig::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{
    amp::AndrAddr,
    error::ContractError,
    testing::mock_querier::{mock_dependencies_custom, MOCK_KERNEL_CONTRACT},
    testing::utils::assert_response,
};

use crate::contract::{execute, instantiate, query, CONTRACT_NAME, CONTRACT_VERSION};

fn mock_env_height(height_delta: u64) -> Env {
    let mut env = mock_env();
    env.block.height += height_delta;
    env
}

fn mock_env_time(time_delta: u64) -> Env {
    let mut env = mock_env();
    env.block.time = env.block.time.plus_seconds(time_delta);
    env
}

fn voter<T: Into<String>>(addr: T, weight: u64) -> Voter {
    Voter {
        addr: AndrAddr::from_string(addr.into()),
        weight,
    }
}

pub const OWNER: &str = "cosmwasm1fsgzj6t7udv8zhf6zj32mkqhcjcpv52yph5qsdcl0qt94jgdckqs2g053y";
const VOTER1: &str = "voter0001";
const VOTER2: &str = "voter0002";
const VOTER3: &str = "voter0003";
const VOTER4: &str = "voter0004";
const VOTER5: &str = "voter0005";
const VOTER6: &str = "voter0006";
const NOWEIGHT_VOTER: &str = "voterxxxx";
const SOMEBODY: &str = "somebody";

#[track_caller]
fn setup_test_case(
    deps: &mut cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        andromeda_std::testing::mock_querier::WasmMockQuerier,
    >,
    info: MessageInfo,
    threshold: Threshold,
    max_voting_period: Duration,
) -> Result<Response<Empty>, ContractError> {
    let voter1 = deps.api.addr_make(VOTER1);
    let voter2 = deps.api.addr_make(VOTER2);
    let voter3 = deps.api.addr_make(VOTER3);
    let voter4 = deps.api.addr_make(VOTER4);
    let voter5 = deps.api.addr_make(VOTER5);
    let voter6 = deps.api.addr_make(VOTER6);
    let noweight_voter = deps.api.addr_make(NOWEIGHT_VOTER);

    // Instantiate a contract with voters
    let voters = vec![
        voter(&info.sender, 1),
        voter(voter1, 1),
        voter(voter2, 2),
        voter(voter3, 3),
        voter(voter4, 4),
        voter(voter5, 5),
        voter(voter6, 1),
        voter(noweight_voter, 0),
    ];

    let instantiate_msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        voters,
        threshold,
        max_voting_period,
    };
    instantiate(deps.as_mut(), mock_env(), info, instantiate_msg)
}

fn get_tally(deps: Deps, proposal_id: u64) -> u64 {
    // Get all the voters on the proposal
    let voters = QueryMsg::ListVotes {
        proposal_id,
        start_after: None,
        limit: None,
    };
    let votes: VoteListResponse = from_json(query(deps, mock_env(), voters).unwrap()).unwrap();
    // Sum the weights of the Yes votes to get the tally
    votes
        .votes
        .iter()
        .filter(|&v| v.vote == Vote::Yes)
        .map(|v| v.weight)
        .sum()
}

#[test]
fn test_instantiate_works() {
    let mut deps: cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        andromeda_std::testing::mock_querier::WasmMockQuerier,
    > = mock_dependencies_custom(&[]);
    let owner = deps.api.addr_make(OWNER);
    let info = message_info(&owner, &[]);

    let max_voting_period = Duration::Time(1234567);

    // No voters fails
    let instantiate_msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        voters: vec![],
        threshold: Threshold::ThresholdQuorum {
            threshold: Decimal::zero(),
            quorum: Decimal::percent(1),
        },
        max_voting_period,
    };
    let err = instantiate(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        instantiate_msg.clone(),
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::CustomError {
            msg: "No voters".to_string()
        }
    );

    // Zero required weight fails
    let instantiate_msg = InstantiateMsg {
        voters: vec![voter(OWNER, 1)],
        ..instantiate_msg
    };
    let err = instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::Threshold(cw_utils::ThresholdError::InvalidThreshold {})
    );

    // Total weight less than required weight not allowed
    let threshold = Threshold::AbsoluteCount { weight: 100 };
    let err = setup_test_case(&mut deps, info.clone(), threshold, max_voting_period).unwrap_err();
    assert_eq!(
        err,
        ContractError::Threshold(cw_utils::ThresholdError::UnreachableWeight {})
    );

    // All valid
    let threshold = Threshold::AbsoluteCount { weight: 1 };
    setup_test_case(&mut deps, info, threshold, max_voting_period).unwrap();

    // Verify
    assert_eq!(
        ContractVersion {
            contract: CONTRACT_NAME.to_string(),
            version: CONTRACT_VERSION.to_string(),
        },
        get_contract_version(&deps.storage).unwrap()
    )
}

#[test]
fn zero_weight_member_cant_vote() {
    let mut deps = mock_dependencies_custom(&[]);

    let threshold = Threshold::AbsoluteCount { weight: 4 };
    let voting_period = Duration::Time(2000000);

    let owner = deps.api.addr_make(OWNER);
    let info = message_info(&owner, &[]);
    setup_test_case(&mut deps, info, threshold, voting_period).unwrap();

    let somebody = deps.api.addr_make(SOMEBODY);
    let bank_msg = BankMsg::Send {
        to_address: somebody.into(),
        amount: vec![coin(1, "uandr")],
    };
    let msgs = vec![CosmosMsg::Bank(bank_msg)];

    let non_voter = deps.api.addr_make(NOWEIGHT_VOTER);
    // Voter without voting power still can create proposal
    let info = message_info(&non_voter, &[]);
    let proposal = ExecuteMsg::Propose {
        title: "Rewarding somebody".to_string(),
        description: "Do we reward her?".to_string(),
        msgs,
        latest: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info, proposal).unwrap();

    // Get the proposal id from the logs
    let proposal_id: u64 = res.attributes[2].value.parse().unwrap();

    // Cast a No vote
    let no_vote = ExecuteMsg::Vote {
        proposal_id,
        vote: Vote::No,
    };
    // Only voters with weight can vote
    let info = message_info(&Addr::unchecked(NOWEIGHT_VOTER), &[]);
    let err = execute(deps.as_mut(), mock_env(), info, no_vote).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});
}

#[test]
fn test_propose_works() {
    let mut deps = mock_dependencies_custom(&[]);

    let threshold = Threshold::AbsoluteCount { weight: 4 };
    let voting_period = Duration::Time(2000000);

    let owner = deps.api.addr_make(OWNER);
    let info = message_info(&owner, &[]);
    setup_test_case(&mut deps, info, threshold, voting_period).unwrap();

    let bank_msg = BankMsg::Send {
        to_address: SOMEBODY.into(),
        amount: vec![coin(1, "BTC")],
    };
    let msgs = vec![CosmosMsg::Bank(bank_msg)];

    // Only voters can propose
    let info = message_info(&Addr::unchecked(SOMEBODY), &[]);
    let proposal = ExecuteMsg::Propose {
        title: "Rewarding somebody".to_string(),
        description: "Do we reward her?".to_string(),
        msgs: msgs.clone(),
        latest: None,
    };
    let err = execute(deps.as_mut(), mock_env(), info, proposal.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // Wrong expiration option fails
    let owner = deps.api.addr_make(OWNER);
    let info = message_info(&owner, &[]);
    let proposal_wrong_exp = ExecuteMsg::Propose {
        title: "Rewarding somebody".to_string(),
        description: "Do we reward her?".to_string(),
        msgs,
        latest: Some(Expiration::AtHeight(123456)),
    };
    let err = execute(deps.as_mut(), mock_env(), info, proposal_wrong_exp).unwrap_err();
    assert_eq!(
        err,
        ContractError::CustomError {
            msg: "Wrong expiration".to_string()
        }
    );

    let voter3 = deps.api.addr_make(VOTER3);
    // Proposal from voter works
    let info = message_info(&voter3, &[]);
    execute(deps.as_mut(), mock_env(), info, proposal.clone()).unwrap();

    let voter4 = deps.api.addr_make(VOTER4);
    // Proposal from voter with enough vote power directly passes
    let info = message_info(&voter4, &[]);
    execute(deps.as_mut(), mock_env(), info, proposal).unwrap();
}

#[test]
fn test_vote_works() {
    let mut deps = mock_dependencies_custom(&[]);

    let threshold = Threshold::AbsoluteCount { weight: 3 };
    let voting_period = Duration::Time(2000000);

    let owner = deps.api.addr_make(OWNER);
    let info = message_info(&owner, &[]);
    setup_test_case(&mut deps, info.clone(), threshold, voting_period).unwrap();

    // Propose
    let bank_msg = BankMsg::Send {
        to_address: SOMEBODY.into(),
        amount: vec![coin(1, "BTC")],
    };
    let msgs = vec![CosmosMsg::Bank(bank_msg)];
    let proposal = ExecuteMsg::Propose {
        title: "Pay somebody".to_string(),
        description: "Do I pay her?".to_string(),
        msgs,
        latest: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), proposal).unwrap();

    // Get the proposal id from the logs
    let proposal_id: u64 = res.attributes[2].value.parse().unwrap();

    // Owner cannot vote (again)
    let yes_vote = ExecuteMsg::Vote {
        proposal_id,
        vote: Vote::Yes,
    };
    let err = execute(deps.as_mut(), mock_env(), info, yes_vote.clone()).unwrap_err();
    assert_eq!(
        err,
        ContractError::CustomError {
            msg: "Already voted".to_string()
        }
    );

    // Only voters can vote
    let info = message_info(&Addr::unchecked(SOMEBODY), &[]);
    let err = execute(deps.as_mut(), mock_env(), info, yes_vote.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // But voter1 can
    let voter1 = deps.api.addr_make(VOTER1);
    let info = message_info(&voter1, &[]);
    execute(deps.as_mut(), mock_env(), info, yes_vote.clone()).unwrap();

    // No/Veto votes have no effect on the tally
    // Get the proposal id from the logs
    let proposal_id: u64 = res.attributes[2].value.parse().unwrap();

    // Compute the current tally
    let tally = get_tally(deps.as_ref(), proposal_id);

    // Cast a No vote
    let no_vote = ExecuteMsg::Vote {
        proposal_id,
        vote: Vote::No,
    };
    let voter2 = deps.api.addr_make(VOTER2);
    let info = message_info(&voter2, &[]);
    execute(deps.as_mut(), mock_env(), info, no_vote.clone()).unwrap();

    // Cast a Veto vote
    let veto_vote = ExecuteMsg::Vote {
        proposal_id,
        vote: Vote::Veto,
    };
    let voter3 = deps.api.addr_make(VOTER3);
    let info = message_info(&voter3, &[]);
    execute(deps.as_mut(), mock_env(), info.clone(), veto_vote).unwrap();

    // Verify
    assert_eq!(tally, get_tally(deps.as_ref(), proposal_id));

    // Once voted, votes cannot be changed
    let err = execute(deps.as_mut(), mock_env(), info.clone(), yes_vote.clone()).unwrap_err();
    assert_eq!(
        err,
        ContractError::CustomError {
            msg: "Already voted".to_string()
        }
    );
    assert_eq!(tally, get_tally(deps.as_ref(), proposal_id));

    // Expired proposals cannot be voted
    let env = match voting_period {
        Duration::Time(duration) => mock_env_time(duration + 1),
        Duration::Height(duration) => mock_env_height(duration + 1),
    };
    let err = execute(deps.as_mut(), env, info, no_vote).unwrap_err();
    assert_eq!(err, ContractError::Expired {});

    let voter4 = deps.api.addr_make(VOTER4);
    // Vote it again, so it passes
    let info = message_info(&voter4, &[]);
    execute(deps.as_mut(), mock_env(), info, yes_vote.clone()).unwrap();

    // Passed proposals can still be voted (while they are not expired or executed)
    let voter5 = deps.api.addr_make(VOTER5);
    let info = message_info(&voter5, &[]);
    execute(deps.as_mut(), mock_env(), info, yes_vote).unwrap();

    // Propose
    let owner = deps.api.addr_make(OWNER);
    let info = message_info(&owner, &[]);
    let bank_msg = BankMsg::Send {
        to_address: SOMEBODY.into(),
        amount: vec![coin(1, "BTC")],
    };
    let msgs = vec![CosmosMsg::Bank(bank_msg)];
    let proposal = ExecuteMsg::Propose {
        title: "Pay somebody".to_string(),
        description: "Do I pay her?".to_string(),
        msgs,
        latest: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info, proposal).unwrap();

    // Get the proposal id from the logs
    let proposal_id: u64 = res.attributes[2].value.parse().unwrap();

    // Cast a No vote
    let no_vote = ExecuteMsg::Vote {
        proposal_id,
        vote: Vote::No,
    };
    // Voter1 vote no, weight 1
    let info = message_info(&voter1, &[]);
    execute(deps.as_mut(), mock_env(), info, no_vote.clone()).unwrap();

    // Voter 4 votes no, weight 4, total weight for no so far 5, need 14 to reject
    let info = message_info(&voter4, &[]);
    execute(deps.as_mut(), mock_env(), info, no_vote.clone()).unwrap();

    // Voter 3 votes no, weight 3, total weight for no far 8, need 14
    let info = message_info(&voter3, &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, no_vote.clone()).unwrap();

    // Voter 5 votes no, weight 5, total weight for no far 13, need 14
    let info = message_info(&voter5, &[]);
    execute(deps.as_mut(), mock_env(), info, no_vote.clone()).unwrap();

    // Voter 2 votes no, weight 2, total weight for no so far 15, need 14.
    // Can now reject
    let info = message_info(&voter2, &[]);
    execute(deps.as_mut(), mock_env(), info, no_vote).unwrap();

    // Rejected proposals can still be voted (while they are not expired)
    let voter6 = deps.api.addr_make(VOTER6);
    let info = message_info(&voter6, &[]);
    let yes_vote = ExecuteMsg::Vote {
        proposal_id,
        vote: Vote::Yes,
    };
    execute(deps.as_mut(), mock_env(), info, yes_vote).unwrap();
}

#[test]
fn test_execute_works() {
    let mut deps = mock_dependencies_custom(&[]);

    let threshold = Threshold::AbsoluteCount { weight: 3 };
    let voting_period = Duration::Time(2000000);

    let owner = deps.api.addr_make(OWNER);
    let info = message_info(&owner, &[]);
    setup_test_case(&mut deps, info.clone(), threshold, voting_period).unwrap();

    // Propose
    let somebody = deps.api.addr_make(SOMEBODY);
    let bank_msg = BankMsg::Send {
        to_address: somebody.into(),
        amount: vec![coin(1, "BTC")],
    };
    let msgs = vec![CosmosMsg::Bank(bank_msg)];
    let proposal = ExecuteMsg::Propose {
        title: "Pay somebody".to_string(),
        description: "Do I pay her?".to_string(),
        msgs: msgs.clone(),
        latest: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), proposal).unwrap();

    // Get the proposal id from the logs
    let proposal_id: u64 = res.attributes[2].value.parse().unwrap();

    // Only Passed can be executed
    let execution = ExecuteMsg::Execute { proposal_id };
    let err = execute(deps.as_mut(), mock_env(), info, execution.clone()).unwrap_err();
    assert_eq!(
        err,
        ContractError::CustomError {
            msg: "Wrong execute status".to_string()
        }
    );

    // Vote it, so it passes
    let vote = ExecuteMsg::Vote {
        proposal_id,
        vote: Vote::Yes,
    };
    let voter3 = deps.api.addr_make(VOTER3);
    let info = message_info(&voter3, &[]);
    execute(deps.as_mut(), mock_env(), info.clone(), vote).unwrap();

    // In passing: Try to close Passed fails
    let closing = ExecuteMsg::Close { proposal_id };
    let err = execute(deps.as_mut(), mock_env(), info, closing).unwrap_err();
    assert_eq!(
        err,
        ContractError::CustomError {
            msg: "Wrong close status".to_string()
        }
    );

    // Execute works. Anybody can execute Passed proposals
    let somebody = deps.api.addr_make(SOMEBODY);
    let info = message_info(&somebody, &[]);
    execute(deps.as_mut(), mock_env(), info.clone(), execution).unwrap();

    // In passing: Try to close Executed fails
    let closing = ExecuteMsg::Close { proposal_id };
    let err = execute(deps.as_mut(), mock_env(), info, closing).unwrap_err();
    assert_eq!(
        err,
        ContractError::CustomError {
            msg: "Wrong close status".to_string()
        }
    );
}

#[test]
fn proposal_pass_on_expiration() {
    let mut deps = mock_dependencies_custom(&[]);

    let threshold = Threshold::ThresholdQuorum {
        threshold: Decimal::percent(51),
        quorum: Decimal::percent(1),
    };
    let voting_period = Duration::Time(2000000);

    let owner = deps.api.addr_make(OWNER);
    let info = message_info(&owner, &[]);
    setup_test_case(&mut deps, info.clone(), threshold, voting_period).unwrap();

    // Propose
    let somebody = deps.api.addr_make(SOMEBODY);
    let bank_msg = BankMsg::Send {
        to_address: somebody.into(),
        amount: vec![coin(1, "BTC")],
    };
    let msgs = vec![CosmosMsg::Bank(bank_msg)];
    let proposal = ExecuteMsg::Propose {
        title: "Pay somebody".to_string(),
        description: "Do I pay her?".to_string(),
        msgs,
        latest: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info, proposal).unwrap();

    // Get the proposal id from the logs
    let proposal_id: u64 = res.attributes[2].value.parse().unwrap();

    // Vote it, so it passes after voting period is over
    let vote = ExecuteMsg::Vote {
        proposal_id,
        vote: Vote::Yes,
    };
    let voter3 = deps.api.addr_make(VOTER3);
    let info = message_info(&voter3, &[]);
    execute(deps.as_mut(), mock_env(), info, vote).unwrap();

    // Wait until the voting period is over
    let env = match voting_period {
        Duration::Time(duration) => mock_env_time(duration + 1),
        Duration::Height(duration) => mock_env_height(duration + 1),
    };

    // Proposal should now be passed
    let prop: ProposalResponse = from_json(
        query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Proposal { proposal_id },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(prop.status, Status::Passed);

    // Closing should NOT be possible
    let somebody = deps.api.addr_make(SOMEBODY);
    let info = message_info(&somebody, &[]);
    let err = execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::Close { proposal_id },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::CustomError {
            msg: "Wrong close status".to_string()
        }
    );

    // Execution should now be possible
    let res = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::Execute { proposal_id },
    )
    .unwrap();
    let expected_res: Response = Response::<Empty>::new()
        .add_attribute("action", "execute")
        .add_attribute("sender", somebody)
        .add_attribute("proposal_id", proposal_id.to_string());
    assert_response(&res, &expected_res, "fixed_multisig_execute");
}

#[test]
fn test_close_works() {
    let mut deps = mock_dependencies_custom(&[]);

    let threshold = Threshold::AbsoluteCount { weight: 3 };
    let voting_period = Duration::Height(2000000);

    let owner = deps.api.addr_make(OWNER);
    let info = message_info(&owner, &[]);
    setup_test_case(&mut deps, info.clone(), threshold, voting_period).unwrap();

    // Propose
    let bank_msg = BankMsg::Send {
        to_address: SOMEBODY.into(),
        amount: vec![coin(1, "BTC")],
    };
    let msgs = vec![CosmosMsg::Bank(bank_msg)];
    let proposal = ExecuteMsg::Propose {
        title: "Pay somebody".to_string(),
        description: "Do I pay her?".to_string(),
        msgs: msgs.clone(),
        latest: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info, proposal).unwrap();

    // Get the proposal id from the logs
    let proposal_id: u64 = res.attributes[2].value.parse().unwrap();

    let closing = ExecuteMsg::Close { proposal_id };

    // Anybody can close
    let somebody = deps.api.addr_make(SOMEBODY);
    let info = message_info(&somebody, &[]);

    // Non-expired proposals cannot be closed
    let err = execute(deps.as_mut(), mock_env(), info, closing).unwrap_err();
    assert_eq!(
        err,
        ContractError::CustomError {
            msg: "Not expired".to_string()
        }
    );

    // Expired proposals can be closed
    let owner = deps.api.addr_make(OWNER);
    let info = message_info(&owner, &[]);

    let proposal = ExecuteMsg::Propose {
        title: "(Try to) pay somebody".to_string(),
        description: "Pay somebody after time?".to_string(),
        msgs,
        latest: Some(Expiration::AtHeight(123456)),
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), proposal).unwrap();

    // Get the proposal id from the logs
    let proposal_id: u64 = res.attributes[2].value.parse().unwrap();

    let closing = ExecuteMsg::Close { proposal_id };

    // Close expired works
    let env = mock_env_height(1234567);
    execute(
        deps.as_mut(),
        env,
        message_info(&somebody, &[]),
        closing.clone(),
    )
    .unwrap();

    // Trying to close it again fails
    let err = execute(deps.as_mut(), mock_env(), info, closing).unwrap_err();
    assert_eq!(
        err,
        ContractError::CustomError {
            msg: "Wrong close status".to_string()
        }
    );
}
