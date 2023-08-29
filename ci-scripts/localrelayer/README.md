Chain Id - localterra-1
<br/>RPC - http://localhost:20131
<br/>LCD - http://localhost:20231
<br/>GRPC - http://localhost:20331

<br/>Chain Id - localandromeda-1
<br/>RPC - http://localhost:20111
<br/>LCD - http://localhost:20211
<br/>GRPC - http://localhost:20311

<br/>Chain Id - localosmosis-1
<br/>RPC - http://localhost:20121
<br/>LCD - http://localhost:20221
<br/>GRPC - http://localhost:20321

<br/>Chain Id - localosmosis-2
<br/>RPC - http://localhost:20122
<br/>LCD - http://localhost:20222
<br/>GRPC - http://localhost:20322

HERMES ENDPOINT = http://localhost:5000

For more details about the setup, read [this doc](https://github.com/osmosis-labs/osmosis/blob/main/tests/localrelayer/README.md)

## Port Standard

DIFFERENT CHAIN INSTANCE CAN BE ADDED BUT BE CAREFUL ABOUT THE PORTS.

As single port has been referenced in many places, we have decided on a simple standard.

All chains will have three exposed ports in this structure

- `201-CHAININDEX-CHAINCOPY` for RPC
- `202-CHAININDEX-CHAINCOPY` for LCD
- `203-CHAININDEX-CHAINCOPY` for GRPC

For the same chain, if faucet is needed it will have same chain index
`80-CHAININDEX-CHAINCOPY`

## State Sync

We are not adding latest states in the git commits, if you want to collaborate with other devs for testing, share `.config` folder and start a new instance

## Genesis Address

You can add your address to the genesis list to get some initial tokens, however this will need a new instance to be created. You can also use faucet if needed.

## Sample hermes command

OSMO to OSMO

```bash
hermes tx ft-transfer --timeout-seconds 1000 --dst-chain localosmosis-1 --src-chain localosmosis-2 --src-port transfer --src-channel channel-0 --amount 100 --denom uosmo
```

ANDR to OSMO

```bash
hermes tx ft-transfer --timeout-seconds 1000 --dst-chain localosmosis-1 --src-chain localandromeda-1 --src-port transfer --src-channel channel-1 --amount 100 --denom uandr
```

OSMO to ANDR

```bash
hermes tx ft-transfer --timeout-seconds 1000 --dst-chain localandromeda-1  --src-chain localosmosis-1 --src-port transfer --src-channel channel-1 --amount 100 --denom uosmo
```

OSMO to TERRA

```bash
hermes tx ft-transfer --timeout-seconds 1000 --dst-chain localterra-1  --src-chain localosmosis-1 --src-port transfer --src-channel channel-2 --amount 100 --denom uosmo
```

TERRA to OSMO

```bash
hermes tx ft-transfer --timeout-seconds 1000 --src-chain localterra-1  --dst-chain localosmosis-1 --src-port transfer --src-channel channel-0 --amount 100 --denom uluna
```

TERRA to ANDR

```bash
hermes tx ft-transfer --timeout-seconds 1000 --src-chain localterra-1  --dst-chain localandromeda-1 --src-port transfer --src-channel channel-1 --amount 100 --denom uluna
```

## Hermes channel

Hermes channel assigned depend on the current state of the chains. Ideally if you have setup chain and hermes without preloaded configs, these channels will be assigned

<br/>`localosmosis-1: transfer/channel-0 --- localosmosis-2: transfer/channel-0`
<br/>`localosmosis-1: transfer/channel-1 --- localandromeda-1: transfer/channel-0`
<br/>`localosmosis-1: transfer/channel-2 --- localterra-1: transfer/channel-0`
<br/>`localosmosis-2: transfer/channel-0 --- localosmosis-1: transfer/channel-0`
<br/>`localandromeda-1: transfer/channel-1 --- localterra-1: transfer/channel-1`

To query what are the current channels avaialble, use this command in hermes terminal

```bash
hermes query channels --chain localosmosis-1 --show-counterparty
hermes query channels --chain localosmosis-2 --show-counterparty
hermes query channels --chain localandromeda-1 --show-counterparty
hermes query channels --chain localterra-1 --show-counterparty
```

```bash
hermes update client --host-chain localterra-1 --client 07-tendermint-0
```

```bash
hermes query connections --chain localosmosis-1 --counterparty-chain localosmosis-2
hermes query connections --counterparty-chain localosmosis-1 --chain localosmosis-2

hermes query connections --chain localosmosis-1 --counterparty-chain localandromeda-1
hermes query connections --counterparty-chain localosmosis-1 --chain localandromeda-1

hermes query connections --chain localosmosis-1 --counterparty-chain localterra-1
hermes query connections --counterparty-chain localosmosis-1 --chain localterra-1

```
