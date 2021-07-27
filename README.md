# Andromeda ADO Contracts

A monorepository containing all the contracts and packages related to Andromeda Protocol using the [Terra](https://www.terra.money/) blockchain.

## Contracts

| Contract      | Description |
| ----------- | ----------- |
| [andromeda_factory](https://github.com/andromedaprotocol/andromeda-contracts/tree/extensions/packages/andromeda_protocol) | Factory contract used to initialise a given token contract using a preset Code ID. Stores a record of all initialised token contracts via a symbol reference. |
| [andromeda_token](https://github.com/andromedaprotocol/andromeda-contracts/tree/main/contracts/andromeda_token)      | Token contract used to store all related tokens and any modules that may be attached to them. Initialised by the `andromeda_factory` contract.|

## Packages
| Contract      | Description |
| ----------- | ----------- |
| [andromeda_modules](https://github.com/andromedaprotocol/andromeda-contracts/tree/main/packages/andromeda_modules) | Package used to define behaviour of Andromeda modules |
| [andromeda_protocol]https://github.com/andromedaprotocol/andromeda-contracts/tree/main/packages/andromeda_protocol)      | Package used to define message types and various utility methods used by Andromeda ADO Contracts. |

## Development

### Environment Setup
To set up your environment follow the documentation provided at [Terra Docs](https://docs.terra.money/contracts/tutorial/).

### Testing
All tests can be run using:

```cargo test --workspace```

### Building
All contracts and packages can be built by running the build script:

```./build.sh```

This will build all contract `.wasm` files in to the `artifacts` directory at the project root.
