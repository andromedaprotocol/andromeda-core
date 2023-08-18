import { Link } from "@confio/relayer";
import {
  CosmWasmSigner,
  randomAddress,
} from "@confio/relayer/build/lib/helpers";
import { ChannelPair } from "@confio/relayer/build/lib/link";
import { assert } from "chai";
import { Order } from "cosmjs-types/ibc/core/channel/v1/channel";
import { step } from "mocha-steps";
import digest from "sha256";

import configs from "./configs";
import Contract from "./contract";
import { setupOS } from "./os";
import { waitForChain, waitForRelayer } from "./relayer";
import {
  assertPacketsFromA,
  assertPacketsFromB,
  awaitMulti,
  createAMPMsg,
  createAMPPacket,
  getAllADONames,
  relayAll,
  setupOsmosisClient,
  setupOsmosisClientB,
  setupRelayerInfo,
  uploadAllADOs,
} from "./utils";

const { osmosisA, osmosisB } = configs;

interface Contracts {
  kernel?: Contract;
  vfs?: Contract;
  economics?: Contract;
  adodb?: Contract;
}

interface State {
  link?: Link;
  channel?: ChannelPair;
  setup: boolean;
  chainA: {
    client?: CosmWasmSigner;
    os: Contracts;
    ics20Chan: string;
    ibcDenom: string;
    name: string;
  };
  chainB: {
    client?: CosmWasmSigner;
    os: Contracts;
    ics20Chan: string;
    ibcDenom: string;
    name: string;
  };
}

const ics20Chan = "channel-0";

let state: State = {
  setup: false,
  chainA: {
    os: {},
    ics20Chan,
    ibcDenom: "",
    name: "osmo-a",
  },
  chainB: {
    os: {},
    ics20Chan,
    ibcDenom: "",
    name: "osmo-b",
  },
};

async function setupState() {
  if (state.setup) return;

  const [osmoA, osmoB] = (await awaitMulti([
    setupOsmosisClient(),
    setupOsmosisClientB(),
  ])) as CosmWasmSigner[];

  const [src, dest] = await setupRelayerInfo(osmosisA, osmosisB);
  console.log("Creating link...");
  state.link = await Link.createWithExistingConnections(
    src,
    dest,
    "connection-0",
    "connection-0"
  );
  console.log("Created link");
  const osmoAIBCDenom = `ibc/${digest(
    `${osmosisB.ics20Port}/${state.chainB.ics20Chan}/uosmo`
  ).toUpperCase()}`;
  const osmoBIBCDenom = `ibc/${digest(
    `${osmosisA.ics20Port}/${state.chainB.ics20Chan}/uosmo`
  ).toUpperCase()}`;
  state = {
    ...state,
    setup: true,
    chainA: {
      ...state.chainA,
      client: osmoA,
      ibcDenom: osmoAIBCDenom,
    },
    chainB: {
      ...state.chainB,
      client: osmoB,
      ibcDenom: osmoBIBCDenom,
    },
  };
}

before(async () => {
  await waitForChain(osmosisA.tendermintUrlHttp);
  await waitForChain(osmosisB.tendermintUrlHttp);
  await waitForRelayer();
  await setupState();
});

