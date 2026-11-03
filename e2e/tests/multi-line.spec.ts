import { expect, expectSwapSettled, selectLanguage, test } from './support/seed';
import type { Locator, Page } from '@playwright/test';

// ADR-0112: the description is a multi-line `textarea`, and a save keeps every line end.
// The test fills the form of the product and reads the page again after each save.

/** The line end that a user types with Enter. The browser value of a `textarea` uses LF. */
const DESCRIPTION = 'Line 1\nLine 2';
/** The new value text of the seeded INT parameter. */
const CHANGED_VALUE = '7';

/** Submits the save form and waits until the swap of the answer ends. Returns the status. */
async function saveAndSettle(page: Page): Promise<number> {
  const answered = page.waitForResponse((response) => response.request().method() === 'POST');
  await page.locator('#parameter-save-submit').click();
  const status = (await answered).status();
  await expectSwapSettled(page);
  return status;
}

/** Reads innerText, which keeps a line end that `white-space: pre-wrap` shows. `toHaveText` folds white space. */
function innerTextOf(locator: Locator): Promise<string> {
  return locator.evaluate((element) => (element as HTMLElement).innerText);
}

test.describe('multi-line description, language en', () => {
  test.beforeEach(async ({ context, baseURL }) => {
    await selectLanguage(context, baseURL, 'en');
  });

  test('a save keeps the line end of the description, also when only a value changes', async ({
    page,
    seed,
  }) => {
    await page.goto(seed.primitive.path);

    // Save the description with two lines.
    const description = page.locator('#description_en');
    await description.fill(DESCRIPTION);
    expect(await saveAndSettle(page)).toBe(200);

    // The summary shows two lines. innerText applies the white-space rule of the page.
    const summaryDescription = page.locator('#parameter-summary dd[lang="en"]').nth(1);
    expect(await innerTextOf(summaryDescription)).toBe(DESCRIPTION);

    // A new page load shows the same value in the `textarea` and the same two lines.
    await page.goto(seed.primitive.path);
    await expect(page.locator('#description_en')).toHaveValue(DESCRIPTION);
    expect(await innerTextOf(summaryDescription)).toBe(DESCRIPTION);

    // A save of a value alone sends the description again and must keep the line end.
    await page.locator('#value-0').fill(CHANGED_VALUE);
    expect(await saveAndSettle(page)).toBe(200);
    await page.goto(seed.primitive.path);
    await expect(page.locator('#value-0')).toHaveValue(CHANGED_VALUE);
    await expect(page.locator('#description_en')).toHaveValue(DESCRIPTION);
    expect(await innerTextOf(summaryDescription)).toBe(DESCRIPTION);
  });
});
