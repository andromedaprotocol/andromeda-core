# Summary
This package is intended for e2e testing of ADOs. It can be used for deploying and testing on various chains.

# Prerequest
1. You need to have necessary artifacts in `ibc-tests/artifacts` and  `packages/andromeda-testing-e2e/artifacts`.
`build.sh` script is updated to copy artifacts to the necessary position.
2. To test ADOs on local environment, you need to run the local network. Currently this package is using `develop` branch of [localrelayer](https://github.com/andromedaprotocol/localrelayer/).
***Do not forget to switch to `develop` branch. Otherwise, it won't work.***

# How to start
You can just run the necessary test.

# How to write custom test
Currently `crowdfund.rs` is fully utilizing the functionality of e2e test framework.
## `setup` function
It is used to generate necessary testing environment and injecting parameters to the test cases.
Parameters of the `setup` function and implementation can be defined depending on detailed requirements.

***Warning*** Do not forget to add `#[fixture]` for the `setup` function

## Injecting environment into test cases
For each test, to inject test environment, function signatures should follow the following signature format.

```rust
#[rstest]
fn <test_name>(#[with(<setup_params>)] setup: TestCase)
```
For `setup_params` you can pass the parameters that is to be passed to the `setup` function. For `crowdfund` test cases, you can pass parameters for `use_native_token`, `chain_info`, and `purchaser_balance`. If one or more parameter is not specified, it is set as default value.

## Custom chain informations
Chain configurations are definded in `ibc-tests/src/constants.rs`. You can add/customize new chain informations.  

## Custom Interfaces
For ADOs to be tested, their uploadable interface should be defined inside `ibc-tests/src/interfaces`.