import {
  LocalTerra,
  MsgStoreCode,
  MsgInstantiateContract,
  getContractAddress,
  getCodeId,
  TreasuryAPI,
  MsgExecuteContract,
} from "@terra-money/terra.js";
import { readFileSync } from "fs";

const lt = new LocalTerra();
const deployer = lt.wallets.validator;
const royaltyReceiver = lt.wallets.test2;
const purchaser = lt.wallets.test1;

const collection_symbol = "MNFT";

// store code
async function storeCode() {
  const fileBytes = readFileSync("artifacts/my_first_contract.wasm").toString(
    "base64"
  );

  const storeCode = new MsgStoreCode(deployer.key.accAddress, fileBytes);

  const tx = await deployer.createAndSignTx({
    msgs: [storeCode],
    feeDenoms: ["uluna"],
    gasPrices: { uluna: "0.015" },
  });

  const result = await lt.tx.broadcast(tx);
  console.log("RES", result);
  return getCodeId(result);
}

// instantiate contract
async function instantiateContract(codeId) {
  const instantiateContract = new MsgInstantiateContract(
    deployer.key.accAddress,
    +codeId,
    {}
  );

  const tx = await deployer.createAndSignTx({
    msgs: [instantiateContract],
    feeDenoms: ["uluna"],
    gasPrices: { uluna: "0.015" },
  });

  const result = await lt.tx.broadcast(tx);
  return getContractAddress(result);
}

async function createToken(contractAddress) {
  const executeContract = new MsgExecuteContract(
    deployer.key.accAddress,
    contractAddress,
    {
      create: {
        name: "My NFT",
        symbol: collection_symbol,
        extensions: [
          {
            WhiteListExtension: {
              moderators: [deployer.key.accAddress],
            },
          },
          {
            RoyaltiesExtension: {
              receivers: [royaltyReceiver.key.accAddress],
              fee: 2,
            },
          },
        ],
      },
    }
  );

  const tx = await deployer.createAndSignTx({
    msgs: [executeContract],
    feeDenoms: ["uluna"],
    gasPrices: { uluna: "0.015" },
  });

  const result = await lt.tx.broadcast(tx);
  return result;
}

async function mint(contractAddress) {
  const executeContract = new MsgExecuteContract(
    deployer.key.accAddress,
    contractAddress,
    {
      mint: { collection_symbol, token_id: 1 },
    }
  );

  const tx = await deployer.createAndSignTx({
    msgs: [executeContract],
    feeDenoms: ["uluna"],
    gasPrices: { uluna: "0.015" },
  });

  const result = await lt.tx.broadcast(tx);
  return result;
}

async function createTransferAgreement(contractAddress) {
  const executeContract = new MsgExecuteContract(
    deployer.key.accAddress,
    contractAddress,
    {
      create_transfer_agreement: {
        collection_symbol,
        token_id: 1,
        purchaser: purchaser.key.accAddress,
        amount: "1000000",
        denom: "uluna",
      },
    }
  );

  const tx = await deployer.createAndSignTx({
    msgs: [executeContract],
    feeDenoms: ["uluna"],
    gasPrices: { uluna: "0.015" },
  });

  const result = await lt.tx.broadcast(tx);
  return result;
}

async function transfer(contractAddress) {
  const executeContract = new MsgExecuteContract(
    purchaser.key.accAddress,
    contractAddress,
    {
      transfer: {
        collection_symbol,
        token_id: 1,
        from: deployer.key.accAddress,
        to: purchaser.key.accAddress,
      },
    },
    {
      uluna: "1000000",
    }
  );

  const tx = await purchaser.createAndSignTx({
    msgs: [executeContract],
    feeDenoms: ["uluna"],
    gasPrices: { uluna: "0.015" },
  });

  const result = await lt.tx.broadcast(tx);
  return result;
}

async function whitelist(contractAddress, address) {
  const executeContract = new MsgExecuteContract(
    deployer.key.accAddress,
    contractAddress,
    {
      whitelist: { collection_symbol, address },
    }
  );

  const tx = await deployer.createAndSignTx({
    msgs: [executeContract],
    feeDenoms: ["uluna"],
    gasPrices: { uluna: "0.015" },
  });

  const result = await lt.tx.broadcast(tx);
  return result;
}

async function getOwner(contractAddress, token_id) {
  return await lt.wasm.contractQuery(contractAddress, {
    get_owner: { collection_symbol, token_id },
  });
}

// async function getBalance(contractAddress, address) {
//   return await lt.wasm.contractQuery(contractAddress, {
//     get_balance: { collection_symbol, address },
//   });
// }

// async function getName(contractAddress) {
//   return await lt.wasm.contractQuery(contractAddress, {
//     get_name: { collection_symbol },
//   });
// }

async function main() {
  const codeId = await storeCode();
  const contractAddress = await instantiateContract(codeId);

  await createToken(contractAddress);
  await whitelist(contractAddress, deployer.key.accAddress);
  const mintRes = await mint(contractAddress);
  await whitelist(contractAddress, purchaser.key.accAddress);
  console.log("MINT", mintRes);
  const xferAgRes = await createTransferAgreement(contractAddress);
  // console.log("XFERAG", xferAgRes);
  const xferRes = await transfer(contractAddress);
  console.log("XFER", xferRes);
  const owner = await getOwner(contractAddress, 1);
  console.log(owner);
}

main();
