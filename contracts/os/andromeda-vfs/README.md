# Overview

The Virtual File System (VFS) is a part of the Andromeda Messaging System (AMP) which was heavily inspired by the linux file system. Users can register their address to a username. They can also register ADOs to paths. These paths can then be used and referenced in our ADO systems.
When an Andromeda App is made, it will register all paths for its child components and also register itself as a child of the instantiating address. Each component under the App is registered by its name, and the App itself is registered under its assigned name.

[VFS Full Documentation](https://docs.andromedaprotocol.io/andromeda/platform-and-framework/andromeda-messaging-protocol/virtual-file-system)