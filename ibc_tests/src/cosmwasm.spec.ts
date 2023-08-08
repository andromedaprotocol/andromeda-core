import { Link, testutils } from "@confio/relayer";
import test from "ava";
import { Order } from "cosmjs-types/ibc/core/channel/v1/channel";
import digest from "sha256";

import configs from "./configs";
import { setupOS } from "./os";
import {
  assertPacketsFromB,
  createAMPMsg,
  createAMPPacket,
  setupOsmosisClient,
  setupOsmosisClientB,
  setupRelayerInfo,
} from "./utils";

let osmosisAAddresses: Record<string, string> = {};
let osmosisBAddresses: Record<string, string> = {};
let link: Link;
let ics20: { wasm: string; osmo: string };

const { setup } = testutils;

const { osmosisA, osmosisB } = configs;

async function setupForTests() {
  const osmoA = await setupOsmosisClient();
  const osmoB = await setupOsmosisClientB();
  const [src, dest] = await setupRelayerInfo(osmosisA, osmosisB);
  console.log("Creating link...");
  link = link ?? (await Link.createWithNewConnections(src, dest));
  console.log("Created link");
  if (!ics20) {
    console.log("Creating ICS20 Channel...");
    const ics20Info = await link.createChannel(
      "A",
      osmosisA.ics20Port,
      osmosisB.ics20Port,
      Order.ORDER_UNORDERED,
      "ics20-1"
    );
    ics20 = {
      wasm: ics20Info.src.channelId,
      osmo: ics20Info.dest.channelId,
    };
    console.log("ICS20 Channel created");
  }
  const osmoAIBCDenom = `ibc/${digest(
    `${osmosisA.ics20Port}/${ics20.wasm}/ucosm`
  ).toUpperCase()}`;
  const osmoBIBCDenom = `ibc/${digest(
    `${osmosisA.ics20Port}/${ics20.wasm}/uosmo`
  ).toUpperCase()}`;
  console.debug("Setup done");
  return { osmoA, osmoB, link, ics20, osmoAIBCDenom, osmoBIBCDenom };
}

test.before(async (t) => {
  const { osmoA, osmoB } = await setupForTests();
  osmosisAAddresses = await setupOS(osmoA);
  osmosisBAddresses = await setupOS(osmoB);
  t.pass();
});

test.serial("Should have assigned all addresses", async (t) => {
  const { osmoA, osmoB } = await setupForTests();

  for (const name in osmosisAAddresses) {
    if (name === "kernel") continue;
    const query = {
      key_address: {
        key: name,
      },
    };
    const resWasm = await osmoA.sign.queryContractSmart(
      osmosisAAddresses.kernel,
      query
    );
    t.is(resWasm, osmosisAAddresses[name]);
    const resOsmo = await osmoB.sign.queryContractSmart(
      osmosisBAddresses.kernel,
      query
    );
    t.is(resOsmo, osmosisBAddresses[name]);
  }
});

test.serial("Set up an IBC relayer and assign the channel", async (t) => {
  const { osmoA, osmoB, ics20 } = await setupForTests();

  const bridgeAddressWasm = osmosisAAddresses["ibc-bridge"];
  const bridgeAddressOsmo = osmosisBAddresses["ibc-bridge"];
  const kernelAddressWasm = osmosisAAddresses["kernel"];
  const kernelAddressOsmo = osmosisBAddresses["kernel"];

  const wasmBridgeMsg = {
    save_channel: {
      channel: ics20.wasm,
      kernel_address: kernelAddressOsmo,
      chain: "osmosisB",
    },
  };
  const osmoBridgeMsg = {
    save_channel: {
      channel: ics20.osmo,
      kernel_address: kernelAddressWasm,
      chain: "wasm",
    },
  };

  await osmoA.sign.execute(
    osmoA.senderAddress,
    bridgeAddressWasm,
    wasmBridgeMsg,
    "auto"
  );
  t.log(`Channel ${ics20.wasm} saved to bridge on wasm`);
  await osmoB.sign.execute(
    osmoB.senderAddress,
    bridgeAddressOsmo,
    osmoBridgeMsg,
    "auto"
  );
  t.log(`Channel ${ics20.osmo} saved to bridge on osmo`);

  t.pass();
});

test.serial("Send a packet from wasm to osmo", async (t) => {
  const { osmoA, osmoB, osmoBIBCDenom, link } = await setupForTests();
  const transferAmount = { amount: "100", denom: "ucosm" };
  const msg = createAMPMsg(
    `ibc://osmosisB/${osmoB.senderAddress}`,
    { send: {} },
    [transferAmount]
  );
  const pkt = createAMPPacket(osmoA.senderAddress, [msg]);
  const kernelMsg = { amp_receive: pkt };
  const res = await osmoA.sign.execute(
    osmoA.senderAddress,
    osmosisAAddresses["kernel"],
    kernelMsg,
    "auto",
    "",
    [transferAmount]
  );
  t.truthy(res.transactionHash);
  const info = await link.relayAll();
  assertPacketsFromB(info, 1, true);

  const omsoBalance = await osmoB.sign.getBalance(
    osmosisBAddresses["kernel"],
    osmoBIBCDenom
  );
  t.assert(
    omsoBalance.amount === transferAmount.amount,
    "Balance is incorrect"
  );
});
