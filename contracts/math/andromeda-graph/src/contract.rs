use andromeda_std::andr_execute_fn;
use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, ensure, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, SignedDecimal,
    StdError, Storage,
};

use andromeda_math::graph::{
    Coordinate, CoordinateInfo, ExecuteMsg, GetAllPointsResponse, GetMapInfoResponse,
    GetMaxPointNumberResponse, InstantiateMsg, MapInfo, QueryMsg, StoredDate,
};
use andromeda_math::point::{GetDataOwnerResponse, PointCoordinate, QueryMsg as PointQueryMsg};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    amp::AndrAddr,
    common::{context::ExecuteContext, encode_binary},
    error::ContractError,
    os::aos_querier::AOSQuerier,
};

use crate::state::{MAP_INFO, MAP_POINT_INFO, TOTAL_POINTS_NUMBER, USER_COORDINATE};

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
    TOTAL_POINTS_NUMBER.save(deps.storage, &0)?;

    Ok(resp)
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateMap { map_info } => execute_update_map(ctx, map_info),
        ExecuteMsg::StoreCoordinate {
            coordinate,
            is_timestamp_allowed,
        } => execute_store_coordinate(ctx, coordinate, is_timestamp_allowed),
        ExecuteMsg::StoreUserCoordinate {
            user_location_paths,
        } => execute_store_user_coordinate(ctx, user_location_paths),
        ExecuteMsg::DeleteUserCoordinate { user } => execute_delete_user_coordinate(ctx, user),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

pub fn execute_update_map(
    ctx: ExecuteContext,
    map_info: MapInfo,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender;

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

    let max_point_number = TOTAL_POINTS_NUMBER.load(ctx.deps.storage)?;

    for point in 1..=max_point_number {
        MAP_POINT_INFO.remove(ctx.deps.storage, &point);
    }

    TOTAL_POINTS_NUMBER.save(ctx.deps.storage, &0)?;

    Ok(Response::new().add_attributes(vec![attr("method", "update_map"), attr("sender", sender)]))
}

