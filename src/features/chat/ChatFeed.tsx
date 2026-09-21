import { useEffect, useRef } from "react";
import {
  safeEmoteUrl,
  visibleMessages,
  type ChatMessage,
} from "../../domain/chat";
import type { Settings } from "../../domain/settings";
import { dictionary } from "../../i18n";

export function ChatFeed({
  messages,
  settings,
}: {
  messages: ChatMessage[];
  settings: Settings;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const visible = visibleMessages(messages, settings);
  useEffect(() => {
    ref.current?.scrollTo({ top: ref.current.scrollHeight });
  }, [messages]);
  return (
    <div
      className="chat-feed"
      ref={ref}
      style={{
        fontSize: settings.fontSize,
        background: `rgba(14,14,16,${settings.opacity})`,
      }}
    >
      {visible.length === 0 && (
        <p className="empty">{dictionary[settings.language].empty}</p>
      )}
      {visible.map((message) => (
        <div
          className="chat-message"
          key={`${message.platform}:${message.channelId}:${message.id}`}
        >
          <div className="chat-meta">
            {settings.platformLabels && (
              <span className="platform-tag">
                {message.platform === "youtube"
                  ? "YT"
                  : message.platform === "kick"
                    ? "K"
                    : "TW"}
              </span>
            )}
            <strong>{message.author}</strong>
            {message.moderator && <span title="Moderator">◇</span>}
            {settings.timestamp && (
              <time>
                {new Date(message.timestamp).toLocaleTimeString(
                  settings.language,
                  { hour: "2-digit", minute: "2-digit" },
                )}
              </time>
            )}
          </div>
          <div className="message-body">
            {message.fragments.map((fragment, index) =>
              fragment.type === "emote" && safeEmoteUrl(fragment.url) ? (
                <img
                  key={index}
                  src={fragment.url}
                  alt={fragment.name}
                  title={fragment.name}
                  referrerPolicy="no-referrer"
                  className="emote"
                  onError={(event) => {
                    event.currentTarget.style.display = "none";
                  }}
                />
              ) : (
                <span key={index}>
                  {fragment.type === "text" ? fragment.text : fragment.name}
                </span>
              ),
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
