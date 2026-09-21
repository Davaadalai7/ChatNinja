import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Platform } from "../../domain/settings";
import { native } from "../../platform/desktop";
import "./connections.css";

interface Status {
  provider: Platform;
  state: string;
  detail: string;
}
interface Connection {
  provider: Platform;
  configured: boolean;
  status: Status;
}
const states: Record<string, [string, string]> = {
  disconnected: ["Disconnected", "Холбогдоогүй"],
  setup_required: [
    "Publisher setup required",
    "App хөгжүүлэгчийн тохиргоо хэрэгтэй",
  ],
  authorizing: [
    "Complete login in your browser",
    "Browser дээр нэвтрэлтийг дуусгаарай",
  ],
  authenticated: ["Signed in", "Нэвтэрсэн"],
  waiting_live: [
    "Waiting for your live broadcast",
    "Таны live эхлэхийг хүлээж байна",
  ],
  connected: ["Receiving live chat", "Live чат холбогдсон"],
  reconnecting: ["Reconnecting", "Дахин холбогдож байна"],
  error: ["Connection failed", "Холболт амжилтгүй"],
};
export function Connections({
  language,
  onConnect,
}: {
  language: "mn" | "en";
  onConnect: () => void;
}) {
  const [connections, setConnections] = useState<Connection[]>([]);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState<Platform | null>(null);
  const mn = language === "mn";
  useEffect(() => {
    if (!native) return;
    let active = true;
    const stop = listen<Status>("connection-status", (event) => {
      if (active)
        setConnections((previous) =>
          previous.map((c) =>
            c.provider === event.payload.provider
              ? { ...c, status: event.payload }
              : c,
          ),
        );
    });
    void stop
      .then(() => invoke<Connection[]>("connection_status"))
      .then((data) => {
        if (active) setConnections(data);
      })
      .catch(() => {
        if (active) setError("Connection state unavailable");
      });
    return () => {
      active = false;
      void stop.then((unlisten) => unlisten()).catch(() => {});
    };
  }, []);
  async function act(
    provider: Platform,
    action: "connect" | "disconnect" | "forget",
  ) {
    setBusy(provider);
    setError("");
    try {
      if (action === "connect") {
        await invoke("connect_provider", { provider });
        onConnect();
      } else
        await invoke("disconnect_provider", {
          provider,
          forget: action === "forget",
        });
    } catch (e) {
      setError(typeof e === "string" ? e : "Connection action failed");
    } finally {
      setBusy(null);
    }
  }
  return (
    <div className="connection-list">
      <p className="notice">
        {mn
          ? "Нэвтрэлт таны browser дээр нээгдэнэ. Token-ууд Windows Credential Manager-д хадгалагдана. Demo-г унтрааж бодит чат руу шилжинэ."
          : "Sign in using your browser. Tokens stay in Windows Credential Manager. Connecting switches off sample chat."}
      </p>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {(["youtube", "kick", "twitch"] as Platform[]).map((provider) => {
        const connection = connections.find((c) => c.provider === provider);
        const state = connection?.status.state ?? "setup_required";
        return (
          <article key={provider} className="card platform-card">
            <div className="platform-heading">
              <strong>
                {provider === "youtube"
                  ? "YouTube"
                  : provider === "kick"
                    ? "Kick"
                    : "Twitch"}
              </strong>
              <small>{provider === "twitch" ? "SECONDARY" : "PRIMARY"}</small>
            </div>
            <p>{(states[state] ?? [state, state])[mn ? 1 : 0]}</p>
            {connection?.status.detail && (
              <p className="hint">{connection.status.detail}</p>
            )}
            {!native && (
              <p className="hint">
                {mn
                  ? "Windows app дээр ашиглана."
                  : "Requires the Windows desktop app."}
              </p>
            )}
            {provider === "kick" && !connection?.configured && (
              <p className="hint">
                {mn
                  ? "Kick OAuth/webhook серверийг тохируулсны дараа идэвхжинэ."
                  : "Requires the Kick OAuth/webhook backend before activation."}
              </p>
            )}
            <div className="connection-actions">
              <button
                className="primary"
                disabled={!native || !connection?.configured || busy !== null}
                onClick={() => void act(provider, "connect")}
              >
                {mn ? "Холбох / дахин оролдох" : "Connect / retry"}
              </button>
              <button
                className="secondary"
                disabled={!native || !connection?.configured || busy !== null}
                onClick={() => void act(provider, "disconnect")}
              >
                {mn ? "Зогсоох" : "Stop"}
              </button>
              <button
                className="secondary"
                disabled={!native || !connection?.configured || busy !== null}
                onClick={() => void act(provider, "forget")}
              >
                {mn ? "Гарах" : "Sign out"}
              </button>
            </div>
          </article>
        );
      })}
    </div>
  );
}
