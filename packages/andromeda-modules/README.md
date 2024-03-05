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
The first step is to save the desired actor to permission pair in the address-list contract's `pub const PERMISSIONS: Map<&Addr, Permission> = Map::new("permissioning")`.
Note that `Permission` of type `Contract` isn't allowed in the address-list contract.

Apply `ExecuteMsg::AddActorPermission { actor, permission }` to the ADO you want. 
To involve the address-list contract, set the Permission to be of type Contract, and input the address-list's contract address. `Permission::Contract(address_list_address)`.
Make sure that the `actor` is the same as the one set in the address-list contract.


The completion of the aforementioned steps will enable permissioning on the ADO of your choice.