pub fn execute_store_coordinate(
    ctx: ExecuteContext,
    coordinate: Coordinate,
    is_timestamp_allowed: bool,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender;
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

    let x_length = SignedDecimal::from_ratio(map_size.x_width, 1);
    let y_length = SignedDecimal::from_ratio(map_size.y_width, 1);
    let z_length = map_size.z_width.map(|z| SignedDecimal::from_ratio(z, 1));

    let is_z_allowed = z_length.is_some();

    // Convert x-coordinate to map_decimal precision
    let x_coordinate = coordinate
        .x_coordinate
        .checked_mul(SignedDecimal::from_str("10")?.pow(map_decimal as u32)) // Multiply the x-coordinate by 10 ^ map_decimal
        .map_err(|_| ContractError::Overflow {})?
        .floor() // Floor the result
        .checked_div(SignedDecimal::from_str("10")?.pow(map_decimal as u32)) // Divide the result by 10 ^ map_decimal
        .map_err(|_| ContractError::Underflow {})?;

    // Convert y-coordinate to map_decimal precision
    let y_coordinate = coordinate
        .y_coordinate
        .checked_mul(SignedDecimal::from_str("10")?.pow(map_decimal as u32)) // Multiply the y-coordinate by 10 ^ map_decimal
        .map_err(|_| ContractError::Overflow {})?
        .floor() // Floor the result
        .checked_div(SignedDecimal::from_str("10")?.pow(map_decimal as u32)) // Divide the result by 10 ^ map_decimal
        .map_err(|_| ContractError::Underflow {})?;

    // Convert z-coordinate to map_decimal precision
    let z_coordinate = match coordinate.z_coordinate {
        Some(z_coordinate) => Some(
            z_coordinate
                .checked_mul(SignedDecimal::from_str("10")?.pow(map_decimal as u32)) // Multiply the z-coordinate by 10 ^ map_decimal
                .map_err(|_| ContractError::Overflow {})?
                .floor() // Floor the result
                .checked_div(SignedDecimal::from_str("10")?.pow(map_decimal as u32)) // Divide the result by 10 ^ map_decimal
                .map_err(|_| ContractError::Underflow {})?,
        ),
        None => None,
    };

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
            // x_ge = - (x_length / 2)
            let x_ge = SignedDecimal::negative_one()
                .checked_mul(
                    x_length
                        .checked_div(SignedDecimal::from_ratio(2, 1))
                        .map_err(|_| ContractError::Underflow {})?,
                )
                .map_err(|_| ContractError::Underflow {})?;

            // x_le = x_length / 2
            let x_le = x_length
                .checked_div(SignedDecimal::from_ratio(2, 1))
                .map_err(|_| ContractError::Underflow {})?;

            // Validate the x_coordinate
            // x_coordinate must be between - (x_length / 2) and x_length / 2
            ensure!(
                validate_coordinate(x_coordinate, x_ge, x_le),
                ContractError::InvalidParameter {
                    error: Some("Wrong X Coordinate Range".to_string())
                }
            );

            // y_ge = - (y_length / 2)
            let y_ge = SignedDecimal::negative_one()
                .checked_mul(
                    y_length
                        .checked_div(SignedDecimal::from_ratio(2, 1))
                        .map_err(|_| ContractError::Underflow {})?,
                )
                .map_err(|_| ContractError::Underflow {})?;
            // y_le = y_length / 2
            let y_le = y_length
                .checked_div(SignedDecimal::from_ratio(2, 1))
                .map_err(|_| ContractError::Underflow {})?;

            // Validate the y_coordinate
            // y_coordinate must be between - (y_length / 2) and y_length / 2
            ensure!(
                validate_coordinate(y_coordinate, y_ge, y_le),
                ContractError::InvalidParameter {
                    error: Some("Wrong Y Coordinate Range".to_string())
                }
            );

            if is_z_allowed {
                if let Some(z_coordinate) = z_coordinate {
                    // z_ge = - (z_length / 2)
                    let z_ge = SignedDecimal::negative_one()
                        .checked_mul(
                            z_length
                                .unwrap()
                                .checked_div(SignedDecimal::from_ratio(2, 1))
                                .map_err(|_| ContractError::Underflow {})?,
                        )
                        .map_err(|_| ContractError::Underflow {})?;
                    // z_le = z_length / 2
                    let z_le = z_length
                        .unwrap()
                        .checked_div(SignedDecimal::from_ratio(2, 1))
                        .map_err(|_| ContractError::Underflow {})?;

                    // Validate the z_coordinate
                    // z_coordinate must be between - (z_length / 2) and z_length / 2
                    ensure!(
                        validate_coordinate(z_coordinate, z_ge, z_le),
                        ContractError::InvalidParameter {
                            error: Some("Wrong Z Coordinate Range".to_string())
                        }
                    );
                }
            }
        }
        false => {
            // x_coordinate must be between 0 and x_length
            ensure!(
                validate_coordinate(x_coordinate, SignedDecimal::zero(), x_length),
                ContractError::InvalidParameter {
                    error: Some("Wrong X Coordinate Range".to_string())
                }
            );

            // y_coordinate must be between 0 and y_length
            ensure!(
                validate_coordinate(y_coordinate, SignedDecimal::zero(), y_length),
                ContractError::InvalidParameter {
                    error: Some("Wrong Y Coordinate Range".to_string())
                }
            );

            if is_z_allowed {
                if let Some(z_coordinate) = z_coordinate {
                    // z_coordinate must be between 0 and z_length
                    ensure!(
                        validate_coordinate(z_coordinate, SignedDecimal::zero(), z_length.unwrap()),
                        ContractError::InvalidParameter {
                            error: Some("Wrong Z Coordinate Range".to_string())
                        }
                    );
                }
            }
        }
    };

    let point_number = TOTAL_POINTS_NUMBER
        .load(ctx.deps.storage)?
        .checked_add(1)
        .unwrap();
    let timestamp = match is_timestamp_allowed {
        true => Some(ctx.env.block.time.nanos()),
        false => None,
    };

    MAP_POINT_INFO.save(
        ctx.deps.storage,
        &point_number,
        &(
            CoordinateInfo {
                x: x_coordinate.to_string(),
                y: y_coordinate.to_string(),
                z: z_coordinate.map(|z_coordinate| z_coordinate.to_string()),
            },
            StoredDate { timestamp },
        ),
    )?;
    TOTAL_POINTS_NUMBER.save(ctx.deps.storage, &point_number)?;

    Ok(Response::new().add_attributes(vec![
        attr("method", "store_coordinate"),
        attr("sender", sender),
    ]))
}

