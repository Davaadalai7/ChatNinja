# Architecture and editing guide

## File map

| Location | Responsibility |
| --- | --- |
| `src/domain/` | Validated settings, frontend chat types, display filters |
| `src/features/channels/` | Native connection controls and status |
| `src/features/chat/` | Safe React renderer and explicit demo data |
| `src/features/overlay/` | Transparent native overlay view |
| `src/platform/desktop.ts` | React/native invoke boundary |
| `src/i18n.ts` | Mongolian/English dashboard copy |
| `src/App.tsx`, `src/styles.css` | Dashboard and visual tokens |
| `crates/chatninja-core/src/auth.rs` | OAuth, bounded HTTP responses, refresh/revoke |
| `crates/chatninja-core/src/vault.rs` | Windows Credential Manager; no plaintext fallback |
| `crates/chatninja-core/src/model.rs` | Typed messages, removal, deduplication and bounded buffers |
| `crates/chatninja-core/src/providers/` | YouTube, Twitch and Kick transports/normalization |
| `src-tauri/src/connections.rs` | Worker lifecycle, native message ownership, UI status events |
| `src-tauri/src/main.rs` | Windows, settings validation, shortcuts and loopback OBS server |
| `src-tauri/src/obs.html` | OBS renderer without native IPC |
| `services/kick-relay/` | Server-side Kick OAuth, signature verification and private delivery |
| `.github/workflows/ci.yml` | Frontend/core/relay checks and Windows packaging |

## Data flow

The dashboard stores only presentation settings in localStorage. Native workers own account credentials and live messages. OAuth runs in the system browser (YouTube/Kick) or a device flow (Twitch). Windows Credential Manager stores provider tokens or the opaque Kick relay session credential.

Provider adapters emit typed message/removal/status events. A bounded native buffer deduplicates by platform, channel and message ID. Deleted IDs remain in the bounded seen set so replay does not immediately resurrect messages. The dashboard receives nonsecret snapshots/status events. Renderer demo snapshots cannot overwrite native live messages.

The overlay polls native snapshots every 250 ms; OBS polls a loopback route every 500 ms. OBS receives no provider credentials. Streamer-only mode strips its messages. These polling transports and the duplicated renderer are intentional development limitations; measure load before broad distribution.

## Providers

YouTube discovers the signed-in user's active broadcast and polls with the API's next-page token and interval. No active broadcast produces a waiting state. Multiple simultaneous broadcasts require a future selector.

Twitch validates tokens, subscribes to official EventSub WebSocket chat/moderation events, watches keepalive deadlines and handles reconnect handover on the official host. Native Twitch emotes become image fragments.

Kick uses a separate HTTPS service because official chat events arrive as webhooks. Its confidential app secret stays on that server. State and PKCE protect authorization; signature verification uses the raw webhook body. Encrypted server storage and per-session channel authorization precede delivery to the desktop. See the relay README for operational limits.

## Security boundaries

- No game injection, graphics hooks, memory access or anti-cheat bypasses.
- No plaintext credential fallback, frontend account tokens, secret logs or telemetry.
- Chat history is bounded and memory-only. Relay account tokens persist encrypted for refresh.
- React text nodes and OBS textContent render untrusted chat. Never interpolate chat into HTML.
- Emote image hosts are exact HTTPS allowlists; keep React, OBS and CSP aligned.
- Connection mutations require the main dashboard window.
- Stopping or signing out aborts and awaits the adapter before credential deletion, preventing a late refresh from restoring removed credentials.
- OBS is read-only, loopback-bound, session-token protected and has no permissive CORS.
- Real provider callbacks, refresh, revocation, relay operations and clean Windows installs still require end-to-end validation.

## Known product limits

Full emote-provider catalogues, stable OBS identity, tray lifecycle and configurable hotkeys are unfinished. Overlay move/resize events now update saved geometry, and missing overlays can be opened by the show shortcut or a recovery button. Occupied global shortcuts produce a dashboard warning. Closing the dashboard terminates the app and its workers. Local Linux Tauri host compilation stopped at missing GTK/pkg-config prerequisites; use the Windows workflow and verification record for native build status. No signing key or code-signing certificate is bundled.

Official references and publisher setup are recorded in [PROVIDER-SETUP.md](PROVIDER-SETUP.md).
