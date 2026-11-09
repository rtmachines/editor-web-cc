import {
  expect,
  test as base,
  type Browser,
  type BrowserContext,
  type Page,
} from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { exportContentFaults } from './support/export-content';
import { contentSha256, type JsonObject } from './support/jcs';

// ADR-0128 (af) to (ai): the flow of the smoke suite. The project `smoke-compose` runs this file
// after `smoke-login`. The Administrator edits the draft. The second user approves it.
// This file never types a password. It loads the session files that `login.setup.ts` saved.

/** The file names that `login.setup.ts` writes into `E2E_STATE_DIR`. */
const ADMIN_STATE_FILE = 'admin.json';
const APPROVER_STATE_FILE = 'approver.json';
/** A machine code is M and a number from 1 to 9999999. The number below has seven digits. */
const NUMBER_BASE = 1_000_000;
const NUMBER_SPAN = 8_999_999;
/** The limits of the parameter. The value lies between them, so the approval finds no violation. */
const PARAMETER = { tag: 'SPEED', type: 'INT', value: '50', min: '0', max: '100' } as const;
/** A new recipe opens with its first draft, which holds the main unit `U1`. */
const VERSION_NUMBER = 1;
const UNIT_PATH = 'U1';
/** The code of the refusal when the editor of a draft approves it (reliability.md section 1.2). */
const EDITOR_REFUSAL_CODE = 'E-RCP-015';
/** The status code of that refusal (api.md section 4). */
const EDITOR_REFUSAL_STATUS = 409;
/** The page selects English, so the texts below do not depend on the default language `pl`. */
const LANGUAGE_COOKIE = { name: 'recipe_lang', value: 'en' } as const;

/** Reads the directory of the session files. The config sets `E2E_STATE_DIR`. */
function stateFile(name: string): string {
  const dir = process.env.E2E_STATE_DIR;
  if (!dir) {
    throw new Error('E2E_STATE_DIR must be set. The config sets it.');
  }
  return path.join(dir, name);
}

/** Returns the names of a new run. A second run on the same stack finds no duplicate. */
function newRun(): { readonly machineCode: string; readonly reference: string } {
  const number = NUMBER_BASE + Math.floor(Math.random() * NUMBER_SPAN);
  return { machineCode: `M${number}`, reference: `R-${number}` };
}

/** Starts a context from a session file with the language `en`. */
async function contextFrom(
  browser: Browser,
  baseURL: string,
  file: string,
): Promise<BrowserContext> {
  if (!fs.existsSync(file)) {
    throw new Error(`The session file ${path.basename(file)} is missing. Run the project smoke-login.`);
  }
  const context = await browser.newContext({
    baseURL,
    ignoreHTTPSErrors: true,
    storageState: file,
  });
  await context.addCookies([{ ...LANGUAGE_COOKIE, url: baseURL }]);
  return context;
}

interface Fixtures {
  /** The page of the second user. The config keeps the trace of a failed test (ADR-0128 (af)). */
  approverPage: Page;
}

const test = base.extend<Fixtures>({
  page: async ({ browser, baseURL }, use) => {
    const context = await contextFrom(browser, baseURL ?? '', stateFile(ADMIN_STATE_FILE));
    await use(await context.newPage());
    await context.close();
  },
  approverPage: async ({ browser, baseURL }, use) => {
    const context = await contextFrom(browser, baseURL ?? '', stateFile(APPROVER_STATE_FILE));
    await use(await context.newPage());
    await context.close();
  },
});

/** Reads the CSRF token that every page of a session carries in its logout form. */
async function csrfTokenOf(page: Page): Promise<string> {
  const token = await page.locator('input[name="csrf_token"]').first().getAttribute('value');
  if (!token) {
    throw new Error('The page holds no csrf_token field.');
  }
  return token;
}

const run = newRun();
/** The path of the draft page. The first test sets it. */
let versionPath = '';
/** The `row_version` of the draft after the last save. The refusal test reads it. */
let draftRowVersion = '';

test.describe.configure({ mode: 'serial' });

