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

## Module Trait
Each module implements the following functions:

#### `validate -> StdResult<bool>`
Validates the module definition and that it does not collide with any other module defined for the token. Errors if the definition is invalid.

#### Parameters

| Parameter      | Type | Description |
| ----------- | ----------- | ----------- |
| `modules`      |   `Vec<ModuleDefinition>` | The vector of modules defined for the given token. |


#### `as_definition -> ModuleDefinition`
Returns the module as a `ModuleDefinition` enum.

#### `pre_publish -> StdResult<HookResponse>`
Validates the module definition and that it does not collide with any other module defined for the token. Errors if the definition is invalid.

#### Parameters

| Parameter      | Type | Description |
| ----------- | ----------- | ----------- |
| `token_id`      |   `i64` | The ID of the ADO to be published. |

#### `pre_publish -> StdResult<HookResponse>`
Validates the module definition and that it does not collide with any other module defined for the token. Errors if the definition is invalid.

#### Parameters

| Parameter      | Type | Description |
| ----------- | ----------- | ----------- |
| `token_id`      |   `i64` | The ID of the ADO to be published. |

#### `pre_publish -> StdResult<HookResponse>`
Validates the module definition and that it does not collide with any other module defined for the token. Errors if the definition is invalid.

#### Parameters

| Parameter      | Type | Description |
| ----------- | ----------- | ----------- |
| `token_id`      |   `i64` | The ID of the ADO to be published. |

<i><b>All `pre-` functions take the standard CosmWasm `deps` and `env` parameters as their first two parameters.<b></i>
