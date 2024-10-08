use andromeda_std::{amp::AndrAddr, andr_exec, andr_instantiate, andr_query, error::ContractError};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub restriction: PointRestriction,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    SetPoint { point: PointCoordinate },
    DeletePoint {},
    UpdateRestriction { restriction: PointRestriction },
}

#[cw_serde]
pub enum PointRestriction {
    Private,
    Public,
    Restricted,
}

#[cw_serde]
pub struct PointCoordinate {
    pub x_coordinate: String,
    pub y_coordinate: String,
    pub z_coordinate: Option<String>,
}

impl PointCoordinate {
    pub fn from_f64(x_coordinate: f64, y_coordinate: f64, z_coordinate: Option<f64>) -> Self {
        let z_coordinate: Option<String> = z_coordinate.map(|z| z.to_string());

        Self {
            x_coordinate: x_coordinate.to_string(),
            y_coordinate: y_coordinate.to_string(),
            z_coordinate,
        }
    }
    pub fn validate(&self) -> Result<(), ContractError> {
        let x_coordinate = self.x_coordinate.parse::<f64>();
        if x_coordinate.is_err() {
            return Err(ContractError::ParsingError {
                err: "x_coordinate: can not parse to f64".to_string(),
            });
        }

        let y_coordinate = self.y_coordinate.parse::<f64>();
        if y_coordinate.is_err() {
            return Err(ContractError::ParsingError {
                err: "y_coordinate: can not parse to f64".to_string(),
            });
        }

        match &self.z_coordinate {
            None => (),
            Some(z) => {
                let z_coordinate = z.parse::<f64>();
                if z_coordinate.is_err() {
                    return Err(ContractError::ParsingError {
                        err: "z_coordinate: can not parse to f64".to_string(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(PointCoordinate)]
    GetPoint {},
    #[returns(GetDataOwnerResponse)]
    GetDataOwner {},
}

#[cw_serde]
pub struct GetDataOwnerResponse {
    pub owner: AndrAddr,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_point_coordinate_valid() {
        let point = PointCoordinate {
            x_coordinate: "10".to_string(),
            y_coordinate: "10".to_string(),
            z_coordinate: Some("10".to_string()),
        };
        let res = point.validate();
        assert!(res.is_ok());

        let point = PointCoordinate {
            x_coordinate: "10".to_string(),
            y_coordinate: "10".to_string(),
            z_coordinate: None,
        };
        let res = point.validate();
        assert!(res.is_ok());
    }

    #[test]
    fn test_validate_point_coordinate_invalid() {
        let point = PointCoordinate {
            x_coordinate: "10.abc".to_string(),
            y_coordinate: "10".to_string(),
            z_coordinate: Some("10".to_string()),
        };
        let res = point.validate().unwrap_err();
        assert_eq!(
            res,
            ContractError::ParsingError {
                err: "x_coordinate: can not parse to f64".to_string()
            }
        );

        let point = PointCoordinate {
            x_coordinate: "10".to_string(),
            y_coordinate: "10.abc".to_string(),
            z_coordinate: Some("10".to_string()),
        };
        let res = point.validate().unwrap_err();
        assert_eq!(
            res,
            ContractError::ParsingError {
                err: "y_coordinate: can not parse to f64".to_string()
            }
        );

        let point = PointCoordinate {
            x_coordinate: "10".to_string(),
            y_coordinate: "10".to_string(),
            z_coordinate: Some("10.xyz".to_string()),
        };
        let res = point.validate().unwrap_err();
        assert_eq!(
            res,
            ContractError::ParsingError {
                err: "z_coordinate: can not parse to f64".to_string()
            }
        );
    }
}
