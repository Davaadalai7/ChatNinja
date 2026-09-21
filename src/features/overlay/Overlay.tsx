import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { defaults } from "../../domain/settings";
import { getSnapshot, native, type Snapshot } from "../../platform/desktop";
import { ChatFeed } from "../chat/ChatFeed";

export function Overlay() {
  const [snapshot, setSnapshot] = useState<Snapshot>({
    settings: defaults,
    messages: [],
  });
  const [error, setError] = useState("");
  useEffect(() => {
    if (!native) return;
    let alive = true;
    const refresh = async () => {
      try {
        const next = await getSnapshot();
        if (alive) {
          setSnapshot(next);
          setError("");
        }
      } catch {
        if (alive) setError("Overlay sync unavailable");
      }
    };
    void refresh();
    const timer = setInterval(() => void refresh(), 250);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  }, []);
  const drag = (resize: boolean) => {
    if (!native) return;
    const window = getCurrentWindow();
    void (
      resize ? window.startResizeDragging("SouthEast") : window.startDragging()
    ).catch(() => setError("Window action failed"));
  };
  return (
    <div className="desktop-overlay">
      {!snapshot.settings.clickThrough && (
        <div className="drag-bar" onPointerDown={() => drag(false)}>
          ⠿ ChatNinja <span>{snapshot.settings.demo ? "DEMO" : "LIVE"}</span>
        </div>
      )}
      {error && <div role="alert">{error}</div>}
      <ChatFeed messages={snapshot.messages} settings={snapshot.settings} />
      {!snapshot.settings.clickThrough && (
        <button
          aria-label="Resize overlay"
          className="resize-handle"
          onPointerDown={() => drag(true)}
        >
          ◢
        </button>
      )}
    </div>
  );
}
