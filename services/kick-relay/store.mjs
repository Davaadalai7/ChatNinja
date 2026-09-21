import { readFile, writeFile, rename, mkdir } from "node:fs/promises";
import { dirname } from "node:path";
import { seal, unseal } from "./security.mjs";

/** Single-process deployment only; use a transactional database before scaling out. */
export class Store {
  constructor(path, key) {
    this.path = path;
    this.key = key;
    this.sessions = {};
    this.writes = Promise.resolve();
  }
  async load() {
    try {
      this.sessions = unseal(await readFile(this.path, "utf8"), this.key);
    } catch (e) {
      if (e.code !== "ENOENT") throw new Error("credential_store_unreadable");
    }
  }
  async persist() {
    const data = seal(this.sessions, this.key);
    const write = this.writes.then(async () => {
      await mkdir(dirname(this.path), { recursive: true, mode: 0o700 });
      await writeFile(`${this.path}.next`, data, { mode: 0o600 });
      await rename(`${this.path}.next`, this.path);
    });
    this.writes = write.catch(() => {});
    await write;
  }
}
