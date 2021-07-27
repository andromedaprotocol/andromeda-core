# Andromeda Factory

A repository containing the NFT contract for Andromeda Protocol on Terra.

## Messages
All message structs are defined [here](https://github.com/andromedaprotocol/andromeda-contracts/blob/main/packages/andromeda_protocol/src/factory.rs)

### Init

```rust
struct InitMsg {
    pub token_code_id: u64,
}
```

| Key | Description |
| --- | ----------- |
| `token_code_id` | The Code ID of the contract to deploy when a new ADO collection is created |


### Handle
#### Create
Creates a new ADO collection contract. Once the contract has been initialized a `HandleMsg::TokenCreationHook` message is sent by the `init` method in order to register itself with the factory contract.

```rust
struct Create {
  name: String,
  symbol: String,
  modules: Vec<ModuleDefinition>,
}
```

| Key | Description |
| --- | ----------- |
| `name` | The name of the ADO collection |
| `symbol` | The unique symbol of the ADO collection |
| `modules` | The module definitions for the ADO collection |

#### Token Creation Hook
A hook called by the initialized ADO collection contract to register the address for the ADO collection's symbol.

```rust
struct TokenCreationHook {
    symbol: String,
    creator: HumanAddr
}
```

| Key | Description |
| --- | ----------- |
| `symbol` | The unique symbol of the ADO collection |
| `creator` | The address of the ADO collection creator |


### Query
#### GetAddress
Queries the address of a given ADO collection symbol

```rust
struct GetAddress { 
  symbol: String
}
```

Response
```rust
struct AddressResponse {
    pub address: HumanAddr,
}
```
