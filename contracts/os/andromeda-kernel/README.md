# Overview
The Andromeda Kernel acts as the core of the operating system. It receives and handles packets from ADOs to be relayed to a specified recipient. The Kernel keeps track of the original sender of the message. It also verifies that the packet is sent by an Andromeda certified ADO before relaying the message. 
The Kernel is also responsible for:

- Relaying any IBC messages across any two chains that have an Andromeda Kernel deployed and a channel set up.
- Keeping track of the other AMP ADOs such as the ADODB, VFS, and Economics.

All of our ADOs have an AMPReceive execute message to handle receiving packets from the Kernel.

[Kernel Full Documentation](https://docs.andromedaprotocol.io/andromeda/platform-and-framework/andromeda-messaging-protocol/kernel)