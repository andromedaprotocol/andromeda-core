<p>&nbsp;</p>
<p align="center">
<img src="https://github.com/andromedaprotocol/andromeda-core/blob/add-readme/asset/core-logo-dark.png" width=500>
</p>

A monorepository containing all the contracts and packages related to Andromeda Protocol. Full documentation for all the contracts can be found [here](https://app.gitbook.com/@andromedaprotocol/s/andromeda/).

## ADO Categories 

The contracts are classified based on their functionality. Currently we have 8 different contract categories.

| Category| Description |                                                                                                                                 
| ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| [app](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/app)| Contracts used for building Andromeda apps. |
| [data-storage](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/data-storage/andromeda-primitive)    | Contract used to store any type of data  (uint, string, bool ect...).|  
| [ecosystem](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/ecosystem) | Contracts that are allow interaction with different ecosystem protocols.|                      
| [finance](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/finance)       |  Contracts used by fungible tokens to perform defi operations.|                                                                                      
| [fungible tokens](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/fungible-tokens)       | Contracts that integrate with fungible tokens (CW-20 tokens).|
| [non-fungible-tokens](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/non-fungible-tokens)         | Contacts that integrate with   non-funible toknes (NFTs).|
| [modules](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/modules) |Andromeda modules that are attached to other ADOs to extend functionality.|
 | [defunct](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/defunct) |Andromeda contracts that are no longer functional.|



## ADOs

| Contract | Category | Description | Documentation |
| ---------------------------|------------------------|-------------------------------------------|----------------------------------------------------- |
| [andromeda-app-contract](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/app/andromeda-app-contract)| app | Contract used to create Andromeda Apps. | [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/andromeda-apps/app)|                                               
| [andromeda-factory](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/app/andromeda-factory)| app |Contract used to save the code Ids of all Andromeda ADOs. | [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/andromeda-apps/andromeda-factory)|                                                                                                                
| [andromeda-primitive](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/data-storage/andromeda-primitive)         | data-storage | Contract that stores any type of data that can be referenced by other ADOs. |[Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/data-storage/primitive) |
| [andromeda-vault](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/ecosystem/andromeda-vault)|ecosystem| Contract that can receive and store funds. Acts as a central bank for projects. | [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/ecosystem/vault) |
| [andromeda-rate-limiting-withdrawals](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/finance/andromeda-rate-limiting-withdrawals)    | finance | Contract that puts restrictions on the withdrawal of funds by users.  | [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/finance/rate-limiting-withdrawals)|
| [andromeda-splitter](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/finance/andromeda-splitter)   | finance| Contract used to split any sent funds amongst defined addresses.  | [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/finance/andromeda-splitter)|
| [andromeda-timelock](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/finance/andromeda-timelock) | finance| Contract used to store funds until a condition has been satisfied before being released, similar to Escrow.|[Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/finance/timelock)|
| [andromeda-vesting](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/finance/andromeda-vesting) |finance | Contract used to custom vest tokens for a single recipient.| [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/finance/vesting-ado)|                                                                                              
| [andromeda-weighted-distribution-splitter](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/finance/andromeda-weighted-distribution-splitter) | finance | Contract used to split any sent funds amongst defined addresses. Similar to the splitter but uses weights instead of percentages.| [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/finance/weighted-splitter)|
| [andromeda-cw20](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/app/andromeda-factory)         | fungible tokens |Contract to create standard cw-20 tokens. | [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/fungible-tokens/cw20-token)
| [andromeda-cw20-staking](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/fungible-tokens/andromeda-cw20-staking)       | fungible tokens | Contract that allows the staking of cw-20 tokens for rewards.    | [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/fungible-tokens/cw20-staking)|
| [andromeda-lockdrop](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/fungible-tokens/andromeda-lockdrop) | fungible tokens| Contract that allows users to deposit a netive token in exchange for the project's cw-20 token   |[Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/fungible-tokens/lockdrop)|
| [andromeda-merkle-airdrop](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/fungible-tokens/andromeda-merkle-airdrop)       | fungible tokens| Contract used to perform a merkle airdrop on cw20-tokens| [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/fungible-tokens/merkle-airdrop)|                        
| [andromeda-auction](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/non-fungible-tokens/andromeda-auction)|non-fungible-tokens| Contract that can receive an NFT and run an auction on it.| [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/non-fungible-tokens/auction)|
| [andromeda-crowdfund](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/non-fungible-tokens/andromeda-crowdfund)|non-fungible-tokens| Contracts used to perform a crowdfund by selling NFTs.|[Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/non-fungible-tokens/crowdfund)|
| [andromeda-cw721](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/non-fungible-tokens/andromeda-cw721)| non-fungible-tokens| Contract used to create cw-721 standard NFTs. Has a custom message that allows selling the NFTs.| [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/non-fungible-tokens/andromeda-digital-object)|
| [andromeda-cw721-staking](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/non-fungible-tokens/andromeda-cw721-staking)|non-fungible-tokens| Contract that allows custom staking of NFTs.|[Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/non-fungible-tokens/cw721-staking)|
| [andromeda-gumball](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/non-fungible-tokens/andromeda-gumball)|non-fungible-tokens| Contract that allows users to pay a price to get a random NFT.| [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/non-fungible-tokens/gumball)|
| [andromeda-nft-timelock](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/non-fungible-tokens/andromeda-nft-timelock)|non-fungible-tokens| Contract that locks an NFT for a certain period of time.| [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/non-fungible-tokens/nft-timelock)|
| [andromeda-wrapped-cw721](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/non-fungible-tokens/andromeda-wrapped-cw721)| non-fungible-tokens| Contract that wraps an NFT and mints an andromeda NFT that can leverage our custom messages and modules instead. The token can be unwrapped.| [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/non-fungible-tokens/wrapped-cw721)|

## Modules

Modules are smart contracts that can be added to other ADOs on instantiation to extend their functionality. The communication between ADOs and our modules is achieved using our custom [Hooks](https://docs.andromedaprotocol.io/andromeda/andromeda-hooks/hooks). We currently have 4 modules:

|Module| Description| Documentation|
|-------------------------------|---------------------------|-----------------------------|
| [address-list](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/modules/andromeda-address-list)| A module used to whitelist/blacklist a list of addresses to interact with the ADO.|[Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/modules/address-list)|
| [rates](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/modules/andromeda-rates)| A module used to add rates (taxes/royalties) on fund transfers| [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/modules/rates)|
| [cw721-offers](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/non-fungible-tokens/andromeda-cw721-offers)|Module that can be attached to the cw721 ADO as another way to buy and sell NFTs.|[Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/modules/nft-offers)|
| [receipts](https://docs.andromedaprotocol.io/andromeda/smart-contracts/modules/receipt-contract)| A module that can be attached to ADOs that saves the events of messages.| [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/modules/receipt-contract)|

## Packages

| Contract                                                                                                             | Description                                                                                                                                          |
| -------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| [andromeda_protocol](https://github.com/andromedaprotocol/andromeda-contracts/tree/main/packages/andromeda_protocol) | Package used to define message types and various utility methods used by Andromeda ADO Contracts.|
## Development

### Environment Setup

To set up your environment follow the documentation provided at [Juno Docs](https://docs.junonetwork.io/juno/readme).

### Testing

All tests can be run using:

`cargo test --workspace`

### Building

All contracts and packages can be built by running the build script:

`./build_all.sh`

This will build all contract `.wasm` files in to the `artifacts` directory at the project root.

To build a single contract, you need to have [wasm-opt](https://command-not-found.com/wasm-opt)
Then run:

`./build.sh [contract name]` or `./build.sh [catogory name]` 



Examples:

`./build.sh andromda vault` to build the vault contract.
or
`./build.sh finance` to build all contracts under the finance category.

They can also be chained to build multiple directories at the same time:

`./build.sh andromeda_app non-fungible-tokens` to build the app contract and all contracts under the non-fungible-tokens category.

### Formatting

Make sure you run `rustfmt` before creating a PR to the repo. You need to install the `nightly` version of `rustfmt`.

```sh
rustup toolchain install nightly
```

To run `rustfmt`,

```sh
cargo fmt
```

### Linting

You should run `clippy` also. This is a lint tool for rust. It suggests more efficient/readable code.
You can see [the clippy document](https://rust-lang.github.io/rust-clippy/master/index.html) for more information.
You need to install `nightly` version of `clippy`.

#### Install

```sh
rustup toolchain install nightly
```

#### Run

```sh
cargo clippy --all --all-targets -- -D warnings
```
