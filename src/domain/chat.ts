import type { Platform, Settings } from "./settings";

export type Fragment =
  | { type: "text"; text: string }
  | {
      type: "emote";
      id: string;
      name: string;
      url: string;
    };
export interface ChatMessage {
  id: string;
  platform: Platform;
  channelId: string;
  author: string;
  timestamp: number;
  fragments: Fragment[];
  moderator: boolean;
}
export interface ChatAdapter {
  platform: Platform;
  connect(
    channelId: string,
    onMessage: (message: ChatMessage) => void,
    signal: AbortSignal,
  ): Promise<void>;
}
export function visibleMessages(
  messages: ChatMessage[],
  settings: Settings,
): ChatMessage[] {
  const bots = new Set(
    settings.botNames
      .split(",")
      .map((name) => name.trim().toLowerCase())
      .filter(Boolean),
  );
  return messages
    .filter(
      (message) =>
        settings.enabledPlatforms.includes(message.platform) &&
        !(settings.hideBots && bots.has(message.author.toLowerCase())),
    )
    .slice(-100);
}
/** Composite keys prevent equal provider IDs from dropping unrelated messages. */
export function appendMessage(
  messages: ChatMessage[],
  incoming: ChatMessage,
): ChatMessage[] {
  const key = (m: ChatMessage) => `${m.platform}:${m.channelId}:${m.id}`;
  if (messages.some((message) => key(message) === key(incoming)))
    return messages;
  return [...messages, incoming].slice(-200);
}
const emoteHosts = [
  "static-cdn.jtvnw.net",
  "yt3.ggpht.com",
  "yt3.googleusercontent.com",
  "files.kick.com",
  "cdn.7tv.app",
  "cdn.betterttv.net",
  "cdn.frankerfacez.com",
];
export function safeEmoteUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return (
      url.protocol === "https:" &&
      !url.username &&
      !url.password &&
      emoteHosts.includes(url.hostname)
    );
  } catch {
    return false;
  }
}
