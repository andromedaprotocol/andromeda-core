import { testutils } from "@confio/relayer";
import { ChainDefinition } from "@confio/relayer/build/lib/helpers";

const { osmosis: oldOsmo } = testutils;

const faucetMnemonic =
  "notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius";
const blockTime = 5000;

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
  blockTime,
  estimatedBlockTime: blockTime,
  estimatedIndexerTime: blockTime,
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
  blockTime,
  estimatedBlockTime: blockTime,
  estimatedIndexerTime: blockTime,
};

export default {
  osmosisA,
  osmosisB,
  faucetA: "http://localhost:8000",
  faucetB: "http://localhost:38000",
};
