// SOURCE CODE: https://github.com/confio/cw-ibc-demo/blob/main/tests/src/utils.ts
import { readdirSync, readFileSync } from "fs";

import {
  AckWithMetadata,
  CosmWasmSigner,
  IbcClient,
  Link,
  RelayInfo,
  testutils,
} from "@confio/relayer";
import { IbcClientOptions } from "@confio/relayer/build/lib/ibcclient";
import { SigningCosmWasmClientOptions } from "@cosmjs/cosmwasm-stargate";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { stringToPath } from "@cosmjs/crypto";
import { fromBase64, fromUtf8 } from "@cosmjs/encoding";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { Coin, GasPrice } from "@cosmjs/stargate";
import { assert } from "@cosmjs/utils";
import axios from "axios";

import { ChainDefinition } from "./configs";
import Contract from "./contract";
import { getChainCache, setChainCache } from "./os";

function encode(data: unknown | string) {
  return Buffer.from(JSON.stringify(data)).toString("base64");
}

const { generateMnemonic } = testutils;

const mnemonic =
  "bounce success option birth apple portion aunt rural episode solution hockey pencil lend session cause hedgehog slender journey system canvas decorate razor catch empty";

export const IbcVersion = "simple-ica-v2";

export async function awaitMulti(promises: Promise<unknown>[]) {
  return await Promise.all(promises);
}

async function signingCosmWasmClient(
  chain: ChainDefinition,
  mnemonic: string
): Promise<CosmWasmSigner> {
  const hdPath =
    chain.prefix === "terra" ? [stringToPath("m/44'/330'/0'/0/0")] : undefined;
  const signer = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, {
    prefix: chain.prefix,
    hdPaths: hdPath as any,
  });
  const { address: senderAddress } = (await signer.getAccounts())[0];
  const options: SigningCosmWasmClientOptions = {
    gasPrice: GasPrice.fromString(chain.minFee),
    // This is just for tests - don't add this in production code
    broadcastPollIntervalMs: 100,
    broadcastTimeoutMs: 15000,
  };
  const sign = await SigningCosmWasmClient.connectWithSigner(
    chain.tendermintUrlHttp,
    signer,
    options
  );

  return { sign, senderAddress };
}

async function ibcClient(
  chain: ChainDefinition,
  clientMnemonic: string
): Promise<IbcClient> {
  const hdPath =
    chain.prefix === "terra" ? [stringToPath("m/44'/330'/0'/0/0")] : undefined;
  const signer = await DirectSecp256k1HdWallet.fromMnemonic(clientMnemonic, {
    prefix: chain.prefix,
    hdPaths: hdPath as any,
  });
  const { address: senderAddress } = (await signer.getAccounts())[0];
  const options: IbcClientOptions = {
    gasPrice: GasPrice.fromString(chain.minFee),
    // This is just for tests - don't add this in production code
    estimatedBlockTime: 1000,
    estimatedIndexerTime: 500,
    broadcastTimeoutMs: 15000,
    broadcastPollIntervalMs: 100,
  };
  const sign = await IbcClient.connectWithSigner(
    chain.tendermintUrlHttp,
    signer,
    senderAddress,
    options
  );

  return sign;
}

export async function setupContracts(
  cosmwasm: CosmWasmSigner,
  contracts: Record<string, string>
): Promise<Record<string, number>> {
  const results: Record<string, number> = {};
  for (const name in contracts) {
    const path = contracts[name];
    const wasm = readFileSync(path);
    const receipt = await cosmwasm.sign.upload(
      cosmwasm.senderAddress,
      wasm,
      "auto",
      `Upload ${name}`
    );
    results[name] = receipt.codeId;
  }

  return results;
}

// This creates a client for the CosmWasm chain, that can interact with contracts
export async function setupChainClient(
  chain: ChainDefinition,
  _mnemonic?: string
): Promise<CosmWasmSigner> {
  _mnemonic = _mnemonic || mnemonic || generateMnemonic();
  const cosmwasm = await signingCosmWasmClient(chain, _mnemonic);
  return cosmwasm;
}

// throws error if not all are success
export function assertAckSuccess(acks: AckWithMetadata[]) {
  for (const ack of acks) {
    const parsed = JSON.parse(fromUtf8(ack.acknowledgement));
    if (parsed.error) {
      throw new Error(`Unexpected error in ack: ${parsed.error}`);
    }
    if (!parsed.result) {
      throw new Error(`Ack result unexpectedly empty`);
    }
  }
}

// throws error if not all are errors
export function assertAckErrors(acks: AckWithMetadata[]) {
  for (const ack of acks) {
    const parsed = JSON.parse(fromUtf8(ack.acknowledgement));
    if (parsed.result) {
      throw new Error(`Ack result unexpectedly set`);
    }
    if (!parsed.error) {
      throw new Error(`Ack error unexpectedly empty`);
    }
  }
}

export function assertPacketsFromA(
  relay: RelayInfo,
  count: number,
  success: boolean
) {
  if (relay.packetsFromA !== count) {
    throw new Error(`Expected ${count} packets, got ${relay.packetsFromA}`);
  }
  if (relay.acksFromB.length !== count) {
    throw new Error(`Expected ${count} acks, got ${relay.acksFromB.length}`);
  }
  if (success) {
    assertAckSuccess(relay.acksFromB);
  } else {
    assertAckErrors(relay.acksFromB);
  }
}

