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
import { ChainDefinition } from "@confio/relayer/build/lib/helpers";
import { IbcClientOptions } from "@confio/relayer/build/lib/ibcclient";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { fromBase64, fromUtf8 } from "@cosmjs/encoding";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";
import { assert } from "@cosmjs/utils";

import configs from "./configs";
import Contract from "./contract";

const { osmosisA, osmosisB } = configs;

function encode(data: unknown | string) {
  return Buffer.from(JSON.stringify(data)).toString("base64");
}

const { generateMnemonic } = testutils;

let mnemonic =
  "notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius";

export const IbcVersion = "simple-ica-v2";

export async function awaitMulti(promises: Promise<unknown>[]) {
  return await Promise.all(promises);
}

async function signingCosmWasmClient(
  chain: ChainDefinition,
  mnemonic: string
): Promise<CosmWasmSigner> {
  const signer = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, {
    prefix: chain.prefix,
  });
  const { address: senderAddress } = (await signer.getAccounts())[0];
  const options = {
    prefix: chain.prefix,
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
  const signer = await DirectSecp256k1HdWallet.fromMnemonic(clientMnemonic, {
    prefix: chain.prefix,
  });
  const { address: senderAddress } = (await signer.getAccounts())[0];
  const options: IbcClientOptions = {
    gasPrice: GasPrice.fromString(chain.minFee),
    // This is just for tests - don't add this in production code
    estimatedBlockTime: 1000,
    estimatedIndexerTime: 500,
    broadcastTimeoutMs: 15000,
    broadcastPollIntervalMs: 100,
    prefix: chain.prefix,
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
export async function setupOsmosisClient(): Promise<CosmWasmSigner> {
  // create apps and fund an account
  mnemonic = mnemonic.length > 0 ? mnemonic : generateMnemonic();
  const cosmwasm = await signingCosmWasmClient(osmosisA, mnemonic);
  // console.debug("Funding account on chain A...");
  // await fundAccount(osmosisA, cosmwasm.senderAddress, "4000000");
  // console.debug("Funded account on chain A");
  return cosmwasm;
}

// This creates a client for the CosmWasm chain, that can interact with contracts
export async function setupOsmosisClientB(): Promise<CosmWasmSigner> {
  // create apps and fund an account
  mnemonic = mnemonic.length > 0 ? mnemonic : generateMnemonic();
  const cosmwasm = await signingCosmWasmClient(osmosisB, mnemonic);
  // console.debug("Funding account on chain B...");
  // await fundAccount(osmosisB, cosmwasm.senderAddress, "4000000");
  // console.debug("Funded account on chain B");
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

export interface AMPMsg {
  recipient: string;
  message: string;
  funds: { amount: string; denom: string }[];
  config: {
    reply_on: "error" | "always" | "never" | "success";
    exit_at_error: boolean;
    gas_limit?: number;
    direct: boolean;
  };
}

export function createAMPMsg(
  recipient: string,
  msg: Record<string, unknown> | "" = "",
  funds: { amount: string; denom: string }[] = []
): AMPMsg {
  return {
    recipient,
    message: msg === "" ? msg : encode(msg),
    funds,
    config: {
      reply_on: "error",
      exit_at_error: false,
      direct: true,
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
  chainB: ChainDefinition
) {
  const newMnemonic =
    "black frequent sponsor nice claim rally hunt suit parent size stumble expire forest avocado mistake agree trend witness lounge shiver image smoke stool chicken";
  const info = [
    await ibcClient(chainA, newMnemonic),
    await ibcClient(chainB, newMnemonic),
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
  while (counter < 10) {
    try {
      const info = await link.relayAll();

      return [counter === 0, info!];
    } catch (error: unknown) {
      const { message } = error as Error;
      if (message.includes("incorrect account sequence")) {
        console.debug("Retrying relayAll");
        counter++;
        await sleep(1000);
      } else {
        throw error;
      }
    }
  }

  throw new Error("Unreachable");
}
