import {
  expect,
  expectSwapSettled,
  GRID_CONTROLS,
  selectLanguage,
  test,
  type Language,
} from './support/seed';
import type { Page } from '@playwright/test';

// ADR-0048 (c) and (d): focus after a swap, WCAG 2.4.11, the summary link, and the long word.
// design-system.md sections 7 and 10.6 and design D16 define the expected focus.
// A test reads the page through the DOM and the computed layout. It submits only the forms of the
// product.

const VIEWPORT_HEIGHT = 800;
/** The widths of WCAG 2.4.11 checks: 320 and three widths between 481 and about 690 CSS px. */
const FOCUS_WIDTHS = [320, 481, 600, 690] as const;
/** The most Tab presses for one run. The page has fewer than 80 focusable elements. */
const MAX_TAB_PRESSES = 120;
/** A word of 200 characters without a space. The field `type` has no maxlength. */
const LONG_WORD = 'A'.repeat(200);

/** The result of the sample of one focused element. */
interface Coverage {
  readonly id: string;
  readonly samplesInViewport: number;
  readonly samplesOpen: number;
  readonly centerCoveredBy: string;
}

/**
 * Samples the box of the focused element with `elementFromPoint`.
 * The samples form a grid of 5 columns and 3 rows. They include the four corners and the center. A sample is open when the element at that
 * point is the focused element or one of its descendants. The function runs inside the page and
 * changes nothing.
 */
function sampleFocusedElement(): Coverage {
  const focused = document.activeElement as HTMLElement | null;
  if (!focused) {
    return { id: '', samplesInViewport: 0, samplesOpen: 0, centerCoveredBy: 'no focus' };
  }
  const box = focused.getBoundingClientRect();
  const inset = 2;
  const viewportWidth = document.documentElement.clientWidth;
  const viewportHeight = document.documentElement.clientHeight;
  const describe = (element: Element | null): string =>
    element ? `${element.tagName.toLowerCase()}#${element.id}.${String(element.className)}` : 'none';
  let samplesInViewport = 0;
  let samplesOpen = 0;
  let centerCoveredBy = 'open';
  for (let row = 0; row < 3; row += 1) {
    for (let column = 0; column < 5; column += 1) {
      const x = box.left + inset + ((box.width - 2 * inset) * column) / 4;
      const y = box.top + inset + ((box.height - 2 * inset) * row) / 2;
      const inside = x >= 0 && x < viewportWidth && y >= 0 && y < viewportHeight;
      if (!inside) {
        continue;
      }
      samplesInViewport += 1;
      const top = document.elementFromPoint(x, y);
      const open = top !== null && (top === focused || focused.contains(top));
      if (open) {
        samplesOpen += 1;
      } else if (row === 1 && column === 2) {
        centerCoveredBy = describe(top);
      }
    }
  }
  return { id: focused.id, samplesInViewport, samplesOpen, centerCoveredBy };
}

/** Returns the id of the focused grid control, or null when the focus is outside the grid. */
function focusedGridControlId(): string | null {
  const active = document.activeElement;
  return active?.closest('.leaf-grid') ? active.id : null;
}

/** Waits for the next POST answer, runs the action that sends it, then waits for the swap. */
async function submitAndSettle(page: Page, action: () => Promise<void>): Promise<number> {
  const answer = page.waitForResponse((response) => response.request().method() === 'POST');
  await action();
  const status = (await answer).status();
  await expectSwapSettled(page);
  return status;
}

