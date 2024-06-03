use andromeda_non_fungible_tokens::crowdfund::{
    CampaignStage, CampaignSummaryResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg,
    PresaleTierOrder, QueryMsg, SimpleTierOrder, Tier, TierMetaData, TierOrder, TierOrdersResponse,
    TiersResponse,
};

use andromeda_non_fungible_tokens::cw721::ExecuteMsg as Cw721ExecuteMsg;
use andromeda_std::ado_base::permissioning::Permission;
use andromeda_std::amp::messages::AMPPkt;
use andromeda_std::amp::{AndrAddr, Recipient};
use andromeda_std::common::denom::{Asset, SEND_CW20_ACTION};
use andromeda_std::common::migration::ensure_compatibility;
use andromeda_std::common::{Milliseconds, MillisecondsExpiration, OrderBy};
use andromeda_std::{ado_base::ownership::OwnershipMessage, common::actions::call_action};
use andromeda_std::{ado_contract::ADOContract, common::context::ExecuteContext};

use andromeda_std::{
    ado_base::{hooks::AndromedaHook, InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    common::encode_binary,
    error::ContractError,
};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, ensure, from_json, wasm_execute, Addr, BankMsg, Binary, Coin, Deps, DepsMut, Env,
    MessageInfo, Reply, Response, StdError, Storage, SubMsg, Uint128, Uint64, WasmMsg,
};

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_utils::nonpayable;

use crate::state::{
    add_tier, clear_user_orders, get_and_increase_tier_token_id, get_config, get_current_cap,
    get_current_stage, get_tier, get_tiers, get_user_orders, is_valid_tiers, remove_tier,
    set_current_cap, set_current_stage, set_tier_orders, set_tiers, update_config, update_tier,
};

const CONTRACT_NAME: &str = "crates.io:andromeda-crowdfund";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env.clone(),
        deps.api,
        &deps.querier,
        info,
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;

    if let Asset::Cw20Token(addr) = msg.campaign_config.denom.clone() {
        let addr = addr.get_raw_address(&deps.as_ref())?;
        ADOContract::default().permission_action(SEND_CW20_ACTION, deps.storage)?;
        ADOContract::set_permission(
            deps.storage,
            SEND_CW20_ACTION,
            addr,
            Permission::Whitelisted(None),
        )?;
    }

    msg.campaign_config.validate(deps.branch(), &env)?;
    update_config(deps.storage, msg.campaign_config)?;

    set_tiers(deps.storage, msg.tiers)?;

    let owner = ADOContract::default().owner(deps.storage)?;
    let mod_resp =
        ADOContract::default().register_modules(owner.as_str(), deps.storage, msg.modules)?;

    Ok(inst_resp
        .add_attributes(mod_resp.attributes)
        .add_submessages(mod_resp.messages))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ensure_compatibility(&deps.as_ref(), "1.1.0")?;
    ADOContract::default().migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let ctx = ExecuteContext::new(deps, info, env);

    match msg {
        ExecuteMsg::AMPReceive(pkt) => {
            ADOContract::default().execute_amp_receive(ctx, pkt, handle_execute)
        }
        _ => handle_execute(ctx, msg),
    }
}

pub fn handle_execute(mut ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let action_response = call_action(
        &mut ctx.deps,
        &ctx.info,
        &ctx.env,
        &ctx.amp_ctx,
        msg.as_ref(),
    )?;
    if !matches!(msg, ExecuteMsg::UpdateAppContract { .. })
        && !matches!(
            msg,
            ExecuteMsg::Ownership(OwnershipMessage::UpdateOwner { .. })
        )
    {
        contract.module_hook::<Response>(
            &ctx.deps.as_ref(),
            AndromedaHook::OnExecute {
                sender: ctx.info.sender.to_string(),
                payload: encode_binary(&msg)?,
            },
        )?;
    }
    let res = match msg {
        ExecuteMsg::AddTier { tier } => execute_add_tier(ctx, tier),
        ExecuteMsg::UpdateTier { tier } => execute_update_tier(ctx, tier),
        ExecuteMsg::RemoveTier { level } => execute_remove_tier(ctx, level),
        ExecuteMsg::StartCampaign {
            start_time,
            end_time,
            presale,
        } => execute_start_campaign(ctx, start_time, end_time, presale),
        ExecuteMsg::PurchaseTiers { orders } => execute_purchase_tiers(ctx, orders),
        ExecuteMsg::Receive(msg) => handle_receive_cw20(ctx, msg),
        ExecuteMsg::EndCampaign {} => execute_end_campaign(ctx, false),
        ExecuteMsg::DiscardCampaign {} => execute_end_campaign(ctx, true),
        ExecuteMsg::Claim {} => execute_claim(ctx),
        _ => ADOContract::default().execute(ctx, msg),
    }?;

    Ok(res
        .add_submessages(action_response.messages)
        .add_attributes(action_response.attributes)
        .add_events(action_response.events))
}

