#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{attr, ensure, Binary, Deps, DepsMut, Env, MessageInfo, Response, Storage};

use andromeda_data_storage::graph::{
    Coordinate, CoordinateResponse, ExecuteMsg, GetAllPointsResponse, GetMapInfoResponse,
    GetMaxPointResponse, InstantiateMsg, MapInfo, QueryMsg,
};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    common::{actions::call_action, context::ExecuteContext, encode_binary},
    error::ContractError,
};

use crate::state::{MAP_INFO, MAP_POINT_INFO, POINT};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-graph";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let resp = ADOContract::default().instantiate(
        deps.storage,
        env,
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

    MAP_INFO.save(deps.storage, &msg.map_info)?;
    POINT.save(deps.storage, &0)?;

    Ok(resp)
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

fn handle_execute(mut ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    let action = msg.as_ref().to_string();

    let action_response = call_action(
        &mut ctx.deps,
        &ctx.info,
        &ctx.env,
        &ctx.amp_ctx,
        msg.as_ref(),
    )?;

    let res = match msg {
        ExecuteMsg::UpdateMap { map_info } => execute_update_map(ctx, map_info, action),
        ExecuteMsg::StoreCoordinate { coordinate } => {
            execute_store_coordinate(ctx, coordinate, action)
        }
        _ => ADOContract::default().execute(ctx, msg),
    }?;

    Ok(res
        .add_submessages(action_response.messages)
        .add_attributes(action_response.attributes)
        .add_events(action_response.events))
}

pub fn execute_update_map(
    ctx: ExecuteContext,
    map_info: MapInfo,
    action: String,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender;
    ensure!(
        ADOContract::default().is_owner_or_operator(ctx.deps.storage, sender.as_ref())?,
        ContractError::Unauthorized {}
    );

    let map = MAP_INFO
        .load(ctx.deps.storage)
        .map_err(|_| ContractError::InvalidParameter {
            error: Some("Map not found".to_string()),
        })?;

    ensure!(
        map != map_info,
        ContractError::InvalidParameter {
            error: Some("Map already exists".to_string())
        }
    );

    MAP_INFO.save(ctx.deps.storage, &map_info)?;

    let max_point = POINT.load(ctx.deps.storage)?;

    for point in 1..=max_point {
        MAP_POINT_INFO.remove(ctx.deps.storage, &point);
    }

    POINT.save(ctx.deps.storage, &0)?;

    Ok(Response::new().add_attributes(vec![attr("action", action), attr("sender", sender)]))
}

