import axios from "axios";

import { RELAYER_URL } from "./configs";

const POLL_INTERVAL = 2000;
const MAX_POLL_COUNT = 60;

async function sleep(timeout: number) {
  return new Promise((resolve) => setTimeout(resolve, timeout));
}

export async function waitForChain(url: string) {
  for (let i = 0; i < MAX_POLL_COUNT; i++) {
    try {
      await axios.get(`${url}/status`);
      return;
    } catch {
      console.error(
        `No response from chain, retrying in ${POLL_INTERVAL / 1000}s (${
          i + 1
        }/${MAX_POLL_COUNT})`
      );
      await sleep(POLL_INTERVAL);
    }
  }

  throw new Error("Timeout reached while waiting for chain");
}

export async function waitForRelayer() {
  for (let i = 0; i < MAX_POLL_COUNT; i++) {
    try {
      await axios.get(`${RELAYER_URL}/state`);
      return;
    } catch {
      console.error(
        `No response from relayer, retrying in ${POLL_INTERVAL / 1000}s (${
          i + 1
        }/${MAX_POLL_COUNT})`
      );
      await sleep(POLL_INTERVAL);
    }
  }

  throw new Error("Timeout reached while waiting for relayer");
}