fn execute_add_tier(ctx: ExecuteContext, tier: Tier) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    tier.validate()?;

    let curr_stage = get_current_stage(deps.storage);
    ensure!(
        curr_stage == CampaignStage::READY,
        ContractError::InvalidCampaignOperation {
            operation: "add_tier".to_string(),
            stage: curr_stage.to_string()
        }
    );

    add_tier(deps.storage, &tier)?;

    let mut resp = Response::new()
        .add_attribute("action", "add_tier")
        .add_attribute("level", tier.level)
        .add_attribute("label", tier.label)
        .add_attribute("price", tier.price);

    if let Some(limit) = tier.limit {
        resp = resp.add_attribute("limit", limit.to_string());
    }

    Ok(resp)
}

fn execute_update_tier(ctx: ExecuteContext, tier: Tier) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    tier.validate()?;

    let curr_stage = get_current_stage(deps.storage);
    ensure!(
        curr_stage == CampaignStage::READY,
        ContractError::InvalidCampaignOperation {
            operation: "update_tier".to_string(),
            stage: curr_stage.to_string()
        }
    );

    update_tier(deps.storage, &tier)?;

    let mut resp = Response::new()
        .add_attribute("action", "update_tier")
        .add_attribute("level", tier.level)
        .add_attribute("label", tier.label)
        .add_attribute("price", tier.price);

    if let Some(limit) = tier.limit {
        resp = resp.add_attribute("limit", limit.to_string());
    }

    Ok(resp)
}

fn execute_remove_tier(ctx: ExecuteContext, level: Uint64) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let curr_stage = get_current_stage(deps.storage);
    ensure!(
        curr_stage == CampaignStage::READY,
        ContractError::InvalidCampaignOperation {
            operation: "remove_tier".to_string(),
            stage: curr_stage.to_string()
        }
    );

    remove_tier(deps.storage, level.into())?;

    let resp = Response::new()
        .add_attribute("action", "remove_tier")
        .add_attribute("level", level);

    Ok(resp)
}

fn execute_start_campaign(
    ctx: ExecuteContext,
    start_time: Option<MillisecondsExpiration>,
    end_time: MillisecondsExpiration,
    presale: Option<Vec<PresaleTierOrder>>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;

    // Only owner can start the campaign
    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    // At least one tier should have no limit to start the campaign
    ensure!(is_valid_tiers(deps.storage), ContractError::InvalidTiers {});

    // Validate parameters
    ensure!(
        !end_time.is_expired(&env.block) && start_time.unwrap_or(Milliseconds::zero()) < end_time,
        ContractError::StartTimeAfterEndTime {}
    );

    // Campaign can only start on READY stage
    let curr_stage = get_current_stage(deps.storage);
    ensure!(
        curr_stage == CampaignStage::READY,
        ContractError::InvalidCampaignOperation {
            operation: "start_campaign".to_string(),
            stage: curr_stage.to_string()
        }
    );

    // Update tier sold amount and update tier orders based on presale
    if let Some(presale) = presale {
        let orders = presale.iter().map(|order| order.clone().into()).collect();
        set_tier_orders(deps.storage, orders)?;
    }

    // Set start time and end time
    let mut config = get_config(deps.storage)?;
    config.start_time = start_time;
    config.end_time = end_time;
    update_config(deps.storage, config)?;

    // update stage
    set_current_stage(deps.storage, CampaignStage::ONGOING)?;

    let resp = Response::new().add_attribute("action", "start_campaign");

    Ok(resp)
}

fn execute_purchase_tiers(
    ctx: ExecuteContext,
    orders: Vec<SimpleTierOrder>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        ref deps, ref info, ..
    } = ctx;

    // Ensure campaign accepting coin is received
    let campaign_config = get_config(deps.storage)?;
    ensure!(
        info.funds.len() == 1,
        ContractError::InvalidFunds {
            msg: format!(
                "Only {} is accepted by the campaign.",
                campaign_config.denom
            ),
        }
    );

    let payment: &Coin = &info.funds[0];

    let sender = info.sender.to_string();
    let denom = payment.denom.clone();
    let amount = payment.amount;
    purchase_tiers(ctx, Asset::NativeToken(denom), amount, sender, orders)
}

