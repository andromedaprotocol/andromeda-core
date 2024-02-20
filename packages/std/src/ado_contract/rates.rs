use crate::amp::recipient::Recipient;
use crate::error::ContractError;
use cw_storage_plus::Item;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Coin, Decimal, Fraction, QuerierWrapper, Storage};

use super::ADOContract;

#[cw_serde]
pub struct PaymentsResponse {
    pub payments: Vec<RateInfo>,
}

#[cw_serde]
pub struct RateInfo {
    pub rate: Rate,
    pub is_additive: bool,
    pub description: Option<String>,
    pub recipients: Vec<Recipient>,
}

#[cw_serde]
/// An enum used to define various types of fees
pub enum Rate {
    /// A flat rate fee
    Flat(Coin),
    /// A percentage fee
    Percent(PercentRate),
    // External(PrimitivePointer),
}

#[cw_serde]
pub struct Config {
    pub rates: Vec<RateInfo>,
}

#[cw_serde] // This is added such that both Rate::Flat and Rate::Percent have the same level of nesting which
            // makes it easier to work with on the frontend.
pub struct PercentRate {
    pub percent: Decimal,
}

impl From<Decimal> for Rate {
    fn from(decimal: Decimal) -> Self {
        Rate::Percent(PercentRate { percent: decimal })
    }
}

impl Rate {
    /// Validates that a given rate is non-zero. It is expected that the Rate is not an
    /// External Rate.
    pub fn is_non_zero(&self) -> Result<bool, ContractError> {
        match self {
            Rate::Flat(coin) => Ok(!coin.amount.is_zero()),
            Rate::Percent(PercentRate { percent }) => Ok(!percent.is_zero()),
            // Rate::External(_) => Err(ContractError::UnexpectedExternalRate {}),
        }
    }

    /// Validates `self` and returns an "unwrapped" version of itself wherein if it is an External
    /// Rate, the actual rate value is retrieved from the Primitive Contract.
    pub fn validate(&self, querier: &QuerierWrapper) -> Result<Rate, ContractError> {
        let rate = self.clone().get_rate(querier)?;
        ensure!(rate.is_non_zero()?, ContractError::InvalidRate {});

        if let Rate::Percent(PercentRate { percent }) = rate {
            ensure!(percent <= Decimal::one(), ContractError::InvalidRate {});
        }

        Ok(rate)
    }

    /// If `self` is Flat or Percent it returns itself. Otherwise it queries the primitive contract
    /// and retrieves the actual Flat or Percent rate.
    fn get_rate(self, _querier: &QuerierWrapper) -> Result<Rate, ContractError> {
        match self {
            Rate::Flat(_) => Ok(self),
            Rate::Percent(_) => Ok(self),
            // Rate::External(primitive_pointer) => {
            //     let primitive = primitive_pointer.into_value(querier)?;
            //     match primitive {
            //         None => Err(ContractError::ParsingError {
            //             err: "Stored primitive is None".to_string(),
            //         }),
            //         Some(primitive) => match primitive {
            //             Primitive::Coin(coin) => Ok(Rate::Flat(coin)),
            //             Primitive::Decimal(value) => Ok(Rate::from(value)),
            //             _ => Err(ContractError::ParsingError {
            //                 err: "Stored rate is not a coin or Decimal".to_string(),
            //             }),
            //         },
            //     }
            // }
        }
    }
}

/// An attribute struct used for any events that involve a payment
pub struct PaymentAttribute {
    /// The amount paid
    pub amount: Coin,
    /// The address the payment was made to
    pub receiver: String,
}

impl ToString for PaymentAttribute {
    fn to_string(&self) -> String {
        format!("{}<{}", self.receiver, self.amount)
    }
}

