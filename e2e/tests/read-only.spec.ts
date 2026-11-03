import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { expect, selectLanguage, test } from './support/seed';
import { READ_ONLY_FILE } from './support/read-only';

// ADR-0113 (h), design D7: the second dev server turns the read-only mode on while the file exists.
// The seed runs before the file exists, because the fixture seeds first and the test body follows.

/** The start of the English text of the banner `readonly-banner`. */
const BANNER_TEXT = 'Read-only mode: the free disk space is low or unknown.';
/** The English text of the notice `readonly-form-unavailable`. */
const FORM_NOTICE_TEXT = 'Read-only mode blocks this form.';
/** The dev server checks the file every 250 ms. The wait covers a few checks. */
const MODE_WAIT_MS = 10_000;
/** The wait for the banner after one page load. */
const BANNER_WAIT_MS = 1_000;

/** Creates the switch file and loads `target` again until the banner shows. */
async function enterReadOnlyMode(page: Page, target: string): Promise<void> {
  fs.mkdirSync(path.dirname(READ_ONLY_FILE), { recursive: true });
  fs.writeFileSync(READ_ONLY_FILE, '');
  await expect(async () => {
    await page.goto(target);
    await expect(page.getByText(BANNER_TEXT)).toBeVisible({ timeout: BANNER_WAIT_MS });
  }).toPass({ timeout: MODE_WAIT_MS });
}

test.describe('read-only mode, language en', () => {
  test.beforeEach(async ({ context, baseURL }) => {
    await selectLanguage(context, baseURL, 'en');
  });

  test.afterAll(() => {
    fs.rmSync(READ_ONLY_FILE, { force: true });
  });

  test('the parameter page shows the read-only grid and only the logout and language forms', async ({
    page,
    seed,
  }) => {
    await enterReadOnlyMode(page, seed.primitive.path);

    await expect(page.locator('table.leaf-grid')).toBeVisible();
    await expect(page.locator('table.leaf-grid input, table.leaf-grid select')).toHaveCount(0);
    await expect(page.locator('#parameter-save-submit')).toHaveCount(0);
    await expect(page.locator('form[action="/logout"]')).toHaveCount(1);
    await expect(page.locator('form[action="/lang"]')).toHaveCount(1);
    await expect(
      page.locator('form[method="post"]:not([action="/logout"]):not([action="/lang"])'),
    ).toHaveCount(0);
  });

  test('the form of a new recipe shows the notice in place of the form', async ({ page }) => {
    await enterReadOnlyMode(page, '/recipes/new');

    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    await expect(page.getByText(FORM_NOTICE_TEXT)).toBeVisible();
    await expect(page.locator('#recipe-new-form')).toHaveCount(0);
  });
});