fn handle_receive_cw20(
    ctx: ExecuteContext,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let ExecuteContext { ref info, .. } = ctx;

    let amount = cw20_msg.amount;
    let sender = cw20_msg.sender;
    let denom = AndrAddr::from_string(info.sender.clone());
    match from_json(&cw20_msg.msg)? {
        Cw20HookMsg::PurchaseTiers { orders } => {
            purchase_tiers(ctx, Asset::Cw20Token(denom), amount, sender, orders)
        }
    }
}

fn execute_end_campaign(
    mut ctx: ExecuteContext,
    is_discard: bool,
) -> Result<Response, ContractError> {
    nonpayable(&ctx.info)?;

    let ExecuteContext {
        ref mut deps,
        ref info,
        ref env,
        ..
    } = ctx;

    // Only owner can end the campaign
    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    // Campaign is finished already successfully
    // NOTE: ending failed campaign has no effect and is ignored
    let curr_stage = get_current_stage(deps.storage);
    let action = if is_discard {
        "discard_campaign"
    } else {
        "end_campaign"
    };

    ensure!(
        curr_stage == CampaignStage::ONGOING
            || (is_discard && curr_stage != CampaignStage::SUCCESS),
        ContractError::InvalidCampaignOperation {
            operation: action.to_string(),
            stage: curr_stage.to_string()
        }
    );

    let current_cap = get_current_cap(deps.storage);
    let campaign_config = get_config(deps.storage)?;
    let soft_cap = campaign_config.soft_cap.unwrap_or(Uint128::one());
    let end_time = campaign_config.end_time;

    // Decide the next stage
    let next_stage = match (
        is_discard,
        current_cap >= soft_cap,
        end_time.is_expired(&env.block),
    ) {
        // discard the campaign as there are some unexpected issues
        (true, _, _) => CampaignStage::FAILED,
        // Capital hit the target capital and thus campaign is successful
        (false, true, _) => CampaignStage::SUCCESS,
        // Capital did hit the target capital and is expired, failed
        (false, false, true) => CampaignStage::FAILED,
        // Capital did not hit the target capital and campaign is not expired
        (false, false, false) => {
            if current_cap != Uint128::zero() {
                // Need to wait until campaign expires
                return Err(ContractError::CampaignNotExpired {});
            }
            // No capital is gained and thus it can be paused and restart again
            CampaignStage::READY
        }
    };

    set_current_stage(deps.storage, next_stage.clone())?;

    let mut resp = Response::new()
        .add_attribute("action", action)
        .add_attribute("result", next_stage.to_string());
    if next_stage == CampaignStage::SUCCESS {
        let campaign_denom = match campaign_config.denom {
            Asset::Cw20Token(ref cw20_token) => Asset::Cw20Token(AndrAddr::from_string(
                cw20_token.get_raw_address(&deps.as_ref())?.to_string(),
            )),
            denom => denom,
        };

        resp = resp.add_submessage(withdraw_to_recipient(
            ctx,
            campaign_config.withdrawal_recipient,
            current_cap,
            campaign_denom,
        )?);
    }
    Ok(resp)
}