describe("Operating System", () => {
  step("should be deployed correctly...", async () => {
    const { chainA, chainB } = state;
    const [addressesA, addressesB] = (await awaitMulti([
      setupOS(chainA.client! as CosmWasmSigner, chainA.name),
      setupOS(chainB.client! as CosmWasmSigner, chainB.name),
    ])) as Record<keyof Contracts, string>[];
    for (const contract in addressesA) {
      chainA.os[contract as keyof Contracts] = Contract.fromAddress(
        addressesA[contract as keyof Contracts]
      );
    }
    for (const contract in addressesB) {
      chainB.os[contract as keyof Contracts] = Contract.fromAddress(
        addressesB[contract as keyof Contracts]
      );
    }
  });

  step("should have assigned key addresses correctly", async () => {
    const { chainA, chainB } = state;

    for (const name in chainA.os) {
      if (name === "kernel") continue;
      const query = {
        key_address: {
          key: name,
        },
      };
      const resChainA = await chainA.os.kernel!.query(query, chainA.client!);
      assert(resChainA == chainA.os[name as keyof Contracts]!.address);
      const resChainB = await chainB.os.kernel!.query(query, chainB.client!);
      assert(resChainB == chainB.os[name as keyof Contracts]!.address);
    }
  });

  step("should create a channel between kernels", async () => {
    const {
      link,
      chainA: { os: osA, client: clientA },
      chainB: { os: osB, client: clientB },
    } = state;
    const portA = await osA.kernel!.getPort(clientA!);
    const portB = await osB.kernel!.getPort(clientB!);
    const channel = await link?.createChannel(
      "A",
      portA,
      portB,
      Order.ORDER_UNORDERED,
      "andr-kernel-1"
    );

    state.channel = channel;
    // await relayAll(link!);
  });

  step("should assign the correct channel on chain A", async () => {
    const { chainA, chainB, channel } = state;

    const osmoABridgeMsg = {
      assign_channels: {
        ics20_channel_id: chainA.ics20Chan,
        kernel_address: chainB.os.kernel!.address,
        chain: chainB.name,
        direct_channel_id: channel?.src.channelId,
      },
    };
    await chainA.os.kernel!.execute(osmoABridgeMsg, chainA.client!);

    const assignedChannels = await chainA.os.kernel!.query<{
      ics20: string;
      direct: string;
    }>(
      {
        channel_info: { chain: chainB.name },
      },
      chainA.client!
    );
    assert(assignedChannels.ics20 == chainA.ics20Chan);
    assert(assignedChannels.direct == channel?.src.channelId);
  });

  step("should assign the correct channel on chain B", async () => {
    const { chainA, chainB, channel } = state;
    const osmoBBridgeMsg = {
      assign_channels: {
        ics20_channel_id: chainB.ics20Chan,
        kernel_address: chainA.os.kernel!.address,
        chain: chainA.name,
        direct_channel_id: channel?.dest.channelId,
      },
    };
    await chainB.os.kernel!.execute(osmoBBridgeMsg, chainB.client!);

    const assignedChannels = await chainB.os.kernel!.query<{
      ics20: string;
      direct: string;
    }>(
      {
        channel_info: { chain: chainA.name },
      },
      chainB.client!
    );
    assert(assignedChannels.ics20 == chainA.ics20Chan);
    assert(assignedChannels.direct == channel?.src.channelId);
  });

  step("should upload all contracts correctly", async () => {
    const { chainA, chainB } = state;
    await awaitMulti([
      uploadAllADOs(chainA.client!, chainA.os.adodb!),
      uploadAllADOs(chainB.client!, chainB.os.adodb!),
    ]);
  });

  step("should have published all contracts correctly", async () => {
    const {
      chainA: { os: osA, client: clientA },
      chainB: { os: osB, client: clientB },
    } = state;
    const names = getAllADONames();
    const queryCodeId = async (name: string) => {
      const query = {
        code_id: {
          key: name,
        },
      };

      const resA = await osA.adodb!.query(query, clientA!);
      const resB = await osB.adodb!.query(query, clientB!);
      assert(resA == resB);
    };

    const promises = names.map((name) => queryCodeId(name));

    await awaitMulti(promises);
  });
});

