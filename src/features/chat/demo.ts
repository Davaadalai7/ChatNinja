import type { ChatMessage } from "../../domain/chat";

const lines = [
  ["youtube", "Temuulen", "Сайн уу! Өнөөдөр ямар тоглоом тоглох вэ?"],
  ["kick", "ninja_bold", "One monitor. All the chat."],
  ["twitch", "saraa", "Энэ цэвэрхэн харагдаж байна ✨"],
  ["youtube", "Namuun", "GG! Сайхан тоглолт байлаа."],
  ["kick", "Bat", "Дараагийн match-д амжилт 🥷"],
  ["twitch", "nightbot", "This bot message can be filtered."],
] as const;
export function demoMessage(index: number): ChatMessage {
  const [platform, author, text] = lines[index % lines.length];
  return {
    id: `demo-${index}`,
    platform,
    channelId: "demo",
    author,
    timestamp: Date.now(),
    fragments: [{ type: "text", text }],
    moderator: index % 5 === 0,
  };
}
export const seedMessages = () =>
  Array.from({ length: 5 }, (_, i) => demoMessage(i));
