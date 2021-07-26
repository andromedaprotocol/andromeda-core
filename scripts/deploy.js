import {
  MsgStoreCode,
  LocalTerra,
  getCodeId,
  MsgInstantiateContract,
  getContractAddress,
  MsgExecuteContract,
} from "@terra-money/terra.js";
import { readFileSync } from "fs";

const lt = new LocalTerra();

const deployer = lt.wallets.validator;

async function storeCodeId(path) {
  try {
    const fileBytes = readFileSync(path).toString("base64");

    const storeCode = new MsgStoreCode(deployer.key.accAddress, fileBytes);

    const tx = await deployer.createAndSignTx({
      msgs: [storeCode],
      feeDenoms: ["uluna"],
      gasPrices: { uluna: "0.015" },
    });

    const result = await lt.tx.broadcast(tx);
    return getCodeId(result);
  } catch (error) {
    console.error(error.response ? error.response.data.error : error);
    return undefined;
  }
}

async function storeFactoryCode() {
  return storeCodeId("./artifacts/andromeda_factory.wasm");
}

async function storeTokenCode() {
  return storeCodeId("./artifacts/andromeda_token.wasm");
}

async function initFactory(factoryCodeId, tokenCodeId) {
  const instantiateContract = new MsgInstantiateContract(
    deployer.key.accAddress,
    +factoryCodeId,
    { token_code_id: parseInt(tokenCodeId) }
  );

  const tx = await deployer.createAndSignTx({
    msgs: [instantiateContract],
    feeDenoms: ["uluna"],
    gasPrices: { uluna: "0.015" },
  });

  const result = await lt.tx.broadcast(tx);
  return getContractAddress(result);
}

async function initToken(addr) {
  const executeContract = new MsgExecuteContract(
    deployer.key.accAddress,
    addr,
    { create: { symbol: "TT", name: "TT", extensions: [] } }
  );

  const tx = await deployer.createAndSignTx({
    msgs: [executeContract],
    feeDenoms: ["uluna"],
    gasPrices: { uluna: "0.015" },
  });

  const res = await lt.tx.broadcast(tx);

  return res;
}

async function queryTokenAddr(addr) {
  return await lt.wasm.contractQuery(addr, {
    get_address: { symbol: "TT" },
  });
}

async function main() {
  const factoryCode = await storeFactoryCode();
  const tokenCode = await storeTokenCode();

  const factoryAddr = await initFactory(factoryCode, tokenCode);
  await initToken(factoryAddr);
  const tokenAddr = await queryTokenAddr(factoryAddr);

  console.log(tokenAddr);
}

main();
