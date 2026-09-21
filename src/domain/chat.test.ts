import { describe, expect, it } from "vitest";
import { appendMessage, safeEmoteUrl, visibleMessages } from "./chat";
import { defaults } from "./settings";
import { demoMessage } from "../features/chat/demo";

describe("chat pipeline", () => {
  it("deduplicates per provider and channel", () => {
    const m = demoMessage(0);
    expect(appendMessage([m], m)).toHaveLength(1);
    expect(appendMessage([m], { ...m, platform: "kick" })).toHaveLength(2);
    expect(appendMessage([m], { ...m, channelId: "other" })).toHaveLength(2);
  });
  it("bounds retained history", () => {
    const messages = Array.from({ length: 200 }, (_, i) => demoMessage(i));
    expect(appendMessage(messages, demoMessage(201))).toHaveLength(200);
  });
  it("filters bots case-insensitively", () =>
    expect(
      visibleMessages([{ ...demoMessage(0), author: "NightBot" }], defaults),
    ).toHaveLength(0));
  it("can show bots", () =>
    expect(
      visibleMessages([demoMessage(5)], { ...defaults, hideBots: false }),
    ).toHaveLength(1));
  it("filters disabled platforms", () =>
    expect(
      visibleMessages([demoMessage(0)], {
        ...defaults,
        enabledPlatforms: ["kick"],
      }),
    ).toHaveLength(0));
  it("caps visible messages at 100", () =>
    expect(
      visibleMessages(
        Array.from({ length: 200 }, (_, i) => demoMessage(i)),
        defaults,
      ),
    ).toHaveLength(100));
  it.each([
    "javascript:alert(1)",
    "http://cdn.7tv.app/a",
    "https://cdn.7tv.app.evil.test/a",
    "https://evil.test/a",
    "https://user:pass@cdn.7tv.app/a",
    "broken",
  ])("rejects unsafe image URLs: %s", (value) =>
    expect(safeEmoteUrl(value)).toBe(false),
  );
  it("accepts allowlisted HTTPS assets", () =>
    expect(safeEmoteUrl("https://cdn.7tv.app/emote/id/2x.webp")).toBe(true));
});