describe("Basic IBC Token Transfers", async () => {
  step("should send tokens from chain A to chain A", async () => {
    const { chainA } = state;
    const receiver = randomAddress("osmo");
    const transferAmount = { amount: "100", denom: "uosmo" };
    const msg = createAMPMsg(`/${receiver}`, undefined, [transferAmount]);
    const kernelMsg = { send: { message: msg } };
    const res = await chainA.os.kernel!.execute(kernelMsg, chainA.client!, [
      transferAmount,
    ]);
    assert(res.transactionHash);

    const balance = await chainA.client!.sign.getBalance(
      receiver,
      transferAmount.denom
    );
    assert(balance.amount === transferAmount.amount, "Balance is incorrect");
  });

  step("should send tokens from chain B to chain B", async () => {
    const { chainB } = state;
    const receiver = randomAddress("osmo");
    const transferAmount = { amount: "100", denom: "uosmo" };
    const msg = createAMPMsg(`/${receiver}`, undefined, [transferAmount]);
    const kernelMsg = { send: { message: msg } };
    const res = await chainB.os.kernel!.execute(kernelMsg, chainB.client!, [
      transferAmount,
    ]);
    assert(res.transactionHash);

    const balance = await chainB.client!.sign.getBalance(
      receiver,
      transferAmount.denom
    );
    assert(balance.amount === transferAmount.amount, "Balance is incorrect");
  });

  step("should send tokens from chain A to chain B", async () => {
    const { link, chainA, chainB } = state;
    const receiver = randomAddress("osmo");
    const transferAmount = { amount: "100", denom: "uosmo" };
    const msg = createAMPMsg(`ibc://${chainB.name}/${receiver}`, undefined, [
      transferAmount,
    ]);
    const kernelMsg = { send: { message: msg } };
    const res = await chainA.os.kernel!.execute(kernelMsg, chainA.client!, [
      transferAmount,
    ]);
    assert(res.transactionHash);
    const [shouldAssert, info] = await relayAll(link!);
    if (shouldAssert) assertPacketsFromA(info, 1, true);
    const osmoBalance = await chainB.client!.sign.getBalance(
      receiver,
      chainB.ibcDenom
    );
    assert(
      osmoBalance.amount === transferAmount.amount,
      "Balance is incorrect"
    );
  });

  step("should send tokens from chain B to chain A", async () => {
    const { link, chainA, chainB } = state;
    const receiver = randomAddress("osmo");
    const transferAmount = { amount: "100", denom: "uosmo" };
    const msg = createAMPMsg(`ibc://${chainA.name}/${receiver}`, undefined, [
      transferAmount,
    ]);
    const kernelMsg = { send: { message: msg } };
    const res = await chainB.os.kernel!.execute(kernelMsg, chainB.client!, [
      transferAmount,
    ]);
    assert(res.transactionHash);
    const [shouldAssert, info] = await relayAll(link!);
    if (shouldAssert) assertPacketsFromB(info, 1, true);
    const osmoBalance = await chainA.client!.sign.getBalance(
      receiver,
      chainA.ibcDenom
    );
    assert(
      osmoBalance.amount === transferAmount.amount,
      "Balance is incorrect"
    );
  });

  step(
    "should send tokens from chain A to chain B and back to chain A",
    async () => {
      const { link, chainA, chainB } = state;
      const receiver = randomAddress("osmo");
      const splitterCodeId: number = await chainB.os.adodb!.query(
        { code_id: { key: "splitter" } },
        chainB.client!
      );
      const splitterInstMsg = {
        kernel_address: chainB.os.kernel!.address,
        recipients: [
          {
            recipient: {
              address: `ibc://${chainA.name}/${receiver}`,
            },
            percent: "1",
          },
        ],
      };
      const splitter = await Contract.fromCodeId(
        splitterCodeId,
        splitterInstMsg,
        chainB.client!
      );

      const transferAmount = { amount: "100", denom: "uosmo" };
      const msg = createAMPMsg(
        `ibc://${chainB.name}/${splitter.address}`,
        { send: {} },
        [transferAmount]
      );
      const kernelMsg = { send: { message: msg } };
      const res = await chainA.os.kernel!.execute(kernelMsg, chainA.client!, [
        transferAmount,
      ]);
      assert(res.transactionHash);
      const [shouldAssertA, infoA] = await relayAll(link!);
      if (shouldAssertA) assertPacketsFromA(infoA, 1, true);
      await relayAll(link!);
      const osmoBalance = await chainA.client!.sign.getBalance(
        receiver,
        "uosmo"
      );
      assert(
        osmoBalance.amount === transferAmount.amount,
        "Balance is incorrect"
      );
    }
  );
});

