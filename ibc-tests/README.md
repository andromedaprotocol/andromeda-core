# Summary
This package is intended for e2e testing of ADOs. 

# Prerequest
1. You need to run the local network for e2e test. Currently this package is using `develop` branch of [localrelayer](https://github.com/andromedaprotocol/localrelayer/).
***Do not forget to switch to `develop` branch. Otherwise, it won't work.***
2. Copy necessary artifacts to `ibc-tests/artifacts` and  `packages/andromeda-testing-e2e/artifacts`.

# Workflow
To run e2e tests, follow the following steps
1. Run the main function to setup aOS and necesary environment for each testing.
2. Run the test