# Andromeda Protocol

A package defining various message structs related to Andromeda ADO contracts.

## Contract Struct Definitions
This package contains structs related to each Andromeda ADO contract:
- [andromeda_factory](https://github.com/andromedaprotocol/andromeda-contracts/blob/main/packages/andromeda_protocol/src/factory.rs)
- [andromeda_token](https://github.com/andromedaprotocol/andromeda-contracts/blob/main/packages/andromeda_protocol/src/token.rs)
- [andromeda_addresslist](https://github.com/andromedaprotocol/andromeda-contracts/blob/main/packages/andromeda_protocol/src/address_list.rs)
- [andromeda_splitter](https://github.com/andromedaprotocol/andromeda-contracts/blob/main/packages/andromeda_protocol/src/splitter.rs)
- [andromeda_timelock](https://github.com/andromedaprotocol/andromeda-contracts/blob/main/packages/andromeda_protocol/src/timelock.rs)
- [andromeda_receipt](https://github.com/andromedaprotocol/andromeda-contracts/blob/main/packages/andromeda_protocol/src/receipt.rs) 

# Address-List As Part of Permissioning
## Getting Started
The first step is to save the desired actor to permission pair in the address-list contract's 
```rust 
pub const PERMISSIONS: Map<&Addr, Permission> = Map::new("permissioning")
```
Note that `Permission` of type `Contract` isn't allowed in the address-list contract.

Apply `ExecuteMsg::AddActorPermission { actor, permission }` to the ADO you want. 
To involve the address-list contract, set the Permission to be of type Contract, and input the address-list's contract address. `Permission::Contract(address_list_address)`.
Make sure that the `actor` is the same as the one set in the address-list contract.


The completion of the aforementioned steps will enable permissioning on the ADO of your choice.

# Rates Module as a Feature
## Implementation
Rates are now stored directly in your ADO without the need of a rates contract, though referring to one is still possible.

You can store rates in your ADO by calling 
```rust
ExecuteMsg::Rates(RatesMessage::SetRate {action: String, rate: Rate}
```

or remove rates by calling 

```rust
ExecuteMsg::Rates(RatesMessage::RemoveRate { action: String }
```

## Types of Rates
There are two types: 

`Local` which doesn't involve calling a rates contract. It's also the only allowed `Rate` type to be stored in the rates contract.

`Contract` which fetches a `LocalRate` from a verified rates contract.
## Compatiblity
The `rates` feature has been enabled on the following ADOs: CW721, CW20, Auction, Crowdfund, and Marketplace.

## How Rates Work
Details about the Rates' workings can be found on https://docs.andromedaprotocol.io/andromeda/andromeda-digital-objects/rates





