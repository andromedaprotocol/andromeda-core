use andromeda_std::{amp::AndrAddr, andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Coin, Decimal, StdError, Uint128};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub restriction: PrimitiveRestriction,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// If key is not specified the default key will be used.
    SetValue {
        key: Option<String>,
        value: Primitive,
    },
    /// If key is not specified the default key will be used.
    DeleteValue {
        key: Option<String>,
    },
    UpdateRestriction {
        restriction: PrimitiveRestriction,
    },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetValueResponse)]
    GetValue { key: Option<String> },
    #[returns(GetTypeResponse)]
    GetType { key: Option<String> },
    #[returns(Vec<String>)]
    AllKeys {},
    #[returns(Vec<String>)]
    OwnerKeys { owner: AndrAddr },
}

#[cw_serde]
pub enum Primitive {
    Uint128(Uint128),
    Decimal(Decimal),
    Coin(Coin),
    Addr(Addr),
    String(String),
    Bool(bool),
    Binary(Binary),
}

impl Primitive {
    pub fn from_string(&self) -> String {
        match self {
            Primitive::Uint128(_) => "Uint128".to_string(),
            Primitive::Decimal(_) => "Decimal".to_string(),
            Primitive::Coin(_) => "Coin".to_string(),
            Primitive::Addr(_) => "Addr".to_string(),
            Primitive::String(_) => "String".to_string(),
            Primitive::Bool(_) => "Bool".to_string(),
            Primitive::Binary(_) => "Binary".to_string(),
        }
    }
}

#[cw_serde]
pub enum PrimitiveRestriction {
    Private,
    Public,
    Restricted,
}

fn parse_error(type_name: String) -> StdError {
    StdError::ParseErr {
        target_type: type_name.clone(),
        msg: format!("Primitive is not a {type_name}"),
    }
}

impl From<String> for Primitive {
    fn from(value: String) -> Self {
        Primitive::String(value)
    }
}

impl From<Uint128> for Primitive {
    fn from(value: Uint128) -> Self {
        Primitive::Uint128(value)
    }
}

impl From<Decimal> for Primitive {
    fn from(value: Decimal) -> Self {
        Primitive::Decimal(value)
    }
}

impl From<bool> for Primitive {
    fn from(value: bool) -> Self {
        Primitive::Bool(value)
    }
}

impl From<Coin> for Primitive {
    fn from(value: Coin) -> Self {
        Primitive::Coin(value)
    }
}

impl From<Addr> for Primitive {
    fn from(value: Addr) -> Self {
        Primitive::Addr(value)
    }
}

impl From<Binary> for Primitive {
    fn from(value: Binary) -> Self {
        Primitive::Binary(value)
    }
}

// These are methods to help the calling user quickly retreive the data in the Primitive as they
// often already know what the type should be.
impl Primitive {
    pub fn try_get_uint128(&self) -> Result<Uint128, StdError> {
        match self {
            Primitive::Uint128(value) => Ok(*value),
            _ => Err(parse_error(String::from("Uint128"))),
        }
    }

    pub fn try_get_decimal(&self) -> Result<Decimal, StdError> {
        match self {
            Primitive::Decimal(value) => Ok(*value),
            _ => Err(parse_error(String::from("Decimal"))),
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

    pub fn try_get_coin(&self) -> Result<Coin, StdError> {
        match self {
            Primitive::Coin(coin) => Ok(coin.clone()),
            _ => Err(parse_error(String::from("Coin"))),
        }
    }

    pub fn try_get_addr(&self) -> Result<Addr, StdError> {
        match self {
            Primitive::Addr(addr) => Ok(addr.clone()),
            _ => Err(parse_error(String::from("Addr"))),
        }
    }

    pub fn try_get_binary(&self) -> Result<Binary, StdError> {
        match self {
            Primitive::Binary(value) => Ok(value.clone()),
            _ => Err(parse_error(String::from("Binary"))),
        }
    }
}

#[cw_serde]
pub struct GetValueResponse {
    pub key: String,
    pub value: Primitive,
}

#[cw_serde]
pub struct GetTypeResponse {
    pub value_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::to_json_binary;

    #[test]
    fn test_from_string() {
        let cases = vec![
            (
                Primitive::Uint128(Uint128::from(5_u128)),
                "Uint128".to_string(),
            ),
            (
                Primitive::Decimal(Decimal::new(Uint128::one())),
                "Decimal".to_string(),
            ),
            (
                Primitive::Coin(Coin {
                    amount: Uint128::new(100),
                    denom: "uatom".to_string(),
                }),
                "Coin".to_string(),
            ),
            (
                Primitive::Addr(Addr::unchecked("cosmos1...v937")),
                "Addr".to_string(),
            ),
            (
                Primitive::String("Some string".to_string()),
                "String".to_string(),
            ),
            (Primitive::Bool(true), "Bool".to_string()),
            (
                Primitive::Binary(to_json_binary(&"data").unwrap()),
                "Binary".to_string(),
            ),
        ];

        for (value, expected_str) in cases.iter() {
            assert_eq!(&value.from_string(), expected_str);
        }
    }

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
    fn try_get_decimal() {
        let primitive = Primitive::Decimal(Decimal::zero());
        assert_eq!(Decimal::zero(), primitive.try_get_decimal().unwrap());

        let primitive = Primitive::String("String".to_string());
        assert_eq!(
            parse_error("Decimal".to_string()),
            primitive.try_get_decimal().unwrap_err()
        );
    }

    #[test]
    fn try_get_binary() {
        let primitive = Primitive::Binary(to_json_binary("data").unwrap());
        assert_eq!(
            to_json_binary("data").unwrap(),
            primitive.try_get_binary().unwrap()
        );

        let primitive = Primitive::String("String".to_string());
        assert_eq!(
            parse_error("Binary".to_string()),
            primitive.try_get_binary().unwrap_err()
        );
    }
}
