import { appendFileSync, writeFileSync } from "fs";

import { Logger, LogMethod } from "@confio/relayer/build/lib/logger";
import { SinonSpy, spy } from "sinon";

export class CustomLogger implements Logger {
  readonly error: SinonSpy & LogMethod;
  readonly warn: SinonSpy & LogMethod;
  readonly info: SinonSpy & LogMethod;
  readonly verbose: SinonSpy & LogMethod;
  readonly debug: SinonSpy & LogMethod;
  readonly child: () => CustomLogger;
  readonly log: SinonSpy & LogMethod;
  constructor() {
    reset();
    const createSpy = (logFn: typeof logger, type: string) => {
      return spy((...args) => {
        logFn(`${type.toUpperCase()}::${new Date().toISOString()}`);
        args.forEach((message) => {
          if (message) logFn(message, "\n👉👉");
        });
        return this;
      }).bind(this);
    };
    this.error = createSpy(logger, "ERROR");
    this.warn = createSpy(logger, "WARN");
    this.info = createSpy(logger, "INFO");
    this.verbose = createSpy(logger, "VERBOSE");
    this.debug = createSpy(logger, "DEBUG");
    this.child = () => this;
    this.log = createSpy(logger, "CUSTOM LOG");
  }
}

const FILE_NAME = `./relayer.log.txt`;
async function logger(data: any, lineBreak = "\n📄") {
  if (typeof data !== "string") data = JSON.stringify(data);
  appendFileSync(FILE_NAME, lineBreak + data);
}

async function reset() {
  writeFileSync(FILE_NAME, "Logger Starting now\n");
}
