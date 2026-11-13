import { expect, test, type BrowserContext, type Request } from '@playwright/test';
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

  test('Add a parameter from a page after the last page saves with one fields POST and one save POST', async ({
    page,
  }) => {
    // The seed draft holds two parameters, so the page 2 lies after the last page and has no #parameter-rows.
    const stamp = Date.now();
    const tag = `STALE${stamp}`;
    // An INT takes 2 bytes on an even offset. The offset follows the stamp, so a rerun on the same server finds a free address (ADR-0149).
    const address = `DB9.DBW${(stamp % 32000) * 2}`;
    const consoleLines: string[] = [];
    const posts: Request[] = [];
    page.on('console', (message) => consoleLines.push(`${message.type()}: ${message.text()}`));
    page.on('request', (request) => {
      if (request.method() === 'POST') {
        posts.push(request);
      }
    });

    await page.goto(`/recipes/${RECIPE}/versions/2?page=2`);
    await expect(page.locator('#parameter-rows')).toHaveCount(0);
    await page.locator('a[href$="/parameters/new"]').click();
    const form = page.locator('#parameter-new-form');
    await expect(form).toBeVisible();
    await form.locator('#tag').fill(tag);
    await form.locator('#name_en').fill('Added from a stale page');
    await form.locator('#description_en').fill('Added from a stale page');
    await form.locator('#type').fill('INT');
    // The machine M7 is an S7 machine, so the parameter needs a PLC address.
    await form.locator('#s7_address').fill(address);
    await form.locator('#parameter-show-fields').click();
    await expect(form.locator('#value-0')).toBeVisible();
    await form.locator('#value-0').fill('5');
    await page.locator('#parameter-add-submit').click();

    await expect(page.locator('#parameter-rows a', { hasText: tag })).toBeVisible();
    const bodies = posts.map((request) => new URLSearchParams(request.postData() ?? ''));
    expect(bodies.filter((body) => body.get('fields') === '1'), 'POST with fields=1').toHaveLength(1);
    expect(bodies.filter((body) => !body.has('fields') && body.get('tag') === tag), 'POST that saves').toHaveLength(1);
    expect(posts, 'every POST of the flow').toHaveLength(2);
    expect(consoleLines.filter((line) => line.includes('htmx:targetError')), 'htmx:targetError').toEqual([]);
    expect(consoleLines.filter((line) => line.startsWith('error')), 'console errors').toEqual([]);
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
