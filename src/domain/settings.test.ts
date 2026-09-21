import { describe, expect, it } from "vitest";
import { defaults, parseSettings, settingsSchema } from "./settings";

describe("settings", () => {
  it("accepts valid defaults", () =>
    expect(settingsSchema.parse(defaults)).toEqual(defaults));
  it.each([
    null,
    {},
    { ...defaults, version: 2 },
    { ...defaults, fontSize: 100 },
    { ...defaults, opacity: -1 },
    { ...defaults, width: 0 },
    { ...defaults, language: "eu" },
    { ...defaults, visibility: "public" },
  ])("recovers invalid saved data: %j", (value) =>
    expect(parseSettings(value)).toEqual(defaults),
  );
  it("preserves user preferences", () =>
    expect(parseSettings({ ...defaults, language: "en", x: -400 }).x).toBe(
      -400,
    ));
  it("rejects invalid platforms", () =>
    expect(
      settingsSchema.safeParse({ ...defaults, enabledPlatforms: ["unknown"] })
        .success,
    ).toBe(false));
});
