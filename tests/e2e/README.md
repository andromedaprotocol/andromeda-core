# Summary
This package is designed for end-to-end (e2e) testing of ADOs. It supports deployment and testing across various blockchain networks.

# Prerequisites
1. Ensure that the necessary artifacts are available in the following directories:
    - `tests/e2e/artifacts`
    - `packages/andromeda-testing-e2e/artifacts`

    The `build.sh` script has been updated to automatically copy artifacts to the required locations in building process.

2. To test ADOs in a local environment, you must run a local network. This package currently relies on the `develop` branch of [localrelayer](https://github.com/andromedaprotocol/localrelayer/).

***Important:*** Switch to the `develop` branch before proceeding.

# How to start
You can begin testing by simply executing the required test cases.

# How to write custom test
The file `crowdfund.rs` fully leverages the functionality of the e2e test framework.
## `setup` function
The `setup` function is responsible for creating the necessary testing environment and injecting parameters into the test cases. You can define the parameters and implementation of the `setup` function based on your specific requirements.

***Warning*** Remember to annotate the `setup` function with `#[fixture]`.

## Injecting environment into test cases
To inject the test environment into each test, the function signature should follow this format:

```rust
#[rstest]
fn <test_name>(#[with(<setup_params>)] setup: TestCase)
```
In the `setup_params`, you can specify parameters that will be passed to the setup function. For the `crowdfund` test cases, you can include parameters such as `use_native_token`, `chain_info`, and `purchaser_balance`. If any parameters are omitted, they will default to predefined values.

## Custom chain informations
Chain configurations are defined in `tests/e2e/src/constants.rs`. You can add or customize new chain information as needed.

## Custom Interfaces
To test ADOs, their uploadable interfaces should be defined within `tests/e2e/src/interfaces`.