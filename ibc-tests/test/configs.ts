import { testutils } from "@confio/relayer";
import { ChainDefinition as RelayerChainDefinition } from "@confio/relayer/build/lib/helpers";

// const BASE_URL = "localhost";
const BASE_URL = "18.212.50.191";

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
  tendermintUrlWs: `ws://${BASE_URL}:20121`,
  tendermintUrlHttp: `http://${BASE_URL}:20121`,
  restUrl: `http://${BASE_URL}:20221`,
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
  tendermintUrlWs: `ws://${BASE_URL}:20122`,
  tendermintUrlHttp: `http://${BASE_URL}:20122`,
  restUrl: `http://${BASE_URL}:20222`,
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
  tendermintUrlWs: `ws://${BASE_URL}:20111`,
  tendermintUrlHttp: `http://${BASE_URL}:20111`,
  restUrl: `http://${BASE_URL}:20211`,
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
  tendermintUrlWs: `ws://${BASE_URL}:20131`,
  tendermintUrlHttp: `http://${BASE_URL}:20131`,
  restUrl: `http://${BASE_URL}:20231`,
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

const junoA: ChainDefinition = {
  tendermintUrlWs: `ws://${BASE_URL}:20141`,
  tendermintUrlHttp: `http://${BASE_URL}:20141`,
  restUrl: `http://${BASE_URL}:20241`,
  chainId: "localjuno-1",
  prefix: "juno",
  denomStaking: "stake",
  denomFee: "ujunox",
  minFee: "2ujunox",
  blockTime,
  faucet: {
    mnemonic:
      "enlist hip relief stomach skate base shallow young switch frequent cry park",
    pubkey0: {
      type: "tendermint/PubKeySecp256k1",
      value: "A9cXhWb8ZpqCzkA8dQCPV29KdeRLV3rUYxrkHudLbQtS",
    },
    address0: "juno14qemq0vw6y3gc3u3e0aty2e764u4gs5lndxgyk",
  },
  ics20Port: "transfer",
  estimatedBlockTime: blockTime,
  estimatedIndexerTime: blockTime,
};

const junoB: ChainDefinition = {
  tendermintUrlWs: `ws://${BASE_URL}:20142`,
  tendermintUrlHttp: `http://${BASE_URL}:20142`,
  restUrl: `http://${BASE_URL}:20242`,
  chainId: "localjuno-2",
  prefix: "juno",
  denomStaking: "stake",
  denomFee: "ujunox",
  minFee: "2ujunox",
  blockTime,
  faucet: {
    mnemonic:
      "enlist hip relief stomach skate base shallow young switch frequent cry park",
    pubkey0: {
      type: "tendermint/PubKeySecp256k1",
      value: "A9cXhWb8ZpqCzkA8dQCPV29KdeRLV3rUYxrkHudLbQtS",
    },
    address0: "juno14qemq0vw6y3gc3u3e0aty2e764u4gs5lndxgyk",
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
  junoA,
  junoB,
};