pub fn execute_store_user_coordinate(
    ctx: ExecuteContext,
    user_location_paths: Vec<AndrAddr>,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender;
    for user_location_path in user_location_paths {
        let address = user_location_path.get_raw_address(&ctx.deps.as_ref())?;
        let contract_info = ctx.deps.querier.query_wasm_contract_info(address.clone());
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
            if ado_type == *"point" {
                let user_point_coordinate: PointCoordinate = ctx
                    .deps
                    .querier
                    .query_wasm_smart(address.clone(), &PointQueryMsg::GetPoint {})?;
                let user_res: GetDataOwnerResponse = ctx
                    .deps
                    .querier
                    .query_wasm_smart(address.clone(), &PointQueryMsg::GetDataOwner {})?;
                let user: AndrAddr = user_res.owner;
                let user_addr = user.get_raw_address(&ctx.deps.as_ref())?;

                USER_COORDINATE.save(ctx.deps.storage, user_addr, &user_point_coordinate)?;
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
    Ok(Response::new().add_attributes(vec![
        attr("method", "store_user_coordinate"),
        attr("sender", sender),
    ]))
}

pub fn execute_delete_user_coordinate(
    ctx: ExecuteContext,
    user: AndrAddr,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender;
    let user_addr = user.get_raw_address(&ctx.deps.as_ref())?;

    USER_COORDINATE.remove(ctx.deps.storage, user_addr);

    Ok(Response::new().add_attributes(vec![
        attr("method", "delete_user_coordinate"),
        attr("sender", sender),
    ]))
}

pub fn validate_coordinate(
    coordinate: SignedDecimal,
    ge: SignedDecimal, // greater equal
    le: SignedDecimal, // less equal
) -> bool {
    coordinate.ge(&ge) && coordinate.le(&le)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetMapInfo {} => encode_binary(&get_map_info(deps.storage)?),
        QueryMsg::GetMaxPointNumber {} => encode_binary(&get_max_point_number(deps.storage)?),
        QueryMsg::GetAllPoints { start, limit } => {
            encode_binary(&get_all_points(deps.storage, start, limit)?)
        }
        QueryMsg::GetUserCoordinate { user } => encode_binary(&get_user_coordinate(deps, user)?),
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

pub fn get_max_point_number(
    storage: &dyn Storage,
) -> Result<GetMaxPointNumberResponse, ContractError> {
    let max_point_number = TOTAL_POINTS_NUMBER.load(storage)?;
    Ok(GetMaxPointNumberResponse { max_point_number })
}

pub fn get_all_points(
    storage: &dyn Storage,
    start: Option<u128>,
    limit: Option<u32>,
) -> Result<GetAllPointsResponse, ContractError> {
    let max_point_number = TOTAL_POINTS_NUMBER.load(storage)?;

    // Set default values for pagination
    let start_point = start.unwrap_or(1); // Start from 1 if no start provided
    let limit = limit.unwrap_or(100); // Default limit to 100 points

    let mut res: Vec<(CoordinateInfo, StoredDate)> = Vec::new();

    // Iterate with pagination
    for point in start_point..=max_point_number {
        if res.len() >= limit as usize {
            break; // Stop when limit is reached
        }

        // Use `may_load` to handle cases where the point may not exist
        if let Some(coordinate) = MAP_POINT_INFO.may_load(storage, &point)? {
            res.push(coordinate);
        }
    }

    Ok(GetAllPointsResponse { points: res })
}

pub fn get_user_coordinate(deps: Deps, user: AndrAddr) -> Result<CoordinateInfo, ContractError> {
    let user_addr = user.get_raw_address(&deps)?;
    let user_coordinate = USER_COORDINATE.load(deps.storage, user_addr)?;

    Ok(CoordinateInfo {
        x: user_coordinate.x_coordinate.to_string(),
        y: user_coordinate.y_coordinate.to_string(),
        z: user_coordinate.z_coordinate.map(|z| z.to_string()),
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
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
