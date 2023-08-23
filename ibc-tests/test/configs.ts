import { testutils } from "@confio/relayer";
import { ChainDefinition as RelayerChainDefinition } from "@confio/relayer/build/lib/helpers";

export interface ChainDefinition extends RelayerChainDefinition {
  restUrl: string;
}

const { osmosis: oldOsmo } = testutils;

const faucetMnemonic =
  "notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius";
const blockTime = 5000;

const osmosisA: ChainDefinition = {
  ...oldOsmo,
  minFee: "0.25uosmo",
  tendermintUrlWs: "ws://localhost:20121",
  tendermintUrlHttp: "http://localhost:20121",
  restUrl: "http://localhost:20221",
  chainId: "localosmosis-1",
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
  minFee: "0.25uosmo",
  tendermintUrlWs: "ws://localhost:20122",
  tendermintUrlHttp: "http://localhost:20122",
  restUrl: "http://localhost:20222",
  chainId: "localosmosis-2",
  faucet: {
    ...oldOsmo.faucet,
    mnemonic: faucetMnemonic,
    address0: "osmo19wpkq20hq9r08qht3qhrvya7fm00cflvrhu6s3",
  },
  blockTime,
  estimatedBlockTime: blockTime,
  estimatedIndexerTime: blockTime,
};

const andromedaA: ChainDefinition = {
  tendermintUrlWs: "ws://localhost:20111",
  tendermintUrlHttp: "http://localhost:20111",
  restUrl: "http://localhost:20211",
  chainId: "localandromeda-1",
  prefix: "andr",
  denomStaking: "stake",
  denomFee: "uandr",
  minFee: "0.25uandr",
  blockTime,
  faucet: {
    mnemonic:
      "enlist hip relief stomach skate base shallow young switch frequent cry park",
    pubkey0: {
      type: "tendermint/PubKeySecp256k1",
      value: "A9cXhWb8ZpqCzkA8dQCPV29KdeRLV3rUYxrkHudLbQtS",
    },
    address0: "andr14qemq0vw6y3gc3u3e0aty2e764u4gs5lndxgyk",
  },
  ics20Port: "transfer",
  estimatedBlockTime: blockTime,
  estimatedIndexerTime: blockTime,
};

const terraA: ChainDefinition = {
  tendermintUrlWs: "ws://localhost:20131",
  tendermintUrlHttp: "http://localhost:20131",
  restUrl: "http://localhost:20231",
  chainId: "localterra-1",
  prefix: "terra",
  denomStaking: "stake",
  denomFee: "uluna",
  minFee: "0.25uluna",
  blockTime,
  faucet: {
    mnemonic:
      "enlist hip relief stomach skate base shallow young switch frequent cry park",
    pubkey0: {
      type: "tendermint/PubKeySecp256k1",
      value: "A9cXhWb8ZpqCzkA8dQCPV29KdeRLV3rUYxrkHudLbQtS",
    },
    address0: "terra14qemq0vw6y3gc3u3e0aty2e764u4gs5lndxgyk",
  },
  ics20Port: "transfer",
  estimatedBlockTime: blockTime,
  estimatedIndexerTime: blockTime,
};

export default {
  osmosisA,
  osmosisB,
  andromedaA,
  terraA,
};