fn purchase_tiers(
    ctx: ExecuteContext,
    denom: Asset,
    amount: Uint128,
    sender: String,
    orders: Vec<SimpleTierOrder>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, env, .. } = ctx;

    let campaign_config = get_config(deps.storage)?;
    let mut current_cap = get_current_cap(deps.storage);

    let current_stage = get_current_stage(deps.storage);

    // Tier can be purchased on ONGOING stage
    ensure!(
        current_stage == CampaignStage::ONGOING,
        ContractError::InvalidCampaignOperation {
            operation: "purchase_tiers".to_string(),
            stage: current_stage.to_string()
        }
    );

    // Need to wait until start_time
    ensure!(
        campaign_config
            .start_time
            .unwrap_or_default()
            .is_expired(&env.block),
        ContractError::CampaignNotStarted {}
    );

    // Campaign is expired or should be ended due to overfunding
    ensure!(
        !campaign_config.end_time.is_expired(&env.block)
            || campaign_config.hard_cap.unwrap_or(current_cap) > current_cap,
        ContractError::CampaignEnded {}
    );

    // Ensure campaign accepting coin is received
    let campaign_denom = match campaign_config.denom {
        Asset::Cw20Token(ref cw20_token) => {
            format!("cw20:{}", cw20_token.get_raw_address(&deps.as_ref())?)
        }
        Asset::NativeToken(ref addr) => format!("native:{}", addr),
    };
    ensure!(
        denom.to_string() == campaign_denom,
        ContractError::InvalidFunds {
            msg: format!(
                "Only {} is accepted by the campaign.",
                campaign_config.denom
            ),
        }
    );

    let mut full_orders = Vec::<TierOrder>::new();

    // Measure the total cost for orders
    let total_cost = orders.iter().try_fold(Uint128::zero(), |sum, order| {
        let tier = get_tier(deps.storage, u64::from(order.level))?;
        let new_sum: Result<Uint128, ContractError> = Ok(sum + tier.price * order.amount);
        full_orders.push(TierOrder {
            orderer: Addr::unchecked(sender.clone()),
            level: order.level,
            amount: order.amount,
            is_presale: false,
        });
        new_sum
    })?;

    // Ensure that enough payment is sent for the order
    ensure!(total_cost <= amount, ContractError::InsufficientFunds {});

    // Update order history and sold amount for the tier
    set_tier_orders(deps.storage, full_orders)?;
    current_cap = current_cap.checked_add(total_cost)?;

    // Update current capital
    set_current_cap(deps.storage, current_cap)?;
    let mut resp = Response::new()
        .add_attribute("action", "purchase_tiers")
        .add_attribute("payment", format!("{0}{1}", amount, denom))
        .add_attribute("total_cost", total_cost.to_string());

    if amount > total_cost {
        resp = resp
            .add_submessage(transfer_asset_msg(sender, amount - total_cost, denom)?)
            .add_attribute("refunded", amount - total_cost);
    }

    Ok(resp)
}

fn transfer_asset_msg(
    to_address: String,
    amount: Uint128,
    denom: Asset,
) -> Result<SubMsg, ContractError> {
    Ok(match denom {
        Asset::NativeToken(denom) => SubMsg::new(BankMsg::Send {
            to_address,
            amount: vec![coin(amount.u128(), denom)],
        }),
        Asset::Cw20Token(denom) => {
            let transfer_msg = Cw20ExecuteMsg::Transfer {
                recipient: to_address,
                amount,
            };
            let wasm_msg = wasm_execute(denom, &transfer_msg, vec![])?;
            SubMsg::new(wasm_msg)
        }
    })
}

fn withdraw_to_recipient(
    ctx: ExecuteContext,
    recipient: Recipient,
    amount: Uint128,
    denom: Asset,
) -> Result<SubMsg, ContractError> {
    match denom {
        Asset::NativeToken(denom) => {
            let kernel_address =
                ADOContract::default().get_kernel_address(ctx.deps.as_ref().storage)?;

            let mut pkt = AMPPkt::from_ctx(ctx.amp_ctx, ctx.env.contract.address.to_string());
            let amp_msg = recipient.generate_amp_msg(
                &ctx.deps.as_ref(),
                Some(vec![coin(amount.u128(), denom.clone())]),
            )?;

            pkt = pkt.add_message(amp_msg);
            pkt.to_sub_msg(kernel_address, Some(vec![coin(amount.u128(), denom)]), 1)
        }
        denom => transfer_asset_msg(
            recipient
                .address
                .get_raw_address(&ctx.deps.as_ref())?
                .to_string(),
            amount,
            denom,
        ),
    }
}

fn execute_claim(ctx: ExecuteContext) -> Result<Response, ContractError> {
    let ExecuteContext { mut deps, info, .. } = ctx;

    let curr_stage = get_current_stage(deps.storage);
    let mut resp = Response::new().add_attribute("action", "claim");

    let sub_response = match curr_stage {
        CampaignStage::SUCCESS => handle_successful_claim(deps.branch(), &info.sender)?,
        CampaignStage::FAILED => handle_failed_claim(deps.branch(), &info.sender)?,
        _ => {
            return Err(ContractError::InvalidCampaignOperation {
                operation: "Claim".to_string(),
                stage: curr_stage.to_string(),
            })
        }
    };
    resp = resp
        .add_attributes(sub_response.attributes)
        .add_submessages(sub_response.messages);

    clear_user_orders(deps.storage, info.sender)?;

    Ok(resp)
}

