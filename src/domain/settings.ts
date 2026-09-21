import { z } from "zod";

export const platformSchema = z.enum(["youtube", "kick", "twitch"]);
export type Platform = z.infer<typeof platformSchema>;
export const settingsSchema = z.object({
  version: z.literal(1),
  language: z.enum(["mn", "en"]),
  fontSize: z.number().int().min(12).max(32),
  opacity: z.number().min(0).max(1),
  width: z.number().int().min(160).max(1600),
  height: z.number().int().min(100).max(1600),
  x: z.number().int().min(-32000).max(32000),
  y: z.number().int().min(-32000).max(32000),
  visibility: z.enum(["streamer", "obs", "both"]),
  clickThrough: z.boolean(),
  timestamp: z.boolean(),
  hideBots: z.boolean(),
  botNames: z.string().max(1000),
  platformLabels: z.boolean(),
  enabledPlatforms: z.array(platformSchema).max(3),
  demo: z.boolean(),
});
export type Settings = z.infer<typeof settingsSchema>;
export const defaults: Settings = {
  version: 1,
  language: "mn",
  fontSize: 17,
  opacity: 0.65,
  width: 360,
  height: 480,
  x: 40,
  y: 80,
  visibility: "streamer",
  clickThrough: false,
  timestamp: false,
  hideBots: true,
  botNames: "nightbot, kicklet, botrix",
  platformLabels: true,
  enabledPlatforms: ["youtube", "kick", "twitch"],
  demo: true,
};
export function parseSettings(value: unknown): Settings {
  const result = settingsSchema.safeParse(value);
  return result.success ? result.data : { ...defaults };
}
export const STORAGE_KEY = "chatninja.settings.v1";
export const OVERLAY_VISIBILITY_KEY = "chatninja.overlay.visible.v1";
export function overlayWanted(): boolean {
  try {
    return localStorage.getItem(OVERLAY_VISIBILITY_KEY) !== "false";
  } catch {
    return true;
  }
}
export function saveOverlayWanted(visible: boolean): void {
  try {
    localStorage.setItem(OVERLAY_VISIBILITY_KEY, String(visible));
  } catch {
    // Window state remains usable for the current session.
  }
}
export function loadSettings(): Settings {
  try {
    return parseSettings(
      JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null"),
    );
  } catch {
    return { ...defaults };
  }
}
