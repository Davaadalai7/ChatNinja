import { createServer } from "node:http";
import { pathToFileURL } from "node:url";
import { Store } from "./store.mjs";
import {
  digest,
  pkceChallenge,
  randomToken,
  validWebhook,
} from "./security.mjs";

// Provider endpoints are fixed; neither desktop callers nor callbacks choose URLs.
const API = "https://api.kick.com/public/v1";
const TOKEN = "https://id.kick.com/oauth/token";
const wantedEvents = ["chat.message.sent", "moderation.banned"];
const minutes = (n) => n * 60_000;
function fail(code, status = 400) {
  return Object.assign(new Error(code), { status });
}
function send(res, status, value) {
  res.writeHead(status, {
    "content-type": "application/json",
    "cache-control": "no-store",
    "referrer-policy": "no-referrer",
    "x-content-type-options": "nosniff",
    "content-security-policy": "default-src 'none'",
  });
  res.end(JSON.stringify(value));
}
async function body(req) {
  const chunks = [];
  let size = 0;
  for await (const chunk of req) {
    size += chunk.length;
    if (size > 262144) throw fail("body_too_large", 413);
    chunks.push(chunk);
  }
  return Buffer.concat(chunks);
}
async function providerJson(fetcher, url, init) {
  const response = await fetcher(url, {
    ...init,
    redirect: "error",
    signal: AbortSignal.timeout(20000),
  });
  if (!response.ok) throw fail("kick_api_request_failed", 502);
  const text = await response.text();
  if (text.length > 2_000_000) throw fail("provider_response_too_large", 502);
  return JSON.parse(text);
}
export async function createRelay(
  config,
  { fetcher = fetch, store: suppliedStore, publicKey: suppliedKey } = {},
) {
  const origin = new URL(config.origin);
  if (
    origin.protocol !== "https:" ||
    origin.username ||
    origin.password ||
    origin.pathname !== "/" ||
    origin.search ||
    origin.hash
  )
    throw new Error("https_origin_required");
  if (!config.clientId || !config.clientSecret)
    throw new Error("kick_developer_app_required");
  const redirect = `${origin.origin}/oauth/callback`;
  const store = suppliedStore ?? new Store(config.storePath, config.key);
  await store.load();
  const publicKey =
    suppliedKey ??
    (await providerJson(fetcher, `${API}/public-key`)).data?.public_key;
  if (typeof publicKey !== "string" || !publicKey.includes("BEGIN PUBLIC KEY"))
    throw new Error("kick_public_key_unavailable");
  const pending = new Map(),
    queues = new Map(),
    seen = new Map(),
    limits = new Map(),
    busy = new Map();
  const epoch = randomToken();
  let seq = 0;
  function prune() {
    const now = Date.now();
    for (const [key, p] of pending) if (p.expires < now) pending.delete(key);
    for (const [key, expiry] of seen) if (expiry < now) seen.delete(key);
    for (const [key, limit] of limits)
      if (limit.until < now) limits.delete(key);
  }
  function rate(key, maximum, duration) {
    const record = limits.get(key) ?? {
      count: 0,
      until: Date.now() + duration,
    };
    if (++record.count > maximum) throw fail("rate_limited", 429);
    limits.set(key, record);
  }
  async function exclusive(id, operation) {
    const old = busy.get(id) ?? Promise.resolve();
    const next = old.catch(() => {}).then(operation);
    busy.set(id, next);
    try {
      return await next;
    } finally {
      if (busy.get(id) === next) busy.delete(id);
    }
  }
  async function refresh(session) {
    if (session.token.expiresAt > Date.now() + minutes(2)) return;
    const token = await providerJson(fetcher, TOKEN, {
      method: "POST",
      body: new URLSearchParams({
        grant_type: "refresh_token",
        client_id: config.clientId,
        client_secret: config.clientSecret,
        refresh_token: session.token.refreshToken,
      }),
    });
    if (!token.access_token || !Number(token.expires_in))
      throw fail("invalid_kick_token", 502);
    session.token = {
      accessToken: token.access_token,
      refreshToken: token.refresh_token || session.token.refreshToken,
      expiresAt: Date.now() + Number(token.expires_in) * 1000,
    };
    await store.persist();
  }
  const server = createServer(async (req, res) => {
    try {
      prune();
      if (limits.size > 10000 || pending.size > 100 || seen.size > 100000)
        throw fail("relay_busy", 503);
      const url = new URL(req.url, "http://localhost");
      if (req.method === "GET" && url.pathname === "/health")
        return send(res, 200, { status: "ok" });
      if (req.method === "POST" && url.pathname === "/oauth/start") {
        rate(`start:${req.socket.remoteAddress}`, 15, minutes(10));
        const pollToken = randomToken(),
          state = randomToken(),
          verifier = randomToken();
        pending.set(state, {
          pollHash: digest(pollToken),
          verifier,
          expires: Date.now() + minutes(5),
          phase: "pending",
        });
        const authorize = new URL("https://id.kick.com/oauth/authorize");
        authorize.search = new URLSearchParams({
          client_id: config.clientId,
          response_type: "code",
          redirect_uri: redirect,
          state,
          scope: "user:read events:subscribe",
          code_challenge: pkceChallenge(verifier),
          code_challenge_method: "S256",
        }).toString();
        return send(res, 200, {
          pollToken,
          authorizationUrl: authorize.toString(),
          expiresIn: 300,
        });
      }
      if (req.method === "GET" && url.pathname === "/oauth/callback") {
        const state = url.searchParams.get("state");
        const transaction = pending.get(state);
        if (
          !transaction ||
          transaction.phase !== "pending" ||
          url.searchParams.getAll("state").length !== 1
        )
          throw fail("invalid_or_expired_oauth_state");
        transaction.phase = "exchanging"; // Consume before first await to reject callback replay.
        try {
          const code = url.searchParams.get("code");
          if (
            url.searchParams.has("error") ||
            !code ||
            code.length > 4096 ||
            url.searchParams.getAll("code").length !== 1
          )
            throw fail("authorization_denied");
          const token = await providerJson(fetcher, TOKEN, {
            method: "POST",
            body: new URLSearchParams({
              grant_type: "authorization_code",
              client_id: config.clientId,
              client_secret: config.clientSecret,
              redirect_uri: redirect,
              code_verifier: transaction.verifier,
              code,
            }),
          });
          if (
            !token.access_token ||
            !token.refresh_token ||
            !Number(token.expires_in)
          )
            throw fail("invalid_kick_token", 502);
          const headers = { authorization: `Bearer ${token.access_token}` };
          const user = (
            await providerJson(fetcher, `${API}/users`, { headers })
          ).data?.[0];
          if (!Number.isSafeInteger(user?.user_id))
            throw fail("invalid_kick_user", 502);
          const existing = (
            await providerJson(fetcher, `${API}/events/subscriptions`, {
              headers,
            })
          ).data;
          const subscribed = new Set(
            Array.isArray(existing)
              ? existing
                  .filter(
                    (s) =>
                      String(s.broadcaster_user_id) === String(user.user_id),
                  )
                  .map((s) => s.event)
              : [],
          );
          const missing = wantedEvents.filter((name) => !subscribed.has(name));
          if (missing.length) {
            const result = await providerJson(
              fetcher,
              `${API}/events/subscriptions`,
              {
                method: "POST",
                headers: { ...headers, "content-type": "application/json" },
                body: JSON.stringify({
                  method: "webhook",
                  events: missing.map((name) => ({ name, version: 1 })),
                }),
              },
            );
            if (
              !Array.isArray(result.data) ||
              missing.some(
                (name) =>
                  !result.data.some(
                    (event) =>
                      event.name === name &&
                      !event.error &&
                      event.subscription_id,
                  ),
              )
            )
              throw fail("kick_subscription_failed", 502);
          }
          for (const [oldId, old] of Object.entries(store.sessions))
            if (old.expires <= Date.now()) delete store.sessions[oldId];
          if (Object.keys(store.sessions).length >= 1000)
            throw fail("relay_capacity_reached", 503);
          const sessionToken = randomToken(),
            id = digest(sessionToken);
          store.sessions[id] = {
            channelId: String(user.user_id),
            expires: Date.now() + 30 * 86400000,
            token: {
              accessToken: token.access_token,
              refreshToken: token.refresh_token,
              expiresAt: Date.now() + Number(token.expires_in) * 1000,
            },
          };
          try {
            await store.persist();
          } catch {
            delete store.sessions[id];
            throw fail("credential_store_write_failed", 503);
          }
          transaction.phase = "complete";
          transaction.sessionToken = sessionToken;
          return send(res, 200, { message: "Signed in. Return to ChatNinja." });
        } catch (error) {
          transaction.phase = "failed";
          transaction.error = "kick_authorization_failed";
          throw error;
        }
      }
      const bearer = /^Bearer ([A-Za-z0-9_-]{43})$/.exec(
        req.headers.authorization ?? "",
      )?.[1];
      if (req.method === "POST" && url.pathname === "/oauth/result") {
        if (!bearer) throw fail("unauthorized", 401);
        rate(`poll:${digest(bearer)}`, 180, minutes(5));
        const entry = [...pending.entries()].find(
          ([, p]) => p.pollHash === digest(bearer),
        );
        if (!entry) throw fail("authorization_expired", 401);
        const [state, p] = entry;
        if (p.phase === "failed") throw fail(p.error);
        if (p.phase !== "complete") return send(res, 202, { state: "pending" });
        // A short retry window tolerates a lost network response; pollToken is secret.
        p.expires = Math.min(p.expires, Date.now() + 30000);
        pending.set(state, p);
        return send(res, 200, {
          sessionToken: p.sessionToken,
          expiresIn: 30 * 86400,
        });
      }
      if (req.method === "POST" && url.pathname === "/webhooks/kick") {
        const raw = await body(req);
        if (!validWebhook(req.headers, raw, publicKey))
          throw fail("invalid_webhook_signature", 401);
        const id = req.headers["kick-event-message-id"];
        if (seen.has(id)) return send(res, 200, { duplicate: true });
        const kind = req.headers["kick-event-type"];
        if (
          req.headers["kick-event-version"] !== "1" ||
          !wantedEvents.includes(kind)
        )
          return send(res, 200, { ignored: true });
        const payload = JSON.parse(raw.toString());
        const channel = String(payload.broadcaster?.user_id ?? "");
        if (
          !Object.values(store.sessions).some(
            (s) => s.channelId === channel && s.expires > Date.now(),
          )
        )
          return send(res, 200, { ignored: true });
        seen.set(id, Date.now() + minutes(10));
        const queue = queues.get(channel) ?? [];
        queue.push({ seq: ++seq, kind, payload });
        if (queue.length > 200) queue.shift();
        queues.set(channel, queue);
        return send(res, 200, { accepted: true });
      }
      if (!bearer) throw fail("unauthorized", 401);
      const id = digest(bearer),
        session = store.sessions[id];
      if (!session || session.expires <= Date.now())
        throw fail("reauthorization_required", 401);
      if (req.method === "GET" && url.pathname === "/events") {
        rate(`events:${id}`, 300, minutes(1));
        await exclusive(id, () => {
          if (store.sessions[id] !== session) throw fail("unauthorized", 401);
          return refresh(session);
        });
        const after = Number(url.searchParams.get("after") ?? 0);
        if (!Number.isSafeInteger(after) || after < 0)
          throw fail("invalid_cursor");
        const cursor = url.searchParams.get("epoch") === epoch ? after : 0;
        return send(res, 200, {
          epoch,
          cursor: seq,
          events: (queues.get(session.channelId) ?? []).filter(
            (e) => e.seq > cursor,
          ),
        });
      }
      if (req.method === "POST" && url.pathname === "/logout") {
        return await exclusive(id, async () => {
          let remoteRevoked = true;
          try {
            const revoke = new URL("https://id.kick.com/oauth/revoke");
            revoke.search = new URLSearchParams({
              token: session.token.refreshToken,
              token_hint_type: "refresh_token",
            }).toString();
            const response = await fetcher(revoke, {
              method: "POST",
              redirect: "error",
              signal: AbortSignal.timeout(20000),
            });
            remoteRevoked = response.ok;
          } catch {
            remoteRevoked = false;
          }
          delete store.sessions[id];
          await store.persist();
          if (
            !Object.values(store.sessions).some(
              (s) => s.channelId === session.channelId,
            )
          )
            queues.delete(session.channelId);
          return send(res, 200, { remoteRevoked });
        });
      }
      throw fail("not_found", 404);
    } catch (error) {
      // Never echo provider response bodies, OAuth codes, token-bearing URLs or secrets.
      if (!res.headersSent)
        send(res, error.status ?? 500, {
          error: error.status ? error.message : "relay_internal_error",
        });
      else res.end();
    }
  });
  server.requestTimeout = 30000;
  server.headersTimeout = 10000;
  server.maxHeadersCount = 40;
  return server;
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  const key = Buffer.from(
    process.env.KICK_TOKEN_ENCRYPTION_KEY ?? "",
    "base64",
  );
  if (key.length !== 32)
    throw new Error(
      "KICK_TOKEN_ENCRYPTION_KEY must be a random 32-byte base64 value",
    );
  const server = await createRelay({
    origin: process.env.KICK_RELAY_ORIGIN,
    clientId: process.env.KICK_CLIENT_ID,
    clientSecret: process.env.KICK_CLIENT_SECRET,
    key,
    storePath: process.env.KICK_TOKEN_STORE ?? "./data/tokens.enc",
  });
  server.listen(Number(process.env.PORT ?? 8787), "127.0.0.1", () =>
    console.log(
      "Kick relay listening on loopback; configure your HTTPS reverse proxy.",
    ),
  );
}