/// Calculates a fee amount given a `Rate` and payment amount.
///
/// ## Arguments
/// * `fee_rate` - The `Rate` of the fee to be paid
/// * `payment` - The amount used to calculate the fee
///
/// Returns the fee amount in a `Coin` struct.
pub fn calculate_fee(fee_rate: Rate, payment: &Coin) -> Result<Coin, ContractError> {
    match fee_rate {
        Rate::Flat(rate) => Ok(Coin::new(rate.amount.u128(), rate.denom)),
        Rate::Percent(PercentRate { percent }) => {
            // [COM-03] Make sure that fee_rate between 0 and 100.
            ensure!(
                // No need for rate >=0 due to type limits (Question: Should add or remove?)
                percent <= Decimal::one() && !percent.is_zero(),
                ContractError::InvalidRate {}
            );
            let mut fee_amount = payment.amount * percent;

            // Always round any remainder up and prioritise the fee receiver.
            // Inverse of percent will always exist.
            let reversed_fee = fee_amount * percent.inv().unwrap();
            if payment.amount > reversed_fee {
                // [COM-1] Added checked add to fee_amount rather than direct increment
                fee_amount = fee_amount.checked_add(1u128.into())?;
            }
            Ok(Coin::new(fee_amount.u128(), payment.denom.clone()))
        } // Rate::External(_) => Err(ContractError::UnexpectedExternalRate {}),
    }
}

pub fn rates<'a>() -> Item<'a, Config> {
    Item::new("rates")
}

impl<'a> ADOContract<'a> {
    /// Sets rates
    pub fn set_rates(store: &mut dyn Storage, config: Config) -> Result<(), ContractError> {
        rates().save(store, &config)?;
        Ok(())
    }
    /// Removes rates
    pub fn remove_rates(store: &mut dyn Storage) -> Result<(), ContractError> {
        rates().remove(store);
        Ok(())
    }
    // /// Determines if the provided actor is authorised to perform the given action
    // ///
    // /// Returns an error if the given action is not permissioned for the given actor
    // pub fn is_permissioned(
    //     &self,
    //     store: &mut dyn Storage,
    //     env: Env,
    //     action: impl Into<String>,
    //     actor: impl Into<String>,
    // ) -> Result<(), ContractError> {
    //     // Converted to strings for cloning
    //     let action_string: String = action.into();
    //     let actor_string: String = actor.into();

    //     if self.is_contract_owner(store, actor_string.as_str())? {
    //         return Ok(());
    //     }

    //     let permission = Self::get_permission(store, action_string.clone(), actor_string.clone())?;
    //     let permissioned_action = self
    //         .permissioned_actions
    //         .may_load(store, action_string.clone())?
    //         .unwrap_or(false);
    //     match permission {
    //         Some(mut permission) => {
    //             ensure!(
    //                 permission.is_permissioned(&env, permissioned_action),
    //                 ContractError::Unauthorized {}
    //             );

    //             // Consume a use for a limited permission
    //             if let Permission::Limited { .. } = permission {
    //                 permission.consume_use();
    //                 permissions().save(
    //                     store,
    //                     (action_string.clone() + actor_string.as_str()).as_str(),
    //                     &PermissionInfo {
    //                         action: action_string,
    //                         actor: actor_string,
    //                         permission,
    //                     },
    //                 )?;
    //             }

    //             Ok(())
    //         }
    //         None => {
    //             ensure!(!permissioned_action, ContractError::Unauthorized {});
    //             Ok(())
    //         }
    //     }
    // }

    // /// Determines if the provided actor is authorised to perform the given action
    // ///
    // /// **Ignores the `PERMISSIONED_ACTIONS` map**
    // ///
    // /// Returns an error if the permission has expired or if no permission exists for a restricted ADO
    // pub fn is_permissioned_strict(
    //     &self,
    //     store: &mut dyn Storage,
    //     env: Env,
    //     action: impl Into<String>,
    //     actor: impl Into<String>,
    // ) -> Result<(), ContractError> {
    //     // Converted to strings for cloning
    //     let action_string: String = action.into();
    //     let actor_string: String = actor.into();

    //     if self.is_contract_owner(store, actor_string.as_str())? {
    //         return Ok(());
    //     }

    //     let permission = Self::get_permission(store, action_string.clone(), actor_string.clone())?;
    //     match permission {
    //         Some(mut permission) => {
    //             ensure!(
    //                 permission.is_permissioned(&env, true),
    //                 ContractError::Unauthorized {}
    //             );

