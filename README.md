<p>&nbsp;</p>
<p align="center">
<img src="https://github.com/andromedaprotocol/andromeda-core/blob/development/asset/core-logo.png" width=1000>
</p>

  AndromedaOS is a revolutionary software layer that provides a massively
  abstracted environment and user experience for the next generation of
  blockchain innovators to create, develop and get paid.

# Introduction to AndromedaOS

### Mission

**Andromeda** **Protocol** is a rapid development framework and a next-generation user interface that brings an Easier, Better, and Faster capability to Web 3.0, and the blockchain industry.


#### What is a blockchain operating system? <a href="#what-is-a-blockchain-operating-system" id="what-is-a-blockchain-operating-system"></a>

In short, a blockchain operating system provides an environment filled with ready to use tooling, common interfaces for applications and features familiar to modern computer users. As Andromeda is the first true operating system designed to run on distributed computing frameworks, the details are quite technical.

AndromedaOS, or _aOS_ for short, is comprised of several interoperating systems that work together to bring clarity and ease of use to the user. It's important to understand the basic concepts and architecture of the system to develop.

A quick description of each of the components that make up the aOS:

* ​[Andromeda Digital Objects](https://docs.andromedaprotocol.io/andromeda/andromeda-digital-objects/introduction-to-ados) - the building blocks of the system
* [​Andromeda Apps](https://docs.andromedaprotocol.io/andromeda/andromeda-apps/introduction-to-apps) - advanced functionality built with ADOs
* ​[aOS Kernel](https://docs.andromedaprotocol.io/andromeda/platform-and-framework/andromeda-messaging-protocol/kernel) - the system for enforcing and coordinating the different systems
* ​[aOS File System](https://docs.andromedaprotocol.io/andromeda/platform-and-framework/andromeda-messaging-protocol/virtual-file-system) - common namespace for referencing ADOs, services, network, etc
* ​[aOS Economics](https://docs.andromedaprotocol.io/andromeda/platform-and-framework/andromeda-messaging-protocol/economics-engine) - developer and creator incentives

It's important to note that each of these systems exist 100% on-chain. All logic, interactions, objects, economics, etc. are deployed on-chain.

### Powered by the Cosmos Ecosystem <a href="#powered-by-the-cosmos-ecosystem" id="powered-by-the-cosmos-ecosystem"></a>

The power and performance of the Rust/CosmWasm combo is what allowed this breakthrough in interoperability and complexity.

#### Infinite Reach Through IBC

Just like an operating system that can be seamlessly installed on any device, the AndromedaOS transcends limitations tied to the Andromeda chain. It is designed for universal compatibility and can be effortlessly deployed on any chain within the expansive Cosmos ecosystem.&#x20;

The operating system can be employed for local development on the chain, yet its true potential shines through Inter-Blockchain Communication (IBC). This capability allows the creation of Apps that extend seamlessly across multiple interconnected chains.

Here is a visual representation of how this system is connected.

<p>&nbsp;</p>
<p align="center">
<img src="https://github.com/andromedaprotocol/andromeda-core/tree/daniel/add-readme-files/asset" width=800>
</p>

As we can see, each of the Cosmos chains has AndromedaOS deployed . Since AndromedaOS can communicate using IBC, then users can build Applications that span accross many chains taking advantage of all the benefits that come along.

For example, a user can build an NFT collection on Stargaze selling the NFTs using one of our ADOs and then using a splitter to send part of the funds to Terra to leverage some protocol and another part to Injective to leverage some functionality there. The user's imagination is the only limit to what can be built using the aOS. &#x20;

**Note**: IBC functionality is being slowly introduced into the system. Not all features mentioned above are currently available.

### What is the benefit of using aOS

* **For Projects:**

Before **Andromeda**, projects would need to hire a full development teams in order to  build their projects and custom smart contracts. Andromeda eliminates this need by providing a very large amount of custom smart contracts that upcoming projects can pick and chose from to achieve their desired utility. These projects can then use our **No-Code-Builder** to build their projects in a matter of minutes on any of the chains that Andromeda is deployed on.

* **For Developers:**

Developers can use our **Andromeda Logic Library** (ALL) which contains all our contracts to build from. Similar to how [**cw-plus**](https://github.com/CosmWasm/cw-plus) contracts are used as a base for production quality builds, the ALL will act as a base for all developers to create their own ADOs that use the superior interoperable system.&#x20;

As it stands, the ALL contains around 25 ADOs which is the tip of the iceberg. More and more ADOs are being added by the Andromeda team, and as we continue building, the ALL will eventually reach a state with thousands of ADOs where every use case imagined can be built using it.&#x20;

Furthermore, developers are incentivized for their contributions and the ADOs they create. This incentive system operates through our[ economic engine](platform-and-framework/andromeda-messaging-protocol/economics-engine.md), enabling developers to set custom fees on their ADOs when it is published. Users utilizing these ADOs pay these fees, which are then returned to the developer responsible for their creation.

* **For Chains:**

&#x20;AndromedaOS provides a whole suite of tooling that can be quickly installed on any chain in the Cosmos ecosystem. Installing the aOS would instantly give a chain and its users access to the following:&#x20;

1. A large number of production ready ADOs to be used.&#x20;
2. The best no-code-builder in Cosmos and perhaps the entire blockchain industry.
3. IBC capable applications.
4. An incredible all in one CLI that is easy to use and manage.
5. Exposure to the chain, as users on any chain that implements the aOS will be able to to see where aOS is also deployed and might consider building applications on said chain.

# Andromeda Core Repo

A monorepository containing all the contracts and packages related to Andromeda Protocol. Full documentation for all the contracts can be found [here](https://docs.andromedaprotocol.io/andromeda/platform-and-framework/introduction).

## ADO Categories 

The contracts are classified based on their functionality. Currently we have 7 different contract categories.

| Category| Description |                                                                                                                                 
| ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| [app](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/app)| Contracts used for building Andromeda apps. |  
| [ecosystem](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/ecosystem) | Contracts that are allow interaction with different ecosystem protocols.|                      
| [finance](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/finance)       |  Contracts used by fungible tokens to perform defi operations.|                                                                                      
| [fungible tokens](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/fungible-tokens)       | Contracts that integrate with fungible tokens (CW-20 tokens).|
| [non-fungible-tokens](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/non-fungible-tokens)         | Contacts that integrate with non-funible toknes (NFTs). Includes a standard CW721 contract with some custom features.|
| [os](https://github.com/andromedaprotocol/andromeda-core/tree/1.0.rc-1/contracts/os)      | Contacts that make up the aOS architecture |


## Audited ADOs
The list of ADOs that have been audited and are available on our web-application.

| Contract | Category | Description | Documentation |
| ---------------------------|------------------------|-------------------------------------------|----------------------------------------------------- |
| [andromeda-app-contract](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/app/andromeda-app-contract)| app | Contract used to create Andromeda Apps. | [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/andromeda-apps/app)|      
| [andromeda-rate-limiting-withdrawals](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/finance/andromeda-rate-limiting-withdrawals)    | finance | Contract that puts restrictions on the withdrawal of funds by users.  | [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/finance/rate-limiting-withdrawals)|
| [andromeda-splitter](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/finance/andromeda-splitter)   | finance| Contract used to split any sent funds amongst defined addresses.  | [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/finance/andromeda-splitter)|
| [andromeda-timelock](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/finance/andromeda-timelock) | finance| Contract used to store funds until a condition has been satisfied before being released, similar to Escrow.|[Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/finance/timelock)|                                                      
| [andromeda-cw20](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/fungible-tokens/andromeda-cw20) | fungible tokens |Contract to create standard CW20 tokens. | [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/fungible-tokens/cw20-token)
| [andromeda-cw20-staking](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/fungible-tokens/andromeda-cw20-staking)       | fungible tokens | Contract that allows the staking of CW20 tokens for rewards.    | [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/fungible-tokens/cw20-staking)|
| [andromeda-cw20-exchange](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/fungible-tokens/andromeda-cw20-exchange)       | fungible tokens | Contract that allows the exchanging native tokens for a specified CW20    | [Gitbook](https://docs.andromedaprotocol.io/andromeda/andromeda-digital-objects/cw20-exchange)|
| [andromeda-lockdrop](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/fungible-tokens/andromeda-lockdrop) | fungible tokens| Contract that allows users to deposit a netive token in exchange for the project's cw-20 token   |[Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/fungible-tokens/lockdrop)|
| [andromeda-merkle-airdrop](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/fungible-tokens/andromeda-merkle-airdrop)       | fungible tokens| Contract used to perform a merkle airdrop on cw20-tokens| [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/fungible-tokens/merkle-airdrop)|                        
| [andromeda-auction](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/non-fungible-tokens/andromeda-auction)|non-fungible-tokens| Contract that can receive an NFT and run an auction on it.| [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/non-fungible-tokens/auction)|
| [andromeda-marketplace](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/non-fungible-tokens/andromeda-marketplace)|non-fungible-tokens| Contract that can receive an NFT and run an a sale on it.| [Gitbook](https://docs.andromedaprotocol.io/andromeda/andromeda-digital-objects/marketplace)|
| [andromeda-crowdfund](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/non-fungible-tokens/andromeda-crowdfund)|non-fungible-tokens| Contracts used to perform a crowdfund by selling NFTs.|[Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/non-fungible-tokens/crowdfund)|
| [andromeda-cw721](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/non-fungible-tokens/andromeda-cw721)| non-fungible-tokens| Contract used to create CW721 standard NFTs. Has a custom message that allows selling the NFTs.| [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/non-fungible-tokens/andromeda-digital-object)|
| [andromeda-adodb](https://github.com/andromedaprotocol/andromeda-core/tree/1.0.rc-1/contracts/os/andromeda-adodb)| os| The ADO database responsible for publishing new ADOs into the aOS| [Gitbook](https://docs.andromedaprotocol.io/andromeda/platform-and-framework/andromeda-messaging-protocol/andromeda-factory)|
| [andromeda-economics](https://github.com/andromedaprotocol/andromeda-core/tree/1.0.rc-1/contracts/os/andromeda-economics)| os | The contract responsible for handling ADO fees| [Gitbook](https://docs.andromedaprotocol.io/andromeda/platform-and-framework/andromeda-messaging-protocol/economics-engine)|
| [andromeda-kernel](https://github.com/andromedaprotocol/andromeda-core/tree/1.0.rc-1/contracts/os/andromeda-kernel)| os | The contract responsible for handling communication between ADOs| [Gitbook](https://docs.andromedaprotocol.io/andromeda/platform-and-framework/andromeda-messaging-protocol/kernel)|
| [andromeda-vfs](https://github.com/andromedaprotocol/andromeda-core/tree/1.0.rc-1/contracts/os/andromeda-vfs)| os| The contract responsible for managing the usernames and paths of ADOs and users in the aOs | [Gitbook](https://docs.andromedaprotocol.io/andromeda/platform-and-framework/andromeda-messaging-protocol/virtual-file-system)|

**Note**: There exists many other ADOs that are scheduled for release but have still to undergo the audit process.

## Packages

| Contract                                                                                                             | Description                                                                                                                                          |
| -------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| [andromeda_protocol](https://github.com/andromedaprotocol/andromeda-core/tree/development/packages) | Package used to define message types and various utility methods used by Andromeda ADO Contracts.|

### ADO Base

The packages also includes the [ado_base](https://github.com/andromedaprotocol/andromeda-core/tree/development/packages/ado-base). Since all our ADOs are built using the same architecture, redundency was inevitable. So we decided to bundle up all the functions/messages/structures that are used by all ADOs into the ado_base which can be referenced by any new ADOs. 

## Development

### Andromeda Template and Crate

A starting template for ADO development can be found [here](https://github.com/andromedaprotocol/andr-cw-template).
The andromeda-std crate can be found [here](https://crates.io/crates/andromeda-std).

### Integration Tests
Check out our cw-multi-test based testing [library](https://github.com/andromedaprotocol/andromeda-core/tree/1.0.rc-1/packages/andromeda-testing) to setup custom ADO integration tests.

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
### Creating and Interacting with ADOs

Andromeda is deployed on many of the Cosmos chains. Usually this will require you to set up an environment for each chain. Luckily, Andromeda has built the Andromeda CLI, an all in one tool to build, interact, and manage ADOs and wallets for any of the chains. The CLI documentation can be found [here](https://docs.andromedaprotocol.io/andromeda/andromeda-cli/introduction).

### Andromeda JS 
Andromeda.js is a JavaScript SDK for writing applications that interact with ADOs on any of the blockchains that Andromeda is deployed on. More on the AndromedaJS can be found [here](https://docs.andromedaprotocol.io/andromeda.js/).

## Licensing

[Terms and Conditions](https://github.com/andromedaprotocol/andromeda-core/blob/development/LICENSE/LICENSE.md)
