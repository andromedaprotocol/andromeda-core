# Andromeda Modules

This package contains the definition of an Andromeda Module, alongside any behaviour that a module should implement.

## Modules

<table>
  <thead>
    <th>Module</th>
    <th>Definition</th>
    <th>Description</th>
  </thead>
  <tbody>
    <tr>
      <td><a href="https://github.com/andromedaprotocol/andromeda-contracts/blob/extensions/packages/andromeda_modules/src/whitelist.rs" target="_blank">Whitelist</a></td>
      <td>
        <pre>struct Whitelist {
  moderators: Vec&lt;HumanAddr&gt;,
}</pre>
      </td>
      <td>Enables a whitelist of addresses that are authorised to interact with the contract's functions.</td>
    </tr>
  </tbody>
</table>

## Structs

Each module is defined using the `ModuleDefinition` enum which contains what data must be sent with a module in order for it to be initialized:

```rust
pub enum ModuleDefinition {
    WhiteList { moderators: Vec<HumanAddr> },
    Taxable { tax: Fee, receivers: Vec<HumanAddr> },
    Royalties { fee: Fee, receivers: Vec<HumanAddr> },
}
```

Several of the `Module` trait's methods return a `HookResponse` struct:

```rust
pub struct HookResponse {
    pub msgs: Vec<CosmosMsg>,
    pub logs: Vec<LogAttribute>,
}

impl HookResponse {
    pub fn default() -> Self {
        HookResponse {
            msgs: vec![],
            logs: vec![],
        }
    }
}
```

## Module Trait

A module is sent with the contract's `InitMsg`, the definitions of these modules is then stored. To operate a module is first converted to a struct which implements `trait Module`. Each module implements the following methods:
<br />

### Validate

Validates the module definition and that it does not collide with any other module defined for the token. Errors if the definition is invalid.

```rust
fn validate(&self, extensions: Vec<ModuleDefinition>) -> StdResult<bool>;
```

#### Parameters

| Parameter | Type                    | Description                                        |
| --------- | ----------------------- | -------------------------------------------------- |
| `modules` | `Vec<ModuleDefinition>` | The vector of modules defined for the given token. |

### As Definition

Returns the module as a `ModuleDefinition` enum.

```rust
fn as_definition(&self) -> ModuleDefinition
```

### Pre Handle

A hook allowing access to any handle message. This hook is called when any `HandleMsg` message is received.

```rust
fn pre_publish<S: Storage, A: Api, Q: Querier>(
    &self,
    deps: &mut Extern<S, A, Q>,
    env: Env,
    token_id: i64,
) -> StdResult<HookResponse
```

### Pre Publish

A hook allowing access to data related to an ADO being published. This hook is called when a `HandleMsg::Publish` message is received.

```rust
fn pre_publish<S: Storage, A: Api, Q: Querier>(
    &self,
    deps: &mut Extern<S, A, Q>,
    env: Env,
    token_id: i64,
) -> StdResult<HookResponse
```

#### Parameters

| Parameter  | Type  | Description                        |
| ---------- | ----- | ---------------------------------- |
| `token_id` | `i64` | The ID of the ADO to be published. |

### Pre Transfer

A hook allowing access to data related to an ADO being transferred. This hook is called when a `HandleMsg::Transfer` message is received.

```rust
fn pre_transfer<S: Storage, A: Api, Q: Querier>(
    &self,
    deps: &mut Extern<S, A, Q>,
    env: Env,
    token_id: i64,
    from: HumanAddr,
    to: HumanAddr,
) -> StdResult<HookResponse>
```

#### Parameters

| Parameter  | Type        | Description                             |
| ---------- | ----------- | --------------------------------------- |
| `token_id` | `i64`       | The ID of the ADO to be transferred.    |
| `from`     | `HumanAddr` | The current owner of the published ADO. |
| `to`       | `HumanAddr` | The receiver of the published ADO.      |

### Pre Transfer Agreement

A hook allowing access to data related to a transfer agreement between the ADO owner and a purchaser. This hook is called when a `HandleMsg::TransferAgreement` message is received.

```rust
fn pre_transfer_agreement<S: Storage, A: Api, Q: Querier>(
    &self,
    deps: &mut Extern<S, A, Q>,
    env: Env,
    token_id: i64,
    amount: Coin,
    purchaser: HumanAddr,
) -> StdResult<HookResponse>
```

#### Parameters

| Parameter   | Type                 | Description                                 |
| ----------- | -------------------- | ------------------------------------------- |
| `token_id`  | `i64`                | The ID of the ADO the agreement relates to. |
| `amount`    | `cosmwasm_std::Coin` | The agreed transfer amount.                 |
| `purchaser` | `HumanAddr`          | The agreed purchaser of the ADO.            |

### Pre Burn

A hook allowing access to data related to an ADO being burnt. This hook is called when a `HandleMsg::Burn` message is received.

```rust
fn pre_burn<S: Storage, A: Api, Q: Querier>(
    &self,
    deps: &mut Extern<S, A, Q>,
    env: Env,
    token_id: i64,
) -> StdResult<HookResponse>
```

#### Parameters

| Parameter  | Type  | Description                        |
| ---------- | ----- | ---------------------------------- |
| `token_id` | `i64` | The ID of the ADO to be published. |

### Pre Archive

A hook allowing access to data related to an ADO being archived. This hook is called when a `HandleMsg::Archive` message is received.

```rust
fn pre_archive<S: Storage, A: Api, Q: Querier>(
    &self,
    deps: &mut Extern<S, A, Q>,
    env: Env,
    token_id: i64,
) -> StdResult<HookResponse>
```

#### Parameters

| Parameter  | Type  | Description                        |
| ---------- | ----- | ---------------------------------- |
| `token_id` | `i64` | The ID of the ADO to be published. |