    //             // Consume a use for a limited permission
    //             if let Permission::Limited { .. } = permission {
    //                 permission.consume_use();
    //                 permissions().save(
    //                     store,
    //                     (action_string.clone() + actor_string.as_str()).as_str(),
    //                     &PermissionInfo {
    //                         action: action_string,
    //                         actor: actor_string,
    //                         permission,
    //                     },
    //                 )?;
    //             }

    //             Ok(())
    //         }
    //         None => Err(ContractError::Unauthorized {}),
    //     }
    // }

    // /// Gets the permission for the given action and actor
    // pub fn get_permission(
    //     store: &dyn Storage,
    //     action: impl Into<String>,
    //     actor: impl Into<String>,
    // ) -> Result<Option<Permission>, ContractError> {
    //     let action = action.into();
    //     let actor = actor.into();
    //     let key = action + &actor;
    //     if let Some(PermissionInfo { permission, .. }) = permissions().may_load(store, &key)? {
    //         Ok(Some(permission))
    //     } else {
    //         Ok(None)
    //     }
    // }

    // /// Sets the permission for the given action and actor
    // pub fn set_permission(
    //     store: &mut dyn Storage,
    //     action: impl Into<String>,
    //     actor: impl Into<String>,
    //     permission: Permission,
    // ) -> Result<(), ContractError> {
    //     let action = action.into();
    //     let actor = actor.into();
    //     let key = action.clone() + &actor;
    //     permissions().save(
    //         store,
    //         &key,
    //         &PermissionInfo {
    //             action,
    //             actor,
    //             permission,
    //         },
    //     )?;
    //     Ok(())
    // }

    // /// Removes the permission for the given action and actor
    // pub fn remove_permission(
    //     store: &mut dyn Storage,
    //     action: impl Into<String>,
    //     actor: impl Into<String>,
    // ) -> Result<(), ContractError> {
    //     let action = action.into();
    //     let actor = actor.into();
    //     let key = action + &actor;
    //     permissions().remove(store, &key)?;
    //     Ok(())
    // }

    // /// Execute handler for setting permission
    // ///
    // /// **Whitelisted/Limited permissions will only work for permissioned actions**
    // ///
    // /// TODO: Add permission for execute context
    // pub fn execute_set_permission(
    //     &self,
    //     ctx: ExecuteContext,
    //     actor: AndrAddr,
    //     action: impl Into<String>,
    //     permission: Permission,
    // ) -> Result<Response, ContractError> {
    //     Self::is_contract_owner(self, ctx.deps.storage, ctx.info.sender.as_str())?;
    //     let actor_addr = actor.get_raw_address(&ctx.deps.as_ref())?;
    //     let action = action.into();
    //     Self::set_permission(
    //         ctx.deps.storage,
    //         action.clone(),
    //         actor_addr.clone(),
    //         permission.clone(),
    //     )?;

    //     Ok(Response::default().add_attributes(vec![
    //         ("action", "set_permission"),
    //         ("actor", actor_addr.as_str()),
    //         ("action", action.as_str()),
    //         ("permission", permission.to_string().as_str()),
    //     ]))
    // }

    // /// Execute handler for setting permission
    // /// TODO: Add permission for execute context
    // pub fn execute_remove_permission(
    //     &self,
    //     ctx: ExecuteContext,
    //     actor: AndrAddr,
    //     action: impl Into<String>,
    // ) -> Result<Response, ContractError> {
    //     Self::is_contract_owner(self, ctx.deps.storage, ctx.info.sender.as_str())?;
    //     let actor_addr = actor.get_raw_address(&ctx.deps.as_ref())?;
    //     let action = action.into();
    //     Self::remove_permission(ctx.deps.storage, action.clone(), actor_addr.clone())?;

    //     Ok(Response::default().add_attributes(vec![
    //         ("action", "remove_permission"),
    //         ("actor", actor_addr.as_str()),
    //         ("action", action.as_str()),
    //     ]))
    // }

    // /// Enables permissioning for a given action
    // pub fn permission_action(
    //     &self,
    //     action: impl Into<String>,
    //     store: &mut dyn Storage,
    // ) -> Result<(), ContractError> {
    //     self.permissioned_actions
    //         .save(store, action.into(), &true)?;
    //     Ok(())
    // }

