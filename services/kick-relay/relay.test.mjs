import { test } from "node:test";
import assert from "node:assert/strict";
import { generateKeyPairSync, randomBytes, sign } from "node:crypto";
import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { once } from "node:events";
import { seal, unseal, validWebhook } from "./security.mjs";
import { Store } from "./store.mjs";
import { createRelay } from "./server.mjs";

const keys = generateKeyPairSync("rsa", { modulusLength: 2048 });
const publicKey = keys.publicKey.export({ type: "spki", format: "pem" });
function webhookHeaders(raw, id = "test-event-1") {
  const stamp = new Date().toISOString();
  return {
    "kick-event-message-id": id,
    "kick-event-message-timestamp": stamp,
    "kick-event-type": "chat.message.sent",
    "kick-event-version": "1",
    "kick-event-signature": sign(
      "RSA-SHA256",
      Buffer.concat([Buffer.from(`${id}.${stamp}.`), raw]),
      keys.privateKey,
    ).toString("base64"),
  };
}
test("token encryption authenticates ciphertext; tampered or wrong-key records fail", () => {
  const key = randomBytes(32),
    value = { refresh: "sensitive-refresh" },
    encrypted = seal(value, key);
  assert(!encrypted.includes("sensitive-refresh"));
  assert.deepEqual(unseal(encrypted, key), value);
  assert.throws(() => unseal(encrypted, randomBytes(32)));
  const tampered = JSON.parse(encrypted);
  tampered.tag = randomBytes(16).toString("base64");
  assert.throws(() => unseal(JSON.stringify(tampered), key));
});
test("webhook signature covers exact raw bytes, id and timestamp; rejects old deliveries", () => {
  const raw = Buffer.from('{"x":1}'),
    headers = webhookHeaders(raw);
  assert(validWebhook(headers, raw, publicKey));
  assert(!validWebhook(headers, Buffer.from('{"x":2}'), publicKey));
  assert(
    !validWebhook(
      { ...headers, "kick-event-message-id": "changed" },
      raw,
      publicKey,
    ),
  );
  assert(!validWebhook(headers, raw, publicKey, Date.now() + 600000));
});
test("encrypted store persists and reloads without plaintext credentials", async () => {
  const dir = await mkdtemp(join(tmpdir(), "chatninja-vault-")),
    key = randomBytes(32),
    path = join(dir, "tokens.enc");
  const store = new Store(path, key);
  await store.load();
  store.sessions.x = { token: "secret-value" };
  await store.persist();
  assert(!(await readFile(path, "utf8")).includes("secret-value"));
  const reloaded = new Store(path, key);
  await reloaded.load();
  assert.equal(reloaded.sessions.x.token, "secret-value");
});
test("relay OAuth → token store → verified webhook → private chat → logout", async () => {
  const calls = [];
  const fetcher = async (url, init = {}) => {
    const path = new URL(url).pathname;
    calls.push(path);
    if (path === "/oauth/token")
      return Response.json({
        access_token: "upstream-access",
        refresh_token: "upstream-refresh",
        expires_in: 3600,
      });
    if (path === "/public/v1/users")
      return Response.json({ data: [{ user_id: 10 }] });
    if (path === "/public/v1/events/subscriptions")
      return Response.json({
        data:
          init.method === "POST"
            ? [
                { name: "chat.message.sent", subscription_id: "a" },
                { name: "moderation.banned", subscription_id: "b" },
              ]
            : [],
      });
    if (path === "/oauth/revoke") return new Response("", { status: 200 });
    throw new Error("Unexpected provider route");
  };
  const store = { sessions: {}, async load() {}, async persist() {} };
  const server = await createRelay(
    {
      origin: "https://relay.example.test",
      clientId: "test-app",
      clientSecret: "test-only",
    },
    { fetcher, store, publicKey },
  );
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const base = `http://127.0.0.1:${server.address().port}`;
  try {
    const start = await (
      await fetch(`${base}/oauth/start`, { method: "POST" })
    ).json();
    const auth = new URL(start.authorizationUrl);
    assert.equal(auth.hostname, "id.kick.com");
    assert.equal(auth.searchParams.get("code_challenge_method"), "S256");
    assert(!start.authorizationUrl.includes("test-only"));
    assert.equal(
      (await fetch(`${base}/oauth/callback?code=code&state=wrong`)).status,
      400,
    );
    assert.equal(calls.length, 0);
    const state = auth.searchParams.get("state");
    assert.equal(
      (await fetch(`${base}/oauth/callback?code=code&state=${state}`)).status,
      200,
    );
    assert.equal(
      (await fetch(`${base}/oauth/callback?code=code&state=${state}`)).status,
      400,
    );
    const result = await (
      await fetch(`${base}/oauth/result`, {
        method: "POST",
        headers: { authorization: `Bearer ${start.pollToken}` },
      })
    ).json();
    assert.equal(
      Object.keys(result).sort().join(","),
      "expiresIn,sessionToken",
    );
    assert(!JSON.stringify(result).includes("upstream-"));
    const authorization = `Bearer ${result.sessionToken}`;
    const raw = Buffer.from(
      JSON.stringify({
        broadcaster: { user_id: 10 },
        sender: { user_id: 20, username: "Bold" },
        content: "Hello",
        message_id: "m1",
      }),
    );
    const headers = webhookHeaders(raw);
    assert.equal(
      (
        await fetch(`${base}/webhooks/kick`, {
          method: "POST",
          headers,
          body: raw,
        })
      ).status,
      200,
    );
    assert.equal(
      (
        await (
          await fetch(`${base}/webhooks/kick`, {
            method: "POST",
            headers,
            body: raw,
          })
        ).json()
      ).duplicate,
      true,
    );
    assert.equal((await fetch(`${base}/events`)).status, 401);
    const events = await (
      await fetch(`${base}/events?after=0`, { headers: { authorization } })
    ).json();
    assert.equal(events.events.length, 1);
    assert.equal(events.events[0].payload.content, "Hello");
    const next = await (
      await fetch(
        `${base}/events?after=${events.cursor}&epoch=${events.epoch}`,
        { headers: { authorization } },
      )
    ).json();
    assert.equal(next.events.length, 0);
    const otherRaw = Buffer.from(
      JSON.stringify({
        broadcaster: { user_id: 99 },
        content: "private-other-channel",
      }),
    );
    await fetch(`${base}/webhooks/kick`, {
      method: "POST",
      headers: webhookHeaders(otherRaw, "test-event-2"),
      body: otherRaw,
    });
    const isolated = await (
      await fetch(`${base}/events?after=0`, { headers: { authorization } })
    ).json();
    assert.equal(isolated.events.length, 1);
    assert.equal(
      (
        await fetch(`${base}/logout`, {
          method: "POST",
          headers: { authorization },
        })
      ).status,
      200,
    );
    assert.equal(
      (await fetch(`${base}/events`, { headers: { authorization } })).status,
      401,
    );
    assert.equal(Object.keys(store.sessions).length, 0);
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
});
