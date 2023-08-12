import axios from "axios";

const URL = "http://localhost:3000";
const POLL_INTERVAL = 1000;
const MAX_POLL_COUNT = 120;

async function sleep(timeout: number) {
  return new Promise((resolve) => setTimeout(resolve, timeout));
}

const clearLastLine = () => {
  process.stdout.moveCursor(0, -1); // up one line
  process.stdout.clearLine(1); // from cursor to end
};

export async function waitForRelayer() {
  for (let i = 0; i < MAX_POLL_COUNT; i++) {
    try {
      await axios.get(`${URL}/state`);
      return;
    } catch {
      if (i > 0) clearLastLine();
      console.error(
        `No response from relayer, retrying in 1s (${i + 1}/${MAX_POLL_COUNT})`
      );
      await sleep(POLL_INTERVAL);
    }
  }

  throw new Error("Timeout reached while waiting for relayer");
}