    // /// Disables permissioning for a given action
    // pub fn disable_action_permission(&self, action: impl Into<String>, store: &mut dyn Storage) {
    //     self.permissioned_actions.remove(store, action.into());
    // }

    // pub fn execute_permission_action(
    //     &self,
    //     ctx: ExecuteContext,
    //     action: impl Into<String>,
    // ) -> Result<Response, ContractError> {
    //     let action_string: String = action.into();
    //     Self::is_contract_owner(self, ctx.deps.storage, ctx.info.sender.as_str())?;
    //     self.permission_action(action_string.clone(), ctx.deps.storage)?;
    //     Ok(Response::default().add_attributes(vec![
    //         ("action", "permission_action"),
    //         ("action", action_string.as_str()),
    //     ]))
    // }

    // pub fn execute_disable_action_permission(
    //     &self,
    //     ctx: ExecuteContext,
    //     action: impl Into<String>,
    // ) -> Result<Response, ContractError> {
    //     let action_string: String = action.into();
    //     Self::is_contract_owner(self, ctx.deps.storage, ctx.info.sender.as_str())?;
    //     Self::disable_action_permission(self, action_string.clone(), ctx.deps.storage);
    //     Ok(Response::default().add_attributes(vec![
    //         ("action", "disable_action_permission"),
    //         ("action", action_string.as_str()),
    //     ]))
    // }

    // /// Queries all permissions for a given actor
    // pub fn query_permissions(
    //     &self,
    //     deps: Deps,
    //     actor: impl Into<String>,
    //     limit: Option<u32>,
    //     start_after: Option<String>,
    // ) -> Result<Vec<PermissionInfo>, ContractError> {
    //     let actor = actor.into();
    //     let min = start_after.map(Bound::inclusive);
    //     let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    //     let permissions = permissions()
    //         .idx
    //         .permissions
    //         .prefix(actor)
    //         .range(deps.storage, min, None, Order::Ascending)
    //         .take(limit)
    //         .map(|p| p.unwrap().1)
    //         .collect::<Vec<PermissionInfo>>();
    //     Ok(permissions)
    // }

    // pub fn query_permissioned_actions(&self, deps: Deps) -> Result<Vec<String>, ContractError> {
    //     let actions = self
    //         .permissioned_actions
    //         .keys(deps.storage, None, None, Order::Ascending)
    //         .map(|p| p.unwrap())
    //         .collect::<Vec<String>>();
    //     Ok(actions)
    // }
}

#[cfg(test)]
mod tests {

    use cosmwasm_std::{coin, Uint128};

    use super::*;

    // #[test]
    // fn test_validate_external_rate() {
    //     let deps = mock_dependencies_custom(&[]);

    //     let rate = Rate::External(PrimitivePointer {
    //         address: MOCK_PRIMITIVE_CONTRACT.to_owned(),

    //         key: Some("percent".to_string()),
    //     });
    //     let validated_rate = rate.validate(&deps.as_ref().querier).unwrap();
    //     let expected_rate = Rate::from(Decimal::percent(1));
    //     assert_eq!(expected_rate, validated_rate);

    //     let rate = Rate::External(PrimitivePointer {
    //         address: MOCK_PRIMITIVE_CONTRACT.to_owned(),
    //         key: Some("flat".to_string()),
    //     });
    //     let validated_rate = rate.validate(&deps.as_ref().querier).unwrap();
    //     let expected_rate = Rate::Flat(coin(1u128, "uusd"));
    //     assert_eq!(expected_rate, validated_rate);
    // }

    #[test]
    fn test_calculate_fee() {
        let payment = coin(101, "uluna");
        let expected = Ok(coin(5, "uluna"));
        let fee = Rate::from(Decimal::percent(4));

        let received = calculate_fee(fee, &payment);

        assert_eq!(expected, received);

        assert_eq!(expected, received);

        let payment = coin(125, "uluna");
        let fee = Rate::Flat(Coin {
            amount: Uint128::from(5_u128),
            denom: "uluna".to_string(),
        });

        let received = calculate_fee(fee, &payment);

        assert_eq!(expected, received);
    }
}
