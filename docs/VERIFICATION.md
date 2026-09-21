# Verification record — 0.1.0 development checkpoint

## Passed locally

- `npm run check`: 24 frontend unit tests, TypeScript check and Vite production build.
- `npm run test:relay`: 4 Node tests covering encryption/tampering, signed webhook verification/timestamp checks, encrypted persistence and a mock OAuth → webhook → private delivery → logout flow with channel isolation.
- `cargo test --locked --manifest-path crates/chatninja-core/Cargo.toml`: 19 native-core tests, plus the empty doctest suite.
- `cargo clippy --locked --manifest-path crates/chatninja-core/Cargo.toml --all-targets -- -D warnings`: passed.
- Rustfmt and Prettier applied. npm/Cargo dependency lockfiles generated.
- Earlier production-dependency npm audit reported no known vulnerabilities; this is not a security certification.
- Tauri icons generated from the included editable SVG.

Tests use fixtures/mocked upstream responses. They do not prove successful authentication or receipt of real YouTube/Kick/Twitch traffic.

## Attempted but blocked

- Tauri host `cargo check` on Linux stopped at missing pkg-config/GTK system dependencies before checking all app code.
- Local browser smoke test could not launch because Chromium installation hit a directory-lock error. The script has not passed.
- The GitHub connector can access the account, but exposes no new-repository operation. Browser repository creation requires sign-in; no successful repository creation, push or Actions run is recorded.

## Not verified

- Windows-specific Credential Manager backend, native Tauri host compilation, NSIS installer, install/launch/uninstall on Windows 10/11.
- Real OAuth consent, refresh, token revocation, live messages, quota behavior or a deployed Kick relay.
- Native overlay/hotkeys/click-through, capture protection, OBS recording and target-game/anti-cheat compatibility.
- All-emote support: Twitch native fragments are implemented; other native and third-party catalogues remain incomplete.
- Full visual/browser interaction tests and release signing.

Vite/Rollup emitted non-failing PURE annotation warnings from Zod. Build completion is verified; these warnings did not fail the build.

## Release interpretation

This deliverable is source code with verified components, not a finished installable live-chat product. A successful Windows workflow, publisher configuration, actual live-account tests and the [release checklist](RELEASE-CHECKLIST.md) are still required.
