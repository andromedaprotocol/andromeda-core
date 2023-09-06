import { readFileSync, writeFileSync } from "fs";

import { CosmWasmSigner } from "@confio/relayer";

import { getADOPath, setupContracts } from "./utils";

export async function instantiateOs(
  client: CosmWasmSigner,
  codeIds: Record<string, number>,
  chainName: string
) {
  const addresses: Record<string, string> = {};
  const res = await client.sign.instantiate(
    client.senderAddress,
    codeIds["kernel"],
    { chain_name: chainName },
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

export async function setupOS(client: CosmWasmSigner, chainName: string) {
  const chainId = await client.sign.getChainId();
  const osCache = await getChainCache(chainId);
  const contracts: Record<string, string> = {
    kernel: getADOPath("kernel"),
    adodb: getADOPath("adodb"),
    vfs: getADOPath("vfs"),
    economics: getADOPath("economics"),
  };
  if (osCache?.OS && !Object.keys(contracts).some((k) => !osCache.OS[k])) {
    console.debug(`Using cache for ${chainId}`);
    return osCache.OS;
  }
  const codeIds = await setupContracts(client, contracts);
  const addresses = await instantiateOs(client, codeIds, chainName);
  await assignKeyAddresses(client, addresses);
  await setChainCache(chainId, {
    OS: addresses,
    ALL_ADO: {},
    client: client.senderAddress,
  });
  return addresses;
}

export interface Cache {
  OS: Record<string, string>;
  ALL_ADO: Record<string, number>;
  client: string;
}

export async function getChainCache(chainId: string) {
  try {
    const cache = readFileSync(`./${chainId}.cache.json`, "utf8");
    return JSON.parse(cache) as Cache;
  } catch (err) {
    return undefined;
  }
}

export async function setChainCache(chainId: string, data: Cache) {
  writeFileSync(`./${chainId}.cache.json`, JSON.stringify(data, undefined, 4));
}