export function assertPacketsFromB(
  relay: RelayInfo,
  count: number,
  success: boolean
) {
  if (relay.packetsFromB !== count) {
    throw new Error(`Expected ${count} packets, got ${relay.packetsFromB}`);
  }
  if (relay.acksFromA.length !== count) {
    throw new Error(`Expected ${count} acks, got ${relay.acksFromA.length}`);
  }
  if (success) {
    assertAckSuccess(relay.acksFromA);
  } else {
    assertAckErrors(relay.acksFromA);
  }
}

export function parseAcknowledgementSuccess(
  ack: AckWithMetadata
): Record<string, unknown> {
  const response = parseString(ack.acknowledgement);
  assert(response.result);
  return parseBinary(response.result as string);
}

export function parseString(str: Uint8Array): Record<string, unknown> {
  return JSON.parse(fromUtf8(str));
}

export function parseBinary(bin: string): Record<string, unknown> {
  return JSON.parse(fromUtf8(fromBase64(bin)));
}

export interface AMPIBCConfig {
  recovery_addr?: string;
}

export interface AMPMsg {
  recipient: string;
  message: string;
  funds: { amount: string; denom: string }[];
  config: {
    reply_on: "error" | "always" | "never" | "success";
    exit_at_error: boolean;
    gas_limit?: number;
    direct: boolean;
    ibc_config?: AMPIBCConfig;
  };
}

export function createAMPMsg(
  recipient: string,
  msg: Record<string, unknown> | "" = "",
  funds: { amount: string; denom: string }[] = [],
  ibcConfig?: AMPIBCConfig
): AMPMsg {
  return {
    recipient,
    message: msg === "" ? msg : encode(msg),
    funds,
    config: {
      reply_on: "error",
      exit_at_error: false,
      direct: true,
      ibc_config: ibcConfig,
    },
  };
}

export function createAMPPacket(sender: string, messages: AMPMsg[]) {
  return {
    messages,
    ctx: {
      origin: sender,
      previous_sender: sender,
      id: 0,
    },
  };
}

export async function setupRelayerInfo(
  chainA: ChainDefinition,
  chainB: ChainDefinition,
  buffer: ChainDefinition
) {
  const newMnemonic =
    "cream sport mango believe inhale text fish rely elegant below earth april wall rug ritual blossom cherry detail length blind digital proof identify ride";
  const info = [
    await ibcClient(chainA, newMnemonic),
    await ibcClient(chainB, newMnemonic),
    await ibcClient(buffer, newMnemonic),
  ];
  return info;
}

export function getADOPath(name: string) {
  const files = readdirSync("./contracts");
  const path = files.find((x) => x.includes(name));

  return `./contracts/${path}`;
}

export function getFileVersion(path: string) {
  const split = path.split("@");
  return split[split.length - 1].replace(".wasm", "");
}

export function getFileName(path: string) {
  const split = path.split("@");
  return split[0].replace("andromeda_", "");
}

export function getAllADONames() {
  const files = readdirSync("./contracts");
  return files.map((x) => getFileName(x));
}

export async function uploadADO(
  name: string,
  client: CosmWasmSigner,
  adodb: Contract
) {
  const chainId = await client.sign.getChainId();
  const cache = await getChainCache(chainId);
  if (cache?.ALL_ADO?.[name]) return;
  const path = getADOPath(name);
  const version = getFileVersion(path);

  const { codeId } = await client.sign.upload(
    client.senderAddress,
    readFileSync(path),
    "auto",
    `Upload ${name}`
  );
  const publishMsg = {
    publish: {
      code_id: codeId,
      ado_type: name,
      version,
    },
  };

  await adodb.execute(publishMsg, client);
  if (cache) {
    await setChainCache(chainId, {
      ...cache,
      ALL_ADO: {
        ...cache.ALL_ADO,
        [name]: codeId,
      },
    });
  }
}

export async function uploadAllADOs(client: CosmWasmSigner, adodb: Contract) {
  const names = getAllADONames();
  for (const name of names) {
    await uploadADO(name, client, adodb);
  }
}

async function sleep(timeout: number) {
  return new Promise((resolve) => setTimeout(resolve, timeout));
}
export async function relayAll(link: Link): Promise<[boolean, RelayInfo]> {
  let counter = 0;
  let err;
  while (counter < 6) {
    try {
      const info = await link.relayAll();
      return [counter === 0, info!];
    } catch (error: unknown) {
      err = error;
      const { message } = error as Error;
      if (
        message.includes("incorrect account sequence") ||
        message.includes("can't be greater than max height")
      ) {
        // Increase counter by 1 as this is expected error
        counter = counter + 1;
      } else {
        // Increate counter by 2 as this is unexpected error
        counter = counter + 2;
      }
      console.debug("Retrying relayAll");
      await sleep(1000);
    }
  }

  throw err || new Error("Unreachable");
}

export async function getBalances(chain: ChainDefinition, address: string) {
  const balances = await axios
    .get(`${chain.restUrl}/cosmos/bank/v1beta1/balances/${address}`)
    .then((res) => res.data.balances);
  console.log(address, balances);
  return balances as Coin[];
}

export async function retryTill<T = any>(cb: () => Promise<T> | T, count = 5) {
  let err;
  while (count > 0) {
    try {
      const res = await cb();
      return res;
    } catch (_err) {
      err = _err;
      console.debug("Retrying...");
      await sleep(100);
      count--;
    }
  }
  throw err;
}
