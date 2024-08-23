use andromeda_std::{amp::AndrAddr, andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::StdError;
use std::fmt::{Display, Formatter, Result as FMTResult};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub restriction: BooleanRestriction,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    SetValue { value: Boolean },
    DeleteValue {},
    UpdateRestriction { restriction: BooleanRestriction },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetValueResponse)]
    GetValue {},
    #[returns(GetDataOwnerResponse)]
    GetDataOwner {},
}

#[cw_serde]
pub enum BooleanRestriction {
    Private,
    Public,
    Restricted,
}

#[cw_serde]
pub struct Boolean(pub bool);

impl Boolean {
    #[inline]
    pub fn into_bool(self) -> bool {
        self.0
    }

    #[inline]
    pub fn from_bool(value: impl Into<bool>) -> Boolean {
        Boolean(value.into())
    }
}

impl From<Boolean> for bool {
    fn from(value: Boolean) -> Self {
        value.0
    }
}

impl From<&Boolean> for bool {
    fn from(value: &Boolean) -> Self {
        value.0.clone()
    }
}

impl PartialEq<Boolean> for bool {
    fn eq(&self, rhs: &Boolean) -> bool {
        *self == rhs.0
    }
}

impl Display for Boolean {
    fn fmt(&self, f: &mut Formatter) -> FMTResult {
        write!(f, "{}", &self.0)
    }
}

impl Boolean {
    pub fn try_get_value(&self) -> Result<bool, StdError> {
        Ok(self.0)
    }
}

#[cw_serde]
pub struct GetValueResponse {
    pub value: bool,
}

#[cw_serde]
pub struct GetDataOwnerResponse {
    pub owner: AndrAddr,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_bool() {
        let cases = vec![(Boolean::from_bool(true), Boolean(true))];

        for (value, expected) in cases.iter() {
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn try_get_value() {
        let boolean = Boolean::from_bool(true);
        assert_eq!(true, boolean.try_get_value().unwrap());
    }
}
