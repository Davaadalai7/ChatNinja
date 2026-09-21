# Jutsu 術

Developed by **star0x7f**. Windows 10/11 x64. Tauri 2, Rust, React, TypeScript and Tailwind CSS.

## Phase 1 scope

Working desktop foundation: settings window, original SVG/icon set, tray, single-instance focus, bounded file logging, versioned JSON settings, validation, migration and corruption recovery. There are no chat adapters, overlay, global shortcuts or updater in this phase. No inactive login or update buttons are presented. This is a development checkpoint, not the completed chatbox.

## Run on Windows

Install Node.js 22.12+ LTS, Rust stable with the `x86_64-pc-windows-msvc` toolchain, Visual Studio Build Tools (Desktop development with C++) and Microsoft Edge WebView2 Runtime. See [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```powershell
npm ci
npm run check
npm run test:config
npm run desktop
```

`npm run dev` only opens the web frontend. It clearly reports that desktop APIs require `npm run desktop`.

For a native binary using bundled frontend assets:

```powershell
npm run build
cargo build --manifest-path src-tauri/Cargo.toml --features tauri/custom-protocol
.\src-tauri\target\debug\jutsu.exe
```

`npm run desktop:build` builds an unsigned NSIS development installer. Release signing, the tag/release workflow and signed updates are Phase 11; no signing key or updater URL is embedded now.

## Verify this phase

1. Launch the desktop app: the settings window must appear immediately.
2. Change the language, Save, fully Quit, reopen: the setting must persist.
3. Open the executable twice: the second launch focuses the existing window and exits.
4. With close-to-tray **off** (default), close ×: `jutsu.exe` must disappear from Task Manager. Reopen and repeat.
5. Enable close-to-tray and Save. Close ×: the tray remains. Left-click its icon or choose Settings to restore. Choose **Quit Jutsu** from the tray: the process must disappear. Reopen.
6. Diagnostics → Open logs folder: `jutsu.log` must exist. Logs do not include config contents or credentials.
7. Quit, back up the config path shown in Diagnostics, replace its content with invalid JSON, reopen: defaults load with a visible warning; the original becomes `config.corrupt-*.json`.
8. Repeat on a real Windows 10 and Windows 11 computer. A hosted CI runner is not equivalent to these machines.

Config tests exercise first-run/restart, migration, malformed/oversized JSON, interrupted replacement, invalid values and newer-version preservation. `scripts/windows-phase1.ps1` is for disposable Windows CI runners only.

## Storage and permissions

- Config: Tauri `app_config_dir()/config.json` (typically `%APPDATA%/dev.star0x7f.jutsu/config.json` on Windows).
- Logs: Tauri `app_log_dir()` (typically `%LOCALAPPDATA%/dev.star0x7f.jutsu/logs`). Each file is bounded to 1 MB. Default log rotation replaces the full log at that limit.
- No tokens, passwords, OAuth secrets or localStorage config. Platform tokens will use Windows Credential Manager when auth is implemented.
- The backend is the sole settings writer; a mutex serializes saves. A synced staging file and previous backup recover an interrupted replacement.
- Newer schema versions are preserved and display an error instead of destructive downgrade.
- Only the local settings window is granted the five app commands. Tray and single-instance run in Rust and expose no frontend plugin APIs. Logging is backend-only.

## Architecture

```text
public/logo.svg                  Original vector mark
src/features/settings/           Settings UI
src/shared/                      Typed IPC and language strings
src-tauri/src/commands.rs         Narrow desktop IPC boundary
src-tauri/src/lifecycle.rs        Tray, focus and shutdown
src-tauri/src/state.rs            Native application state
src-tauri/src/main.rs             Plugin order and startup
src-tauri/capabilities/           Window permission scope
src-tauri/icons/                  Generated Windows icons
crates/jutsu-config/              Platform-independent config and tests
scripts/windows-phase1.ps1       Native lifecycle smoke test
.github/workflows/phase1.yml      Windows build/test verification
```

Each future feature belongs in its own frontend feature and backend module. Provider authentication and adapters will be shared by features. No licensing or payments are implemented.

## Designing out the previous failures

| Symptom | Prevention |
| --- | --- |
| Window created but UI hangs, hotkeys/close ignored | Tauri documents a Windows WebView2 deadlock when dynamically creating windows in synchronous commands/event handlers. This phase creates its window declaratively at startup. Phase 2 must use an async command or separate thread for dynamic creation. |
| Closing and reopening creates hidden competing processes | Register single-instance first; show, unminimize and focus the existing settings window. Explicit Quit calls the desktop exit path. Default × fully quits. |
| Hidden tray process surprises user | Close-to-tray is opt-in, saved only if tray creation succeeded. Tray failure leaves a usable visible settings window and normal Quit. |
| Corrupt config prevents startup | Strict typed validation, bounded input, quarantine and defaults. Filesystem permission errors and future schema versions are displayed without deleting the file. |
| Missing Tauri permissions | Generate an app command manifest and grant only those commands to the settings window. Add plugin permissions in the phase that uses their frontend API. |

Monitor geometry, overlay state and shortcut conflicts are implemented and verified in Phases 2–3, not claimed as finished here. No game injection is used. Exclusive fullscreen and anti-cheat compatibility will be documented with the overlay phase.

## Official references

- [Single-instance](https://v2.tauri.app/plugin/single-instance/)
- [Tray](https://v2.tauri.app/learn/system-tray/)
- [File logging](https://v2.tauri.app/plugin/logging/)
- [Window builder and Windows deadlock warning](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html)

Phase 5–9 will document the actual platform app registrations and official auth requirements. This phase needs no API key. `.env.example` intentionally contains no invented endpoints or secret fields.
