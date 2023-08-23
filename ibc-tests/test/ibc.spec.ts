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
import { setupOS } from "./os";
import { waitForChain, waitForRelayer } from "./relayer";
import {
  assertPacketsFromA,
  assertPacketsFromB,
  awaitMulti,
  createAMPMsg,
  createAMPPacket,
  getAllADONames,
  getBalances,
  relayAll,
  setupChainClient,
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

interface ChainEnd {
  client?: CosmWasmSigner;
  os: Contracts;
  ics20Chan: string;
  ibcDenom: string;
  name: string;
  definition: ChainDefinition;
  denom: string;
  connection: string;
}
interface State {
  link?: Link;
  channel?: ChannelPair;
  setup: boolean;
  chainA: ChainEnd;
  chainB: ChainEnd;
}

let state: State = {
  setup: false,
  chainA: {
    os: {},
    ics20Chan: "channel-2",
    ibcDenom: "",
    name: "osmo-a",
    definition: osmosisA,
    denom: "uosmo",
    connection: "connection-2",
  },
  chainB: {
    os: {},
    ics20Chan: "channel-0",
    ibcDenom: "",
    name: "osmo-b",
    definition: osmosisB,
    denom: "uosmo",
    connection: "connection-0",
  },
};

async function setupState() {
  if (state.setup) return;

  const [chainA, chainB] = (await awaitMulti([
    setupChainClient(state.chainA.definition),
    setupChainClient(state.chainB.definition),
  ])) as CosmWasmSigner[];

  const [src, dest] = await setupRelayerInfo(
    state.chainA.definition,
    state.chainB.definition
  );
  console.log("Creating link...");
  state.link = await Link.createWithExistingConnections(
    src,
    dest,
    state.chainA.connection,
    state.chainB.connection
  );
  console.log("Created link");
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
    console.log(channel);

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
    const receiver = randomAddress(chainA.definition.prefix);
    const transferAmount = { amount: "100", denom: chainA.denom };
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
    const receiver = randomAddress(chainB.definition.prefix);
    const transferAmount = { amount: "100", denom: chainB.denom };
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
    const receiver = randomAddress(chainB.definition.prefix);
    const transferAmount = { amount: "105", denom: chainA.denom };
    const msg = createAMPMsg(`ibc://${chainB.name}/${receiver}`, undefined, [
      transferAmount,
    ]);
    const kernelMsg = { send: { message: msg } };
    const res = await chainA.os.kernel!.execute(kernelMsg, chainA.client!, [
      transferAmount,
    ]);
    assert(res.transactionHash);
    const [shouldAssert, info] = await relayAll(link!);
    await getBalances(chainA.definition, chainA.client!.senderAddress);
    await getBalances(chainA.definition, chainA.os.kernel!.address);
    await getBalances(chainB.definition, chainB.os.kernel!.address);
    await getBalances(chainB.definition, receiver);
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
    const transferAmount = { amount: "100", denom: chainB.denom };
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

      const transferAmount = { amount: "100", denom: chainA.denom };
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

// describe("IBC Fund Recovery", async () => {
//   step(
//     "should recover funds on a failed IBC hooks message from chain A to chain B",
//     async () => {
//       const { link, chainA, chainB } = state;
//       const receiver = randomAddress(chainA.definition.prefix);
//       const recoveryAddr = chainA.client!.senderAddress;
//       const splitterCodeId: number = await chainB.os.adodb!.query(
//         { code_id: { key: "splitter" } },
//         chainB.client!
//       );
//       const splitterInstMsg = {
//         kernel_address: chainB.os.kernel!.address,
//         recipients: [
//           {
//             recipient: {
//               address: `ibc://${chainA.name}/${receiver}`,
//             },
//             percent: "1",
//           },
//         ],
//       };
//       const splitter = await Contract.fromCodeId(
//         splitterCodeId,
//         splitterInstMsg,
//         chainB.client!
//       );

//       const transferAmount = { amount: "100", denom: chainA.denom };
//       // ERROR HERE
//       const msg = createAMPMsg(
//         `ibc://${chainB.name}/${splitter.address}`,
//         { not_a_valid_message: {} },
//         [transferAmount],
//         {
//           recovery_addr: recoveryAddr,
//         }
//       );
//       const kernelMsg = { send: { message: msg } };
//       const res = await chainA.os.kernel!.execute(kernelMsg, chainA.client!, [
//         transferAmount,
//       ]);
//       assert(res.transactionHash);
//       const [shouldAssertA, infoA] = await relayAll(link!);
//       if (shouldAssertA) assertPacketsFromA(infoA, 1, false);
//       const recoveryQuery = {
//         recoveries: { addr: recoveryAddr },
//       };
//       const recoveries: { amount: string; denom: string }[] =
//         await chainA.os.kernel!.query(recoveryQuery, chainA.client!);
//       assert(recoveries.length === 1, "No recovery found");
//       assert(
//         recoveries[0].amount === transferAmount.amount,
//         "Incorrect amount"
//       );
//       assert(recoveries[0].denom === transferAmount.denom, "Incorrect denom");

//       const recoveryRes = await chainA.os.kernel!.execute(
//         { recover: {} },
//         chainA.client!
//       );
//       assert(recoveryRes.transactionHash);
//     }
//   );

//   step(
//     "should recover funds on a failed IBC hooks message after sending from chain A to chain B and responding back to chain A",
//     async () => {
//       const { link, chainA, chainB } = state;
//       const receiver = randomAddress(chainA.definition.prefix);
//       const recoveryAddr = chainB.client!.senderAddress;
//       const splitterCodeId: number = await chainB.os.adodb!.query(
//         { code_id: { key: "splitter" } },
//         chainB.client!
//       );
//       // Will error with recipient as it is not a splitter contract
//       const splitterInstMsg = {
//         kernel_address: chainB.os.kernel!.address,
//         recipients: [
//           {
//             recipient: {
//               address: `ibc://${chainA.name}/${receiver}`,
//               msg: "eyJzZW5kIjp7fX0=", // { send: {} } encoded
//               ibc_recovery_address: recoveryAddr,
//             },
//             percent: "1",
//           },
//         ],
//       };
//       const splitter = await Contract.fromCodeId(
//         splitterCodeId,
//         splitterInstMsg,
//         chainB.client!
//       );

//       const transferAmount = { amount: "100", denom: chainA.denom };
//       const msg = createAMPMsg(
//         `ibc://${chainB.name}/${splitter.address}`,
//         { send: {} },
//         [transferAmount],
//         {
//           recovery_addr: recoveryAddr,
//         }
//       );
//       const kernelMsg = { send: { message: msg } };
//       const res = await chainA.os.kernel!.execute(kernelMsg, chainA.client!, [
//         transferAmount,
//       ]);
//       assert(res.transactionHash);
//       const [shouldAssertA, infoA] = await relayAll(link!);
//       if (shouldAssertA) assertPacketsFromA(infoA, 1, true);
//       await relayAll(link!);
//       const recoveryQuery = {
//         recoveries: { addr: recoveryAddr },
//       };
//       const recoveries: { amount: string; denom: string }[] =
//         await chainB.os.kernel!.query(recoveryQuery, chainB.client!);
//       assert(recoveries.length === 1, "No recovery found");
//       assert(
//         recoveries[0].amount === transferAmount.amount,
//         "Incorrect amount"
//       );
//       assert(recoveries[0].denom === chainA.ibcDenom, "Incorrect denom");
//       const recoveryRes = await chainB.os.kernel!.execute(
//         { recover: {} },
//         chainB.client!
//       );
//       assert(recoveryRes.transactionHash);
//     }
//   );

//   step(
//     "should assign the original sender as the recovery address in an AMP packet when none is provided",
//     async () => {
//       const { link, chainA, chainB } = state;
//       const receiver = randomAddress(chainA.definition.prefix);
//       const recoveryAddr = chainB.client!.senderAddress;
//       const splitterCodeId: number = await chainB.os.adodb!.query(
//         { code_id: { key: "splitter" } },
//         chainB.client!
//       );
//       // Will error with recipient as it is not a splitter contract
//       const splitterInstMsg = {
//         kernel_address: chainB.os.kernel!.address,
//         recipients: [
//           {
//             recipient: {
//               address: `ibc://${chainA.name}/${receiver}`,
//               msg: "eyJzZW5kIjp7fX0=", // { send: {} } encoded
//             },
//             percent: "1",
//           },
//         ],
//       };
//       const splitter = await Contract.fromCodeId(
//         splitterCodeId,
//         splitterInstMsg,
//         chainB.client!
//       );

//       const transferAmount = { amount: "100", denom: chainA.denom };
//       const msg = createAMPMsg(splitter.address, { send: {} }, [
//         transferAmount,
//       ]);
//       const kernelMsg = { send: { message: msg } };
//       const res = await chainB.os.kernel!.execute(kernelMsg, chainB.client!, [
//         transferAmount,
//       ]);
//       assert(res.transactionHash);
//       const [shouldAssertB, infoB] = await relayAll(link!);
//       if (shouldAssertB) assertPacketsFromB(infoB, 1, false);
//       const recoveryQuery = {
//         recoveries: { addr: recoveryAddr },
//       };
//       const recoveries: { amount: string; denom: string }[] =
//         await chainB.os.kernel!.query(recoveryQuery, chainB.client!);
//       assert(recoveries.length === 1, "No recovery found");
//       assert(
//         recoveries[0].amount === transferAmount.amount,
//         "Incorrect amount"
//       );
//       assert(recoveries[0].denom === transferAmount.denom, "Incorrect denom");
//       const recoveryRes = await chainB.os.kernel!.execute(
//         { recover: {} },
//         chainB.client!
//       );
//       assert(recoveryRes.transactionHash);
//     }
//   );
// });