test.describe('focus after an HTMX swap', () => {
  for (const language of ['pl', 'en'] as const satisfies readonly Language[]) {
    test.describe(`in ${language}`, () => {
      test.beforeEach(async ({ context, baseURL }) => {
        await selectLanguage(context, baseURL, language);
      });

      test('a click on Save keeps the focus on the Save button', async ({ page, seed }) => {
        await page.goto(seed.struct.path);
        const save = page.locator('#parameter-save-submit');
        const status = await submitAndSettle(page, () => save.click());
        expect(status).toBe(200);
        await expect(page.locator('#toast .toast')).toBeVisible();
        await expect(save).toBeFocused();
      });

      test('Enter in a value input keeps the focus on that input', async ({ page, seed }) => {
        await page.goto(seed.struct.path);
        const input = page.locator('#value-0');
        await input.focus();
        const status = await submitAndSettle(page, () => input.press('Enter'));
        expect(status).toBe(200);
        await expect(page.locator('#toast .toast')).toBeVisible();
        await expect(input).toBeFocused();
      });

      test('an error answer does not move the focus from the Save button', async ({
        page,
        seed,
      }) => {
        await page.goto(seed.struct.path);
        await page.locator('#value-0').fill('not-a-number');
        const save = page.locator('#parameter-save-submit');
        const status = await submitAndSettle(page, () => save.click());
        expect(status).toBe(422);
        await expect(page.locator('#error-panel .alert-error')).toBeVisible();
        await expect(page.locator('#value-0')).toHaveAttribute('aria-invalid', 'true');
        await expect(save).toBeFocused();
      });

      test('"Show fields" on the stand-alone page moves the focus to the new heading', async ({
        page,
        seed,
      }) => {
        // The answer swaps #content, so the h1 has tabindex -1 and autofocus (design-system.md 10.1).
        await page.goto(seed.newParameterPath);
        await page.locator('#tag').fill('SHOWFIELDS');
        await page.locator('#name_en').fill('Show fields');
        await page.locator('#description_en').fill('Show fields');
        await page.locator('#type').fill('INT');
        const status = await submitAndSettle(page, () =>
          page.locator('#parameter-show-fields').click(),
        );
        expect(status).toBe(200);
        await expect(page.locator('#value-0')).toBeVisible();
        await expect(page.locator('main h1')).toBeFocused();
      });

      test('"Show fields" in the in-page form keeps the focus on its button', async ({
        page,
        seed,
      }) => {
        // The answer swaps only the form (HX-Retarget), and the fixed id keeps the focus (ADR-0041 (e)).
        await page.goto(seed.draftPath);
        await page.locator('a[href$="/parameters/new"]').click();
        const form = page.locator('#parameter-new-form');
        await expect(form).toBeVisible();
        await form.locator('#tag').fill('SHOWFIELDS');
        await form.locator('#name_en').fill('Show fields');
        await form.locator('#description_en').fill('Show fields');
        await form.locator('#type').fill('INT');
        const show = page.locator('#parameter-show-fields');
        const status = await submitAndSettle(page, () => show.click());
        expect(status).toBe(200);
        await expect(page.locator('#value-0')).toBeVisible();
        await expect(show).toBeFocused();
      });

      test('adding a parameter moves the focus to the link of the new row', async ({
        page,
        seed,
      }) => {
        // The tag is unique for each run, so a repeated run on one worker never collides.
        const tag = `ADD${Date.now()}`;
        await page.goto(seed.draftPath);
        await page.locator('a[href$="/parameters/new"]').click();
        const form = page.locator('#parameter-new-form');
        await expect(form).toBeVisible();
        await form.locator('#tag').fill(tag);
        await form.locator('#name_en').fill('Added by the focus test');
        await form.locator('#description_en').fill('Added by the focus test');
        await form.locator('#type').fill('INT');
        await submitAndSettle(page, () => form.locator('#parameter-show-fields').click());
        await form.locator('#value-0').fill('5');
        const status = await submitAndSettle(page, () =>
          page.locator('#parameter-add-submit').click(),
        );
        expect(status).toBe(200);
        const link = page.locator('#parameter-rows a', { hasText: tag });
        await expect(link).toBeVisible();
        await expect(link).toBeFocused();
      });
    });
  }
});

