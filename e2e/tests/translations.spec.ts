import { expect, expectSwapSettled, selectLanguage, test } from './support/seed';
import type { Page } from '@playwright/test';

// Design D6 (lang attribute) and D10 (unique danger notice ids), ADR-0110.
// ADR-0111: the deletion answer swaps one row, and a save of the definition refreshes the base text.
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

/** The sibling row whose unsaved input must survive (ADR-0111). */
const SIBLING_LANG = 'pl';
const TYPED_TEXT = 'Wpisany tekst bez zapisu';
const SIBLING_SAVED_TEXT = 'Predkosc';

/** Reads the value of the hidden `row_version` field of a translation form. */
async function formRowVersion(page: Page, lang: string): Promise<number> {
  const value = await page
    .locator(`#translation-${lang}-form input[name="row_version"]`)
    .inputValue();
  return Number(value);
}

/** Runs an action that submits a form. Returns the status of the answer after the swap settled. */
async function answerOf(page: Page, action: () => Promise<void>): Promise<number> {
  const answered = page.waitForResponse((response) => response.request().method() === 'POST');
  await action();
  const status = (await answered).status();
  await expectSwapSettled(page);
  return status;
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

  // ADR-0111 (a) to (e): the deletion swaps only the row of the deleted tag.
  test('deleting row de keeps the unsaved input of row pl and the next save gives no 409', async ({
    page,
    seed,
  }) => {
    await page.goto(seed.primitive.path);
    await saveTranslation(page);

    const siblingName = page.locator(`#translation-${SIBLING_LANG}-name`);
    await siblingName.fill(TYPED_TEXT);
    const versionBefore = await formRowVersion(page, SIBLING_LANG);

    const row = page.locator(`#translation-${TRANSLATION_LANG}`);
    await row.locator('details.danger > summary').click();
    const remove = page.locator(`#translation-${TRANSLATION_LANG}-delete`);
    await remove.focus();
    expect(await answerOf(page, () => remove.press('Enter'))).toBe(200);

    await check(siblingName).toHaveValue(TYPED_TEXT);
    await check(row.locator('h3')).toBeFocused();
    const versionAfter = await formRowVersion(page, SIBLING_LANG);
    check(versionAfter).toBeGreaterThan(versionBefore);
    check(versionAfter).toBe(await formRowVersion(page, TRANSLATION_LANG));

    // The save of the sibling row sends the new row_version without a page load.
    await siblingName.fill(SIBLING_SAVED_TEXT);
    const save = page.locator(`#translation-${SIBLING_LANG}-save`);
    expect(await answerOf(page, () => save.click())).toBe(200);
    await check(page.locator('#toast .toast')).toBeVisible();
    await check(page.locator('#error-panel')).toBeEmpty();

    // The last stored translation goes away and leaves an empty form without a disclosure.
    const siblingRow = page.locator(`#translation-${SIBLING_LANG}`);
    await siblingRow.locator('details.danger > summary').click();
    const removeSibling = page.locator(`#translation-${SIBLING_LANG}-delete`);
    expect(await answerOf(page, () => removeSibling.click())).toBe(200);
    await check(page.locator(`#translation-${SIBLING_LANG}-name`)).toHaveValue('');
    await check(siblingRow.locator('details.danger')).toHaveCount(0);
  });

  // ADR-0111 (i): the answer of a save of the definition also refreshes the base text.
  test('saving the definition refreshes the base text and keeps the unsaved input of row pl', async ({
    page,
    seed,
  }) => {
    const newName = 'Renamed by the base text test';
    await page.goto(seed.primitive.path);

    const siblingName = page.locator(`#translation-${SIBLING_LANG}-name`);
    await siblingName.fill(TYPED_TEXT);
    await page.locator('#name_en').fill(newName);
    const save = page.locator('#parameter-save-submit');
    expect(await answerOf(page, () => save.click())).toBe(200);

    await check(page.locator('#translations-base')).toContainText(newName);
    await check(siblingName).toHaveValue(TYPED_TEXT);
  });
});
