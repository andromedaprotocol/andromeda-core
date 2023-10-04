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

import configs, { ChainDefinition } from "./configs";
import Contract from "./contract";
import { CustomLogger } from "./logger";
import { setupOS } from "./os";
import { waitForChain } from "./relayer";
import {
  assertPacketsFromA,
  assertPacketsFromB,
  awaitMulti,
  createAMPMsg,
  getAllADONames,
  relayAll,
  retryTill,
  setupChainClient,
  setupRelayerInfo,
  uploadAllADOs,
} from "./utils";

const { osmosisA, osmosisB, terraA, andromedaA, junoA, junoB } = configs;

interface Contracts {
  kernel?: Contract;
  vfs?: Contract;
  economics?: Contract;
  adodb?: Contract;
}

interface ChainEnd {
  client?: CosmWasmSigner;
  os: Contracts;
  ics20Chan: string;
  ibcDenom: string;
  name: string;
  definition: ChainDefinition;
  denom: string;
}
interface State {
  link?: Link;
  channel?: ChannelPair;
  setup: boolean;
  chainA: ChainEnd;
  chainB: ChainEnd;
  logger: CustomLogger;
}

let state: State = {
  setup: false,
  logger: new CustomLogger(),
  chainA: {
    os: {},
    ics20Chan: "",
    ibcDenom: "",
    name: "chain-ba",
    definition: osmosisA,
    denom: osmosisA.denomFee,
  },
  chainB: {
    os: {},
    ics20Chan: "",
    ibcDenom: "",
    name: "chain-aa",
    definition: osmosisB,
    denom: osmosisB.denomFee,
  },
};

async function setupState() {
  if (state.setup) return;

  const [chainA, chainB] = (await awaitMulti([
    setupChainClient(state.chainA.definition),
    setupChainClient(state.chainB.definition),
  ])) as CosmWasmSigner[];

  const [src, dest, buffer] = await retryTill(() =>
    setupRelayerInfo(state.chainA.definition, state.chainB.definition)
  );

  console.log("Creating link...");
  state.link = await Link.createWithNewConnections(src, dest, state.logger);
  state.logger.log(
    "Link A",
    state.link.endA.clientID,
    state.link.endA.connectionID
  );
  state.logger.log(
    "Link B",
    state.link.endB.clientID,
    state.link.endB.connectionID
  );
  console.log("Created link");

  console.log("Creating ics20 channel...");
  const channel = await state.link.createChannel(
    "A",
    state.chainA.definition.ics20Port,
    state.chainB.definition.ics20Port,
    Order.ORDER_UNORDERED,
    "ics20-1"
  );
  state.logger.log("Channel Created", channel);
  state.chainA.ics20Chan = channel.src.channelId;
  state.chainB.ics20Chan = channel.dest.channelId;
  console.log("Created ics20 channel...");

  const chainAIBCDenom = `ibc/${digest(
    `${state.chainB.definition.ics20Port}/${state.chainB.ics20Chan}/${state.chainA.denom}`
  ).toUpperCase()}`;
  const chainBIBCDenom = `ibc/${digest(
    `${state.chainA.definition.ics20Port}/${state.chainA.ics20Chan}/${state.chainB.denom}`
  ).toUpperCase()}`;
  state = {
    ...state,
    setup: true,
    chainA: {
      ...state.chainA,
      client: chainA,
      ibcDenom: chainAIBCDenom,
    },
    chainB: {
      ...state.chainB,
      client: chainB,
      ibcDenom: chainBIBCDenom,
    },
  };
}

before(async () => {
  await waitForChain(state.chainA.definition.tendermintUrlHttp);
  await waitForChain(state.chainB.definition.tendermintUrlHttp);
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

  step("should clear existing recoveries", async () => {
    await state.chainA.os
      .kernel!.execute({ recover: {} }, state.chainA.client!)
      .catch(() => {
        return;
      });
    await state.chainB.os
      .kernel!.execute({ recover: {} }, state.chainB.client!)
      .catch(() => {
        return;
      });
  });

  step("should create a channel between kernels", async () => {
    const {
      link,
      chainA: { os: osA, client: clientA },
      chainB: { os: osB, client: clientB },
    } = state;
    const portA = await osA.kernel!.getPort(clientA!);
    const portB = await osB.kernel!.getPort(clientB!);

    console.debug("Creating direct channel");
    const channel = await retryTill(() =>
      link?.createChannel(
        "A",
        portA,
        portB,
        Order.ORDER_UNORDERED,
        "andr-kernel-1"
      )
    );
    assert(!!channel, "channel not created");
    state.channel = channel;
    // await relayAll(link!);
  });

  step("should assign the correct channel on chain A", async () => {
    const { chainA, chainB, channel } = state;

    const chainABridgeMsg = {
      assign_channels: {
        ics20_channel_id: chainA.ics20Chan,
        kernel_address: chainB.os.kernel!.address,
        chain: chainB.name,
        direct_channel_id: channel?.src.channelId,
      },
    };
    await chainA.os.kernel!.execute(chainABridgeMsg, chainA.client!);

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
    const chainBBridgeMsg = {
      assign_channels: {
        ics20_channel_id: chainB.ics20Chan,
        kernel_address: chainA.os.kernel!.address,
        chain: chainA.name,
        direct_channel_id: channel?.dest.channelId,
      },
    };
    await chainB.os.kernel!.execute(chainBBridgeMsg, chainB.client!);

    const assignedChannels = await chainB.os.kernel!.query<{
      ics20: string;
      direct: string;
    }>(
      {
        channel_info: { chain: chainA.name },
      },
      chainB.client!
    );
    assert(assignedChannels.ics20 == chainB.ics20Chan);
    assert(assignedChannels.direct == channel?.dest.channelId);
  });

  step("should upload all contracts correctly", async () => {
    const { chainA, chainB } = state;
    await awaitMulti([
      uploadAllADOs(chainA.client!, chainA.os.adodb!),
      uploadAllADOs(chainB.client!, chainB.os.adodb!),
    ]);
    assert(true);
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
    };

    const promises = names.map((name) => queryCodeId(name));

    await awaitMulti(promises);
  });
});

