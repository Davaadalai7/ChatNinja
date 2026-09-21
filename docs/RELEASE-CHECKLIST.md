# Release gate

Do not mark an item complete without evidence.

- [x] Windows CI compiles native code, passes Rust tests and Clippy, produces NSIS installer.
- [x] Silent install, startup, normal shutdown and uninstall smoke test on Windows CI runner.
- [x] Include both Cargo.lock files and pin native dependency resolution.
- [ ] Install/launch/uninstall on clean Windows 10 and Windows 11 machines, including non-admin account and non-ASCII username.
- [ ] Test WebView2 absent/offline, installer upgrades, rollback and permissions.
- [ ] Review app identifier ownership, application icon, license and dependency licenses.
- [ ] Test overlay on Dota 2, CS2, Steam PUBG and Valheim in windowed/borderless; record game version, anti-cheat version and capture settings. No claim of universal anti-cheat safety.
- [ ] Test 100%, 125%, 150%, 200% DPI; monitor disconnect, negative coordinates, offscreen recovery, Alt+Tab and focus behavior.
- [ ] Test click-through recovery, occupied shortcuts, drag/resize, hide/show and OBS-only restrictions.
- [ ] Test desktop/OBS/both modes using actual recordings with Game, Window and Display Capture. Never promise guaranteed capture exclusion.
- [ ] Test both languages, keyboard-only use, long Mongolian names, Unicode, empty chat and 200-message bursts.
- [x] Implement YouTube, Kick and Twitch OAuth, secure storage and live adapter code.
- [ ] Auth-test all three with developer-owned applications and actual accounts.
- [ ] Test emote providers, native fragments, animated emotes, fallback text, deletion, rate limits and revoked permissions.
- [ ] Audit token storage, logout, refresh, scopes, logs and privacy documentation.
- [ ] Implement stable OBS source identity and robust server/adapter health state.
- [ ] Measure FPS, memory, CPU and network impact during actual games.
- [ ] Sign Windows distribution; address SmartScreen reputation honestly. Unsigned builds are development-only.
- [ ] If auto-updates are implemented, require signature verification, secure key storage and rollback strategy.

## Initial verification record

See `docs/VERIFICATION.md` for checks actually executed. Workflow files are not evidence that CI ran.
