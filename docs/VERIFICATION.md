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

## GitHub verification

Repository: https://github.com/Davaadalai7/ChatNinja

- Frontend checks and the Kick relay tests have passed on GitHub's Ubuntu runner.
- Native-core tests and Clippy have passed on Ubuntu.
- Native-core tests, including the Windows Credential Manager roundtrip/logout test,
  have passed on Windows (20 tests in that configuration).
- Tauri host compilation, its 3 unit tests and Windows Clippy have passed in the
  initial workflow. Later builds include graceful shortcut-conflict handling,
  explicit app shutdown and an installer smoke test.
- Final workflow **35559945720**, commit **61f750c494729e033407b1f3eb7a3b8133bf3672**,
  completed successfully: https://github.com/Davaadalai7/ChatNinja/actions/runs/35559945720
- Its Windows job passed 20 core tests, 3 Tauri host tests, Clippy, release compilation
  and NSIS packaging. The Ubuntu job passed 24 frontend tests, 4 relay tests,
  TypeScript/Vite and the Linux core checks.
- Installer smoke test passed: silent install, native process alive after 10 seconds,
  normal dashboard close/shutdown and silent uninstall.
- Runner image was **windows-2025-vs2026**. This is process/startup verification,
  not visual UI validation or a Windows 10/11 game-compatibility result.
- Artifact: ChatNinja_0.1.0_x64-setup.exe (unsigned development installer).
  The artifact archive and extracted installer were checked against their SHA-256 values.

## Attempted but blocked

- Tauri host `cargo check` on Linux stopped at missing pkg-config/GTK system dependencies before checking all app code.
- Local browser smoke test could not launch because Chromium installation hit a directory-lock error. The script has not passed.
- Cloud-browser inspection of the local preview was rejected because file URLs are
  disallowed. No workaround was used. Full visual/UI interaction testing remains
  unverified.

## Not verified

- Clean-machine install/launch/uninstall on actual Windows 10/11. A Windows hosted
  CI runner is a separate environment and does not replace those tests.
- Real OAuth consent, refresh, token revocation, live messages, quota behavior or a deployed Kick relay.
- Native overlay/hotkeys/click-through, capture protection, OBS recording and target-game/anti-cheat compatibility.
- All-emote support: Twitch native fragments are implemented; other native and third-party catalogues remain incomplete.
- Full visual/browser interaction tests and release signing.

Vite/Rollup emitted non-failing PURE annotation warnings from Zod. Build completion is verified; these warnings did not fail the build.

## Release interpretation

This deliverable includes source code and an unsigned installer with passing CI and
installation smoke checks. It is not yet a ready-to-use live-chat release: publisher
OAuth configuration, a deployed Kick relay, actual live-account testing and the
[release checklist](RELEASE-CHECKLIST.md) remain required. Missing publisher
configuration leaves Connect disabled; demo/overlay configuration remains usable.
