import fs from "fs";

import { CosmWasmSigner } from "@confio/relayer";

export default class Contract {
  constructor(public address: string, public codeId?: number) {}

  static fromAddress(address: string) {
    return new Contract(address);
  }

  static async fromCodeId(
    codeId: number,
    instantiateMsg: Record<string, unknown>,
    signer: CosmWasmSigner
  ) {
    const instantiate = await signer.sign.instantiate(
      signer.senderAddress,
      codeId,
      instantiateMsg,
      "Instantiate",
      "auto"
    );
    return new Contract(instantiate.contractAddress, codeId);
  }

  static async fromWasmCode(
    wasmPath: string,
    instantiateMsg: Record<string, unknown>,
    signer: CosmWasmSigner
  ) {
    const wasm = fs.readFileSync(wasmPath);
    const { codeId } = await signer.sign.upload(
      signer.senderAddress,
      wasm,
      "auto"
    );
    return Contract.fromCodeId(codeId, instantiateMsg, signer);
  }

  async execute(
    msg: Record<string, unknown>,
    signer: CosmWasmSigner,
    funds: { amount: string; denom: string }[] = [],
    memo = "auto",
    senderAddress?: string
  ) {
    return signer.sign.execute(
      senderAddress ?? signer.senderAddress,
      this.address,
      msg,
      "auto",
      memo,
      funds
    );
  }

  async query<T>(
    msg: Record<string, unknown>,
    signer: CosmWasmSigner
  ): Promise<T> {
    return signer.sign.queryContractSmart(this.address, msg);
  }

  async getPort(signer: CosmWasmSigner): Promise<string> {
    const { ibcPortId } = await signer.sign.getContract(this.address);
    return ibcPortId!;
  }
}
