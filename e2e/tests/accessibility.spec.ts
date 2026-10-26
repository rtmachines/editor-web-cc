import AxeBuilder from '@axe-core/playwright';
import { expect, test, type BrowserContext, type Page } from '@playwright/test';
import type { Result } from 'axe-core';
import { adminCookies, LOGGED_IN_PAGES, LOGIN_PAGE, seedSweepData, type SweepPage } from './support/pages';
import {
  expectSwapSettled,
  selectLanguage,
  test as seededTest,
  type Language,
} from './support/seed';

// WCAG 2.2 AA audit layer (ADR-0048). The axe engine scans every page of the sweep and three
// dynamic states. A test fails when axe reports one violation. The tests never disable a rule,
// never exclude an element, and never change a product file. The result lists the violations.

/** The widths of the audit, in CSS px. 320 is the reflow width of WCAG 1.4.10. */
const AUDIT_WIDTHS = [320, 1280] as const;
/** The viewport height. Only the width matters for the audit. */
const VIEWPORT_HEIGHT = 900;
/** The seed step hashes passwords, so a debug build of the dev server needs time. */
const SEED_TIMEOUT_MS = 180_000;
/** The rule sets that axe runs. `wcag22aa` holds `target-size` (WCAG 2.5.8). */
const WCAG_TAGS = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];
/** The rules that the audit must run. A scan that skips one of them proves nothing. */
const REQUIRED_RULES = ['color-contrast', 'target-size'] as const;
/** The most nodes that a violation prints. */
const MAX_NODES_PRINTED = 3;
/** The most Tab presses that the focused-state test needs to reach the first form field. */
const MAX_TAB_PRESSES = 30;
/** The first field that a user reaches with Tab. */
const FIRST_FIELD = 'main input:not([type="hidden"]), main select, main textarea';
const NO_SESSION = { cookies: [], origins: [] };

/** Formats one violation for the failure message of a test. */
function describeViolation(violation: Result): string {
  const nodes = violation.nodes.slice(0, MAX_NODES_PRINTED).map((node) => {
    const target = node.target.map(String).join(' >> ');
    return `    target: ${target}\n    ${node.failureSummary?.replace(/\n/g, '\n    ') ?? ''}`;
  });
  return [
    `rule: ${violation.id} (impact: ${violation.impact ?? 'n/a'}, ${violation.nodes.length} nodes)`,
    `  tags: ${violation.tags.join(', ')}`,
    `  help: ${violation.help}`,
    `  url: ${violation.helpUrl}`,
    ...nodes,
  ].join('\n');
}

/**
 * Runs axe on the current page and asserts that it finds no violation.
 * The scan also checks that the rules `color-contrast` and `target-size` ran on this page.
 * Rules that axe cannot decide are `incomplete`. They go to the annotations of the test.
 */
async function expectNoViolations(page: Page, label: string, testInfo: { annotations: { type: string; description?: string }[] }): Promise<void> {
  await page.evaluate(() => document.fonts.ready);
  const results = await new AxeBuilder({ page }).withTags(WCAG_TAGS).analyze();
  const ran = new Set([...results.violations, ...results.passes, ...results.incomplete].map((rule) => rule.id));
  const inapplicable = new Set(results.inapplicable.map((rule) => rule.id));
  for (const rule of REQUIRED_RULES) {
    // A rule without a matching element is `inapplicable`. That is a valid result.
    expect(ran.has(rule) || inapplicable.has(rule), `axe evaluated the rule ${rule} on ${label}`).toBe(true);
  }
  for (const rule of results.incomplete) {
    testInfo.annotations.push({
      type: 'axe-incomplete',
      description: `${rule.id}: ${rule.nodes.length} nodes need a manual check`,
    });
  }
  const report = results.violations.map(describeViolation).join('\n');
  expect(
    results.violations.length,
    `${results.violations.length} axe violations on ${label}\n${report}`,
  ).toBe(0);
}

/** Opens a page and checks that it is the page that the test expects. */
async function openPage(page: Page, entry: SweepPage): Promise<void> {
  const path = entry.path();
  const response = await page.goto(path);
  expect(response?.status(), `status of ${path}`).toBe(200);
  expect(new URL(page.url()).pathname, `final path of ${path}`).toBe(new URL(path, page.url()).pathname);
  if (entry.area === 'auth') {
    await expect(page.locator('form[action="/login"]')).toBeVisible();
  } else {
    await expect(page.locator('.user-login .data')).toHaveText('admin');
  }
}

/** Adds the session of `admin` that the seed step opened (see `adminCookies`). */
async function signInAsAdmin({ context }: { context: BrowserContext }): Promise<void> {
  await context.addCookies(adminCookies());
}

