# Kick relay (development implementation)

This is a **separate server**, not part of the Windows installer. It implements Kick OAuth with PKCE, encrypted token persistence, refresh, webhook signature/replay checks, channel-isolated polling and logout. Mock integration tests pass; real Kick authorization/delivery and deployment have not been tested.

## Publisher setup

Use a single Node 22.12+ process on a persistent server behind an HTTPS reverse proxy. Do not deploy multiple replicas against the file store. Configure these server-only environment variables using your hosting provider's secret manager:

| Variable | Meaning |
| --- | --- |
| `KICK_RELAY_ORIGIN` | Public HTTPS origin, e.g. `https://chat.example.com` |
| `KICK_CLIENT_ID` | Registered Kick developer app ID |
| `KICK_CLIENT_SECRET` | Confidential Kick secret; never in desktop/GitHub source |
| `KICK_TOKEN_ENCRYPTION_KEY` | Random 32 bytes, base64 encoded; keep a secure backup |
| `KICK_TOKEN_STORE` | Absolute path on a persistent volume, e.g. `/data/chatninja/tokens.enc` |
| `PORT` | Internal loopback port; default 8787 |

Generate the encryption key in the hosting environment, not in chat:

```sh
node -e "process.stdout.write(require('node:crypto').randomBytes(32).toString('base64'))"
node services/kick-relay/server.mjs
```

Register exact URLs in the Kick developer app:

- OAuth callback: `<KICK_RELAY_ORIGIN>/oauth/callback`
- Enabled webhook URL: `<KICK_RELAY_ORIGIN>/webhooks/kick`
- Requested scopes: `user:read events:subscribe`

The desktop build uses only `CHATNINJA_KICK_RELAY_ORIGIN`. The relay returns an opaque session credential stored in Windows Credential Manager; Kick access/refresh tokens stay encrypted on the server. This differs intentionally from YouTube/Twitch direct token storage.

## Deployment gates

1. Choose a host/domain and authorize deployment. This repository does not select or deploy one automatically.
2. Configure HTTPS, request-size/rate limits, access logging with OAuth query strings and Authorization headers redacted, and a durable encrypted volume. The application does not log tokens.
3. Run `npm run test:relay`; test real account consent, cancellation, expiry, refresh, webhook delivery, provider disconnection and logout.
4. Exercise provider rate limits and load; tune the per-IP limiter for the trusted reverse proxy. It deliberately ignores spoofable forwarded headers and may aggregate users behind one proxy.
5. Back up the encryption key and token store securely. A lost key invalidates saved credentials. Encrypted atomic file writes are a development store, not a transactional multi-replica database.
6. Monitor Kick key rotation and subscription revocation. The current key is fetched at server startup; restart after confirmed key rotation. Subscription health/reconciliation must be verified before public release.

## Limits

- Bounded 200-event per-channel memory queue; restart/long disconnect can lose chat history. Epoch changes reset client cursors. Not a durable message archive.
- Session lifetime 30 days; sign in again after expiry. Pending login lasts 5 minutes.
- Rejects signed webhook timestamps older/newer than five minutes, which can drop delayed deliveries; validate retry behavior with Kick before release.
- One single-process instance, at most 1,000 stored sessions. Not a multi-tenant production service certification.
- No access-token or OAuth code is returned by callback pages. Startup errors do not include provider response bodies.
- A stopped desktop retains its local credential; Sign out deletes it and attempts relay/provider revocation. Network failure requires revoking access from provider account settings as well.
- Kick subscription cleanup/health and account-wide revocation semantics still need production hardening. The relay drops unowned channels rather than exposing them.
- All emote catalogues, billing, account management, support, production privacy terms and code-signing are outside this development checkpoint.
