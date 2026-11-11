import { expect, test, type BrowserContext, type Page } from '@playwright/test';

// ADR-0154: a Process engineer saves the PLC connection of a TwinCat3 machine with a password.
// The form of the next visit shows an empty password input and the state "password stored".
// No page holds the password text (R31, A47).

/** The seed step and the login hash passwords, so a debug build of the dev server needs time. */
const SPEC_TIMEOUT_MS = 180_000;
/** The login of the seeded Process engineer. The dev server seeds it with `E2E_PASSWORD`. */
const ENGINEER = 'engineer';
/** The browser sends this header on a same-origin form post. The CSRF layer checks it (api.md 7.3). */
const SAME_ORIGIN = 'same-origin';
/** A machine code is M and a number from 1 to 9999999. The test picks a number above this base. */
const MACHINE_NUMBER_BASE = 2_000_000;
const MACHINE_NUMBER_SPAN = 7_000_000;
/** The CSRF token sits in the `hx-headers` attribute of the body (api.md 7.3). */
const CSRF_HEADER_PATTERN = /hx-headers='\{"X-CSRF-Token":"([^"]+)"\}'/;
/** The connection values of the test. The mode Ads needs a login together with a password. */
const CONNECTION = {
  host: '192.168.10.5',
  port: '851',
  address: '192.168.10.5.1.1',
  mode: 'Ads',
  login: 'ads_user',
} as const;

test.describe.configure({ mode: 'default' });
test.setTimeout(SPEC_TIMEOUT_MS);

/** Signs in as the Process engineer through the login form and selects the language `en`. */
async function signInAsEngineer(context: BrowserContext, page: Page, password: string): Promise<void> {
  await context.addCookies([{ name: 'recipe_lang', value: 'en', domain: '127.0.0.1', path: '/' }]);
  await page.goto('/login');
  await page.locator('#login').fill(ENGINEER);
  await page.locator('#password').fill(password);
  await page.locator('form[action="/login"] button[type="submit"]').click();
  await page.waitForURL((url) => url.pathname !== '/login');
}

/** Creates a TwinCat3 machine with the form route, as the browser does, and returns its code. */
async function createTwinCatMachine(context: BrowserContext): Promise<string> {
  const reply = await context.request.get('/machines');
  const csrf = CSRF_HEADER_PATTERN.exec(await reply.text())?.[1];
  if (!csrf) {
    throw new Error('The test found no CSRF token. The session may have expired.');
  }
  const code = `M${MACHINE_NUMBER_BASE + Math.floor(Math.random() * MACHINE_NUMBER_SPAN)}`;
  const created = await context.request.post('/machines', {
    form: { code, name: `TwinCat line ${code}`, plc_platform: 'TwinCat3', main_unit_name: 'Line' },
    headers: { 'X-CSRF-Token': csrf, 'Sec-Fetch-Site': SAME_ORIGIN },
    maxRedirects: 0,
  });
  expect(created.status(), 'the machine form answers 303').toBe(303);
  return code;
}

test('a Process engineer saves a TwinCat3 connection and the form shows a stored password', async ({
  page,
  context,
}) => {
  const password = process.env.E2E_PASSWORD ?? '';
  // The password of the connection is a probe text. It must not appear in any page after the save.
  const secret = `probe-${Math.random().toString(36).slice(2)}-secret`;
  await signInAsEngineer(context, page, password);
  const code = await createTwinCatMachine(context);

  await page.goto(`/machines/${code}`);
  await expect(page.getByText('The machine has no stored connection.')).toBeVisible();
  await page.getByRole('link', { name: 'Edit connection' }).click();
  await page.waitForURL(`**/machines/${code}/connection/edit`);

  const form = page.locator('form#connection-form');
  await form.locator('[name="host"]').fill(CONNECTION.host);
  await form.locator('[name="port"]').fill(CONNECTION.port);
  await form.locator('[name="address"]').fill(CONNECTION.address);
  await form.locator('[name="comm_mode"]').selectOption(CONNECTION.mode);
  await form.locator('[name="login"]').fill(CONNECTION.login);
  await form.locator('[name="password"]').fill(secret);
  await form.locator('#connection-submit').click();
  await expect(page.locator('#toast')).toContainText(`The connection of machine ${code} is saved.`);
  await expect(page.locator('#error-panel')).toBeEmpty();

  // The next visit loads the form again. The password input is empty and the hint names the state.
  await page.goto(`/machines/${code}/connection/edit`);
  const reloaded = page.locator('form#connection-form');
  await expect(reloaded.locator('[name="host"]')).toHaveValue(CONNECTION.host);
  await expect(reloaded.locator('[name="login"]')).toHaveValue(CONNECTION.login);
  await expect(reloaded.locator('[name="password"]')).toHaveValue('');
  await expect(reloaded.locator('[name="password"]')).not.toHaveAttribute('value', /.+/);
  await expect(reloaded).toContainText('The system stores a password.');
  expect(await page.content(), 'the form page never holds the password').not.toContain(secret);

  // The machine page shows the state and never the password.
  await page.goto(`/machines/${code}`);
  const record = page.locator('#machine-connection');
  await expect(record).toContainText(CONNECTION.host);
  await expect(record).toContainText('Password stored');
  expect(await page.content(), 'the machine page never holds the password').not.toContain(secret);
  expect(await page.locator('body').innerText()).not.toContain(secret);
});
