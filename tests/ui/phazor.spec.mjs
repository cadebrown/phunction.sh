// The UI-functionality contract for /phazor, as executable tests: boot,
// transport, the step editor's three gestures, topbar mind lighting, zen,
// and the reload-resume invariant. These run on the release dist in CI and
// locally via `just ui-test` — a UI claim without a green test here is a lie.
import { test, expect } from "@playwright/test";

/** Boot the workstation: navigate, power on, wait for the topbar. */
async function powerOn(page) {
  await page.goto("/phazor");
  await page.getByRole("button", { name: /power on/ }).click();
  await expect(page.locator(".phz-topbar")).toBeVisible({ timeout: 20_000 });
}

/** Unfold the sequence panel if it booted folded. Folded bodies are
 * rendered but hidden, so the guard must be VISIBILITY, not count. */
async function openSequencer(page) {
  const first = page.locator(".step").first();
  if (!(await first.isVisible().catch(() => false))) {
    await page.locator(".rack-latch", { hasText: "sequence" }).click();
  }
  await expect(first).toBeVisible();
}

test("boots and the clock advances", async ({ page }) => {
  await powerOn(page);
  const lcd = page.locator(".phz-lcd").first();
  await expect(lcd).toContainText("beat");
  const beat = async () =>
    parseFloat((await lcd.innerText()).match(/beat\s+([\d.]+)/)[1]);
  const before = await beat();
  await expect(async () => {
    expect(await beat()).toBeGreaterThan(before + 0.5);
  }).toPass({ timeout: 15_000 });
});

test("step editor: tap toggles, drag walks pitch, shift-drag writes velocity", async ({
  page,
}) => {
  await powerOn(page);
  await openSequencer(page);
  const step = page.locator(".step").nth(2);
  const note = step.locator(".note");

  // tap → on, with a note name
  await step.click();
  await expect(step).toHaveClass(/on/);
  const name0 = await note.innerText();
  expect(name0).toMatch(/^[a-g]#?\d$/);

  // vertical drag → the pitch walks scale degrees
  const box = await step.boundingBox();
  const cx = box.x + box.width / 2;
  const cy = box.y + box.height / 2;
  await page.mouse.move(cx, cy);
  await page.mouse.down();
  await page.mouse.move(cx, cy - 22, { steps: 4 });
  await page.mouse.up();
  await expect(note).not.toHaveText(name0);

  // shift-drag → velocity changes (the --vel custom property drives the bar)
  const vel = async () =>
    parseFloat(await step.evaluate((el) => el.style.getPropertyValue("--vel")));
  const v0 = await vel();
  await page.keyboard.down("Shift");
  await page.mouse.move(cx, cy);
  await page.mouse.down();
  await page.mouse.move(cx, cy + 25, { steps: 4 });
  await page.mouse.up();
  await page.keyboard.up("Shift");
  expect(await vel()).toBeLessThan(v0);

  // tap again → off
  await step.click();
  await expect(step).not.toHaveClass(/on/);
});

test("edited pattern survives a reload (state v3)", async ({ page }) => {
  await powerOn(page);
  await openSequencer(page);
  const step = page.locator(".step").nth(5);
  await step.click();
  await expect(step).toHaveClass(/on/);
  const name = await step.locator(".note").innerText();
  // the state effect writes on change; confirm it landed before reloading
  await expect(async () => {
    const part = await page.evaluate(
      () => localStorage.getItem("phazor:state")?.split(";")[17],
    );
    expect(part.split(",")[5]).toMatch(/^\d+:\d+$/);
  }).toPass({ timeout: 5_000 });

  await page.reload();
  await powerOn(page);
  await openSequencer(page);
  await expect(page.locator(".step").nth(5)).toHaveClass(/on/);
  await expect(page.locator(".step").nth(5).locator(".note")).toHaveText(name);
});

test("topbar lights the active mind and switches live", async ({ page }) => {
  await powerOn(page);
  const lit = page.locator(".phz-topbar .ctrl-btn.lit", {
    hasText: /^(?!▶)/,
  });
  await expect(lit).toHaveCount(1);
  await page.locator(".phz-topbar .ctrl-btn", { hasText: /^gyroid$/ }).click();
  await expect(
    page.locator(".phz-topbar .ctrl-btn.lit", { hasText: /^gyroid$/ }),
  ).toBeVisible();
  // persisted for the next session
  expect(await page.evaluate(() => localStorage.getItem("phazor:mind"))).toBe(
    "gyroid",
  );
});

test("the research minds compile and switch cleanly (indra, hopf, lenia)", async ({
  page,
}) => {
  // a broken WGSL mind surfaces as a GPU validation error in the console
  // (create_shader_module / pipeline creation) and renders black; catch it
  const gpuErrors = [];
  page.on("console", (m) => {
    if (m.type() === "error" && /shader|wgsl|pipeline|validation/i.test(m.text()))
      gpuErrors.push(m.text());
  });
  page.on("pageerror", (e) => gpuErrors.push(String(e)));

  await powerOn(page);
  for (const mind of ["indra", "hopf", "lenia"]) {
    await page
      .locator(".phz-topbar .ctrl-btn", { hasText: new RegExp(`^${mind}$`) })
      .click();
    // the lit state only follows if the swap actually took
    await expect(
      page.locator(".phz-topbar .ctrl-btn.lit", {
        hasText: new RegExp(`^${mind}$`),
      }),
    ).toBeVisible();
    await page.waitForTimeout(2500); // let feedback sims run / cameras orbit
  }
  expect(gpuErrors, gpuErrors.join("\n")).toHaveLength(0);
});

test("zen hides the chrome and z brings it back", async ({ page }) => {
  await powerOn(page);
  // zen slides the strip away with transform+opacity (still "visible" to
  // Playwright's bounding-box check) — assert the computed opacity
  const opacity = () =>
    page
      .locator(".phz-topbar")
      .evaluate((el) => getComputedStyle(el).opacity);
  await page.locator(".phz-corner .ctrl-btn", { hasText: "zen" }).click();
  await expect(async () => expect(await opacity()).toBe("0")).toPass();
  await page.keyboard.press("z");
  await expect(async () => expect(await opacity()).toBe("1")).toPass();
});

test("transport buttons drive the engine", async ({ page }) => {
  await powerOn(page);
  const play = page.locator(".phz-controls .ctrl-btn.hot");
  await expect(play).toHaveClass(/lit/, { timeout: 15_000 }); // autoplay world
  await page.locator(".phz-controls .ctrl-btn", { hasText: "■" }).click();
  await expect(play).not.toHaveClass(/lit/);
  await play.click();
  await expect(play).toHaveClass(/lit/);
});