pub fn execute_store_coordinate(
    ctx: ExecuteContext,
    coordinate: Coordinate,
    action: String,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender;
    ensure!(
        ADOContract::default().is_owner_or_operator(ctx.deps.storage, sender.as_ref())?,
        ContractError::Unauthorized {}
    );

    let map = MAP_INFO
        .load(ctx.deps.storage)
        .map_err(|_| ContractError::InvalidParameter {
            error: Some("Map not found".to_string()),
        })
        .unwrap();

    let MapInfo {
        map_size,
        allow_negative,
        map_decimal,
    } = map;
    let x_length = map_size.x_width as f64;
    let y_length = map_size.y_width as f64;
    let z_length = match map_size.z_width {
        Some(z) => Some(z as f64),
        None => None,
    };

    let is_z_allowed = match z_length {
        Some(_) => true,
        None => false,
    };

    let x_coordinate = ((coordinate.x_coordinate * 10_f64.powf(map_decimal as f64)) as i64) as f64
        / 10_f64.powf(map_decimal as f64);
    let y_coordinate = ((coordinate.y_coordinate * 10_f64.powf(map_decimal as f64)) as i64) as f64
        / 10_f64.powf(map_decimal as f64);
    let z_coordinate = match coordinate.z_coordinate {
        Some(z) => Some(
            ((z * 10_f64.powf(map_decimal as f64)) as i64) as f64 / 10_f64.powf(map_decimal as f64),
        ),
        None => None,
    };

    match z_coordinate {
        Some(_) => {
            ensure!(
                is_z_allowed == true,
                ContractError::InvalidParameter {
                    error: Some("Z-axis is not allowed".to_string())
                }
            );
        }
        None => {
            ensure!(
                is_z_allowed == false,
                ContractError::InvalidParameter {
                    error: Some("Z-axis is allowed".to_string())
                }
            );
        }
    }

    match allow_negative {
        true => {
            ensure!(
                x_coordinate >= -(x_length / 2_f64) && x_coordinate <= x_length / 2_f64,
                ContractError::InvalidParameter {
                    error: Some("Wrong X Coordinate Range".to_string())
                }
            );

            ensure!(
                y_coordinate >= -(y_length / 2_f64) && y_coordinate <= y_length / 2_f64,
                ContractError::InvalidParameter {
                    error: Some("Wrong Y Coordinate Range".to_string())
                }
            );

            if is_z_allowed == true {
                if let Some(z_coordinate) = z_coordinate {
                    ensure!(
                        z_coordinate >= -(z_length.unwrap() / 2_f64)
                            && z_coordinate <= z_length.unwrap() / 2_f64,
                        ContractError::InvalidParameter {
                            error: Some("Wrong Z Coordinate Range".to_string())
                        }
                    );
                }
            }
        }
        false => {
            ensure!(
                x_coordinate >= 0_f64 && x_coordinate <= x_length,
                ContractError::InvalidParameter {
                    error: Some("Wrong X Coordinate Range".to_string())
                }
            );

            ensure!(
                y_coordinate >= 0_f64 && y_coordinate <= y_length,
                ContractError::InvalidParameter {
                    error: Some("Wrong Y Coordinate Range".to_string())
                }
            );

            if is_z_allowed == true {
                if let Some(z_coordinate) = z_coordinate {
                    ensure!(
                        z_coordinate >= 0_f64 && z_coordinate <= z_length.unwrap(),
                        ContractError::InvalidParameter {
                            error: Some("Wrong Z Coordinate Range".to_string())
                        }
                    );
                }
            }
        }
    };

    let point = POINT.load(ctx.deps.storage)?.checked_add(1).unwrap();

    MAP_POINT_INFO.save(
        ctx.deps.storage,
        &point,
        &CoordinateResponse {
            x: x_coordinate.to_string(),
            y: y_coordinate.to_string(),
            z: match z_coordinate {
                Some(z_coordinate) => Some(z_coordinate.to_string()),
                None => None,
            },
        },
    )?;
    POINT.save(ctx.deps.storage, &point)?;

    Ok(Response::new().add_attributes(vec![attr("action", action), attr("sender", sender)]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetMapInfo {} => encode_binary(&get_map_info(deps.storage)?),
        QueryMsg::GetMaxPoint {} => encode_binary(&get_max_point(deps.storage)?),
        QueryMsg::GetAllPoints {} => encode_binary(&get_all_points(deps.storage)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

pub fn get_map_info(storage: &dyn Storage) -> Result<GetMapInfoResponse, ContractError> {
    let map_info = MAP_INFO
        .load(storage)
        .map_err(|_| ContractError::InvalidParameter {
            error: Some("Map not found".to_string()),
        });
    match map_info {
        Ok(map_info) => Ok(GetMapInfoResponse { map_info }),
        Err(err) => Err(err),
    }
}

pub fn get_max_point(storage: &dyn Storage) -> Result<GetMaxPointResponse, ContractError> {
    let max_point = POINT.load(storage)?;
    Ok(GetMaxPointResponse { max_point })
}

pub fn get_all_points(storage: &dyn Storage) -> Result<GetAllPointsResponse, ContractError> {
    let max_point = POINT.load(storage)?;

    let mut res: Vec<CoordinateResponse> = Vec::new();

    for point in 1..=max_point {
        let coordinate = MAP_POINT_INFO.load(storage, &point)?;
        res.push(coordinate);
    }
    Ok(GetAllPointsResponse { points: res })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}
