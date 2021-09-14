# Andromeda Modules

This package contains the definition of an Andromeda Module, alongside any behaviour that a module should implement. Each module is built upon the [CW721 Spec](https://github.com/CosmWasm/cosmwasm-plus/blob/main/packages/cw721/README.md).

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
  moderators: Vec&lt;String&gt;,
}</pre>
      </td>
      <td>Enables a whitelist of addresses that are authorised to interact with the contract's functions.</td>
    </tr>
        <tr>
      <td><a href="https://github.com/andromedaprotocol/andromeda-contracts/blob/extensions/packages/andromeda_modules/src/taxable.rs" target="_blank">Taxable</a></td>
      <td>
        <pre>struct Taxable {
  tax: u128,
  receivers: Vec&lt;String&gt;,
}</pre>
      </td>
      <td>Adds a percentage (rounded) tax to any agreed transfer between ADOs. The tax is then sent to each address in the receiver vector (non-split).</td>
    </tr>
  </tbody>
</table>

## Structs

Each module is defined using the `ModuleDefinition` enum which contains what data must be sent with a module in order for it to be initialized:

```rust
enum ModuleDefinition {
    Whitelist { moderators: Vec<String> },
    Taxable { tax: Fee, receivers: Vec<String> },
    Royalties { fee: Fee, receivers: Vec<String> },
    Receipt,
}
```

Several of the `Module` trait's methods return a `HookResponse` struct:

