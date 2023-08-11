import { testutils } from "@confio/relayer";
import { ChainDefinition } from "@confio/relayer/build/lib/helpers";

const { osmosis: oldOsmo } = testutils;

const faucetMnemonic =
  "notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius";

const osmosisA: ChainDefinition = {
  ...oldOsmo,
  minFee: "0.025uosmo",
  tendermintUrlWs: "ws://localhost:26657",
  tendermintUrlHttp: "http://localhost:26657",
  chainId: "localosmosis-a",
  faucet: {
    ...oldOsmo.faucet,
    mnemonic: faucetMnemonic,
    address0: "osmo19wpkq20hq9r08qht3qhrvya7fm00cflvrhu6s3",
  },
  blockTime: 1000,
  estimatedBlockTime: 1000,
  estimatedIndexerTime: 3000,
};

const osmosisB: ChainDefinition = {
  ...oldOsmo,
  minFee: "0.025uosmo",
  tendermintUrlWs: "ws://localhost:36657",
  tendermintUrlHttp: "http://localhost:36657",
  chainId: "localosmosis-b",
  faucet: {
    ...oldOsmo.faucet,
    mnemonic: faucetMnemonic,
    address0: "osmo19wpkq20hq9r08qht3qhrvya7fm00cflvrhu6s3",
  },
  blockTime: 1000,
  estimatedBlockTime: 1000,
  estimatedIndexerTime: 3000,
};

export default {
  osmosisA,
  osmosisB,
  faucetA: "http://localhost:8000",
  faucetB: "http://localhost:38000",
};
