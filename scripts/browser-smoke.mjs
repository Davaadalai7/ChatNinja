// Run against npm run dev. Requires Playwright and Chromium installed separately.
// CHATNINJA_PLAYWRIGHT may point to an existing Playwright module in CI/tooling.
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
const { chromium } = await import(
  process.env.CHATNINJA_PLAYWRIGHT || "playwright"
);
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 1000 } });
const errors = [];
page.on("pageerror", (error) => errors.push(error.message));
try {
  await page.goto("http://127.0.0.1:1420");
  await page.getByRole("heading", { name: "Тоглоомдоо зай гарга." }).waitFor();
  await page.getByRole("combobox", { name: "Хэл" }).selectOption("en");
  await page
    .getByRole("heading", { name: "Make room for your game." })
    .waitFor();
  assert(
    await page
      .getByRole("button", { name: "Open desktop overlay" })
      .isDisabled(),
  );
  await page.getByRole("switch", { name: "Show timestamps" }).check();
  await page.reload();
  await page
    .getByRole("heading", { name: "Make room for your game." })
    .waitFor();
  assert(
    await page.getByRole("switch", { name: "Show timestamps" }).isChecked(),
  );
  await page.getByRole("button", { name: "Channels", exact: false }).click();
  assert.equal(
    await page.getByText("Publisher setup required", { exact: true }).count(),
    3,
  );
  await page.getByRole("button", { name: "Preferences", exact: false }).click();
  await page.getByRole("switch", { name: "Sample chat" }).uncheck();
  await page.getByText("Waiting for messages").waitFor();
  await page.getByRole("switch", { name: "Sample chat" }).check();
  await page
    .getByRole("button", { name: "Overlay", exact: false })
    .first()
    .click();
  await mkdir("artifacts", { recursive: true });
  await page.screenshot({
    path: "artifacts/chatninja-desktop.png",
    fullPage: true,
  });
  await page.setViewportSize({ width: 760, height: 1000 });
  await page.screenshot({
    path: "artifacts/chatninja-narrow.png",
    fullPage: true,
  });
  assert.equal(
    await page.evaluate(
      () => document.documentElement.scrollWidth > innerWidth,
    ),
    false,
  );
  assert.deepEqual(errors, []);
  console.log(
    "PASS: bilingual UI, settings persistence, offline providers, demo toggle, desktop gating, responsive width; no browser errors",
  );
} finally {
  await browser.close();
}
