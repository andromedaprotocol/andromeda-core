use crate::{
    ado_base::primitive::{GetValueResponse, Primitive},
    ado_base::query_get,
    error::ContractError,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Coin, Decimal, QuerierWrapper, StdError, Uint128};

#[cw_serde]
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

#[cw_serde]
pub struct PrimitivePointer {
    /// The address of the primitive contract.
    pub address: String,
    /// The optional key for the stored data.
    pub key: Option<String>,
}

impl PrimitivePointer {
    /// Queries the related primitive contract instance to get the stored `Primitive`. If it does
    /// not exist, `Ok(None)` is returned so it is on the receiver to handle the case of a missing
    /// primitive.
    pub fn into_value(self, querier: &QuerierWrapper) -> Result<Option<Primitive>, ContractError> {
        let primitive_address = self.address;
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
        querier: &QuerierWrapper,
        func: fn(Primitive) -> Result<T, StdError>,
    ) -> Result<Option<T>, ContractError> {
        match self {
            Value::Raw(value) => Ok(Some(value)),
            Value::Pointer(pointer) => {
                let primitive = pointer.into_value(querier)?;
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
        querier: &QuerierWrapper,
    ) -> Result<Option<String>, ContractError> {
        self.try_into_value(querier, |p| p.try_get_string())
    }
}

impl Value<Uint128> {
    pub fn try_into_uint128(
        self,
        querier: &QuerierWrapper,
    ) -> Result<Option<Uint128>, ContractError> {
        self.try_into_value(querier, |p| p.try_get_uint128())
    }
}

impl Value<Decimal> {
    pub fn try_into_decimal(
        self,
        querier: &QuerierWrapper,
    ) -> Result<Option<Decimal>, ContractError> {
        self.try_into_value(querier, |p| p.try_get_decimal())
    }
}

impl Value<Coin> {
    pub fn try_into_coin(self, querier: &QuerierWrapper) -> Result<Option<Coin>, ContractError> {
        self.try_into_value(querier, |p| p.try_get_coin())
    }
}

impl Value<bool> {
    pub fn try_into_bool(self, querier: &QuerierWrapper) -> Result<Option<bool>, ContractError> {
        self.try_into_value(querier, |p| p.try_get_bool())
    }
}

impl Value<Vec<Primitive>> {
    pub fn try_into_vec(
        self,
        querier: &QuerierWrapper,
    ) -> Result<Option<Vec<Primitive>>, ContractError> {
        self.try_into_value(querier, |p| p.try_get_vec())
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
            address: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            key: None,
        };

        let res = pointer.into_value(&deps.as_ref().querier).unwrap();
        assert_eq!(Some(Primitive::Decimal(Decimal::zero())), res);
    }

    #[test]
    fn test_primitive_pointer_into_value_with_key() {
        let deps = mock_dependencies_custom(&[]);

        let pointer = PrimitivePointer {
            address: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            key: Some("String".to_string()),
        };

        let res = pointer.into_value(&deps.as_ref().querier).unwrap();

        assert_eq!(Some(Primitive::String("Value".to_string())), res);
    }

    #[test]
    fn test_primitive_pointer_into_value_non_existing() {
        let deps = mock_dependencies_custom(&[]);

        let pointer = PrimitivePointer {
            address: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            key: Some("non_existing_key".to_string()),
        };

        let res = pointer.into_value(&deps.as_ref().querier).unwrap();

        assert_eq!(None, res);
    }

    #[test]
    fn test_value_into_string() {
        let deps = mock_dependencies_custom(&[]);
        let value = Value::Raw("Value".to_string());
        assert_eq!(
            Some("Value".to_string()),
            value.try_into_string(&deps.as_ref().querier).unwrap()
        );

        let value = Value::Pointer(PrimitivePointer {
            address: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            key: Some("String".to_string()),
        });
        assert_eq!(
            Some("Value".to_string()),
            value.try_into_string(&deps.as_ref().querier,).unwrap()
        );
    }

    #[test]
    fn test_value_into_uint128() {
        let deps = mock_dependencies_custom(&[]);
        let value = Value::Raw(Uint128::new(10));
        assert_eq!(
            Some(Uint128::new(10)),
            value.try_into_uint128(&deps.as_ref().querier,).unwrap()
        );

        let value = Value::Pointer(PrimitivePointer {
            address: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            key: Some("Uint128".to_string()),
        });
        assert_eq!(
            Some(Uint128::new(10)),
            value.try_into_uint128(&deps.as_ref().querier,).unwrap()
        );
    }

    #[test]
    fn test_value_into_decimal() {
        let deps = mock_dependencies_custom(&[]);
        let value = Value::Raw(Decimal::percent(1));
        assert_eq!(
            Some(Decimal::percent(1)),
            value.try_into_decimal(&deps.as_ref().querier,).unwrap()
        );

        let value = Value::Pointer(PrimitivePointer {
            address: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            key: Some("Decimal".to_string()),
        });
        assert_eq!(
            Some(Decimal::percent(1)),
            value.try_into_decimal(&deps.as_ref().querier,).unwrap()
        );
    }

    #[test]
    fn test_value_into_coin() {
        let deps = mock_dependencies_custom(&[]);
        let value = Value::Raw(Coin::new(100, "uusd"));
        assert_eq!(
            Some(Coin::new(100, "uusd")),
            value.try_into_coin(&deps.as_ref().querier,).unwrap()
        );

        let value = Value::Pointer(PrimitivePointer {
            address: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            key: Some("Coin".to_string()),
        });
        assert_eq!(
            Some(Coin::new(100, "uusd")),
            value.try_into_coin(&deps.as_ref().querier,).unwrap()
        );
    }

    #[test]
    fn test_value_into_bool() {
        let deps = mock_dependencies_custom(&[]);
        let value = Value::Raw(true);
        assert_eq!(
            Some(true),
            value.try_into_bool(&deps.as_ref().querier,).unwrap()
        );

        let value = Value::Pointer(PrimitivePointer {
            address: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            key: Some("Bool".to_string()),
        });
        assert_eq!(
            Some(true),
            value.try_into_bool(&deps.as_ref().querier,).unwrap()
        );
    }

    #[test]
    fn test_value_into_vec() {
        let deps = mock_dependencies_custom(&[]);
        let vec = vec![Primitive::from("String".to_string())];
        let value = Value::Raw(vec.clone());
        assert_eq!(
            Some(vec.clone()),
            value.try_into_vec(&deps.as_ref().querier,).unwrap()
        );

        let value = Value::Pointer(PrimitivePointer {
            address: MOCK_PRIMITIVE_CONTRACT.to_owned(),
            key: Some("Vec".to_string()),
        });
        assert_eq!(
            Some(vec),
            value.try_into_vec(&deps.as_ref().querier,).unwrap()
        );
    }
}
