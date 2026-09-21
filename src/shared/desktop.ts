import { invoke, isTauri } from "@tauri-apps/api/core";
export interface Config {
  version: 1;
  app: { language: "mn" | "en"; closeToTray: boolean };
}
export interface Bootstrap {
  config: Config;
  version: string;
  configPath: string;
  logsPath: string;
  warning: string | null;
  trayAvailable: boolean;
}
export const desktop = isTauri();
export const bootstrap = () => invoke<Bootstrap>("bootstrap");
export const saveSettings = (config: Config) =>
  invoke<Config>("save_settings", { config });
export const resetSettings = () => invoke<Config>("reset_settings");
export const openLogs = () => invoke<void>("open_logs_folder");
export const quit = () => invoke<void>("quit_app");
export function errorText(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}
