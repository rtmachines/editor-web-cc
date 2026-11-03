import { expect, selectLanguage, test } from './support/seed';
import type { Page } from '@playwright/test';

// Design D6 (lang attribute) and D10 (unique danger notice ids), ADR-0110.
// The test saves a German translation of the seeded parameter through the form of the product.
// The dev server offers the translation languages `pl` and `de` (task 2.4).

/** The language tag of the translation row under test. */
const TRANSLATION_LANG = 'de';
const NAME_TEXT = 'Geschwindigkeit';
const DESCRIPTION_TEXT = 'Die Geschwindigkeit der Linie';
/** The ids of the notices that the two delete buttons name (ADR-0110). */
const PARAMETER_NOTICE_ID = 'danger-notice-parameter';
const TRANSLATION_NOTICE_ID = `danger-notice-translation-${TRANSLATION_LANG}`;

/** Soft checks with a short timeout, so one run reports every failed check before the test timeout. */
const check = expect.configure({ soft: true, timeout: 1_500 });

/** Returns every `id` that occurs more than once in the document. */
function duplicateIds(): string[] {
  const seen = new Set<string>();
  const duplicates = new Set<string>();
  for (const element of Array.from(document.querySelectorAll('[id]'))) {
    if (seen.has(element.id)) {
      duplicates.add(element.id);
    }
    seen.add(element.id);
  }
  return Array.from(duplicates);
}

/** Fills the row `de` and saves it. Saving the same texts again also works, so a run is idempotent. */
async function saveTranslation(page: Page): Promise<void> {
  await page.locator(`#translation-${TRANSLATION_LANG}-name`).fill(NAME_TEXT);
  await page.locator(`#translation-${TRANSLATION_LANG}-description`).fill(DESCRIPTION_TEXT);
  const answered = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' && response.url().includes('/translations/'),
  );
  await page.locator(`#translation-${TRANSLATION_LANG}-save`).click();
  expect((await answered).ok()).toBe(true);
  await expect(page.locator('.htmx-request, .htmx-swapping, .htmx-settling')).toHaveCount(0);
}

test.describe('translation row, language en', () => {
  test.beforeEach(async ({ context, baseURL }) => {
    await selectLanguage(context, baseURL, 'en');
  });

  test('row de marks its inputs with lang and the delete buttons name unique notices', async ({
    page,
    seed,
  }) => {
    await page.goto(seed.primitive.path);
    await saveTranslation(page);

    await check(page.locator(`#translation-${TRANSLATION_LANG}-name`)).toHaveValue(NAME_TEXT);
    await check(page.locator(`#translation-${TRANSLATION_LANG}-name`)).toHaveAttribute(
      'lang',
      TRANSLATION_LANG,
    );
    await check(page.locator(`#translation-${TRANSLATION_LANG}-description`)).toHaveAttribute(
      'lang',
      TRANSLATION_LANG,
    );

    await check(page.locator('#parameter-delete-submit')).toHaveAttribute(
      'aria-describedby',
      new RegExp(`(^|\\s)${PARAMETER_NOTICE_ID}(\\s|$)`),
    );
    await check(page.locator(`#translation-${TRANSLATION_LANG}-delete`)).toHaveAttribute(
      'aria-describedby',
      new RegExp(`(^|\\s)${TRANSLATION_NOTICE_ID}(\\s|$)`),
    );
    await check(page.locator(`#${PARAMETER_NOTICE_ID}`)).toHaveCount(1);
    await check(page.locator(`#${TRANSLATION_NOTICE_ID}`)).toHaveCount(1);

    check(await page.evaluate(duplicateIds)).toEqual([]);
  });
});