describe("IBC Fund Recovery", async () => {
  step(
    "should recover funds on a failed IBC hooks message from chain A to chain B",
    async () => {
      const { link, chainA, chainB } = state;
      const receiver = randomAddress("osmo");
      const recoveryAddr = chainA.client!.senderAddress;
      const splitterCodeId: number = await chainB.os.adodb!.query(
        { code_id: { key: "splitter" } },
        chainB.client!
      );
      const splitterInstMsg = {
        kernel_address: chainB.os.kernel!.address,
        recipients: [
          {
            recipient: {
              address: `ibc://${chainA.name}/${receiver}`,
            },
            percent: "1",
          },
        ],
      };
      const splitter = await Contract.fromCodeId(
        splitterCodeId,
        splitterInstMsg,
        chainB.client!
      );

      const transferAmount = { amount: "100", denom: "uosmo" };
      // ERROR HERE
      const msg = createAMPMsg(
        `ibc://${chainB.name}/${splitter.address}`,
        { not_a_valid_message: {} },
        [transferAmount],
        {
          recovery_addr: recoveryAddr,
        }
      );
      const kernelMsg = { send: { message: msg } };
      const res = await chainA.os.kernel!.execute(kernelMsg, chainA.client!, [
        transferAmount,
      ]);
      assert(res.transactionHash);
      const [shouldAssertA, infoA] = await relayAll(link!);
      if (shouldAssertA) assertPacketsFromA(infoA, 1, false);
      const recoveryQuery = {
        recoveries: { addr: recoveryAddr },
      };
      const recoveries: { amount: string; denom: string }[] =
        await chainA.os.kernel!.query(recoveryQuery, chainA.client!);
      assert(recoveries.length === 1, "No recovery found");
      assert(
        recoveries[0].amount === transferAmount.amount,
        "Incorrect amount"
      );
      assert(recoveries[0].denom === transferAmount.denom, "Incorrect denom");

      const recoveryRes = await chainA.os.kernel!.execute(
        { recover: {} },
        chainA.client!
      );
      assert(recoveryRes.transactionHash);
    }
  );

  step(
    "should recover funds on a failed IBC hooks message after sending from chain A to chain B and responding back to chain A",
    async () => {
      const { link, chainA, chainB } = state;
      const receiver = randomAddress("osmo");
      const recoveryAddr = chainB.client!.senderAddress;
      const splitterCodeId: number = await chainB.os.adodb!.query(
        { code_id: { key: "splitter" } },
        chainB.client!
      );
      // Will error with recipient as it is not a splitter contract
      const splitterInstMsg = {
        kernel_address: chainB.os.kernel!.address,
        recipients: [
          {
            recipient: {
              address: `ibc://${chainA.name}/${receiver}`,
              msg: "eyJzZW5kIjp7fX0=", // { send: {} } encoded
              ibc_recovery_address: recoveryAddr,
            },
            percent: "1",
          },
        ],
      };
      const splitter = await Contract.fromCodeId(
        splitterCodeId,
        splitterInstMsg,
        chainB.client!
      );

      const transferAmount = { amount: "100", denom: "uosmo" };
      const msg = createAMPMsg(
        `ibc://${chainB.name}/${splitter.address}`,
        { send: {} },
        [transferAmount],
        {
          recovery_addr: recoveryAddr,
        }
      );
      const kernelMsg = { send: { message: msg } };
      const res = await chainA.os.kernel!.execute(kernelMsg, chainA.client!, [
        transferAmount,
      ]);
      assert(res.transactionHash);
      const [shouldAssertA, infoA] = await relayAll(link!);
      if (shouldAssertA) assertPacketsFromA(infoA, 1, true);
      await relayAll(link!);
      const recoveryQuery = {
        recoveries: { addr: recoveryAddr },
      };
      const recoveries: { amount: string; denom: string }[] =
        await chainB.os.kernel!.query(recoveryQuery, chainB.client!);
      assert(recoveries.length === 1, "No recovery found");
      assert(
        recoveries[0].amount === transferAmount.amount,
        "Incorrect amount"
      );
      assert(recoveries[0].denom === chainA.ibcDenom, "Incorrect denom");
      const recoveryRes = await chainB.os.kernel!.execute(
        { recover: {} },
        chainB.client!
      );
      assert(recoveryRes.transactionHash);
    }
  );

  step(
    "should assign the original sender as the recovery address in an AMP packet when none is provided",
    async () => {
      const { link, chainA, chainB } = state;
      const receiver = randomAddress("osmo");
      const recoveryAddr = chainB.client!.senderAddress;
      const splitterCodeId: number = await chainB.os.adodb!.query(
        { code_id: { key: "splitter" } },
        chainB.client!
      );
      // Will error with recipient as it is not a splitter contract
      const splitterInstMsg = {
        kernel_address: chainB.os.kernel!.address,
        recipients: [
          {
            recipient: {
              address: `ibc://${chainA.name}/${receiver}`,
              msg: "eyJzZW5kIjp7fX0=", // { send: {} } encoded
            },
            percent: "1",
          },
        ],
      };
      const splitter = await Contract.fromCodeId(
        splitterCodeId,
        splitterInstMsg,
        chainB.client!
      );

      const transferAmount = { amount: "100", denom: "uosmo" };
      const msg = createAMPMsg(splitter.address, { send: {} }, [
        transferAmount,
      ]);
      const kernelMsg = { send: { message: msg } };
      const res = await chainB.os.kernel!.execute(kernelMsg, chainB.client!, [
        transferAmount,
      ]);
      assert(res.transactionHash);
      const [shouldAssertB, infoB] = await relayAll(link!);
      if (shouldAssertB) assertPacketsFromB(infoB, 1, false);
      const recoveryQuery = {
        recoveries: { addr: recoveryAddr },
      };
      const recoveries: { amount: string; denom: string }[] =
        await chainB.os.kernel!.query(recoveryQuery, chainB.client!);
      assert(recoveries.length === 1, "No recovery found");
      assert(
        recoveries[0].amount === transferAmount.amount,
        "Incorrect amount"
      );
      assert(recoveries[0].denom === transferAmount.denom, "Incorrect denom");
      const recoveryRes = await chainB.os.kernel!.execute(
        { recover: {} },
        chainB.client!
      );
      assert(recoveryRes.transactionHash);
    }
  );
});
