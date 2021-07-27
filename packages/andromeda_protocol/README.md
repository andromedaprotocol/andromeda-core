# Andromeda Protocol

A repository containing the NFT contract for Andromeda Protocol on Terra.

## Building

The contract was built using a basic CosmWasm template and as such can be built using `cargo wasm`, for more info on building and deploying contracts on Terra check the docs [here](https://docs.terra.money/contracts/). The repository contains a build script to simplify the build process, this can be run using:

```./build.sh```

## Interaction

### Collection Creation
The `InitMsg` for this contract does not currently take any parameters. To create a new NFT collection please use the `Create` message:

``` 
Create {
  name: String,
  symbol: String,
  extensions: Vec<Extension>
}
```

The `symbol` field is used to uniquely identify the collection. The following extensions are currently available:

```
pub enum Extension {
    WhiteListExtension { moderators: Vec<HumanAddr> },
    TaxableExtension { tax: Fee, receivers: Vec<HumanAddr> },
    RoyaltiesExtension { fee: Fee, receivers: Vec<HumanAddr> },
}
```

### Minting A Token
To mint a token the `Mint` message is used:

```
Mint {
    collection_symbol: String,
    token_id: i64,
}
```

This will mint a token with the given ID under the given collection owned by the address that sends the message. Any token transactions under the provided collection will be subject to the collection's extensions.

### Further Interaction
The following interactions are available and the required parameters can be seen in `src/msg.rs`:
- `Burn`
- `Archive`
- `CreateTransferAgreement`
- `Transfer`
