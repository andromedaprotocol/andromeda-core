# Andromeda ADO Contracts

A monorepository containing all the contracts and packages related to Andromeda Protocol using the [Terra](https://www.terra.money/) blockchain. All related docs can be found [here](https://app.gitbook.com/@andromedaprotocol/s/andromeda/).

## Contracts

| Contract      | Description |
| ----------- | ----------- |
| [andromeda_factory](https://github.com/andromedaprotocol/andromeda-contracts/tree/main/contracts/andromeda_factory) | Factory contract used to initialise a given token contract using a preset Code ID. Stores a record of all initialised token contracts via a symbol reference. |
| [andromeda_token](https://github.com/andromedaprotocol/andromeda-contracts/tree/main/contracts/andromeda_token)      | Token contract used to store all related tokens and any modules that may be attached to them. Initialised by the `andromeda_factory` contract.|
| [andromeda_addresslist](https://github.com/andromedaprotocol/andromeda-contracts/tree/main/contracts/andromeda_addresslist)      | Contract used to store a list of addresses. Queriable for inclusion of a given address. Used for both whitelist and blacklist modules.|
| [andromeda_splitter](https://github.com/andromedaprotocol/andromeda-contracts/tree/main/contracts/andromeda_splitter)      | Contract used to split any sent funds amongst defined addresses.|
| [andromeda_timelock](https://github.com/andromedaprotocol/andromeda-contracts/tree/main/contracts/andromeda_timelock)      | Contract used to store funds for a defined period of time before being released, similar to Escrow. |
| [andromeda_receipt](https://github.com/andromedaprotocol/andromeda-contracts/tree/main/contracts/andromeda_receipt)      | Contract used to mint receipts. |

## Packages
| Contract      | Description |
| ----------- | ----------- |
| [andromeda_protocol](https://github.com/andromedaprotocol/andromeda-contracts/tree/main/packages/andromeda_protocol)      | Package used to define message types and various utility methods used by Andromeda ADO Contracts. Andromeda modules are also defined in this package. |

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
