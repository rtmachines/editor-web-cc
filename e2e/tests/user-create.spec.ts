import { expect, test, type BrowserContext, type Page } from '@playwright/test';
import { adminCookies, seedSweepData } from './support/pages';

// Design D11 (b) of the change review-design-fixes (F-06, ADR-0172): the form of a new user
// drops a second submit while the first request runs. The server answers the first request after
// a delay, so the double click falls inside that window. The test counts the requests
// `POST /users` and reads the one-time password on the result page.

/** The seed step hashes passwords, so a debug build of the dev server needs time. */
const SEED_TIMEOUT_MS = 180_000;
/** The delay of the first answer. The second click arrives inside this window. */
const ANSWER_DELAY_MS = 750;
/** The one-time password has 20 characters of the alphabet of `api.md` §2.8. */
const ONE_TIME_PASSWORD = /^[23456789A-HJ-NP-Z]{20}$/;

test.describe.configure({ mode: 'default' });

test.beforeAll(async ({ browser }, testInfo) => {
  test.setTimeout(SEED_TIMEOUT_MS);
  const baseURL = testInfo.project.use.baseURL;
  const password = process.env.E2E_PASSWORD;
  if (!baseURL || !password) {
    throw new Error('The user create spec needs the baseURL of the config and E2E_PASSWORD.');
  }
  await seedSweepData(browser, baseURL, password);
});

/** Signs in as `admin` and selects the language `en`, so the texts below do not depend on `pl`. */
async function signInInEnglish({ context }: { context: BrowserContext }): Promise<void> {
  await context.addCookies([
    ...adminCookies(),
    { name: 'recipe_lang', value: 'en', domain: '127.0.0.1', path: '/' },
  ]);
}

/** Returns a login that no earlier run of the dev server used. */
function uniqueLogin(): string {
  return `new${Date.now().toString(36)}`;
}

/** Collects the text of every console error and every uncaught page error of the page. */
function collectConsoleErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on('console', (message) => {
    if (message.type() === 'error') {
      errors.push(message.text());
    }
  });
  page.on('pageerror', (error: Error) => {
    errors.push(error.message);
  });
  return errors;
}

test.describe('form of a new user, language en', () => {
  test.beforeEach(signInInEnglish);

  test('A double submit sends one request and shows the one-time password', async ({ page }) => {
    const consoleErrors = collectConsoleErrors(page);
    const posts: string[] = [];
    page.on('request', (request) => {
      if (request.method() === 'POST' && new URL(request.url()).pathname === '/users') {
        posts.push(request.url());
      }
    });
    await page.route(
      (url) => url.pathname === '/users',
      async (route) => {
        if (route.request().method() === 'POST') {
          await new Promise((resolve) => setTimeout(resolve, ANSWER_DELAY_MS));
        }
        await route.continue();
      },
    );

    await page.goto('/users/new');
    const form = page.locator('#user-new-form');
    await expect(form).toBeVisible();
    await expect(form).toHaveAttribute('hx-sync', 'closest form:drop');
    await form.locator('input[name="login"]').fill(uniqueLogin());

    await form.locator('#user-new-submit').dblclick();

    const password = page.locator('#one-time-password');
    await expect(password).toBeVisible();
    await expect(password).toHaveText(ONE_TIME_PASSWORD);
    expect(posts).toHaveLength(1);
    await expect(page.locator('#error-panel')).toBeEmpty();
    expect(consoleErrors).toEqual([]);
  });
});
