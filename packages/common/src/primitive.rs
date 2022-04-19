use crate::{ado_base::query_get, error::ContractError, mission::AndrAddress};
use cosmwasm_std::{to_binary, Addr, Api, Coin, Decimal, QuerierWrapper, StdError, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub enum Value<T> {
    Raw(T),
    Pointer(PrimitivePointer),
}

pub struct PrimitivePointer {
    pub address: AndrAddress,
    pub key: Option<String>,
}

impl PrimitivePointer {
    pub(crate) fn get_value(
        self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        mission_address: Option<Addr>,
    ) -> Result<Option<Primitive>, ContractError> {
        let primitive_address = self.address.get_address(api, querier, mission_address)?;
        let key = self
            .key
            .map(|k| to_binary(&k))
            // Flip Option<Result> to Result<Option>
            .map_or(Ok(None), |v| v.map(Some));

        let res: Result<GetValueResponse, ContractError> =
            query_get(key?, primitive_address, querier);

        match res {
            Err(_) => Ok(None),
            Ok(res) => Ok(Some(res.value)),
        }
    }
}

impl<T> Value<T> {
    pub(crate) fn get_value(
        self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        mission_address: Option<Addr>,
        func: fn(Primitive) -> Result<T, StdError>,
    ) -> Result<Option<T>, ContractError> {
        match self {
            Value::Raw(value) => Ok(Some(value)),
            Value::Pointer(pointer) => {
                let primitive = pointer.get_value(api, querier, mission_address)?;
                if let Some(primitive) = primitive {
                    Ok(Some(func(primitive)?))
                } else {
                    Ok(None)
                }
            }
        }
    }
}

// Idea: impl a 'get' funciton for each one that calls the requisite try_get_blah function. This
// would abstract needing to deal with those functions and the primitive pointer.
impl Value<String> {
    pub fn get(
        self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        mission_address: Option<Addr>,
    ) -> Result<Option<String>, ContractError> {
        self.get_value(api, querier, mission_address, |p| p.try_get_string())
    }
}

impl Value<Uint128> {
    pub fn get(
        self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        mission_address: Option<Addr>,
    ) -> Result<Option<Uint128>, ContractError> {
        self.get_value(api, querier, mission_address, |p| p.try_get_uint128())
    }
}

impl Value<Coin> {
    pub fn get(
        self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        mission_address: Option<Addr>,
    ) -> Result<Option<Coin>, ContractError> {
        self.get_value(api, querier, mission_address, |p| p.try_get_coin())
    }
}

impl Value<Vec<Primitive>> {
    pub fn get(
        self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        mission_address: Option<Addr>,
    ) -> Result<Option<Vec<Primitive>>, ContractError> {
        self.get_value(api, querier, mission_address, |p| p.try_get_vec())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Primitive {
    Uint128(Uint128),
    Decimal(Decimal),
    Coin(Coin),
    String(String),
    Bool(bool),
    Vec(Vec<Primitive>),
}

fn parse_error(type_name: String) -> StdError {
    StdError::ParseErr {
        target_type: type_name.clone(),
        msg: format!("Primitive is not a {}", type_name),
    }
}

// These are methods to help the calling user quickly retreive the data in the Primitive as they
// often already know what the type should be.
impl Primitive {
    pub fn is_invalid(&self) -> bool {
        match self {
            // Avoid infinite recursion problem by not allowing nested vectors.
            Primitive::Vec(vector) => vector
                .iter()
                .any(|p| matches!(p, Primitive::Vec(_)) || p.is_invalid()),
            _ => false,
        }
    }

    pub fn try_get_uint128(&self) -> Result<Uint128, StdError> {
        match self {
            Primitive::Uint128(value) => Ok(*value),
            _ => Err(parse_error(String::from("Uint128"))),
        }
    }

    pub fn try_get_string(&self) -> Result<String, StdError> {
        match self {
            Primitive::String(value) => Ok(value.to_string()),
            _ => Err(parse_error(String::from("String"))),
        }
    }

    pub fn try_get_bool(&self) -> Result<bool, StdError> {
        match self {
            Primitive::Bool(value) => Ok(*value),
            _ => Err(parse_error(String::from("bool"))),
        }
    }

    pub fn try_get_vec(&self) -> Result<Vec<Primitive>, StdError> {
        match self {
            Primitive::Vec(vector) => Ok(vector.to_vec()),
            _ => Err(parse_error(String::from("Vec"))),
        }
    }

    pub fn try_get_coin(&self) -> Result<Coin, StdError> {
        match self {
            Primitive::Coin(coin) => Ok(coin.clone()),
            _ => Err(parse_error(String::from("Coin"))),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetValueResponse {
    pub key: String,
    pub value: Primitive,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error() {
        assert_eq!(
            StdError::ParseErr {
                target_type: "target_type".to_string(),
                msg: "Primitive is not a target_type".to_string()
            },
            parse_error("target_type".to_string())
        );
    }

    #[test]
    fn try_get_uint128() {
        let primitive = Primitive::Uint128(Uint128::from(5_u128));
        assert_eq!(Uint128::from(5_u128), primitive.try_get_uint128().unwrap());

        let primitive = Primitive::Bool(true);
        assert_eq!(
            parse_error("Uint128".to_string()),
            primitive.try_get_uint128().unwrap_err()
        );
    }

    #[test]
    fn try_get_string() {
        let primitive = Primitive::String("String".to_string());
        assert_eq!("String".to_string(), primitive.try_get_string().unwrap());

        let primitive = Primitive::Bool(true);
        assert_eq!(
            parse_error("String".to_string()),
            primitive.try_get_string().unwrap_err()
        );
    }

    #[test]
    fn try_get_bool() {
        let primitive = Primitive::Bool(true);
        assert!(primitive.try_get_bool().unwrap());

        let primitive = Primitive::String("String".to_string());
        assert_eq!(
            parse_error("bool".to_string()),
            primitive.try_get_bool().unwrap_err()
        );
    }

    #[test]
    fn try_get_vec() {
        let primitive = Primitive::Vec(vec![Primitive::Bool(true)]);
        assert_eq!(
            vec![Primitive::Bool(true)],
            primitive.try_get_vec().unwrap()
        );

        let primitive = Primitive::String("String".to_string());
        assert_eq!(
            parse_error("Vec".to_string()),
            primitive.try_get_vec().unwrap_err()
        );
    }

    #[test]
    fn is_valid() {
        let valid_primitive = Primitive::Bool(true);
        assert!(!valid_primitive.is_invalid());

        let valid_primitive = Primitive::Uint128(Uint128::new(1_u128));
        assert!(!valid_primitive.is_invalid());

        let valid_primitive = Primitive::String("String".to_string());
        assert!(!valid_primitive.is_invalid());

        let valid_primitive = Primitive::Vec(vec![
            Primitive::Bool(true),
            Primitive::Uint128(Uint128::new(1_u128)),
            Primitive::String("String".to_string()),
        ]);
        assert!(!valid_primitive.is_invalid());

        let invalid_primitive = Primitive::Vec(vec![Primitive::Bool(true), Primitive::Vec(vec![])]);
        assert!(invalid_primitive.is_invalid());
    }
}
