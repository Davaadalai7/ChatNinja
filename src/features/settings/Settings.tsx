import { useEffect, useState } from "react";
import {
  bootstrap,
  desktop,
  errorText,
  openLogs,
  quit,
  resetSettings,
  saveSettings,
  type Bootstrap,
  type Config,
} from "../../shared/desktop";
import { strings } from "../../shared/i18n";
type Tab = "general" | "diagnostics" | "about";

export function Settings() {
  const [boot, setBoot] = useState<Bootstrap | null>(null);
  const [draft, setDraft] = useState<Config | null>(null);
  const [tab, setTab] = useState<Tab>("general");
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const [confirmReset, setConfirmReset] = useState(false);
  const t = strings[draft?.app.language ?? "mn"];
  async function load() {
    setError("");
    try {
      const result = await bootstrap();
      setBoot(result);
      setDraft(result.config);
    } catch (reason) {
      setError(errorText(reason));
    }
  }
  useEffect(() => {
    if (desktop) void load();
  }, []);
  useEffect(() => {
    document.documentElement.lang = draft?.app.language ?? "mn";
  }, [draft?.app.language]);
  async function action(task: () => Promise<unknown>) {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await task();
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      setBusy(false);
    }
  }
  async function save(reset = false) {
    if (!draft) return;
    await action(async () => {
      const result = reset ? await resetSettings() : await saveSettings(draft);
      setDraft(result);
      setBoot((previous) =>
        previous ? { ...previous, config: result } : previous,
      );
      setNotice(strings[result.app.language].saved);
      setConfirmReset(false);
    });
  }
  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="brand">
          <img src="/logo.svg" alt="術" />
          <div>
            <strong>
              Jutsu<span>術</span>
            </strong>
            <small>STREAM TOOLS</small>
          </div>
        </div>
        <p className="tagline">{t.subtitle}</p>
        <nav aria-label="Settings sections">
          {(["general", "diagnostics", "about"] as const).map((item, index) => (
            <button
              aria-current={tab === item ? "page" : undefined}
              key={item}
              onClick={() => setTab(item)}
            >
              <span className="nav-number">0{index + 1}</span>
              {t[item]}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="status-dot" />
          {t.scope}
          <small>Developed by star0x7f</small>
        </div>
      </aside>
      <main>
        <header>
          <div>
            <div className="eyebrow">JUTSU / {t.foundation}</div>
            <h1>
              {tab === "general"
                ? t.preferences
                : tab === "diagnostics"
                  ? t.diagnose
                  : "Jutsu 術"}
            </h1>
            <p>
              {tab === "general"
                ? t.intro
                : tab === "diagnostics"
                  ? t.diagnoseHelp
                  : t.aboutBody}
            </p>
          </div>
          <span className="version">v{boot?.version ?? "0.1.0"}</span>
        </header>
        {!desktop ? (
          <section className="card">
            <h2>{t.desktopRequired}</h2>
            <p>{t.desktopHelp}</p>
            <code>npm run desktop</code>
          </section>
        ) : (
          <>
            {error && (
              <div className="error" role="alert">
                <strong>{t.startupError}</strong>
                <p>{error}</p>
                {!boot && (
                  <button onClick={() => void load()}>{t.retry}</button>
                )}
              </div>
            )}
            {!boot && !error && <p role="status">Jutsu…</p>}
            {boot?.warning && (
              <div className="warning" role="status">
                {boot.warning}
              </div>
            )}
            {notice && (
              <div className="notice" role="status">
                {notice}
              </div>
            )}
            {boot && draft && tab === "general" && (
              <>
                <section className="card">
                  <div className="section-heading">
                    <h2>{t.general}</h2>
                    <span>01</span>
                  </div>
                  <label className="setting-row">
                    <span>{t.language}</span>
                    <select
                      disabled={busy}
                      value={draft.app.language}
                      onChange={(event) =>
                        setDraft({
                          ...draft,
                          app: {
                            ...draft.app,
                            language: event.target.value === "en" ? "en" : "mn",
                          },
                        })
                      }
                    >
                      <option value="mn">Монгол</option>
                      <option value="en">English</option>
                    </select>
                  </label>
                  <label className="setting-row">
                    <span>
                      {t.close}
                      <small>{t.closeHelp}</small>
                    </span>
                    <input
                      type="checkbox"
                      role="switch"
                      checked={draft.app.closeToTray}
                      disabled={busy || !boot.trayAvailable}
                      onChange={(event) =>
                        setDraft({
                          ...draft,
                          app: {
                            ...draft.app,
                            closeToTray: event.target.checked,
                          },
                        })
                      }
                    />
                  </label>
                  <div className="actions">
                    <button
                      className="primary"
                      disabled={busy}
                      onClick={() => void save()}
                    >
                      {busy ? t.saving : t.save}
                    </button>
                    <button
                      disabled={busy}
                      onClick={() => setConfirmReset(true)}
                    >
                      {t.reset}
                    </button>
                  </div>
                  {confirmReset && (
                    <div
                      className="confirm"
                      role="group"
                      aria-label={t.resetAsk}
                    >
                      <p>{t.resetAsk}</p>
                      <button disabled={busy} onClick={() => void save(true)}>
                        {t.confirm}
                      </button>
                      <button
                        disabled={busy}
                        onClick={() => setConfirmReset(false)}
                      >
                        {t.cancel}
                      </button>
                    </div>
                  )}
                </section>
                <section className="scope">
                  <span>術</span>
                  <div>
                    <h2>{t.scope}</h2>
                    <p>{t.scopeHelp}</p>
                  </div>
                </section>
              </>
            )}
            {boot && tab === "diagnostics" && (
              <section className="card">
                <div className="section-heading">
                  <h2>{t.diagnostics}</h2>
                  <span>02</span>
                </div>
                <dl>
                  <dt>{t.tray}</dt>
                  <dd>{boot.trayAvailable ? t.ready : t.missing}</dd>
                  <dt>{t.config}</dt>
                  <dd>
                    <code>{boot.configPath}</code>
                  </dd>
                  <dt>{t.logPath}</dt>
                  <dd>
                    <code>{boot.logsPath}</code>
                  </dd>
                </dl>
                <p className="muted">{t.privacy}</p>
                <button
                  className="primary"
                  disabled={busy}
                  onClick={() => void action(openLogs)}
                >
                  {t.logs}
                </button>
              </section>
            )}
            {tab === "about" && (
              <section className="card about">
                <img src="/logo.svg" alt="術" />
                <h2>Jutsu</h2>
                <p>{t.aboutBody}</p>
                <strong>Developed by star0x7f</strong>
                <p>
                  {t.version} {boot?.version ?? "0.1.0"} · MIT
                </p>
              </section>
            )}
            <footer>
              <span>{t.appearance}</span>
              <div>
                <button disabled={busy} onClick={() => void action(openLogs)}>
                  {t.logs}
                </button>
                <button disabled={busy} onClick={() => void action(quit)}>
                  {t.quit}
                </button>
              </div>
            </footer>
          </>
        )}
      </main>
    </div>
  );
}
