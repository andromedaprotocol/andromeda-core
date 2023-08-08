import { testutils } from "@confio/relayer";
import { ChainDefinition } from "@confio/relayer/build/lib/helpers";

const { osmosis: oldOsmo } = testutils;

const faucetMnemonic =
  "increase bread alpha rigid glide amused approve oblige print asset idea enact lawn proof unfold jeans rabbit audit return chuckle valve rather cactus great";

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
  blockTime: 15000,
  estimatedBlockTime: 15000,
  estimatedIndexerTime: 250,
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
  blockTime: 15000,
  estimatedBlockTime: 15000,
  estimatedIndexerTime: 250,
};

export default {
  osmosisA,
  osmosisB,
  faucetA: "http://localhost:8000",
  faucetB: "http://localhost:38000",
};