fn handle_successful_claim(deps: DepsMut, sender: &Addr) -> Result<Response, ContractError> {
    let campaign_config = get_config(deps.storage)?;

    let orders = get_user_orders(deps.storage, sender.clone(), None, None, true, None);
    ensure!(!orders.is_empty(), ContractError::NoPurchases {});

    // mint tier token to the owner
    let token_address = campaign_config
        .token_address
        .get_raw_address(&deps.as_ref())?;

    let mut resp = Response::new();
    for order in orders {
        let metadata = get_tier(deps.storage, order.level.into())?.metadata;
        for _ in 0..order.amount.into() {
            let mint_resp = mint(
                deps.storage,
                token_address.to_string(),
                metadata.clone(),
                sender.to_string(),
            )?;
            resp = resp
                .add_attributes(mint_resp.attributes)
                .add_submessages(mint_resp.messages);
        }
    }
    Ok(resp)
}

fn handle_failed_claim(deps: DepsMut, sender: &Addr) -> Result<Response, ContractError> {
    let campaign_config = get_config(deps.storage)?;

    let orders = get_user_orders(deps.storage, sender.clone(), None, None, false, None);
    ensure!(!orders.is_empty(), ContractError::NoPurchases {});

    // refund
    let total_cost = orders.iter().try_fold(Uint128::zero(), |sum, order| {
        let tier = get_tier(deps.storage, u64::from(order.level))?;
        let new_sum: Result<Uint128, ContractError> =
            Ok(sum.checked_add(tier.price.checked_mul(order.amount)?)?);
        new_sum
    })?;
    let mut resp = Response::new();

    let campaign_denom = match campaign_config.denom {
        Asset::Cw20Token(ref cw20_token) => Asset::Cw20Token(AndrAddr::from_string(
            cw20_token.get_raw_address(&deps.as_ref())?.to_string(),
        )),
        denom => denom,
    };

    let sub_msg = transfer_asset_msg(sender.to_string(), total_cost, campaign_denom)?;
    resp = resp.add_submessage(sub_msg);

    Ok(resp)
}

fn mint(
    storage: &mut dyn Storage,
    tier_contract: String,
    tier_metadata: TierMetaData,
    owner: String,
) -> Result<Response, ContractError> {
    let token_id = get_and_increase_tier_token_id(storage)?.to_string();

    Ok(Response::new().add_message(WasmMsg::Execute {
        contract_addr: tier_contract,
        msg: encode_binary(&Cw721ExecuteMsg::Mint {
            token_id,
            owner,
            token_uri: tier_metadata.token_uri,
            extension: tier_metadata.extension,
        })?,
        funds: vec![],
    }))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::CampaignSummary {} => encode_binary(&query_campaign_summary(deps)?),
        QueryMsg::TierOrders {
            orderer,
            start_after,
            limit,
            order_by,
        } => encode_binary(&query_user_orders(
            deps,
            orderer,
            start_after,
            limit,
            order_by,
        )?),
        QueryMsg::Tiers {
            start_after,
            limit,
            order_by,
        } => encode_binary(&query_tiers(deps, start_after, limit, order_by)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

fn query_campaign_summary(deps: Deps) -> Result<CampaignSummaryResponse, ContractError> {
    let current_cap = get_current_cap(deps.storage);
    let current_stage = get_current_stage(deps.storage);
    let config = get_config(deps.storage)?;
    Ok(CampaignSummaryResponse {
        title: config.title,
        description: config.description,
        banner: config.banner,
        url: config.url,
        token_address: config.token_address,
        denom: config.denom,
        withdrawal_recipient: config.withdrawal_recipient,
        soft_cap: config.soft_cap,
        hard_cap: config.hard_cap,
        start_time: config.start_time,
        end_time: config.end_time,
        current_stage: current_stage.to_string(),
        current_cap: current_cap.into(),
    })
}

fn query_user_orders(
    deps: Deps,
    orderer: String,
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> Result<TierOrdersResponse, ContractError> {
    let orders = get_user_orders(
        deps.storage,
        Addr::unchecked(orderer),
        start_after,
        limit,
        true,
        order_by,
    );
    Ok(TierOrdersResponse { orders })
}
fn query_tiers(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> Result<TiersResponse, ContractError> {
    let tiers = get_tiers(deps.storage, start_after, limit, order_by)?;
    Ok(TiersResponse { tiers })
}
