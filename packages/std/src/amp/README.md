# Andromeda Messaging Protocol

The primary use of this crate is to define the Andromeda Messaging Protocol; a simple wrapper around contract communication that allows for simpler message routing and a larger context.

## Usage

There are two core components to the Andromeda Messaging Protocol the [`AndrAddr`](https://docs.andromedaprotocol.io/andromeda/platform-and-framework/common-types#andraddr) struct and the `AMPPkt` struct:

```rust
// Found in addresses.rs
pub struct AndrAddr(String);

// Found in messages.rs
pub struct AMPPkt {
    /// Any messages associated with the packet
    pub messages: Vec<AMPMsg>,
    pub ctx: AMPCtx,
}
```

### Andromeda Addresses

The `AndrAddr` struct is a wrapper around a `String` to allow a user to define with a standard human readable address or a valid VFS path. This struct provides a lot of utility methods to help with validating and resolving these paths. The primary logic for validation is to assume the provided `String` is an address and validate it like so. If this fails we validate that it is a correct VFS path. An example use of this would be:

```rust
let user_andr_addr = AndrAddr::from_string("~user1/app1/component");
let addr = user_andr_addr.get_raw_address(&deps)?;

let raw_andr_addr = AndrAddr::from_string("andr1someaddress123");
let addr = raw_andr_addr.get_raw_address(&deps)?;
```

In the first case we provide a VFS path, this will query the VFS contract that is registered with the kernel to resolve the address. The second is validated as a human readable address and returned straight away.

### AMP Packet

An AMP (Andromeda Messaging Protocol) Packet allows a user to specify several messages to be routed via the kernel. The benefit of this is to allow VFS routing (including IBC) and also to allow the receiving contract access to `AMPCtx`:

```rust
#[cw_serde]
pub struct AMPCtx {
    origin: String,
    pub previous_sender: String,
    pub id: u64,
}
```

This provides the `origin` field similar to a smart contract written in Solidity. The `origin` field specifies the original sender of the message (not the current sub message). However, as this is a security risk this context struct is only provided when one of three conditions are met upon the packet reaching the kernel:

1. The origin field is equivalent to the message sender
2. The sender is a valid ADO
3. The sender is the kernel (Only applicable when an ADO receives the packet)

The other benefit to using AMP is the use of `AndrAddr` in the messages:

```rust
pub struct AMPMsg {
    /// The message recipient, can be a contract/wallet address or a namespaced URI
    pub recipient: AndrAddr,
    /// The message to be sent to the recipient
    pub message: Binary,
    /// Any funds to be attached to the message, defaults to an empty vector
    pub funds: Vec<Coin>,
    /// When the message should reply, defaults to Always
    pub config: AMPMsgConfig,
}
```

Here the `recipient` field is using an Andromeda Address, when the kernel receives this address it is resolved and routed accordingly. The rest of the struct is extremely similar to a standard `WasmMsg` with the exception of `message`. When `Binary::default()` is provided as the message then the kernel assumes the provided message is a `BankMsg::Send` and routes accordingly.

For more advanced users `AndrAddr` can be used via the kernel to route messages over IBC:

```rust
let ibc_addr = AndrAddr::from_string("ibc://terra/home/terra-user/terra-app/terra-component");
```

Our kernel will receive this address and route the message accordingly (to the Terra blockchain in this example).