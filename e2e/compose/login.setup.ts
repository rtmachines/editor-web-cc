import { expect, test, type Locator } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';

// ADR-0128 (ae): the project `smoke-login` types every password. It runs with trace, screenshot,
// and video off. This file never prints a password and never puts one into a title, a label, or a step.

/** The file of the session of the Administrator. The flow project loads it. */
const ADMIN_STATE_FILE = 'admin.json';
/** The file of the session of the new user. The flow project loads it as the approver. */
const APPROVER_STATE_FILE = 'approver.json';
/** The role value of the form `POST /users`. The new user is a ProcessEngineer. */
const PROCESS_ENGINEER_ROLE = 'process_engineer';
/**
 * The default login of the Administrator. The dev server seeds `admin`.
 * The Compose stack creates `ci-admin`, so its run sets `E2E_ADMIN_LOGIN`.
 */
const DEFAULT_ADMIN_LOGIN = 'admin';
/** The login of the Administrator. An unset or empty `E2E_ADMIN_LOGIN` gives the default. */
const ADMIN_LOGIN = process.env.E2E_ADMIN_LOGIN || DEFAULT_ADMIN_LOGIN;

/** Reads a password from the environment. The config already checked its length. */
function passwordFrom(name: 'E2E_ADMIN_PASSWORD' | 'E2E_APPROVER_PASSWORD'): string {
  const value = process.env[name];
  if (!value) {
    throw new Error(`${name} must be set.`);
  }
  return value;
}

/** Returns the directory of the session files. The config sets `E2E_STATE_DIR`. */
function stateDir(): string {
  const dir = process.env.E2E_STATE_DIR;
  if (!dir) {
    throw new Error('E2E_STATE_DIR must be set. The config sets it.');
  }
  fs.mkdirSync(dir, { recursive: true });
  return dir;
}

/** Owner-only file mode. The one-time password file holds a secret. */
const SECRET_FILE_MODE = 0o600;

/** Writes the one-time password to `E2E_ONE_TIME_FILE`. It does nothing when the variable is unset. */
function saveOneTimePassword(value: string): void {
  const file = process.env.E2E_ONE_TIME_FILE;
  if (!file) {
    return;
  }
  fs.writeFileSync(file, value, { mode: SECRET_FILE_MODE });
  fs.chmodSync(file, SECRET_FILE_MODE);
}

/**
 * Sets a secret into a field. The step `locator.fill` shows its value in the title of the step.
 * The HTML reporter stores that title. The step `locator.evaluate` shows no argument.
 */
async function setSecret(field: Locator, value: string): Promise<void> {
  await field.evaluate((element, secret) => {
    const input = element as HTMLInputElement;
    input.value = secret;
    input.dispatchEvent(new Event('input', { bubbles: true }));
  }, value);
}

/** Returns a login that is unique for each run, so a second run finds no clash. */
function uniqueLogin(): string {
  const suffix = `${Date.now().toString(36)}${Math.floor(Math.random() * 1_296).toString(36)}`;
  return `smoke${suffix}`;
}

test('log in as the administrator and create the approver', async ({
  browser,
  page,
  baseURL,
}) => {
  if (!baseURL) {
    throw new Error('The project needs a baseURL.');
  }
  const dir = stateDir();
  const login = uniqueLogin();

  // Step 1: the Administrator logs in. A redirect away from /login shows a session.
  await page.goto('/login');
  await page.locator('#login').fill(ADMIN_LOGIN);
  await setSecret(page.locator('#password'), passwordFrom('E2E_ADMIN_PASSWORD'));
  await page.locator('form[action="/login"] button[type="submit"]').click();
  await page.waitForURL((url) => url.pathname !== '/login');
  await page.context().storageState({ path: path.join(dir, ADMIN_STATE_FILE) });

  // Step 2: the Administrator creates the user. The answer shows the one-time password once.
  await page.goto('/users/new');
  await page.locator('#login').fill(login);
  await page.locator('#role').selectOption(PROCESS_ENGINEER_ROLE);
  await page.locator('#user-new-form button[type="submit"]').click();
  const shown = page.locator('#one-time-password');
  await expect(shown).toBeVisible();
  const oneTimePassword = (await shown.textContent())?.trim() ?? '';
  expect(oneTimePassword.length).toBeGreaterThan(0);
  saveOneTimePassword(oneTimePassword);

  // Step 3: the new user logs in with the one-time password. The app forces the change.
  const userContext = await browser.newContext({ baseURL, ignoreHTTPSErrors: true });
  try {
    const userPage = await userContext.newPage();
    await userPage.goto('/login');
    await userPage.locator('#login').fill(login);
    await setSecret(userPage.locator('#password'), oneTimePassword);
    await userPage.locator('form[action="/login"] button[type="submit"]').click();
    await userPage.waitForURL((url) => url.pathname === '/me/password');

    // Step 4: the new user replaces the one-time password at once. The session cookie rotates.
    await setSecret(userPage.locator('#current_password'), oneTimePassword);
    await setSecret(userPage.locator('#password'), passwordFrom('E2E_APPROVER_PASSWORD'));
    const answer = userPage.waitForResponse(
      (response) =>
        new URL(response.url()).pathname === '/me/password' &&
        response.request().method() === 'POST',
    );
    // The answer sets HX-Refresh. The page reloads, so wait for that load before the next step.
    const reloaded = userPage.waitForEvent('load');
    await userPage.locator('form[action="/me/password"] button[type="submit"]').click();
    expect((await answer).status()).toBe(200);
    await reloaded;

    // Step 5: the forced change is over. A normal page opens without a redirect.
    await userPage.goto('/machines');
    await expect(userPage).toHaveURL(/\/machines$/);
    await userContext.storageState({ path: path.join(dir, APPROVER_STATE_FILE) });
  } finally {
    await userContext.close();
  }
});
