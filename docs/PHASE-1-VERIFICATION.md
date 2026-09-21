# Phase 1 verification

Windows GitHub Actions run: https://github.com/Davaadalai7/ChatNinja/actions/runs/35621079412

Tested application source: `b73aee5b960c0fcc91fd9781cf3e436fd9175e47`.

## Passed

- Clean `npm ci`, strict TypeScript check and Vite production build.
- Eight config tests: fresh setup/restart, corruption quarantine, interrupted replacement recovery, v0 migration, newer-version preservation, invalid values, oversized input and rejecting invalid saves.
- Rustfmt and native Tauri Clippy with warnings denied.
- Windows x64 native executable compilation with bundled assets (`tauri/custom-protocol`).
- Settings window visible on first launch.
- Bundled React frontend invokes the capability-protected native bootstrap command.
- A second process exits, with the original settings window still visible.
- Default × close fully exits the process with status 0.
- Reopening after shutdown displays the window; closing again exits normally.
- File logging creates a log in the expected Windows application log directory.

Verification archive SHA-256 was checked:
`3dea64e757ce83d26e89bf5cb0c201e52863434d12b8851531b555d71160c47a`.
Its native Cargo.lock is committed so later builds use the dependency graph that passed.

Locally, the frontend build, eight config tests, config Clippy and Rustfmt passed.
Native Linux compilation was not used: this environment lacks GTK development libraries and cannot fetch uncached native dependencies. Windows CI is the native compilation evidence.

## Manual gates on actual Windows 10/11

- Tray icon layout, left-click restore, menu Settings/Quit and opt-in close-to-tray behavior.
- Change/save language and close behavior through the UI, then restart.
- Diagnostics → Open logs folder opens Explorer.
- Reboot and launch from the user's machine.
- High-DPI visual layout and keyboard/screen-reader accessibility.

The README contains exact steps. Hosted Windows CI does not replace user-device testing.

## Scope

Phase 1 only: no claim of working live chat, overlay, global shortcuts, OAuth, emotes, OBS output, installer signing or updater delivery. These remain later phases in the approved order.

## Previous freeze: concrete finding

The previous code created a WebView inside a synchronous Tauri command and global-shortcut handler. Tauri's `WebviewWindowBuilder` warns that this can deadlock Windows WebView2. That is consistent with a partially created window and frozen close/geometry behavior; it is not a direct trace of the user's machine.

Jutsu Phase 1 avoids that path: settings is declarative and created during startup. No new WebView is created from a menu, shortcut, close callback or synchronous IPC handler. Future dynamic overlay creation must be asynchronous or run on a separate thread.

Official API reference: https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html
