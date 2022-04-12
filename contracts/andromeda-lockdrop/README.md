# Lockdrop

The lockdrop contract allows users to lock their UST for selected duration against which they are given MARS tokens pro-rata to their wighted share to the total UST deposited in the contract.

Upon expiration of the deposit window, all the locked UST is deposited in the Red Bank and users are allowed to claim their MARS allocations.

UST deposited in the Red Bank keeps accruing XMARS tokens which are claimable by the users.

Upon expiration of the lockup, users can withdraw their deposits as interest bearing maUST tokens, redeemable against UST via the Red Bank.

Note - Users can open muliple lockup positions with different lockup periods with the lockdrop contract

## Contract Design

### Handle Messages

| Message                           | Description                                                                                                                                             |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ExecuteMsg::UpdateConfig`        | Can only be called by the admin. Facilitates updating configuration parameters, for eq. red bank address, ma_ust address, lockup durations among others |
| `ExecuteMsg::DepositUst`          | Increases user's deposited UST balance in the lockup position for the selected duration. Can only be called when deposit window is open                 |
| `ExecuteMsg::WithdrawUst`         | Decreases user's deposited UST balance in the lockup position for the selected duration. Can only be called when withdrawal window is open              |
| `ExecuteMsg::DepositUstInRedBank` | Admin function to deposit net total locked UST into the Red Bank. Called after the deposit window is over.                                              |
| `ExecuteMsg::ClaimRewards`        | Facilitates xMARS reward claim which accrue per block. Claim lockdrop reward (MARS) in-addition to xMars when called for the first time by the user     |
| `ExecuteMsg::Unlock`              | Unlocks the selected lockup position and transfers maUST along with accrued rewards (xMars) back to the user                                            |

### Handle Messages :: Callback

| Message                                    | Description                                                                                                                                                                        |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `CallbackMsg::UpdateStateOnRedBankDeposit` | Callback function called by `DepositUstInRedBank` to update contract state after UST is deposited into the Red Bank                                                                |
| `CallbackMsg::UpdateStateOnClaim`          | Callback function called by `ClaimRewards` and `Unlock` to update state and transfer user's accrued rewards post Lockdrop contract's xMars claim call to the `incentives` contract |
| `CallbackMsg::DissolvePosition`            | Callback function called by `Unlock` to dissolve lockup position after user's accrued rewards have been claimed successfully                                                       |

### Query Messages

| Message                | Description                                                                                                                |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `QueryMsg::Config`     | Returns the config info                                                                                                    |
| `QueryMsg::State`      | Returns the contract's global state. Can be used to estimate future cycle rewards by providing the corresponding timestamp |
| `QueryMsg::StakerInfo` | Returns info of a user's staked position. Can be used to estimate future rewards by providing the corresponding timestamp  |
| `QueryMsg::Timestamp`  | Returns the current timestamp                                                                                              |

#

## Build schema and run unit-tests

```
cargo schema
cargo test
```

## License

TBD
