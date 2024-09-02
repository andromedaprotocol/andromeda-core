# Andromeda STD

This crate defines how a smart contract can be integrated in to the aOS and also provides several methods to aid in doing so.

## Usage

### ADO Base

This module contains all the message and struct definitions used by the base aOS contracts and ADOs. The primary struct provided by this module are the `AndromedaMsg`, `AndromedaQuery`and `InstantiateMsg` structs.

There are also several response structs to the provided queries available through this crate.

### ADO Contract

This module contains the base logic of an ADO, primarily provided via the `ADOContract` struct.

### AMP

This module contains packet and message structs used to define how the Andromeda Message Protocol can be utilised, there are three primary structs:

Using these a contract can easily make use of the Andromeda Messaging protocol.

### Common

Contains several utility methods and structs to help with consistency across our ADOs.

### OS

This module contains the message definitions and a few utility methods for the core of the aOS.