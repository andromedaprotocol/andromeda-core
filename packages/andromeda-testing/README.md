# Andromeda Testing

A `cw-multi-test` based testing library for integration testing custom ADO contracts and how they interact with the aOS.

## Getting Started

To get started you must first create an instance of the `MockAndromeda` struct:

```rust
use cosmwasm_std::Addr;
use cw_multi_test::App;
use andromeda_testing::MockAndromeda;

fn mock_andromeda(app: &mut App, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}
```

This process will set up the aOS mock contracts and make them accessible through this struct.

```rust
pub struct MockAndromeda {
    pub admin_address: Addr,
    pub kernel: MockKernel,
    pub adodb: MockADODB,
    pub economics: MockEconomics,
    pub vfs: MockVFS,
}
```

The `admin_address` has ownership of the aOS and as such can make adjustments as required.

The next step is to add some ADOs to the setup, this can be done with the `store_ado` method:

```rust
let andr = mock_andromeda();
andr.store_ado(&mut router, mock_andromeda_app(), "app");
```

Here the second parameter is a `cw-multi-test` mock contract and the third is a name for the ADO. This can be used to access the code id by calling `andr.get_code_id(&router, "app")`. Repeat this process for any ADOs you wish to add (including your own). 

## Creating a Mock Contract

All our mock contracts implement two structs; `MockContract` and `MockADO`. The `MockContract` struct exposes simple query and execute methods while `MockADO` exposes any messages that an ADO may provide such as ownership and permissioning. To implement both these structs in a quick and easy manner we have provided a macro:

```rust
use andromeda_test::{mock_ado, MockADO, MockContract};
use crate::msg::{ExecuteMsg, QueryMsg};

pub struct MyMockADO(Addr);
mock_ado!(MyMockADO, ExecuteMsg, QueryMsg);

impl MyMockADO {
 // Expose any useful methods in here
}
```

Once this is done you can either create the contract directly or you can create it as part of an app. If you create it using the `MockApp` struct provided in `andromeda-app-contract` the mock contract struct can be created via a query:

```rust
  let app = MockApp::instantiate(...);
  let my_ado: MyMockADO = app.query_ado_by_component_name(&router, my_ado_name);
```

Mock structs have been provided for most ADOs however they are still a work in progress.
