import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  defaults,
  loadSettings,
  STORAGE_KEY,
  type Platform,
  type Settings,
} from "./domain/settings";
import { appendMessage, type ChatMessage } from "./domain/chat";
import { dictionary } from "./i18n";
import { demoMessage, seedMessages } from "./features/chat/demo";
import { ChatFeed } from "./features/chat/ChatFeed";
import {
  controlOverlay,
  getObsUrl,
  native,
  syncSnapshot,
} from "./platform/desktop";
import { Connections } from "./features/channels/Connections";

type Tab = "channels" | "overlay" | "obs" | "general";
function Toggle({
  label,
  checked,
  change,
}: {
  label: string;
  checked: boolean;
  change: (value: boolean) => void;
}) {
  return (
    <label className="toggle-row">
      <span>{label}</span>
      <input
        type="checkbox"
        role="switch"
        checked={checked}
        onChange={(event) => change(event.target.checked)}
      />
    </label>
  );
}
export function App() {
  const [settings, setSettings] = useState(loadSettings);
  const [tab, setTab] = useState<Tab>("overlay");
  const [messages, setMessages] = useState<ChatMessage[]>(
    settings.demo ? seedMessages : [],
  );
  const [liveMessages, setLiveMessages] = useState<ChatMessage[]>([]);
  const [error, setError] = useState("");
  const [obsUrl, setObsUrl] = useState("");
  const [copied, setCopied] = useState(false);
  const t = dictionary[settings.language];
  const displayedMessages = settings.demo ? messages : liveMessages;
  useEffect(() => {
    if (!native) return;
    let active = true;
    const unlisten = listen<ChatMessage[]>("live-messages", (event) => {
      if (active) setLiveMessages(event.payload);
    });
    return () => {
      active = false;
      void unlisten.then((stop) => stop()).catch(() => {});
    };
  }, []);
  useEffect(() => {
    if (!native) return;
    const unlisten = listen<boolean>("overlay-lock", (event) =>
      setSettings((previous) => ({ ...previous, clickThrough: event.payload })),
    );
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);
  function update<K extends keyof Settings>(key: K, value: Settings[K]) {
    setSettings((previous) => ({ ...previous, [key]: value }));
  }
  useEffect(() => {
    document.documentElement.lang = settings.language;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
    } catch {
      setError(dictionary[settings.language].settingsError);
    }
  }, [settings]);
  useEffect(() => {
    setMessages(settings.demo ? seedMessages() : []);
    if (!settings.demo) return;
    let index = 5;
    const interval = setInterval(
      () =>
        setMessages((previous) =>
          appendMessage(previous, demoMessage(index++)),
        ),
      4200,
    );
    return () => clearInterval(interval);
  }, [settings.demo]);
  useEffect(() => {
    void syncSnapshot({ settings, messages }).catch(() =>
      setError("Desktop sync failed. Retry opening the overlay."),
    );
  }, [settings, messages]);
  useEffect(() => {
    if (native)
      void getObsUrl()
        .then(setObsUrl)
        .catch(() => setError("Local OBS server unavailable."));
  }, []);
  async function action(kind: "show" | "hide" | "apply") {
    try {
      await controlOverlay(kind, settings);
      setError("");
    } catch (error) {
      setError(
        error instanceof Error ? error.message : "Overlay action failed",
      );
    }
  }
  function platformToggle(platform: Platform, enabled: boolean) {
    update(
      "enabledPlatforms",
      enabled
        ? [...settings.enabledPlatforms, platform]
        : settings.enabledPlatforms.filter((value) => value !== platform),
    );
  }
  const range = (
    key: "fontSize" | "opacity",
    label: string,
    min: number,
    max: number,
    step: number,
  ) => (
    <label className="range-field">
      <span>
        {label}
        <b>
          {key === "opacity"
            ? `${Math.round(settings[key] * 100)}%`
            : `${settings[key]} px`}
        </b>
      </span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={settings[key]}
        onChange={(event) => update(key, Number(event.target.value))}
      />
    </label>
  );
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <a
          className="brand"
          href="#"
          onClick={(event) => {
            event.preventDefault();
            setTab("overlay");
          }}
        >
          <span className="brand-mark">≋</span>ChatNinja
          <span className="alpha">α</span>
        </a>
        <p className="tagline">{t.subtitle}</p>
        <div className="nav-label">{t.workspace}</div>
        <nav>
          {(["channels", "overlay", "obs", "general"] as Tab[]).map(
            (item, index) => (
              <button
                key={item}
                className={tab === item ? "nav-item active" : "nav-item"}
                aria-current={tab === item ? "page" : undefined}
                onClick={() => setTab(item)}
              >
                <span>{["☷", "▣", "↗", "⚙"][index]}</span>
                {t[item]}
              </button>
            ),
          )}
        </nav>
        <div className="sidebar-footer">
          <span className="status-dot" />
          {t.local}
          <p>Windows 10 / 11</p>
        </div>
      </aside>
      <main>
        <div className="topbar">
          <span>ChatNinja / {t[tab]}</span>
          <select
            aria-label={t.language}
            value={settings.language}
            onChange={(event) =>
              update("language", event.target.value as "mn" | "en")
            }
          >
            <option value="mn">Монгол</option>
            <option value="en">English</option>
          </select>
        </div>
        <div className="page-content">
          <header>
            <div className="eyebrow">{t[tab]}</div>
            <h1>
              {tab === "channels"
                ? t.connection
                : tab === "obs"
                  ? t.obsTitle
                  : tab === "general"
                    ? t.general
                    : t.appearance}
            </h1>
            <p>
              {tab === "channels"
                ? t.connectionNote
                : tab === "obs"
                  ? t.obsNote
                  : t.appearanceNote}
            </p>
          </header>
          {error && (
            <div className="error" role="alert">
              {error}
              <button aria-label="Dismiss error" onClick={() => setError("")}>
                ×
              </button>
            </div>
          )}
          <div className="content-grid">
            <section className="controls">
              {tab === "channels" && (
                <>
                  <Connections
                    language={settings.language}
                    onConnect={() => update("demo", false)}
                  />
                  <article className="card">
                    <h2>{t.platformLabels}</h2>
                    {(["youtube", "kick", "twitch"] as Platform[]).map(
                      (platform) => (
                        <Toggle
                          key={platform}
                          label={platform}
                          checked={settings.enabledPlatforms.includes(platform)}
                          change={(value) => platformToggle(platform, value)}
                        />
                      ),
                    )}
                  </article>
                </>
              )}
              {tab === "overlay" && (
                <>
                  <article className="card">
                    <h2>{t.appearanceTitle}</h2>
                    {range("fontSize", t.font, 12, 32, 1)}
                    {range("opacity", t.opacity, 0, 1, 0.05)}
                    <Toggle
                      label={t.platformLabels}
                      checked={settings.platformLabels}
                      change={(value) => update("platformLabels", value)}
                    />
                    <Toggle
                      label={t.timestamp}
                      checked={settings.timestamp}
                      change={(value) => update("timestamp", value)}
                    />
                  </article>
                  <article className="card">
                    <h2>{t.position}</h2>
                    <div className="number-grid">
                      {(["width", "height", "x", "y"] as const).map((key) => (
                        <label key={key}>
                          {key === "width"
                            ? t.width
                            : key === "height"
                              ? t.height
                              : key.toUpperCase()}
                          <input
                            type="number"
                            value={settings[key]}
                            min={
                              key === "width"
                                ? 240
                                : key === "height"
                                  ? 180
                                  : -32000
                            }
                            max={
                              key === "width"
                                ? 1200
                                : key === "height"
                                  ? 1600
                                  : 32000
                            }
                            onChange={(event) => {
                              const n = event.target.valueAsNumber;
                              if (
                                Number.isFinite(n) &&
                                event.target.validity.valid
                              )
                                update(key, Math.round(n));
                            }}
                          />
                        </label>
                      ))}
                    </div>
                    <button
                      className="secondary wide"
                      disabled={!native}
                      onClick={() => void action("apply")}
                    >
                      {t.apply}
                    </button>
                    <Toggle
                      label={t.clickThrough}
                      checked={settings.clickThrough}
                      change={(value) => update("clickThrough", value)}
                    />
                    <p className="hint">{t.instructions}</p>
                  </article>
                </>
              )}
              {tab === "obs" && (
                <article className="card">
                  <h2>{t.mode}</h2>
                  <select
                    className="wide"
                    value={settings.visibility}
                    onChange={(event) =>
                      update(
                        "visibility",
                        event.target.value as Settings["visibility"],
                      )
                    }
                  >
                    <option value="streamer">{t.streamer}</option>
                    <option value="obs">{t.obsOnly}</option>
                    <option value="both">{t.both}</option>
                  </select>
                  <label className="source-label">
                    {t.source}
                    <input
                      readOnly
                      value={obsUrl}
                      placeholder={t.desktopOnly}
                    />
                  </label>
                  <button
                    className="secondary"
                    disabled={!obsUrl}
                    onClick={() => {
                      void navigator.clipboard
                        .writeText(obsUrl)
                        .then(() => setCopied(true))
                        .catch(() => setError("Clipboard unavailable"));
                    }}
                  >
                    {copied ? t.copied : t.copy}
                  </button>
                  <p className="hint">{t.sourceNote}</p>
                  {settings.visibility === "streamer" && (
                    <p className="notice">{t.obsDisabled}</p>
                  )}
                  <p className="notice">{t.privacy}</p>
                </article>
              )}
              {tab === "general" && (
                <article className="card">
                  <h2>{t.general}</h2>
                  <Toggle
                    label={t.demo}
                    checked={settings.demo}
                    change={(value) => update("demo", value)}
                  />
                  <Toggle
                    label={t.bots}
                    checked={settings.hideBots}
                    change={(value) => update("hideBots", value)}
                  />
                  <label className="source-label">
                    {t.botNames}
                    <textarea
                      maxLength={1000}
                      value={settings.botNames}
                      onChange={(event) =>
                        update("botNames", event.target.value)
                      }
                    />
                  </label>
                  <p className="notice">{t.safety}</p>
                  <button
                    className="secondary"
                    onClick={() =>
                      setSettings((previous) => ({
                        ...defaults,
                        language: previous.language,
                      }))
                    }
                  >
                    {t.reset}
                  </button>
                </article>
              )}
            </section>
            <aside className="preview-panel">
              <div className="preview-heading">
                <h2>{t.preview}</h2>
                <span className="badge">
                  {settings.demo ? "DEMO" : "LIVE MODE"}
                </span>
              </div>
              <div className="preview-stage">
                <div className="scene-lines" />
                <div className="preview-chat">
                  <ChatFeed messages={displayedMessages} settings={settings} />
                </div>
                <span className="preview-caption">
                  {settings.width} × {settings.height}
                </span>
              </div>
              <p className="hint">{settings.demo ? t.demoNote : t.liveNote}</p>
              <button
                className="primary wide"
                disabled={!native || settings.visibility === "obs"}
                onClick={() => void action("show")}
              >
                {t.open}
                <span>↗</span>
              </button>
              <button
                className="secondary wide"
                disabled={!native}
                onClick={() => void action("hide")}
              >
                {t.hide}
              </button>
              {!native && <p className="hint">{t.desktopOnly}</p>}
            </aside>
          </div>
        </div>
        <footer>
          CHATNINJA <span>Less switching. More streaming.</span>
        </footer>
      </main>
    </div>
  );
}
