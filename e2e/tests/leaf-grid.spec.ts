import {
  expect,
  expectSwapSettled,
  GRID_CONTROLS,
  selectLanguage,
  test,
  type Language,
} from './support/seed';
import type { Page } from '@playwright/test';

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

// Design D7: a plain link after the grid brings the save button back into view at 320 px.
// Design D13: no element of the link or the grid has a delayed trigger.
test.describe('jump link after the leaf grid', () => {
  /** The URL of the sentinel request. The page sends it after the jump, so a stray request is already in the list. */
  const SENTINEL_PATH = '/static/app.css';
  /** Waits for the link only briefly, so a missing link fails fast. */
  const LINK_TIMEOUT_MS = 2000;
  /** The fake time between the two sentinels. It is longer than the HTMX timeout of 45000 ms. */
  const CLOCK_ADVANCE_MS = 60_000;
  /** Matches an `hx-trigger` with `delay:`, `throttle:`, or `every`. Each delays a request. */
  const DELAYED_TRIGGER = String.raw`delay:|throttle:|\bevery\b`;
  /** Selects the jump link, the grid and its frames, and each element inside the grid. */
  const TRIGGER_SCOPE =
    'a[href="#parameter-save-submit"], .tree:has(.leaf-grid), .table-wrap:has(.leaf-grid), ' +
    '.leaf-grid, .leaf-grid *';

  /** Returns one line for each scoped element whose `hx-trigger` starts a delayed request. */
  async function delayedTriggers(page: Page): Promise<string[]> {
    return page.locator(TRIGGER_SCOPE).evaluateAll((elements, pattern) => {
      const delayed = new RegExp(pattern);
      return elements.flatMap((element) => {
        const trigger = element.getAttribute('hx-trigger');
        if (trigger === null || !delayed.test(trigger)) {
          return [];
        }
        const name = `${element.tagName.toLowerCase()}${element.id ? `#${element.id}` : ''}`;
        return [`${name} hx-trigger="${trigger}"`];
      });
    }, DELAYED_TRIGGER);
  }

  /** Sends the sentinel fetch. It ends after any earlier request started. */
  async function sendSentinel(page: Page): Promise<void> {
    // `no-store` keeps the cache from hiding the sentinel.
    await page.evaluate(async (path) => {
      await fetch(path, { cache: 'no-store' });
    }, SENTINEL_PATH);
  }

  for (const subject of ['struct', 'array'] as const) {
    test(`the link after the ${subject} grid shows the save button at 320px without a request`, async ({
      page,
      seed,
    }) => {
      await page.clock.install();
      await page.setViewportSize({ width: 320, height: VIEWPORT_HEIGHT });
      await page.goto(seed[subject].path);

      const link = page.locator('a[href="#parameter-save-submit"]');
      await expect(link, 'the page has the jump link after the grid').toHaveCount(1, {
        timeout: LINK_TIMEOUT_MS,
      });
      await expect(page.locator('.leaf-grid'), 'the page has the grid').toHaveCount(1);
      expect(await delayedTriggers(page), 'no scoped element has a delayed hx-trigger').toEqual([]);
      await link.scrollIntoViewIfNeeded();
      const save = page.locator('#parameter-save-submit');
      await expect(save, 'the save button is above the grid, out of view').not.toBeInViewport();

      const requests: string[] = [];
      page.on('request', (request) => requests.push(`${request.method()} ${request.url()}`));
      await link.click();
      await expect(save, 'the link brings the save button into view').toBeInViewport();
      // The fake clock stops `page.waitForFunction`, so this poll runs on the test side.
      await expect
        .poll(() => page.evaluate(() => window.location.hash), { message: 'the hash is set' })
        .toBe('#parameter-save-submit');
      await sendSentinel(page);
      // A delayed request fires in the fake time and starts before the second sentinel.
      await page.clock.runFor(CLOCK_ADVANCE_MS);
      await sendSentinel(page);
      expect(
        requests.map((request) => request.replace(/^GET https?:\/\/[^/]+/, 'GET ')),
        'the link sends no request, only the two sentinels',
      ).toEqual([`GET ${SENTINEL_PATH}`, `GET ${SENTINEL_PATH}`]);
    });
  }
});

test.describe('mode rows of a nested array', () => {
  /** The type levels of `ARRAY[1..2] OF ARRAY[1..3] OF INT`: the outer array and its elements. */
  const TYPE_LEVELS = 2;
  /** The leaves of `NEST`: two arrays of three elements. */
  const LEAVES = 6;

  test('the grid of NEST has one mode button for each type level', async ({ page, seed }) => {
    await page.goto(seed.nest.path);
    await expect(
      page.locator('.leaf-grid input[name^="value["]'),
      'the grid has one value input for each leaf',
    ).toHaveCount(LEAVES);
    await expect(
      page.locator('.leaf-grid button[name="limit_mode"]'),
      'the grid has one mode button for each type level, not for each array level',
    ).toHaveCount(TYPE_LEVELS);
  });
});
