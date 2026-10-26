import {
  expect,
  expectSwapSettled,
  GRID_CONTROLS,
  selectLanguage,
  test,
  type Language,
} from './support/seed';

// ADR-0048 (c) and (d): the keyboard path through the leaf grid and the Enter key.
// A test reads the page and submits the product's own forms. It never changes a style or a script.

/** The widths of ADR-0048 (d), in CSS px. */
const WIDTHS = [320, 768, 1280] as const;
/** The height is large enough for the top bar and a few rows. The grid scrolls inside its frame. */
const VIEWPORT_HEIGHT = 800;
/** The most Tab presses for one run. The page has fewer than 80 focusable elements. */
const MAX_TAB_PRESSES = 120;

/** Which seeded parameter a test opens. */
type Subject = 'struct' | 'array';

/** Returns the id of the focused grid control, or null when the focus is outside the grid. */
function focusedGridControlId(): string | null {
  const active = document.activeElement;
  return active?.closest('.leaf-grid') ? active.id : null;
}

test.describe('keyboard path through the leaf grid', () => {
  const cases: ReadonlyArray<{ subject: Subject; width: number; language: Language }> = [
    ...(['struct', 'array'] as const).flatMap((subject) =>
      WIDTHS.map((width) => ({ subject, width, language: 'pl' as const })),
    ),
    { subject: 'struct', width: 320, language: 'en' },
  ];

  for (const { subject, width, language } of cases) {
    test(`Tab reaches each control of the ${subject} grid at ${width}px in ${language}`, async ({
      page,
      context,
      baseURL,
      seed,
    }) => {
      await selectLanguage(context, baseURL, language);
      await page.setViewportSize({ width, height: VIEWPORT_HEIGHT });
      await page.goto(seed[subject].path);

      const expected = await page
        .locator(GRID_CONTROLS)
        .evaluateAll((controls) => controls.map((control) => control.id));
      expect(expected.length, 'the grid has controls').toBeGreaterThan(5);
      expect(expected.every((id) => id.length > 0), 'each control has an id').toBe(true);

      // Start on the last field before the grid, then press Tab until the last grid control.
      await page.locator('#ordinal').focus();
      const visited: string[] = [];
      for (let press = 0; press < MAX_TAB_PRESSES; press += 1) {
        await page.keyboard.press('Tab');
        const id = await page.evaluate(focusedGridControlId);
        if (id === null) {
          continue;
        }
        visited.push(id);
        // The focused control must be at least partly inside the viewport.
        await expect(page.locator(`[id="${id}"]`)).toBeInViewport();
        if (visited.length === expected.length) {
          break;
        }
      }
      expect(visited, 'Tab visits the controls in DOM order').toEqual(expected);
    });
  }
});

test.describe('Enter in a value input saves the value', () => {
  /** Reads the form fields of a captured request body. */
  function fieldsOf(body: string | null): URLSearchParams {
    return new URLSearchParams(body ?? '');
  }

  const cases: ReadonlyArray<{
    subject: Subject;
    input: string;
    name: string;
    text: string;
    language: Language;
  }> = [
    { subject: 'struct', input: '#value-0', name: 'value.setpoint', text: '2.25', language: 'pl' },
    { subject: 'struct', input: '#value-0', name: 'value.setpoint', text: '3.5', language: 'en' },
    // The grid of an array has a button with the name `limit_mode`. Enter must not press it.
    { subject: 'array', input: '#value-0', name: 'value[1]', text: '4.5', language: 'pl' },
    { subject: 'array', input: '#limits-0-min', name: 'limits[0].min', text: '1', language: 'pl' },
  ];

  for (const { subject, input, name, text, language } of cases) {
    test(`Enter in ${input} of the ${subject} grid saves it in ${language}`, async ({
      page,
      context,
      baseURL,
      seed,
    }) => {
      await selectLanguage(context, baseURL, language);
      await page.goto(seed[subject].path);

      const control = page.locator(input);
      await control.fill(text);
      const requestPromise = page.waitForRequest(
        (request) => request.method() === 'POST' && request.url().endsWith('/definition'),
      );
      await control.press('Enter');
      const request = await requestPromise;

      const fields = fieldsOf(request.postData());
      expect(fields.has('fields'), 'the request sends no "fields"').toBe(false);
      expect(fields.has('limit_mode'), 'the request sends no "limit_mode"').toBe(false);
      expect(fields.get(name), 'the request sends the typed text').toBe(text);
      expect(fields.get('row_version'), 'the request sends row_version').toMatch(/^\d+$/);

      // The answer shows the success toast and no error. A new load shows the saved text.
      await expect(page.locator('#toast .toast')).toBeVisible();
      await expect(page.locator('#error-panel .alert-error')).toHaveCount(0);
      await expectSwapSettled(page);
      await page.reload();
      await expect(page.locator(input)).toHaveValue(text);
    });
  }
});
