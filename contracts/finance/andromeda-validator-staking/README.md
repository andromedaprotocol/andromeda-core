# Andromeda Validator Staking

## Introduction

The `Andromeda Validator Staking`  is a smart contract designed for staking native tokens to validators. This contract facilitates the process of staking by allowing delegators to send messages along with funds to stake. The contract owner has the privilege to claim rewards, unstake, and withdraw staked funds following a specified unbonding period.

<b>Ado_type:</b> validator-staking

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub default_validator: Addr,
    pub kernel_address: String,
    pub owner: Option<String>
}
```
```json
{
    "default_validator": "andr...",
    "kernel_address": "...",
}
```

The `default_validator` parameter denotes the address of the validator utilized when no specific validator is indicated in staking operations.


## ExecuteMsg
### Stake
A `Stake` message, accompanied by funds, initiates staking with the designated `validator`. If no validator is specified, the `default_validator` is utilized.
```rust
Stake {
    validator: Option<Addr>,
}
```
```json
{
    "stake":  {
        "validator": "andr..." 
    },
}
```

### Claim
A `Claim` message is used to claim staking rewards from a validator. If no validator is specified, rewards are claimed from the `default_validator`. The `recipient` can be either a vfs or account address. If no `recipient` is specified, the sender of the message is used as the recipient address.

_**Note:** Only contract owner can claim the staking rewards._
```rust
Claim {
    validator: Option<Addr>,
    recipient: Option<AndrAddr>,
}
```
```json
{
    "claim":  {
        "validator": "andr...",
        "recipient": "andr...",
    },
}
```

### Unstake
A `Unstake` message is employed to unstake tokens from a validator. If no validator is specified, the delegation to the `default_validator` is removed. 

_**Warning:** Only contract owner can unstake._

```rust
Unstake {
    validator: Option<Addr>,
}
```
```json
{
    "unstake":  {
        "validator": "andr...",
    },
}
```

### WithdrawFunds
A `WithdrawFunds` message enables the withdrawal of unstaked tokens. Once tokens are undelegated, they become inaccessible for a specified unbonding period. After this period, the funds are released to the contract, allowing the owner to withdraw them using the `WithdrawFunds` message.

_**Note:** Only contract owner can withdraw funds._

```rust
WithdrawFunds {},
```

```json
{
    "withdraw_funds":  {},
}
```

## QueryMsg
### StakedTokens
```rust
pub enum QueryMsg {
    #[returns(Option<FullDelegation>)]
    StakedTokens { validator: Option<Addr> },
}
```
```json
{
    "staked_tokens":  {
        "validator": "andr..." 
    },
}
```

The `StakedTokens` message retrieves staking information associated with a specified validator. It returns <a href="https://docs.rs/cosmwasm-std/latest/cosmwasm_std/struct.FullDelegation.html" target="blank">FullDelegation</a> structure.


### UnstakedTokens
```rust
pub enum QueryMsg {
    #[returns(Option<Vec<UnstakingTokens>>)]
    UnstakedTokens {},
}
```
```json
{
    "unstaked_tokens":  {},
}
```

The `UnstakedTokens` message retrieves a list of tokens and their respective payout times that have been unstaked but not yet withdrawn. It returns a vector of `UnstakingTokens`.

```rust
pub struct UnstakingTokens {
    pub fund: Coin,
    pub payout_at: Timestamp,
}
```
```json
{
    "fund":  "",
    "payout_at":  "",
}
```

The `fund` field specifies the denomination and amount of the token that has not yet been withdrawn, while `payout_at` indicates the timestamp (in nanoseconds) when the unstaked tokens become accessible.
