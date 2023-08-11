import { CosmWasmSigner } from "@confio/relayer";

import { setupContracts } from "./utils";

export async function instantiateOs(
  client: CosmWasmSigner,
  codeIds: Record<string, number>
) {
  const addresses: Record<string, string> = {};
  const res = await client.sign.instantiate(
    client.senderAddress,
    codeIds["kernel"],
    {},
    "Kernel",
    "auto"
  );
  addresses["kernel"] = res.contractAddress;
  for (const name in codeIds) {
    if (name === "kernel") {
      continue;
    } else {
      const res = await client.sign.instantiate(
        client.senderAddress,
        codeIds[name],
        { kernel_address: addresses["kernel"] },
        name,
        "auto"
      );
      addresses[name] = res.contractAddress;
    }
  }

  return addresses;
}

async function assignKeyAddresses(
  client: CosmWasmSigner,
  addresses: Record<string, string>
) {
  const kernelAddress = addresses["kernel"];
  for (const name in addresses) {
    if (name === "kernel") continue;
    await client.sign.execute(
      client.senderAddress,
      kernelAddress,
      { upsert_key_address: { key: name, value: addresses[name] } },
      "auto"
    );
  }
}

export async function setupOS(client: CosmWasmSigner) {
  const contracts: Record<string, string> = {
    kernel: "./contracts/andromeda_kernel.wasm",
    adodb: "./contracts/andromeda_adodb.wasm",
    vfs: "./contracts/andromeda_vfs.wasm",
    economics: "./contracts/andromeda_economics.wasm",
    "ibc-bridge": "./contracts/andromeda_message_bridge.wasm",
  };
  const codeIds = await setupContracts(client, contracts);
  const addresses = await instantiateOs(client, codeIds);
  await assignKeyAddresses(client, addresses);
  return addresses;
}