```rust
struct HookResponse {
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

A module is sent with the contract's `InstantiateMsg`, the definitions of these modules is then stored. To operate a module is first converted to a struct which implements `trait Module`. The `Module` trait implements several hooks that provide data related to each message type. Every base message type defined in the [CW721 Spec](https://github.com/CosmWasm/cosmwasm-plus/blob/main/packages/cw721/README.md) has a related hook the provides the message data as parameters. Each module implements the following methods:
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

### On Execute

A hook allowing access to any handle message. This hook is called when any `ExecuteMsg` message is received.

```rust
fn on_execute(
    &self,
    deps: &DepsMut,
    info: MessageInfo,
    env: Env,
) -> StdResult<HookResponse>
```

### On Mint

A hook allowing access to data related to an ADO being minted. This hook is called when a `ExecuteMsg::Mint` message is received.

```rust
fn on_mint(
    &self,
    deps: &DepsMut,
    info: MessageInfo,
    env: Env,
    token_id: String,
) -> StdResult<HookResponse>
```

#### Parameters

| Parameter  | Type     | Description                     |
| ---------- | -------- | ------------------------------- |
| `token_id` | `String` | The ID of the ADO to be minted. |

### On Transfer

A hook allowing access to data related to an ADO being transferred. This hook is called when a `ExecuteMsg::TransferNft` message is received.

```rust
fn on_transfer(
    &self,
    deps: &DepsMut,
    info: MessageInfo,
    env: Env,
    recipient: String,
    token_id: String,
) -> StdResult<HookResponse>
```

#### Parameters

| Parameter   | Type     | Description                          |
| ----------- | -------- | ------------------------------------ |
| `token_id`  | `String` | The ID of the ADO to be transferred. |
| `recipient` | `String` | The recipient of the ADO.            |

### On Send

A hook allowing access to data related to an ADO being sent to another CW721 contract. This hook is called when a `ExecuteMsg::SendNft` message is received.

```rust
fn on_send(
    &self,
    deps: &DepsMut,
    info: MessageInfo,
    env: Env,
    contract: String,
    token_id: String,
) -> StdResult<HookResponse>
```

#### Parameters

| Parameter  | Type     | Description                     |
| ---------- | -------- | ------------------------------- |
| `token_id` | `String` | The ID of the ADO to be sent.   |
| `contract` | `String` | The recieving contract address. |

### On Approve

A hook allowing access to data related to any approval being assigned for an NFT. This hook is called when a `ExecuteMsg::Approve` message is received.

```rust
fn on_approve(
    &self,
    deps: &DepsMut,
    info: MessageInfo,
    env: Env,
    sender: String,
    token_id: String,
    expires: Option<Expiration>,
) -> StdResult<HookResponse>
```

#### Parameters

| Parameter  | Type                        | Description                                                                                   |
| ---------- | --------------------------- | --------------------------------------------------------------------------------------------- |
| `token_id` | `String`                    | The ID of the ADO to be sent.                                                                 |
| `sender`   | `String`                    | The address to be approved for given token                                                    |
| `expires`  | `Option<CW721::Expiration>` | An optional expiration time, defined in the CW721 package. Defaults to `Expiration::Never{}`. |

### On Revoke

A hook allowing access to data related to any approval being revoked for an NFT. This hook is called when a `ExecuteMsg::Revoke` message is received.

```rust
fn on_revoke(
    &self,
    deps: &DepsMut,
    info: MessageInfo,
    env: Env,
    sender: String,
    token_id: String,
) -> StdResult<HookResponse>
```

#### Parameters

| Parameter  | Type     | Description                                |
| ---------- | -------- | ------------------------------------------ |
| `token_id` | `String` | The ID of the ADO to be sent.              |
| `sender`   | `String` | The address to be approved for given token |

### On Approve All

A hook allowing access to data related to any operator being assigned. This hook is called when a `ExecuteMsg::ApproveAll` message is received.

```rust
fn on_approve_all(
    &self,
    deps: &DepsMut,
    info: MessageInfo,
    env: Env,
    operator: String,
    expires: Option<Expiration>
) -> StdResult<HookResponse>
```

#### Parameters

| Parameter  | Type                        | Description                                                                                   |
| ---------- | --------------------------- | --------------------------------------------------------------------------------------------- |
| `operator` | `String`                    | The address to be assigned as an operator for the message sender                              |
| `expires`  | `Option<CW721::Expiration>` | An optional expiration time, defined in the CW721 package. Defaults to `Expiration::Never{}`. |

### On Approve All

A hook allowing access to data related to any operator privileges being revoked. This hook is called when a `ExecuteMsg::RevokeAll` message is received.

```rust
fn on_revoke_all(
    &self,
    deps: &DepsMut,
    info: MessageInfo,
    env: Env,
    operator: String,
) -> StdResult<HookResponse>
```

#### Parameters

| Parameter  | Type     | Description                                                      |
| ---------- | -------- | ---------------------------------------------------------------- |
| `operator` | `String` | The address to be assigned as an operator for the message sender |

### On Transfer Agreement

A hook allowing access to data related to a transfer agreement between the ADO owner and a purchaser. This hook is called when a `ExecuteMsg::TransferAgreement` message is received.

```rust
fn on_transfer_agreement(
    &self,
    deps: &DepsMut,
    info: MessageInfo,
    env: Env,
    token_id: String,
    amount: Coin,
    purchaser: String,
) -> StdResult<HookResponse>
```

#### Parameters

| Parameter   | Type                 | Description                                 |
| ----------- | -------------------- | ------------------------------------------- |
| `token_id`  | `String`             | The ID of the ADO the agreement relates to. |
| `amount`    | `cosmwasm_std::Coin` | The agreed transfer amount.                 |
| `purchaser` | `String`             | The agreed purchaser of the ADO.            |

### On Burn

A hook allowing access to data related to an ADO being burnt. This hook is called when a `ExecuteMsg::Burn` message is received.

```rust
fn on_burn(
    &self,
    deps: &DepsMut,
    info: MessageInfo,
    env: Env,
    token_id: String,
) -> StdResult<HookResponse>
```

#### Parameters

| Parameter  | Type     | Description                        |
| ---------- | -------- | ---------------------------------- |
| `token_id` | `String` | The ID of the ADO to be published. |

### On Archive

A hook allowing access to data related to an ADO being archived. This hook is called when a `ExecuteMsg::Archive` message is received.

```rust
fn on_archive(
    &self,
    deps: &DepsMut,
    info: MessageInfo,
    env: Env,
    token_id: String,
) -> StdResult<HookResponse>
```

#### Parameters

| Parameter  | Type     | Description                        |
| ---------- | -------- | ---------------------------------- |
| `token_id` | `String` | The ID of the ADO to be published. |

### On Agreed Transfer

A hook allowing access to data related to an ADO being transfered via an agreement. This hook is called when a `ExecuteMsg::Transfer` message is received for an ADO with a transfer agreement.

```rust
 fn on_agreed_transfer(
    &self,
    env: Env,
    payments: &mut Vec<BankMsg>,
    owner: String,
    purchaser: String,
    amount: Coin,
) -> StdResult<bool>
```

#### Parameters

| Parameter   | Type                | Description                                                                                                                                                                                                               |
| ----------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `payments`  | `&mut Vec<BankMsg>` | A mutable vector of payment messages, this is passed between every registered module that may interact with the outgoing payments from the transfer, as such ordering of registered modules may impact outgoing payments. |
| `owner`     | `String`            | The address of the ADO owner.                                                                                                                                                                                             |
| `purchaser` | `String`            | The address of the ADO purchaser.                                                                                                                                                                                         |
| `amount`    | `Coin`              | The agreed purchase amount.                                                                                                                                                                                               |
