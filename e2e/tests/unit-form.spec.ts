import { expect, test, type BrowserContext, type Page } from '@playwright/test';
import { adminCookies, seedSweepData } from './support/pages';

// Design D3 of the change ui-form-fixes: the form of a new unit names its parent in
// the heading and in a text line, has no control for the parent, and proposes a free segment. The
// seed step makes the machine `M7` with the unit `U1` and the unit `U1.2`, so the proposed segment
// is the first free number under `U1`. The test reads the proposal and does not fix its value.

/** The seed step hashes passwords, so a debug build of the dev server needs time. */
const SEED_TIMEOUT_MS = 180_000;
/** The machine that the seed step makes. */
const MACHINE = 'M7';
/** The parent unit of the new unit. */
const PARENT = 'U1';

test.describe.configure({ mode: 'default' });

test.beforeAll(async ({ browser }, testInfo) => {
  test.setTimeout(SEED_TIMEOUT_MS);
  const baseURL = testInfo.project.use.baseURL;
  const password = process.env.E2E_PASSWORD;
  if (!baseURL || !password) {
    throw new Error('The unit form spec needs the baseURL of the config and E2E_PASSWORD.');
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

/** Returns a unit name that no earlier run of the dev server used. */
function uniqueUnitName(): string {
  return `Zone ${Date.now().toString(36)}`;
}

test.describe('form of a new unit, language en', () => {
  test.beforeEach(signInInEnglish);

  test('Proposed segment saves in the browser', async ({ page }) => {
    const consoleErrors = collectConsoleErrors(page);
    await page.goto(`/machines/${MACHINE}`);

    const addLink = page
      .locator('#unit-tree')
      .getByRole('link', { name: `Add a sub-unit to ${PARENT}`, exact: true });
    await addLink.focus();
    await page.keyboard.press('Enter');

    const form = page.locator('#unit-add-form');
    await expect(form).toBeVisible();
    await expect(page.locator('#unit-form h2')).toHaveText(`New unit under ${PARENT}`);
    await expect(page.locator('#parent-line')).toContainText('Parent unit');
    await expect(page.locator('#parent-line .data')).toHaveText(PARENT);
    await expect(form.locator('select')).toHaveCount(0);
    await expect(form.locator('input[name="parent_path"]')).toHaveValue(PARENT);

    const segmentInput = form.locator('#segment');
    const segment = await segmentInput.inputValue();
    expect(segment).toMatch(/^[1-9][0-9]?$/);

    await form.locator('input[name="name"]').fill(uniqueUnitName());
    await form.locator('#unit-create-submit').click();

    const newPath = `${PARENT}.${segment}`;
    await expect(
      page.locator('#unit-tree .tree-path').filter({ hasText: new RegExp(`^${newPath.replace('.', '\\.')}$`) }),
    ).toHaveCount(1);
    await expect(page.locator('#error-panel')).toBeEmpty();
    await expect(page.locator('#unit-tree')).toContainText(newPath);
    expect(consoleErrors).toEqual([]);
  });
});
