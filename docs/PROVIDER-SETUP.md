# Publisher configuration and actual release requirements

End users should only install ChatNinja and click Connect. **The publisher must perform the setup below once**, before building that installer. Do not ask ordinary users to create developer apps or paste access tokens.

## YouTube

- Create a Google Cloud project and enable YouTube Data API v3.
- Configure the OAuth consent screen and register a **Desktop app** OAuth client. Add test accounts while in testing; complete verification/publishing as required for broader use.
- Build variable `CHATNINJA_GOOGLE_CLIENT_ID` contains that desktop client ID.
- `CHATNINJA_GOOGLE_DESKTOP_SECRET` is the Google-installed-app client secret if required by the issued desktop credentials. Installed apps cannot keep this value confidential. Never substitute a confidential web-client secret.
- The implemented flow opens the system browser with PKCE S256 and a random loopback port/state. Only `youtube.readonly` is requested. Refresh tokens go to Windows Credential Manager.
- Own active broadcasts are discovered. With multiple active broadcasts, the client stops with `multiple_broadcasts_select_required`; a broadcast selector remains to be added.
- Chat uses official REST `liveChatMessages.list` with provider-supplied polling intervals. Move to/test `streamList` before broad distribution to manage quotas. No quota budget or verification approval is assumed.

## Twitch

- Register a **Public** Twitch developer app supporting device authorization.
- Set build variable `CHATNINJA_TWITCH_CLIENT_ID`. Do not compile a Twitch client secret into the app.
- Device login requests `user:read:chat`. Native code validates token client ID/scopes, stores tokens in Credential Manager, refreshes rotating tokens and validates again periodically.
- EventSub WebSocket subscribes to the signed-in user's own channel: messages, deletion, chat clear and user clear. Other streamers' channels are not yet supported.
- Reconnection URLs must stay on the official `wss://eventsub.wss.twitch.tv` host. Native emote fragments are normalized to Twitch CDN assets.

## Kick

Follow `services/kick-relay/README.md`. Register the app and choose an HTTPS host/domain. The confidential secret and provider refresh tokens stay on that server. Set `CHATNINJA_KICK_RELAY_ORIGIN` for the desktop build after the relay is actually deployed and tested. Official webhook receive support is used; no scraping or unofficial websocket is implemented.

## GitHub Windows build

The included workflow reads client IDs/relay origin from repository Variables and Google desktop secret from a repository Secret. Kick secret and encryption key belong only to the relay host. Core and frontend tests run before Windows installer packaging. No GitHub repository or CI execution has been created by this handoff.

No provider client IDs have been supplied or invented. Missing configuration disables Connect and explains setup is required. That is intentional and means the current source is **not yet a ready-to-use live release**.

## Remaining gates before an installer is handed to end users

- Windows CI passes, including the Tauri host, keyring backend, NSIS packaging and clean-machine install/uninstall.
- Real end-to-end authentication and live chat on all three platforms, restart/refresh/revocation tests.
- Kick production server configuration and security review; trust/deletion/privacy documentation.
- Windows 10/11 overlay, capture mode, hotkey and game compatibility matrix.
- Signing and appropriate distribution/release handling. Unsigned executables can trigger warnings.
- All planned native/7TV/BTTV/FFZ emote integration and performance tests; only Twitch native emote rendering is currently implemented end-to-end in adapter code.

Official references checked during development:
- https://developers.google.com/identity/protocols/oauth2/native-app
- https://developers.google.com/youtube/v3/live/docs/liveChatMessages/list
- https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/
- https://dev.twitch.tv/docs/eventsub/handling-websocket-events/
- https://docs.kick.com/getting-started/generating-tokens-oauth2-flow
- https://docs.kick.com/events/webhook-security
- https://docs.kick.com/events/subscribe-to-events
