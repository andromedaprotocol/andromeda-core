#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{attr, ensure, Binary, Deps, DepsMut, Env, MessageInfo, Response, Storage};

use andromeda_data_storage::graph::{
    Coordinate, CoordinateInfo, ExecuteMsg, GetAllPointsResponse, GetMapInfoResponse,
    GetMaxPointResponse, InstantiateMsg, MapInfo, QueryMsg, StoredDate,
};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    amp::AndrAddr,
    common::{actions::call_action, context::ExecuteContext, encode_binary},
    error::ContractError,
    os::aos_querier::AOSQuerier,
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
        ExecuteMsg::StoreCoordinate {
            coordinate,
            is_timestamp_allowed,
        } => execute_store_coordinate(ctx, coordinate, is_timestamp_allowed, action),
        ExecuteMsg::StoreUserCoordinate {
            user_location_paths,
        } => execute_store_user_coordinate(ctx, user_location_paths, action),
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
    is_timestamp_allowed: bool,
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

    let MapInfo {
        map_size,
        allow_negative,
        map_decimal,
    } = map;
    let x_length = map_size.x_width as f64;
    let y_length = map_size.y_width as f64;
    let z_length = map_size.z_width.map(|z| z as f64);

    let is_z_allowed = z_length.is_some();

    let x_coordinate = ((coordinate.x_coordinate * 10_f64.powf(map_decimal as f64)) as i64) as f64
        / 10_f64.powf(map_decimal as f64);
    let y_coordinate = ((coordinate.y_coordinate * 10_f64.powf(map_decimal as f64)) as i64) as f64
        / 10_f64.powf(map_decimal as f64);
    let z_coordinate = coordinate.z_coordinate.map(|z| {
        ((z * 10_f64.powf(map_decimal as f64)) as i64) as f64 / 10_f64.powf(map_decimal as f64)
    });

    ensure!(
        z_coordinate.is_some() == is_z_allowed,
        ContractError::InvalidParameter {
            error: Some(if is_z_allowed {
                "Z-axis is allowed".to_string()
            } else {
                "Z-axis is not allowed".to_string()
            })
        }
    );

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

            if is_z_allowed {
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

            if is_z_allowed {
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
    let timestamp = match is_timestamp_allowed {
        true => Some(ctx.env.block.time.nanos()),
        false => None,
    };

    MAP_POINT_INFO.save(
        ctx.deps.storage,
        &point,
        &(
            CoordinateInfo {
                x: x_coordinate.to_string(),
                y: y_coordinate.to_string(),
                z: z_coordinate.map(|z_coordinate| z_coordinate.to_string()),
            },
            StoredDate { timestamp },
        ),
    )?;
    POINT.save(ctx.deps.storage, &point)?;

    Ok(Response::new().add_attributes(vec![attr("action", action), attr("sender", sender)]))
}

pub fn execute_store_user_coordinate(
    ctx: ExecuteContext,
    user_location_paths: Vec<AndrAddr>,
    action: String,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender;
    ensure!(
        ADOContract::default().is_owner_or_operator(ctx.deps.storage, sender.as_ref())?,
        ContractError::Unauthorized {}
    );
    for user_location_path in user_location_paths {
        let address = user_location_path.get_raw_address(&ctx.deps.as_ref())?;
        let contract_info = ctx.deps.querier.query_wasm_contract_info(address);
        if let Ok(contract_info) = contract_info {
            let code_id = contract_info.code_id;
            let adodb_addr =
                ADOContract::default().get_adodb_address(ctx.deps.storage, &ctx.deps.querier)?;
            let ado_type = AOSQuerier::ado_type_getter(&ctx.deps.querier, &adodb_addr, code_id)?;

            if ado_type.is_none() {
                return Err(ContractError::InvalidADOType {
                    msg: Some("ADO Type must be point: None".to_string()),
                });
            }
            let ado_type = ado_type.unwrap();
            if ado_type == "point".to_string() {
            } else {
                return Err(ContractError::InvalidADOType {
                    msg: Some(format!("ADO Type must be point: {:?}", ado_type)),
                });
            }
        } else {
            // Not a contract
            return Err(ContractError::InvalidAddress {});
        }
    }
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

    let mut res: Vec<(CoordinateInfo, StoredDate)> = Vec::new();

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
