import { MsgStoreCode, LocalTerra } from '@terra-money/terra.js';
import { readFileSync } from 'fs';

const lt = new LocalTerra();

const deployer = lt.wallets.validator;

async function storeCodeId(path) {

    try {
        const fileBytes = readFileSync(path).toString(
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
    } catch (error) {
        console.error(error.response.data.error)
       return undefined; 
    }
}

async function storeFactoryCode() {
    return storeCodeId("./artifacts/andromeda_factory.wasm");
}

async function storeTokenCode() {
    return storeCodeId("./artifacts/andromeda_token.wasm");
}

async function main() {
    const factoryCode = await storeFactoryCode();
    const tokenCode = await storeTokenCode();

    console.log(factoryCode, tokenCode);
}

main();