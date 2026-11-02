import { expect, test, type BrowserContext } from '@playwright/test';
import { adminCookies, seedSweepData } from './support/pages';

// ADR-0108: the version page cuts a long value, the type page shows the S7 layout, and the compare
// form of the version page ends on the diff page. The seed step is the one of the page sweep.

/** The seed step hashes passwords, so a debug build of the dev server needs time. */
const SEED_TIMEOUT_MS = 180_000;
/** The recipe, the machine, and the type that the seed step makes (see `support/pages.ts`). */
const RECIPE = 'R-500';
const NOTE_RECIPE = 'R-501';
const MACHINE = 'M7';
const S7_STRUCT = 'ST_Panel';
/** The setting `table.value_chars` of `reliability.md` section 7.11, in characters. */
const VALUE_CHARS = 64;
const ELLIPSIS = '…';

test.describe.configure({ mode: 'default' });

test.beforeAll(async ({ browser }, testInfo) => {
  test.setTimeout(SEED_TIMEOUT_MS);
  const baseURL = testInfo.project.use.baseURL;
  const password = process.env.E2E_PASSWORD;
  if (!baseURL || !password) {
    throw new Error('The version specs need the baseURL of the config and E2E_PASSWORD.');
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

test.describe('version and type pages, language en', () => {
  test.beforeEach(signInInEnglish);

  test('version page cuts a long value at the limit and ends it with an ellipsis', async ({ page }) => {
    await page.goto(`/recipes/${NOTE_RECIPE}/versions/1`);
    const cell = page.locator('tr', { hasText: 'NOTE' }).locator('td', { hasText: ELLIPSIS });
    await expect(cell).toHaveCount(1);
    const text = (await cell.innerText()).trim();
    expect(text.endsWith(ELLIPSIS)).toBe(true);
    expect(text.length).toBe(VALUE_CHARS + ELLIPSIS.length);
  });

  test('type page of an S7 struct shows the S7 layout and its size', async ({ page }) => {
    await page.goto(`/machines/${MACHINE}/types/${S7_STRUCT}`);
    const layout = page.getByRole('region', { name: 'S7 layout' });
    await expect(layout).toBeVisible();
    await expect(layout.locator('tbody tr').first().locator('td').nth(2)).toHaveText('0.0');
    await expect(page.getByText(/The type has a size of \d+ bytes\./)).toBeVisible();
  });

  test('compare form ends on the diff page and the address bar holds its address', async ({ page }) => {
    await page.goto(`/recipes/${RECIPE}/versions/2`);
    // A marker on the window object survives a boosted swap and is lost by a full page load.
    await page.evaluate(() => {
      (window as unknown as { __boosted: boolean }).__boosted = true;
    });
    const form = page.locator('form[action$="/compare"]');
    await expect(form).toBeVisible();
    // The lower number first: the route sorts the numbers, so the address names 2 before 1.
    await form.locator('select[name="n"]').selectOption('1');
    await form.locator('select[name="m"]').selectOption('2');
    const request = page.waitForRequest((candidate) => new URL(candidate.url()).pathname.endsWith('/compare'));
    await form.locator('button[type="submit"]').click();
    expect((await request).headers()['hx-request'], 'the compare request is boosted').toBe('true');

    const diffPath = `/recipes/${RECIPE}/versions/2/diff/1`;
    await expect(page.locator('h1')).toHaveText(`Changes of version 2 of recipe ${RECIPE} against version 1`);
    expect(new URL(page.url()).pathname).toBe(diffPath);
    expect(await page.evaluate(() => (window as unknown as { __boosted?: boolean }).__boosted)).toBe(true);
  });
});