describe("Basic IBC Token Transfers", async () => {
  step("should send tokens from chain A to chain A", async () => {
    const { chainA } = state;
    const receiver = randomAddress(chainA.definition.prefix);
    const transferAmount = { amount: "100", denom: chainA.denom };
    const msg = createAMPMsg(`${receiver}`, undefined, [transferAmount]);
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
    const receiver = randomAddress(chainB.definition.prefix);
    const transferAmount = { amount: "100", denom: chainB.denom };
    const msg = createAMPMsg(`${receiver}`, undefined, [transferAmount]);
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
    const receiver = randomAddress(chainB.definition.prefix);
    const transferAmount = { amount: "105", denom: chainA.denom };
    const msg = createAMPMsg(
      `ibc://${chainB.name}/home/${receiver}`,
      undefined,
      [transferAmount]
    );
    const kernelMsg = { send: { message: msg } };
    const res = await chainA.os.kernel!.execute(kernelMsg, chainA.client!, [
      transferAmount,
    ]);
    assert(res.transactionHash);
    const [shouldAssert, info] = await relayAll(link!);
    if (shouldAssert) assertPacketsFromA(info, 1, true);
    const chainBBalance = await chainB.client!.sign.getBalance(
      receiver,
      chainA.ibcDenom
    );
    assert(
      chainBBalance.amount === transferAmount.amount,
      "Balance is incorrect"
    );
  });

  step("should send tokens from chain B to chain A", async () => {
    const { link, chainA, chainB } = state;
    const receiver = randomAddress(chainA.definition.prefix);
    const transferAmount = { amount: "105", denom: chainB.denom };
    const msg = createAMPMsg(
      `ibc://${chainA.name}/home/${receiver}`,
      undefined,
      [transferAmount]
    );
    const kernelMsg = { send: { message: msg } };
    const res = await chainB.os.kernel!.execute(kernelMsg, chainB.client!, [
      transferAmount,
    ]);
    assert(res.transactionHash);
    const [shouldAssert, info] = await relayAll(link!);
    if (shouldAssert) assertPacketsFromB(info, 1, true);
    const chainABalance = await chainA.client!.sign.getBalance(
      receiver,
      chainB.ibcDenom
    );
    assert(
      chainABalance.amount === transferAmount.amount,
      "Balance is incorrect"
    );
  });

  step(
    "should send chain A tokens from chain A to chain B using splitter",
    async () => {
      const { link, chainA, chainB } = state;
      const receiver = randomAddress(chainB.definition.prefix);
      const splitterCodeId: number = await chainA.os.adodb!.query(
        { code_id: { key: "splitter" } },
        chainA.client!
      );
      const splitterInstMsg = {
        kernel_address: chainA.os.kernel!.address,
        recipients: [
          {
            recipient: {
              address: `ibc://${chainB.name}/home/${receiver}`,
            },
            percent: "1",
          },
        ],
      };
      const splitter = await Contract.fromCodeId(
        splitterCodeId,
        splitterInstMsg,
        chainA.client!
      );

      state.logger.log(
        "chain A tokens from chain A to chain B using splitter - Splitter address",
        splitter.address
      );
      state.logger.log(
        "chain A tokens from chain A to chain B using splitter - Receiver address",
        receiver
      );

      const transferAmount = { amount: "100", denom: chainA.denom };
      const msg = createAMPMsg(`${splitter.address}`, { send: {} }, [
        transferAmount,
      ]);
      const kernelMsg = { send: { message: msg } };
      const res = await chainA.os.kernel!.execute(kernelMsg, chainA.client!, [
        transferAmount,
      ]);
      assert(res.transactionHash);
      const [shouldAssertA, infoB] = await relayAll(link!);
      if (shouldAssertA) assertPacketsFromA(infoB, 1, true);
      await relayAll(link!);
      const chainABalance = await chainB.client!.sign.getBalance(
        receiver,
        chainA.ibcDenom
      );
      assert(
        chainABalance.amount === transferAmount.amount,
        "Balance is incorrect"
      );
    }
  );

  step(
    "should send chain B tokens from chain A to chain B using splitter",
    async () => {
      const { link, chainA, chainB } = state;
      const receiver = randomAddress(chainB.definition.prefix);
      const splitterCodeId: number = await chainA.os.adodb!.query(
        { code_id: { key: "splitter" } },
        chainA.client!
      );
      const splitterInstMsg = {
        kernel_address: chainA.os.kernel!.address,
        recipients: [
          {
            recipient: {
              address: `ibc://${chainB.name}/home/${receiver}`,
            },
            percent: "1",
          },
        ],
      };
      const splitter = await Contract.fromCodeId(
        splitterCodeId,
        splitterInstMsg,
        chainA.client!
      );

      state.logger.log(
        "chain B tokens from chain A to chain B using splitter - Splitter address",
        splitter.address
      );
      state.logger.log(
        "chain B tokens from chain A to chain B using splitter - Receiver address",
        receiver
      );

      const transferAmount = { amount: "100", denom: chainB.denom };
      const transferMsg = createAMPMsg(
        `ibc://${chainA.name}/home/${chainA.client!.senderAddress}`,
        undefined,
        [transferAmount]
      );
      const transferKernelMsg = { send: { message: transferMsg } };
      let res = await chainB.os.kernel!.execute(
        transferKernelMsg,
        chainB.client!,
        [transferAmount]
      );
      assert(res.transactionHash);
      const [shouldAssert, info] = await relayAll(link!);
      if (shouldAssert) assertPacketsFromB(info, 1, true);

      // Now send the ibc denom received above to the splitter
      transferAmount.denom = chainB.ibcDenom;
      const msg = createAMPMsg(`${splitter.address}`, { send: {} }, [
        transferAmount,
      ]);
      const kernelMsg = { send: { message: msg } };
      res = await chainA.os.kernel!.execute(kernelMsg, chainA.client!, [
        transferAmount,
      ]);
      assert(res.transactionHash);
      const [shouldAssertA, infoB] = await relayAll(link!);
      if (shouldAssertA) assertPacketsFromA(infoB, 1, true);
      await relayAll(link!);
      const chainABalance = await chainB.client!.sign.getBalance(
        receiver,
        chainB.denom
      );
      assert(
        chainABalance.amount === transferAmount.amount,
        "Balance is incorrect"
      );
    }
  );

  step(
    "should send tokens from chain A to chain B and back to chain A",
    async () => {
      const { link, chainA, chainB } = state;
      const receiver = randomAddress(chainA.definition.prefix);
      const splitterCodeId: number = await chainB.os.adodb!.query(
        { code_id: { key: "splitter" } },
        chainB.client!
      );
      const splitterInstMsg = {
        kernel_address: chainB.os.kernel!.address,
        recipients: [
          {
            recipient: {
              address: `ibc://${chainA.name}/home/${receiver}`,
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

      state.logger.log(
        "Chain A to B to Chain A - Splitter address",
        splitter.address
      );
      state.logger.log("Chain A to B to Chain A - Receiver address", receiver);

      const transferAmount = { amount: "100", denom: chainA.denom };
      const msg = createAMPMsg(
        `ibc://${chainB.name}/home/${splitter.address}`,
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
      const chainABalance = await chainA.client!.sign.getBalance(
        receiver,
        chainA.denom
      );
      assert(
        chainABalance.amount === transferAmount.amount,
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
      const receiver = randomAddress(chainA.definition.prefix);
      const recoveryAddr = chainA.client!.senderAddress;
      const recoveryQuery = {
        recoveries: { addr: recoveryAddr },
      };
      const splitterCodeId: number = await chainB.os.adodb!.query(
        { code_id: { key: "splitter" } },
        chainB.client!
      );
      const splitterInstMsg = {
        kernel_address: chainB.os.kernel!.address,
        recipients: [
          {
            recipient: {
              address: `ibc://${chainA.name}/home/${receiver}`,
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

      state.logger.log(
        "Chain A to B IBC Fail Recovery - Splitter address",
        splitter.address
      );
      state.logger.log(
        "Chain A to B IBC Fail Recovery - Receiver address",
        receiver
      );

      const transferAmount = { amount: "100", denom: chainA.denom };
      // ERROR HERE
      const msg = createAMPMsg(
        `ibc://${chainB.name}/home/${splitter.address}`,
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
      await relayAll(link!);
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
      const receiver = randomAddress(chainA.definition.prefix);
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
              address: `ibc://${chainA.name}/home/${receiver}`,
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

      state.logger.log(
        "Chain A to B to Chain A IBC Fail Recovery - Splitter address",
        splitter.address
      );
      state.logger.log(
        "Chain A to B to Chain A IBC Fail Recovery - Receiver address",
        receiver
      );

      const transferAmount = { amount: "100", denom: chainA.denom };
      const msg = createAMPMsg(
        `ibc://${chainB.name}/home/${splitter.address}`,
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
      const receiver = randomAddress(chainA.definition.prefix);
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
              address: `ibc://${chainA.name}/home/${receiver}`,
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

      const transferAmount = { amount: "100", denom: chainB.denom };
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
