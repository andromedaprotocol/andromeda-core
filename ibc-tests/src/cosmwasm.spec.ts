import { Link } from "@confio/relayer";
import { randomAddress } from "@confio/relayer/build/lib/helpers";
import test from "ava";
import digest from "sha256";

import configs from "./configs";
import { setupOS } from "./os";
import { waitForRelayer } from "./relayer";
import {
  assertPacketsFromA,
  awaitMulti,
  createAMPMsg,
  createAMPPacket,
  setupOsmosisClient,
  setupOsmosisClientB,
  setupRelayerInfo,
} from "./utils";

let osmosisAAddresses: Record<string, string> = {};
let osmosisBAddresses: Record<string, string> = {};
let link: Link;
const ics20: { wasm: string; osmo: string } = {
  wasm: "channel-0",
  osmo: "channel-0",
};

const { osmosisA, osmosisB } = configs;

async function setupForTests() {
  const [osmoA, osmoB] = await awaitMulti([
    setupOsmosisClient(),
    setupOsmosisClientB(),
  ]);

  if (!link) {
    const [src, dest] = await setupRelayerInfo(osmosisA, osmosisB);
    console.log("Creating link...");
    link = await Link.createWithExistingConnections(
      src,
      dest,
      "connection-0",
      "connection-0"
    );
    console.log("Created link");
  }
  const osmoAIBCDenom = `ibc/${digest(
    `${osmosisA.ics20Port}/${ics20.wasm}/ucosm`
  ).toUpperCase()}`;
  const osmoBIBCDenom = `ibc/${digest(
    `${osmosisA.ics20Port}/${ics20.wasm}/uosmo`
  ).toUpperCase()}`;
  return { osmoA, osmoB, link, ics20, osmoAIBCDenom, osmoBIBCDenom };
}

test.before(async (t) => {
  await waitForRelayer();
  const { osmoA, osmoB } = await setupForTests();
  const [addressesA, addressesB] = await awaitMulti([
    setupOS(osmoA),
    setupOS(osmoB),
  ]);
  osmosisAAddresses = addressesA;
  osmosisBAddresses = addressesB;
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

  const bridgeAddressOsmoA = osmosisAAddresses["ibc-bridge"];
  const bridgeAddressOsmoB = osmosisBAddresses["ibc-bridge"];
  const kernelAddressOsmoA = osmosisAAddresses["kernel"];
  const kernelAddressOsmoB = osmosisBAddresses["kernel"];

  const osmoABridgeMsg = {
    save_channel: {
      channel: ics20.wasm,
      kernel_address: kernelAddressOsmoA,
      chain: "osmosis-b",
    },
  };
  await osmoA.sign.execute(
    osmoA.senderAddress,
    bridgeAddressOsmoA,
    osmoABridgeMsg,
    "auto"
  );
  t.log(`Channel ${ics20.wasm} saved to bridge on osmo-a`);
  t.log(`Kernel address (${kernelAddressOsmoA}) saved to bridge on osmo-a`);

  const osmoBBridgeMsg = {
    save_channel: {
      channel: ics20.osmo,
      kernel_address: kernelAddressOsmoB,
      chain: "osmosis-a",
    },
  };
  await osmoB.sign.execute(
    osmoB.senderAddress,
    bridgeAddressOsmoB,
    osmoBBridgeMsg,
    "auto"
  );
  t.log(`Channel ${ics20.osmo} saved to bridge on osmo-b`);
  t.log(`Kernel address (${kernelAddressOsmoB}) saved to bridge on osmo-b`);
  t.pass();
});

test.serial("Send a packet from osmo-a to osmo-a", async (t) => {
  const { osmoA } = await setupForTests();
  const receiver = randomAddress("osmo");
  const transferAmount = { amount: "100", denom: "uosmo" };
  const msg = createAMPMsg(`/${receiver}`, undefined, [transferAmount]);
  const kernelMsg = { send: { message: msg } };
  const res = await osmoA.sign.execute(
    osmoA.senderAddress,
    osmosisAAddresses["kernel"],
    kernelMsg,
    "auto",
    "",
    [transferAmount]
  );
  t.truthy(res.transactionHash);

  const balance = await osmoA.sign.getBalance(receiver, transferAmount.denom);
  t.assert(balance.amount === transferAmount.amount, "Balance is incorrect");

  t.log(`Sent ${transferAmount.amount} ${transferAmount.denom} to ${receiver}`);
});

test.serial("Send a packet from osmo-b to osmo-b", async (t) => {
  const { osmoB } = await setupForTests();
  const receiver = randomAddress("osmo");
  const transferAmount = { amount: "100", denom: "uosmo" };
  const msg = createAMPMsg(`/${receiver}`, undefined, [transferAmount]);
  const kernelMsg = { send: { message: msg } };
  const res = await osmoB.sign.execute(
    osmoB.senderAddress,
    osmosisBAddresses["kernel"],
    kernelMsg,
    "auto",
    "",
    [transferAmount]
  );
  t.truthy(res.transactionHash);

  const balance = await osmoB.sign.getBalance(receiver, transferAmount.denom);
  t.assert(balance.amount === transferAmount.amount, "Balance is incorrect");

  t.log(`Sent ${transferAmount.amount} ${transferAmount.denom} to ${receiver}`);
});

test.serial("Send a packet from osmo-a to osmo-b", async (t) => {
  const { osmoA, osmoB, osmoBIBCDenom, link } = await setupForTests();
  const receiver = randomAddress("osmo");
  const transferAmount = { amount: "100", denom: "uosmo" };
  const msg = createAMPMsg(`ibc://osmosis-b/${receiver}`, undefined, [
    transferAmount,
  ]);
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
  assertPacketsFromA(info, 1, true);
  const omsoBalance = await osmoB.sign.getBalance(receiver, osmoBIBCDenom);
  t.log(omsoBalance);
  t.assert(
    omsoBalance.amount === transferAmount.amount,
    "Balance is incorrect"
  );
  t.log(
    `Sent ${transferAmount.amount} ${transferAmount.denom} to ${receiver} as ${osmoBIBCDenom} from osmo-a to osmo-b`
  );
});
