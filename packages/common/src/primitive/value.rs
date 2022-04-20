use crate::{
    ado_base::query_get,
    error::ContractError,
    mission::AndrAddress,
    primitive::{GetValueResponse, Primitive},
};
use cosmwasm_std::{to_binary, Addr, Api, Coin, Decimal, QuerierWrapper, StdError, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Value<T>
where
    // This restriction is to ensure that `T` is a type that can be stored as a `Primitive`. It
    // could work without, but we could easily get cases where `T` cannot be a `Primitive` and this
    // gives us compile-time insurance.
    T: Into<Primitive>,
{
    /// The raw value.
    Raw(T),
    /// The pointer to the primitive. This SHOULD be of the same underlying type as `T`. For
    /// example, if `T` is `String`, then `PrimitivePointer` should point to a Primitive::String(..).
    /// This cannot be enforced at compile time though, so it is up to the discretion of the user.
    Pointer(PrimitivePointer),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PrimitivePointer {
    /// The address of the primitive contract.
    pub address: AndrAddress,
    /// The optional key for the stored data.
    pub key: Option<String>,
}

impl PrimitivePointer {
    /// Queries the related primitive contract instance to get the stored `Primitive`. If it does
    /// not exist, `Ok(None)` is returned so it is on the receiver to handle the case of a missing
    /// primitive.
    pub fn into_value(
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

impl<T: Into<Primitive>> Value<T> {
    /// Consumes the instance to return the underlying value. If it is a pointer, it queries the
    /// primitive contract and attempts to get the underlying type according to the value of `T`.
    ///
    /// The `func` parameter is a function that retrieves the "inner" value of the primitive of
    /// type `T`.
    fn try_into_value(
        self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        mission_address: Option<Addr>,
        func: fn(Primitive) -> Result<T, StdError>,
    ) -> Result<Option<T>, ContractError> {
        match self {
            Value::Raw(value) => Ok(Some(value)),
            Value::Pointer(pointer) => {
                let primitive = pointer.into_value(api, querier, mission_address)?;
                if let Some(primitive) = primitive {
                    Ok(Some(func(primitive)?))
                } else {
                    Ok(None)
                }
            }
        }
    }
}

impl Value<String> {
    pub fn try_into_string(
        self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        mission_address: Option<Addr>,
    ) -> Result<Option<String>, ContractError> {
        self.try_into_value(api, querier, mission_address, |p| p.try_get_string())
    }
}

impl Value<Uint128> {
    pub fn try_into_uint128(
        self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        mission_address: Option<Addr>,
    ) -> Result<Option<Uint128>, ContractError> {
        self.try_into_value(api, querier, mission_address, |p| p.try_get_uint128())
    }
}

impl Value<Decimal> {
    pub fn try_into_decimal(
        self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        mission_address: Option<Addr>,
    ) -> Result<Option<Decimal>, ContractError> {
        self.try_into_value(api, querier, mission_address, |p| p.try_get_decimal())
    }
}

impl Value<Coin> {
    pub fn try_into_coin(
        self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        mission_address: Option<Addr>,
    ) -> Result<Option<Coin>, ContractError> {
        self.try_into_value(api, querier, mission_address, |p| p.try_get_coin())
    }
}

impl Value<bool> {
    pub fn try_into_bool(
        self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        mission_address: Option<Addr>,
    ) -> Result<Option<bool>, ContractError> {
        self.try_into_value(api, querier, mission_address, |p| p.try_get_bool())
    }
}

impl Value<Vec<Primitive>> {
    pub fn try_into_vec(
        self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        mission_address: Option<Addr>,
    ) -> Result<Option<Vec<Primitive>>, ContractError> {
        self.try_into_value(api, querier, mission_address, |p| p.try_get_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::mock_querier::{mock_dependencies_custom, MOCK_PRIMITIVE_CONTRACT};

    #[test]
    fn test_primitive_pointer_into_value() {
        let deps = mock_dependencies_custom(&[]);

        let pointer = PrimitivePointer {
            address: AndrAddress {
                identifier: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            },
            key: None,
        };

        let res = pointer
            .into_value(deps.as_ref().api, &deps.as_ref().querier, None)
            .unwrap();

        assert_eq!(Some(Primitive::Decimal(Decimal::zero())), res);
    }

    #[test]
    fn test_primitive_pointer_into_value_non_existing() {
        let deps = mock_dependencies_custom(&[]);

        let pointer = PrimitivePointer {
            address: AndrAddress {
                identifier: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            },
            key: Some("non_existing_key".to_string()),
        };

        let res = pointer
            .into_value(deps.as_ref().api, &deps.as_ref().querier, None)
            .unwrap();

        assert_eq!(None, res);
    }

    #[test]
    fn test_value_into_string() {
        let deps = mock_dependencies_custom(&[]);
        let value = Value::Raw("Value".to_string());
        assert_eq!(
            Some("Value".to_string()),
            value
                .try_into_string(deps.as_ref().api, &deps.as_ref().querier, None)
                .unwrap()
        );

        let value = Value::Pointer(PrimitivePointer {
            address: AndrAddress {
                identifier: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            },
            key: Some("String".to_string()),
        });
        assert_eq!(
            Some("Value".to_string()),
            value
                .try_into_string(deps.as_ref().api, &deps.as_ref().querier, None)
                .unwrap()
        );
    }

    #[test]
    fn test_value_into_uint128() {
        let deps = mock_dependencies_custom(&[]);
        let value = Value::Raw(Uint128::new(10));
        assert_eq!(
            Some(Uint128::new(10)),
            value
                .try_into_uint128(deps.as_ref().api, &deps.as_ref().querier, None)
                .unwrap()
        );

        let value = Value::Pointer(PrimitivePointer {
            address: AndrAddress {
                identifier: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            },
            key: Some("Uint128".to_string()),
        });
        assert_eq!(
            Some(Uint128::new(10)),
            value
                .try_into_uint128(deps.as_ref().api, &deps.as_ref().querier, None)
                .unwrap()
        );
    }

    #[test]
    fn test_value_into_decimal() {
        let deps = mock_dependencies_custom(&[]);
        let value = Value::Raw(Decimal::percent(1));
        assert_eq!(
            Some(Decimal::percent(1)),
            value
                .try_into_decimal(deps.as_ref().api, &deps.as_ref().querier, None)
                .unwrap()
        );

        let value = Value::Pointer(PrimitivePointer {
            address: AndrAddress {
                identifier: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            },
            key: Some("Decimal".to_string()),
        });
        assert_eq!(
            Some(Decimal::percent(1)),
            value
                .try_into_decimal(deps.as_ref().api, &deps.as_ref().querier, None)
                .unwrap()
        );
    }

    #[test]
    fn test_value_into_coin() {
        let deps = mock_dependencies_custom(&[]);
        let value = Value::Raw(Coin::new(100, "uusd"));
        assert_eq!(
            Some(Coin::new(100, "uusd")),
            value
                .try_into_coin(deps.as_ref().api, &deps.as_ref().querier, None)
                .unwrap()
        );

        let value = Value::Pointer(PrimitivePointer {
            address: AndrAddress {
                identifier: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            },
            key: Some("Coin".to_string()),
        });
        assert_eq!(
            Some(Coin::new(100, "uusd")),
            value
                .try_into_coin(deps.as_ref().api, &deps.as_ref().querier, None)
                .unwrap()
        );
    }

    #[test]
    fn test_value_into_bool() {
        let deps = mock_dependencies_custom(&[]);
        let value = Value::Raw(true);
        assert_eq!(
            Some(true),
            value
                .try_into_bool(deps.as_ref().api, &deps.as_ref().querier, None)
                .unwrap()
        );

        let value = Value::Pointer(PrimitivePointer {
            address: AndrAddress {
                identifier: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            },
            key: Some("Bool".to_string()),
        });
        assert_eq!(
            Some(true),
            value
                .try_into_bool(deps.as_ref().api, &deps.as_ref().querier, None)
                .unwrap()
        );
    }

    #[test]
    fn test_value_into_vec() {
        let deps = mock_dependencies_custom(&[]);
        let vec = vec![Primitive::from("String".to_string())];
        let value = Value::Raw(vec.clone());
        assert_eq!(
            Some(vec.clone()),
            value
                .try_into_vec(deps.as_ref().api, &deps.as_ref().querier, None)
                .unwrap()
        );

        let value = Value::Pointer(PrimitivePointer {
            address: AndrAddress {
                identifier: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            },
            key: Some("Vec".to_string()),
        });
        assert_eq!(
            Some(vec),
            value
                .try_into_vec(deps.as_ref().api, &deps.as_ref().querier, None)
                .unwrap()
        );
    }
}
