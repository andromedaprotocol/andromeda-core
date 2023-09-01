import { Link } from "@confio/relayer";
import {
  CosmWasmSigner,
  randomAddress,
} from "@confio/relayer/build/lib/helpers";
import { assert } from "chai";
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
  "ibc-bridge"?: Contract;
  adodb?: Contract;
}

interface State {
  link?: Link;
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
      setupOS(chainA.client! as CosmWasmSigner),
      setupOS(chainB.client! as CosmWasmSigner),
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

  step("should assign the correct channel on chain A", async () => {
    const { chainA, chainB } = state;

    const osmoABridgeMsg = {
      save_channel: {
        channel: chainA.ics20Chan,
        kernel_address: chainB.os.kernel!.address,
        chain: chainB.name,
      },
    };
    await chainA.os["ibc-bridge"]!.execute(osmoABridgeMsg, chainA.client!);
  });

  step("should assign the correct channel on chain B", async () => {
    const { chainA, chainB } = state;
    const osmoBBridgeMsg = {
      save_channel: {
        channel: chainB.ics20Chan,
        kernel_address: chainA.os.kernel!.address,
        chain: chainA.name,
      },
    };
    await chainB.os["ibc-bridge"]!.execute(osmoBBridgeMsg, chainB.client!);
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
    const pkt = createAMPPacket(chainA.client!.senderAddress, [msg]);
    const kernelMsg = { amp_receive: pkt };
    const res = await chainA.os.kernel!.execute(kernelMsg, chainA.client!, [
      transferAmount,
    ]);
    assert(res.transactionHash);
    const info = await link!.relayAll();
    assertPacketsFromA(info, 1, true);
    const omsoBalance = await chainB.client!.sign.getBalance(
      receiver,
      chainB.ibcDenom
    );
    assert(
      omsoBalance.amount === transferAmount.amount,
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
    const pkt = createAMPPacket(chainB.client!.senderAddress, [msg]);
    const kernelMsg = { amp_receive: pkt };
    const res = await chainB.os.kernel!.execute(kernelMsg, chainB.client!, [
      transferAmount,
    ]);
    assert(res.transactionHash);
    const info = await link!.relayAll();
    assertPacketsFromB(info, 1, true);
    const omsoBalance = await chainA.client!.sign.getBalance(
      receiver,
      chainA.ibcDenom
    );
    assert(
      omsoBalance.amount === transferAmount.amount,
      "Balance is incorrect"
    );
  });

  // TODO: Handle Unwrapping
  // step(
  //   "should send tokens from chain A to chain B and back to chain A",
  //   async () => {
  //     const { link, chainA, chainB } = state;
  //     const receiver = randomAddress("osmo");

  //     const splitterCodeId: number = await chainB.os.adodb!.query(
  //       { code_id: { key: "splitter" } },
  //       chainB.client!
  //     );
  //     const splitterInstMsg = {
  //       kernel_address: chainB.os.kernel!.address,
  //       recipients: [
  //         {
  //           recipient: {
  //             address: `ibc://${chainA.name}/${receiver}`,
  //           },
  //           percent: "1",
  //         },
  //       ],
  //     };
  //     const splitter = await Contract.fromCodeId(
  //       splitterCodeId,
  //       splitterInstMsg,
  //       chainB.client!
  //     );

  //     const transferAmount = { amount: "100", denom: "uosmo" };
  //     const msg = createAMPMsg(
  //       `ibc://${chainB.name}/${splitter.address}`,
  //       { send: {} },
  //       [transferAmount]
  //     );
  //     const pkt = createAMPPacket(chainB.client!.senderAddress, [msg]);
  //     const kernelMsg = { amp_receive: pkt };
  //     const res = await chainA.os.kernel!.execute(kernelMsg, chainA.client!, [
  //       transferAmount,
  //     ]);
  //     assert(res.transactionHash);
  //     const infoA = await link!.relayAll();
  //     assertPacketsFromA(infoA, 1, true);
  //     const infoB = await link!.relayAll();
  //     assertPacketsFromB(infoB, 1, true);
  //     const omsoBalance = await chainA.client!.sign.getBalance(
  //       receiver,
  //       "uosmo"
  //     );
  //     assert(
  //       omsoBalance.amount === transferAmount.amount,
  //       "Balance is incorrect"
  //     );
  //   }
  // );
});
