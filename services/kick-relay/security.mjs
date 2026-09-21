import {
  createCipheriv,
  createDecipheriv,
  createHash,
  randomBytes,
  verify,
} from "node:crypto";
export const randomToken = () => randomBytes(32).toString("base64url");
export const digest = (value) =>
  createHash("sha256").update(value).digest("hex");
export const pkceChallenge = (verifier) =>
  createHash("sha256").update(verifier).digest("base64url");
export function seal(value, key) {
  const iv = randomBytes(12),
    cipher = createCipheriv("aes-256-gcm", key, iv);
  const ciphertext = Buffer.concat([
    cipher.update(JSON.stringify(value)),
    cipher.final(),
  ]);
  return JSON.stringify({
    version: 1,
    iv: iv.toString("base64"),
    tag: cipher.getAuthTag().toString("base64"),
    data: ciphertext.toString("base64"),
  });
}
export function unseal(value, key) {
  const record = JSON.parse(value);
  if (record.version !== 1) throw new Error("unsupported_storage_version");
  const decipher = createDecipheriv(
    "aes-256-gcm",
    key,
    Buffer.from(record.iv, "base64"),
  );
  decipher.setAuthTag(Buffer.from(record.tag, "base64"));
  return JSON.parse(
    Buffer.concat([
      decipher.update(Buffer.from(record.data, "base64")),
      decipher.final(),
    ]).toString(),
  );
}
export function validWebhook(headers, body, publicKey, now = Date.now()) {
  const id = headers["kick-event-message-id"],
    stamp = headers["kick-event-message-timestamp"],
    signature = headers["kick-event-signature"];
  if (
    typeof id !== "string" ||
    typeof stamp !== "string" ||
    typeof signature !== "string" ||
    id.length > 128 ||
    !/^[A-Za-z0-9_-]+$/.test(id)
  )
    return false;
  const time = Date.parse(stamp);
  if (!Number.isFinite(time) || Math.abs(now - time) > 300_000) return false;
  try {
    return verify(
      "RSA-SHA256",
      Buffer.concat([Buffer.from(`${id}.${stamp}.`), body]),
      publicKey,
      Buffer.from(signature, "base64"),
    );
  } catch {
    return false;
  }
}
