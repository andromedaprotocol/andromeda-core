use andromeda_std::{common::context::ExecuteContext, error::ContractError};
use cosmwasm_std::{CosmosMsg, Empty, Response};
use cw3::{Ballot, Proposal, Status, Vote, Votes};
use cw_utils::Expiration;
use std::cmp::Ordering;

use crate::state::{next_id, BALLOTS, CONFIG, PROPOSALS, VOTERS};

pub fn execute_propose(
    ctx: ExecuteContext,
    title: String,
    description: String,
    msgs: Vec<CosmosMsg>,
    latest: Option<Expiration>,
) -> Result<Response<Empty>, ContractError> {
    // only members of the multisig can create a proposal
    let vote_power = VOTERS
        .may_load(ctx.deps.storage, &ctx.info.sender)?
        .ok_or(ContractError::Unauthorized {})?;

    let cfg = CONFIG.load(ctx.deps.storage)?;

    // max expires also used as default
    let max_expires = cfg.max_voting_period.after(&ctx.env.block);
    let mut expires = latest.unwrap_or(max_expires);
    let comp = expires.partial_cmp(&max_expires);
    if let Some(Ordering::Greater) = comp {
        expires = max_expires;
    } else if comp.is_none() {
        return Err(ContractError::CustomError {
            msg: "Wrong expiration".to_string(),
        });
    }

    let mut prop = Proposal {
        title,
        description,
        start_height: ctx.env.block.height,
        expires,
        msgs,
        status: Status::Open,
        votes: Votes::yes(vote_power),
        threshold: cfg.threshold,
        total_weight: cfg.total_weight,
        proposer: ctx.info.sender.clone(),
        deposit: None,
    };
    prop.update_status(&ctx.env.block);
    let id = next_id(ctx.deps.storage)?;
    PROPOSALS.save(ctx.deps.storage, id, &prop)?;

    // add the first yes vote from voter
    let ballot = Ballot {
        weight: vote_power,
        vote: Vote::Yes,
    };
    BALLOTS.save(ctx.deps.storage, (id, &ctx.info.sender), &ballot)?;

    Ok(Response::new()
        .add_attribute("action", "propose")
        .add_attribute("sender", ctx.info.sender)
        .add_attribute("proposal_id", id.to_string())
        .add_attribute("status", format!("{:?}", prop.status)))
}

pub fn execute_vote(
    ctx: ExecuteContext,
    proposal_id: u64,
    vote: Vote,
) -> Result<Response<Empty>, ContractError> {
    // only members of the multisig with weight >= 1 can vote
    let voter_power = VOTERS.may_load(ctx.deps.storage, &ctx.info.sender)?;
    let vote_power = match voter_power {
        Some(power) if power >= 1 => power,
        _ => return Err(ContractError::Unauthorized {}),
    };

    // ensure proposal exists and can be voted on
    let mut prop = PROPOSALS.load(ctx.deps.storage, proposal_id)?;
    // Allow voting on Passed and Rejected proposals too,
    if ![Status::Open, Status::Passed, Status::Rejected].contains(&prop.status) {
        return Err(ContractError::CustomError {
            msg: "Not open".to_string(),
        });
    }
    // if they are not expired
    if prop.expires.is_expired(&ctx.env.block) {
        return Err(ContractError::Expired {});
    }

    // cast vote if no vote previously cast
    BALLOTS.update(
        ctx.deps.storage,
        (proposal_id, &ctx.info.sender),
        |bal| match bal {
            Some(_) => Err(ContractError::CustomError {
                msg: "Already voted".to_string(),
            }),
            None => Ok(Ballot {
                weight: vote_power,
                vote,
            }),
        },
    )?;

    // update vote tally
    prop.votes.add_vote(vote, vote_power);
    prop.update_status(&ctx.env.block);
    PROPOSALS.save(ctx.deps.storage, proposal_id, &prop)?;

    Ok(Response::new()
        .add_attribute("action", "vote")
        .add_attribute("sender", ctx.info.sender)
        .add_attribute("proposal_id", proposal_id.to_string())
        .add_attribute("status", format!("{:?}", prop.status)))
}

pub fn execute_execute(ctx: ExecuteContext, proposal_id: u64) -> Result<Response, ContractError> {
    // anyone can trigger this if the vote passed

    let mut prop = PROPOSALS.load(ctx.deps.storage, proposal_id)?;
    prop.update_status(&ctx.env.block);
    if prop.status != Status::Passed {
        return Err(ContractError::CustomError {
            msg: "Wrong execute status".to_string(),
        });
    }

    // set it to executed
    prop.status = Status::Executed;
    PROPOSALS.save(ctx.deps.storage, proposal_id, &prop)?;

    Ok(Response::new()
        .add_messages(prop.msgs)
        .add_attribute("action", "execute")
        .add_attribute("sender", ctx.info.sender)
        .add_attribute("proposal_id", proposal_id.to_string()))
}

pub fn execute_close(
    ctx: ExecuteContext,
    proposal_id: u64,
) -> Result<Response<Empty>, ContractError> {
    // anyone can trigger this if the vote passed

    let mut prop = PROPOSALS.load(ctx.deps.storage, proposal_id)?;
    if [Status::Executed, Status::Rejected, Status::Passed].contains(&prop.status) {
        return Err(ContractError::CustomError {
            msg: "Wrong close status".to_string(),
        });
    }
    // Avoid closing of Passed due to expiration proposals
    if prop.current_status(&ctx.env.block) == Status::Passed {
        return Err(ContractError::CustomError {
            msg: "Wrong close status".to_string(),
        });
    }
    if !prop.expires.is_expired(&ctx.env.block) {
        return Err(ContractError::CustomError {
            msg: "Not expired".to_string(),
        });
    }

    // set it to failed
    prop.status = Status::Rejected;
    PROPOSALS.save(ctx.deps.storage, proposal_id, &prop)?;

    Ok(Response::new()
        .add_attribute("action", "close")
        .add_attribute("sender", ctx.info.sender)
        .add_attribute("proposal_id", proposal_id.to_string()))
}