test.describe('WCAG 2.4.11 Focus Not Obscured in the leaf grid', () => {
  const cases: ReadonlyArray<{
    subject: 'struct' | 'array';
    width: number;
    withToast: boolean;
    language: Language;
  }> = [
    ...(['struct', 'array'] as const).flatMap((subject) =>
      FOCUS_WIDTHS.flatMap((width) =>
        [false, true].map((withToast) => ({
          subject,
          width,
          withToast,
          language: 'pl' as const,
        })),
      ),
    ),
    { subject: 'struct', width: 600, withToast: true, language: 'en' },
  ];

  for (const { subject, width, withToast, language } of cases) {
    const state = withToast ? 'with the success toast' : 'without a toast';
    test(`no element hides a focused ${subject} grid control at ${width}px ${state} in ${language}`, async ({
      page,
      context,
      baseURL,
      seed,
    }) => {
      await selectLanguage(context, baseURL, language);
      await page.setViewportSize({ width, height: VIEWPORT_HEIGHT });
      await page.goto(seed[subject].path);
      if (withToast) {
        // The toast sticks at the top of the view (design-system.md 10.7). It must not hide a focus.
        await submitAndSettle(page, () => page.locator('#parameter-save-submit').click());
        await expect(page.locator('#toast .toast')).toBeVisible();
      }

      const expectedCount = await page.locator(GRID_CONTROLS).count();
      expect(expectedCount, 'the grid has controls').toBeGreaterThan(5);

      await page.locator('#ordinal').focus();
      let checked = 0;
      for (let press = 0; press < MAX_TAB_PRESSES && checked < expectedCount; press += 1) {
        await page.keyboard.press('Tab');
        if ((await page.evaluate(focusedGridControlId)) === null) {
          continue;
        }
        checked += 1;
        const coverage = await page.evaluate(sampleFocusedElement);
        const where = `control "${coverage.id}" at ${width}px`;
        expect(coverage.samplesInViewport, `${where} lies inside the viewport`).toBeGreaterThan(0);
        expect(
          coverage.samplesOpen,
          `${where} is fully covered by ${coverage.centerCoveredBy} ` +
            `(${coverage.samplesInViewport} samples in the viewport)`,
        ).toBeGreaterThan(0);
      }
      expect(checked, 'the check visited every grid control').toBe(expectedCount);
    });
  }
});

test.describe('the summary link of the error panel', () => {
  for (const language of ['pl', 'en'] as const satisfies readonly Language[]) {
    test(`a link to a value input moves the focus to it in ${language}`, async ({
      page,
      context,
      baseURL,
      seed,
    }) => {
      await selectLanguage(context, baseURL, language);
      await page.goto(seed.struct.path);
      await page.locator('#value-0').fill('not-a-number');
      const status = await submitAndSettle(page, () =>
        page.locator('#parameter-save-submit').click(),
      );
      expect(status).toBe(422);

      const link = page.locator('#error-panel a[href="#value-0"]');
      await expect(link).toBeVisible();
      await link.click();
      await expect(page.locator('#value-0')).toBeFocused();
    });

    test(`a link to the type input moves the focus to it in ${language}`, async ({
      page,
      context,
      baseURL,
      seed,
    }) => {
      await selectLanguage(context, baseURL, language);
      await page.goto(seed.newParameterPath);
      await page.locator('#tag').fill('BADTYPE');
      await page.locator('#name_en').fill('Bad type');
      await page.locator('#description_en').fill('Bad type');
      await page.locator('#type').fill('not a type');
      const status = await submitAndSettle(page, () =>
        page.locator('#parameter-show-fields').click(),
      );
      expect(status).toBeGreaterThanOrEqual(400);

      const link = page.locator('#error-panel a[href="#type"]');
      await expect(link).toBeVisible();
      await link.click();
      await expect(page.locator('#type')).toBeFocused();
    });
  }
});

test.describe('a long word in the error summary', () => {
  for (const language of ['pl', 'en'] as const satisfies readonly Language[]) {
    test(`does not widen the page at 320px in ${language}`, async ({
      page,
      context,
      baseURL,
      seed,
    }) => {
      await selectLanguage(context, baseURL, language);
      await page.setViewportSize({ width: 320, height: VIEWPORT_HEIGHT });
      await page.goto(seed.newParameterPath);
      await page.locator('#tag').fill('LONGWORD');
      await page.locator('#name_en').fill('Long word');
      await page.locator('#description_en').fill('Long word');
      await page.locator('#type').fill(LONG_WORD);
      const status = await submitAndSettle(page, () =>
        page.locator('#parameter-show-fields').click(),
      );
      expect(status).toBeGreaterThanOrEqual(400);

      const summaryLink = page.locator('#error-panel a[href="#type"]');
      await expect(summaryLink).toContainText(LONG_WORD);

      const widths = await page.evaluate(() => {
        const root = document.documentElement;
        const link = document.querySelector('#error-panel a[href="#type"]');
        return {
          scroll: root.scrollWidth,
          client: root.clientWidth,
          linkRight: link ? link.getBoundingClientRect().right : Number.POSITIVE_INFINITY,
        };
      });
      expect(widths.scroll, 'the page has no sideways scroll').toBeLessThanOrEqual(widths.client);
      expect(widths.linkRight, 'the summary link ends inside the view').toBeLessThanOrEqual(
        widths.client,
      );
    });
  }
});