test('the Administrator creates a machine, a recipe, a parameter, and its limits', async ({ page }) => {
  // Step 1: the machine.
  await page.goto('/machines/new');
  await page.locator('#code').fill(run.machineCode);
  await page.locator('#machine_name').fill(`Smoke machine ${run.machineCode}`);
  await page.locator('#plc_platform').selectOption('CodesysIec');
  await page.locator('#main_unit_name').fill('Line');
  await page.locator('#machine-create-submit').click();
  await expect(page).toHaveURL(new RegExp(`/machines/${run.machineCode}$`));

  // Step 2: the recipe. The answer opens the page of its first draft.
  await page.goto('/recipes/new');
  await page.locator('#machine_code').selectOption(run.machineCode);
  await page.locator('#reference_number').fill(run.reference);
  await page.locator('#recipe_name').fill(`Smoke recipe ${run.reference}`);
  await page.locator('#change_note').fill('Created by the smoke suite');
  await page.locator('#recipe-create-submit').click();
  versionPath = `/recipes/${run.reference}/versions/${VERSION_NUMBER}`;
  await expect(page).toHaveURL(new RegExp(`${versionPath}$`));
  await expect(page.locator('.badge-large').first()).toContainText('Draft');

  // Step 3: the parameter. The first submit shows the value fields. The second one saves.
  await page.goto(`${versionPath}/parameters/new?unit=${UNIT_PATH}`);
  await page.locator('#tag').fill(PARAMETER.tag);
  await page.locator('#name_en').fill('Speed');
  await page.locator('#description_en').fill('Speed of the line');
  await page.locator('#type').fill(PARAMETER.type);
  await page.locator('#ordinal').fill('0');
  await page.locator('#parameter-show-fields').click();
  await page.locator('#value-0').fill(PARAMETER.value);
  await page.locator('#parameter-add-submit').click();
  await expect(page).toHaveURL(new RegExp(`${versionPath}$`));
  const link = page.locator(`a[href^="${versionPath}/parameters/"]`, { hasText: PARAMETER.tag });
  await expect(link).toBeVisible();

  // Step 4: the limits. The edit page of the parameter holds the limit inputs.
  await link.click();
  await page.locator('#limits-0-min').fill(PARAMETER.min);
  await page.locator('#limits-0-max').fill(PARAMETER.max);
  await page.locator('#parameter-save-submit').click();
  await expect(page.locator('#toast .toast')).toBeVisible();
  await expect(page.locator('#error-panel .alert-error')).toHaveCount(0);
  await page.reload();
  await expect(page.locator('#limits-0-min')).toHaveValue(PARAMETER.min);
  await expect(page.locator('#limits-0-max')).toHaveValue(PARAMETER.max);

  // Read the row_version of the draft for the refusal test.
  await page.goto(versionPath);
  draftRowVersion =
    (await page.locator('form[action$="/discard"] input[name="row_version"]').getAttribute('value')) ?? '';
  expect(draftRowVersion).toMatch(/^\d+$/);
});

test('the editor of the draft cannot approve it (E-RCP-015)', async ({ page }) => {
  const approvePath = `${versionPath}/approve`;
  await page.goto(approvePath);
  // The preview names the refusal and offers no approval form.
  await expect(page.locator('.alert-code', { hasText: EDITOR_REFUSAL_CODE })).toBeVisible();
  await expect(page.locator('#approve-submit')).toHaveCount(0);

  // A forged post gets the same refusal. The page sends it, so the browser sets the origin headers.
  const answer = await page.evaluate(
    async ({ path: target, csrf, rowVersion }) => {
      const response = await fetch(target, {
        method: 'POST',
        body: new URLSearchParams({ csrf_token: csrf, row_version: rowVersion }),
        redirect: 'manual',
      });
      return { status: response.status, text: await response.text() };
    },
    { path: approvePath, csrf: await csrfTokenOf(page), rowVersion: draftRowVersion },
  );
  expect(answer.status).toBe(EDITOR_REFUSAL_STATUS);
  expect(answer.text).toContain(EDITOR_REFUSAL_CODE);

  // The draft keeps its status.
  await page.goto(versionPath);
  await expect(page.locator('.badge-large').first()).toContainText('Draft');
});

test('the second user approves the draft', async ({ approverPage: page }) => {
  await page.goto(`${versionPath}/approve`);
  await page.locator('#approve-submit').click();
  await expect(page).toHaveURL(new RegExp(`${versionPath}$`));
  await expect(page.locator('.badge-large').first()).toContainText('Approved');
  await expect(page.locator('#error-panel .alert-error')).toHaveCount(0);
});

test('the Administrator exports the approved version, and the hash matches', async ({ page }) => {
  // The page fixture is the Administrator session (ADR-0128 (ah)). The second user only approves.
  await page.goto(versionPath);
  const exportLink = page.locator('a[href*="/export/"]');
  await expect(exportLink).toHaveCount(1);

  // The link is a plain download link. The click sends `Sec-Fetch-Site: same-origin` (ADR-0055).
  const [response, download] = await Promise.all([
    page.waitForResponse((candidate) => new URL(candidate.url()).pathname.includes('/export/')),
    page.waitForEvent('download'),
    exportLink.click(),
  ]);
  expect(response.status()).toBe(200);
  // The answer is a file, so the browser saves it. The test reads the saved file.
  const savedPath = await download.path();
  const document = JSON.parse(fs.readFileSync(savedPath, 'utf8')) as JsonObject;
  expect(document.loadable).toBe(true);
  expect(document.content_sha256).toMatch(/^[0-9a-f]{64}$/);
  // The consumer rebuilds the hash from the hashed fields (ADR-0128 (ai)).
  expect(contentSha256(document)).toBe(document.content_sha256);
  // The body holds the values that the earlier tests entered (design D6 of test-gates-round-2).
  expect(
    exportContentFaults(document, {
      machineCode: run.machineCode,
      referenceNumber: run.reference,
      versionNumber: VERSION_NUMBER,
      unitPath: UNIT_PATH,
      tag: PARAMETER.tag,
      type: PARAMETER.type,
      value: PARAMETER.value,
      min: PARAMETER.min,
      max: PARAMETER.max,
    }),
  ).toEqual([]);
});

test('the audit page shows the entry of the approval', async ({ page }) => {
  // The history page of the recipe links to the audit page of the whole recipe.
  await page.goto(`/recipes/${run.reference}/history`);
  const whole = page.locator('a[href^="/audit?aggregate="]');
  await expect(whole).toHaveCount(1);
  const href = await whole.getAttribute('href');
  expect(href).not.toBeNull();

  // Other runs leave entries in the log. The aggregate keeps only the entries of this recipe.
  await page.goto(`${href}&action=VersionApproved`);
  await expect(page.getByRole('heading', { level: 1, name: 'Audit log' })).toBeVisible();
  const rows = page.getByRole('region', { name: 'Audit entries' }).locator('tbody tr');
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText('Version approved');
});