/** Sets the cookie `recipe_lang` to `en` (api.md section 8). */
async function selectEnglish({ context }: { context: BrowserContext }): Promise<void> {
  await context.addCookies([{ name: 'recipe_lang', value: 'en', domain: '127.0.0.1', path: '/' }]);
}

/** Registers one axe scan for each page at the current width. */
function registerScans(entries: readonly SweepPage[], language: Language): void {
  for (const entry of entries) {
    test(`${entry.name} has no WCAG 2.2 AA violation in ${language}`, async ({ page }, testInfo) => {
      await openPage(page, entry);
      await expect(page.locator('html')).toHaveAttribute('lang', language);
      await expectNoViolations(page, `${entry.name} (${entry.path()})`, testInfo);
    });
  }
}

test.describe('axe scan of every page', () => {
  // The tests share one dev server and one seed. One worker runs this file, so seeds cannot race.
  test.describe.configure({ mode: 'default' });

  test.beforeAll(async ({ browser }, testInfo) => {
    test.setTimeout(SEED_TIMEOUT_MS);
    const baseURL = testInfo.project.use.baseURL;
    const password = process.env.E2E_PASSWORD;
    if (!baseURL || !password) {
      throw new Error('The audit needs the baseURL of the config and E2E_PASSWORD.');
    }
    await seedSweepData(browser, baseURL, password);
  });

  for (const width of AUDIT_WIDTHS) {
    test.describe(`at ${width} CSS px`, () => {
      test.use({ viewport: { width, height: VIEWPORT_HEIGHT } });

      test.describe('without a session, language pl', () => {
        test.use({ storageState: NO_SESSION });
        registerScans([LOGIN_PAGE], 'pl');
      });

      test.describe('as admin, language pl', () => {
        test.beforeEach(signInAsAdmin);
        registerScans(LOGGED_IN_PAGES, 'pl');
      });

      test.describe('without a session, language en', () => {
        test.use({ storageState: NO_SESSION });
        test.beforeEach(selectEnglish);
        registerScans([LOGIN_PAGE], 'en');
      });

      test.describe('as admin, language en, one page of each area', () => {
        test.beforeEach(signInAsAdmin);
        test.beforeEach(selectEnglish);
        registerScans(LOGGED_IN_PAGES.filter((entry) => entry.english), 'en');
      });
    });
  }
});

// The dynamic states use the seed of `support/seed.ts`. It makes a struct parameter with six
// leaf rows, so the leaf grid is open. Each worker logs in on its own, with no shared session file.
seededTest.describe('axe scan of the dynamic states', () => {
  for (const width of AUDIT_WIDTHS) {
    seededTest.describe(`at ${width} CSS px`, () => {
      seededTest.use({ viewport: { width, height: VIEWPORT_HEIGHT } });

      seededTest.beforeEach(async ({ context, baseURL }) => {
        await selectLanguage(context, baseURL, 'pl');
      });

      seededTest('the error panel after a 422 on the parameter form', async ({ page, seed }, testInfo) => {
        await page.goto(seed.struct.path);
        await page.locator('#value-0').fill('not-a-number');
        const answer = page.waitForResponse((response) => response.request().method() === 'POST');
        await page.locator('#parameter-save-submit').click();
        expect((await answer).status()).toBe(422);
        await expectSwapSettled(page);
        await expect(page.locator('#error-panel .alert-error')).toBeVisible();
        await expectNoViolations(page, 'error panel after 422', testInfo);
      });

      seededTest('the open leaf grid with the success toast', async ({ page, seed }, testInfo) => {
        await page.goto(seed.struct.path);
        const answer = page.waitForResponse((response) => response.request().method() === 'POST');
        await page.locator('#parameter-save-submit').click();
        expect((await answer).status()).toBe(200);
        await expectSwapSettled(page);
        await expect(page.locator('#toast .toast')).toBeVisible();
        await expect(page.locator('.leaf-grid')).toBeVisible();
        await expectNoViolations(page, 'leaf grid with success toast', testInfo);
      });

      seededTest('the focused state of the first input after Tab', async ({ page, seed }, testInfo) => {
        await page.goto(seed.newParameterPath);
        const first = page.locator(FIRST_FIELD).first();
        let reached = false;
        for (let press = 0; press < MAX_TAB_PRESSES && !reached; press += 1) {
          await page.keyboard.press('Tab');
          reached = await first.evaluate((element) => element === document.activeElement);
        }
        expect(reached, 'Tab reaches the first form field').toBe(true);
        await expect(first).toBeFocused();
        await expectNoViolations(page, 'first input focused after Tab', testInfo);
      });
    });
  }
});
