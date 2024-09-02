# Overview

The Primitive ADO is a smart contract that is used to store data. It is a simple contract that allows us to store key/value pairs acting like a database. It supports storage of the following data types:

```rust
pub enum Primitive {
    Uint128(Uint128),
    Decimal(Decimal),
    Coin(Coin),
    Addr(Addr),
    String(String),
    Bool(bool),
    Binary(Binary),
}
```
The Primitive ADO can be set to one of the following:
- Private: Only accessible by the contract owner of the ADO.
- Public: Accessible by anyone.
- Restricted: Only accessible to the key owner.

**Note**: This ADO is not released yet. Once released, a link to full documentation will be provided.