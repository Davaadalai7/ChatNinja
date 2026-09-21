# ChatNinja

A minimal Windows 10/11 chat overlay for single-monitor streamers. YouTube and Kick are the primary platforms; Twitch is also supported by adapter code. React + TypeScript + Tailwind CSS + Tauri 2.

**0.1.0 development build. Windows CI and installer install/startup/shutdown/uninstall checks pass. Authentication and live adapters are implemented, but publisher OAuth configuration, Kick deployment and real-account end-to-end tests remain required. A source ZIP is not an installer.**

## What is implemented

- Mongolian/English dashboard, charcoal/white theme, labelled demo mode.
- Transparent always-on-top desktop overlay, click-through, drag/resize, placement, font/opacity controls.
- Local OBS Browser Source with streamer-only, OBS-only and combined modes.
- YouTube system-browser OAuth with PKCE; Twitch device authorization; Kick authorization through a separately hosted relay.
- Windows Credential Manager token storage, refresh, stop/sign-out controls and connection status.
- YouTube official live-chat polling, Twitch EventSub WebSocket, Kick verified webhook relay.
- Unified typed chat messages, bounded history, deduplication, platform/bot filters and supported moderation removal events.
- Twitch native emote fragments. Other native emotes and 7TV/BTTV/FFZ catalogues remain incomplete.
- Automated frontend, native-core and relay tests; Windows NSIS build workflow.

## Run the interface

Install Node.js 22.12+ and run:

```sh
npm ci
npm run dev
```

Open http://127.0.0.1:1420. Browser preview cannot open native overlay windows, use Credential Manager or start the OBS server. Its desktop controls are disabled.

## Run and build on Windows

Install Rust stable, Microsoft C++ Build Tools with the Desktop development with C++ workload/Windows SDK, and Microsoft Edge WebView2. Configure publisher-owned applications as explained in [PROVIDER-SETUP.md](docs/PROVIDER-SETUP.md).

```sh
npm ci
npm run desktop
```

End users should receive a configured installer and click Connect. They should not have to register developer apps. Missing publisher configuration disables the corresponding Connect button; no client IDs or secrets are invented.

```sh
npm run check
npm run test:relay
cargo test --locked --manifest-path crates/chatninja-core/Cargo.toml
cargo clippy --locked --manifest-path crates/chatninja-core/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run desktop:build
```

NSIS output: `src-tauri/target/release/bundle/nsis/`. The included GitHub Actions Windows job builds an **unsigned development installer**. A workflow file is not evidence of a successful build. Both Cargo lockfiles and the npm lockfile are included.

## Overlay and OBS

Open the overlay from the dashboard. Alt+Shift+O toggles visibility; Alt+Shift+L toggles click-through. Open the overlay once before using those shortcuts. If another app occupies a shortcut, ChatNinja shows a warning and the dashboard controls remain available. Drag its header or bottom-right grip when unlocked. Numeric geometry persists; dragged geometry is session-only. Closing the dashboard exits the app, overlay and chat workers; there is no tray lifecycle yet.

Choose OBS only or Desktop + OBS and copy the local URL into OBS → Sources → Browser. The server uses loopback, a random port and a random session token. The URL changes each app launch. Streamer-only mode clears OBS messages.

Capture protection is best-effort: test an actual recording. Exclusive fullscreen can cover a normal overlay; use windowed/borderless mode. No game injection, graphics hooks, memory reads or anti-cheat bypasses are used. Compatibility with each game/anti-cheat still needs Windows testing.

## Editing and release

- [Architecture and file map](docs/ARCHITECTURE.md)
- [Publisher OAuth and relay setup](docs/PROVIDER-SETUP.md)
- [Kick relay deployment](services/kick-relay/README.md)
- [Checks actually executed](docs/VERIFICATION.md)
- [Монгол суулгах заавар](docs/INSTALL-MN.md)
- [Release checklist](docs/RELEASE-CHECKLIST.md)

Remaining product work includes full emote catalogues, multiple-broadcast selection, stable OBS URLs, tray behavior, configurable shortcuts, performance/game validation and signed releases. No license is asserted; choose one before public distribution.

## GitHub

Source: https://github.com/Davaadalai7/ChatNinja. Keep credentials, relay data, generated installers, dependency folders and build output out of source control. See Actions for exact build results and the verification record for remaining release gates.
