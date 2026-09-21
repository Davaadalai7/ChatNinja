import { invoke, isTauri } from "@tauri-apps/api/core";
import type { Settings } from "../domain/settings";
import type { ChatMessage } from "../domain/chat";

export interface Snapshot {
  settings: Settings;
  messages: ChatMessage[];
}
export const native = isTauri();
export async function syncSnapshot(snapshot: Snapshot): Promise<void> {
  if (native) await invoke("set_snapshot", { snapshot });
}
export async function controlOverlay(
  action: "show" | "hide" | "apply",
  settings: Settings,
): Promise<void> {
  if (!native) throw new Error("Windows desktop app required.");
  await invoke("control_overlay", { action, settings });
}
export async function getSnapshot(): Promise<Snapshot> {
  return invoke("get_snapshot");
}
export async function getObsUrl(): Promise<string> {
  return invoke("get_obs_url");
}
