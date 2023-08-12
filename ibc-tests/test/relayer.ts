import axios from "axios";

const URL = "http://localhost:3000";
const POLL_INTERVAL = 1000;
const MAX_POLL_COUNT = 120;

async function sleep(timeout: number) {
  return new Promise((resolve) => setTimeout(resolve, timeout));
}

export async function waitForRelayer() {
  for (let i = 0; i < MAX_POLL_COUNT; i++) {
    try {
      await axios.get(`${URL}/state`);
      return;
    } catch {
      console.error("No response from relayer, retrying in 1s");
      await sleep(POLL_INTERVAL);
    }
  }

  throw new Error("Timeout reached while waiting for relayer");
}
