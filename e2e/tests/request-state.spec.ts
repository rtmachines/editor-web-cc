import { expect, expectSwapSettled, test } from './support/seed';
import type { Page, Route } from '@playwright/test';

// ADR-0158 (b), (d) and design D1 of the change ui-request-state: the lock of a form while its
// request is in flight, and the unlock by the XMLHttpRequest of the answer.
// Scenarios of specs/web-ui/spec.md that this file covers:
// - "Typing during a save that answers 200"
// - "Typing during a save that answers 422"
// The translation row tests cover the same two scenarios for a row form that HTMX swaps by itself.
// The scenario "Readonly of the page stays" needs an input that the page made readonly. The edit
// form of a parameter has none, so the test of task 4.6 covers it with `page.evaluate`.

/** The edit form of a parameter. A save posts it to `.../definition`. */
const FORM = '#parameter-edit-form';
/** The save button of the edit form. */
const SAVE_BUTTON = '#parameter-save-submit';
/** The first value input of the seeded struct parameter. */
const VALUE_INPUT = '#value-0';
/** The text that the server accepts for the value input. */
const ACCEPTED_TEXT = '2.25';
/** The text that the server refuses with a field error on the value input. */
const REFUSED_TEXT = 'not-a-number';
/** The text that a user types while the form is locked. It never reaches the input. */
const TYPED_WHILE_BUSY = '999';
/** The text that a user types after the unlock. */
const TYPED_AFTER_UNLOCK = '3.5';

/** The language tag of the translation row under test. The dev server offers `de`. */
const LANG = 'de';
/** The form of the row `de`, its name input, description textarea, save button, and delete form. */
const ROW_FORM = `#translation-${LANG}-form`;
const ROW_NAME = `#translation-${LANG}-name`;
const ROW_DESCRIPTION = `#translation-${LANG}-description`;
const ROW_SAVE = `#translation-${LANG}-save`;
const ROW_DELETE_FORM = `#translation-${LANG} details.danger form`;
/** The text that the row saves before the request. */
const SENT_TEXT = 'Sent before request';
/** The text that a user types while the row is locked. It never reaches the input. */
const TYPED_WHILE_SAVING = 'Typed while saving';
/** A blank name. It passes the `required` check of the browser. The server refuses it as empty. */
const BLANK_NAME = '   ';

/**
 * Holds each POST whose URL matches the pattern until the test calls `release`.
 * Then it answers with `answer`, or lets the request through by default.
 */
async function holdRequests(
  page: Page,
  pattern: string,
  answer: (route: Route) => Promise<void> = (route) => route.continue(),
): Promise<{ release: () => void }> {
  let release: () => void = () => undefined;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route(pattern, async (route) => {
    if (route.request().method() !== 'POST') {
      await route.continue();
      return;
    }
    await gate;
    await answer(route);
  });
  return { release };
}

/** Holds each save request of the parameter form until the test calls `release`. */
function holdSaveRequests(page: Page): Promise<{ release: () => void }> {
  return holdRequests(page, '**/definition');
}

/** Holds each request of a translation row until the test calls `release`. */
function holdTranslationRequests(page: Page): Promise<{ release: () => void }> {
  return holdRequests(page, '**/translations/**');
}

test.describe('lock of the parameter form during a save', () => {
  test('locks the inputs while a save that answers 200 is in flight', async ({ page, seed }) => {
    await page.goto(seed.struct.path);
    const hold = await holdSaveRequests(page);
    const form = page.locator(FORM);
    const input = page.locator(VALUE_INPUT);
    await input.fill(ACCEPTED_TEXT);
    await input.focus();
    await input.press('Enter');

    await expect(form).toHaveAttribute('aria-busy', 'true');
    await expect(input).toHaveAttribute('readonly', '');
    await input.pressSequentially(TYPED_WHILE_BUSY);
    await expect(input).toHaveValue(ACCEPTED_TEXT);
    await expect(input).toBeFocused();

    hold.release();
    await expect(form).not.toHaveAttribute('aria-busy', /.*/);
    await expectSwapSettled(page);
    await expect(page.locator('#toast .toast')).toBeVisible();
    await expect(input).not.toHaveAttribute('readonly', /.*/);
    await expect(input).not.toHaveAttribute('data-request-lock', /.*/);
    await expect(input).toHaveValue(ACCEPTED_TEXT);

    await input.fill(TYPED_AFTER_UNLOCK);
    await expect(input).toHaveValue(TYPED_AFTER_UNLOCK);
  });

  // The scenario "Enter and click start the same lock": the other tests send by Enter.
  test('locks the inputs while a save by a click on the save button is in flight', async ({
    page,
    seed,
  }) => {
    await page.goto(seed.struct.path);
    const hold = await holdSaveRequests(page);
    const form = page.locator(FORM);
    const input = page.locator(VALUE_INPUT);
    await input.fill(ACCEPTED_TEXT);
    await page.locator(SAVE_BUTTON).click();

    await expect(form).toHaveAttribute('aria-busy', 'true');
    await expect(input).toHaveAttribute('readonly', '');
    await expect(input).toHaveAttribute('data-request-lock', '');

    hold.release();
    await expect(form).not.toHaveAttribute('aria-busy', /.*/);
    await expectSwapSettled(page);
    await expect(page.locator('#toast .toast')).toBeVisible();
    await expect(input).not.toHaveAttribute('readonly', /.*/);
    await expect(input).not.toHaveAttribute('data-request-lock', /.*/);
    await expect(input).toHaveValue(ACCEPTED_TEXT);
  });

  test('locks the inputs while a save that answers 422 is in flight', async ({ page, seed }) => {
    await page.goto(seed.struct.path);
    const hold = await holdSaveRequests(page);
    const form = page.locator(FORM);
    const input = page.locator(VALUE_INPUT);
    await input.fill(REFUSED_TEXT);
    await input.focus();
    const answer = page.waitForResponse((response) => response.request().method() === 'POST');
    await input.press('Enter');

    await expect(form).toHaveAttribute('aria-busy', 'true');
    await expect(input).toHaveAttribute('readonly', '');
    await input.pressSequentially(TYPED_WHILE_BUSY);
    await expect(input).toHaveValue(REFUSED_TEXT);
    await expect(input).toBeFocused();

    hold.release();
    expect((await answer).status()).toBe(422);
    await expect(form).not.toHaveAttribute('aria-busy', /.*/);
    await expectSwapSettled(page);
    await expect(page.locator('#error-panel .alert-error')).toBeVisible();
    await expect(input).toHaveAttribute('aria-invalid', 'true');
    await expect(input).not.toHaveAttribute('readonly', /.*/);
    await expect(input).toHaveValue(REFUSED_TEXT);

    await input.fill(TYPED_AFTER_UNLOCK);
    await expect(input).toHaveValue(TYPED_AFTER_UNLOCK);
  });

  // `htmx:abort` makes HTMX call `xhr.abort()`. The XMLHttpRequest fires `abort`, not `error`.
  // The vendored HTMX then fires `htmx:afterRequest` and no `htmx:sendError`, so the listener of
  // `htmx:afterRequest` unlocks and the lost-answer message stays hidden.
  test('unlocks the form after a save that the script aborts', async ({ page, seed }) => {
    await page.goto(seed.struct.path);
    const hold = await holdSaveRequests(page);
    const form = page.locator(FORM);
    const input = page.locator(VALUE_INPUT);
    await input.fill(ACCEPTED_TEXT);
    await input.focus();
    await input.press('Enter');

    await expect(form).toHaveAttribute('aria-busy', 'true');
    await expect(input).toHaveAttribute('readonly', '');
    await form.evaluate((element) => {
      const { htmx } = window as unknown as { htmx: { trigger: (el: Element, name: string) => void } };
      htmx.trigger(element, 'htmx:abort');
    });
    await expect(form).not.toHaveAttribute('aria-busy', /.*/);
    await expect(input).not.toHaveAttribute('readonly', /.*/);
    await expect(input).not.toHaveAttribute('data-request-lock', /.*/);
    await expect(input).toHaveValue(ACCEPTED_TEXT);
    await expect(input).toBeFocused();
    await expect(page.locator('#error-panel [data-transport-error]')).toHaveCount(0);

    await input.fill(TYPED_AFTER_UNLOCK);
    await expect(input).toHaveValue(TYPED_AFTER_UNLOCK);
    hold.release();
  });

  // The header `HX-Retarget: #missing` makes `handleAjaxResponse` of the vendored HTMX 2 fire
  // `htmx:targetError` and throw. The `onload` handler catches the throw, fires
  // `htmx:onLoadError`, and rethrows. It never reaches `htmx:afterRequest`, so only the listener
  // of `htmx:onLoadError` unlocks.
  test('unlocks the form after an answer that HTMX cannot place', async ({ page, seed }) => {
    await page.goto(seed.struct.path);
    const hold = await holdRequests(page, '**/definition', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'text/html',
        headers: { 'HX-Retarget': '#missing' },
        body: '<p>Unplaceable</p>',
      }),
    );
    const form = page.locator(FORM);
    const input = page.locator(VALUE_INPUT);
    await input.fill(ACCEPTED_TEXT);
    await input.focus();
    await input.press('Enter');

    await expect(form).toHaveAttribute('aria-busy', 'true');
    await expect(input).toHaveAttribute('readonly', '');
    hold.release();
    await expect(form).not.toHaveAttribute('aria-busy', /.*/);
    await expect(input).not.toHaveAttribute('readonly', /.*/);
    await expect(input).not.toHaveAttribute('data-request-lock', /.*/);
    await input.fill(TYPED_AFTER_UNLOCK);
    await expect(input).toHaveValue(TYPED_AFTER_UNLOCK);
  });
});

/** The two ways to send the row form: Enter in the text input and a click on the save button. */
const TRIGGERS = {
  Enter: async (page: Page) => {
    const input = page.locator(ROW_NAME);
    await input.focus();
    await input.press('Enter');
  },
  click: async (page: Page) => {
    await page.locator(ROW_SAVE).click();
  },
};

test.describe('lock of a translation row during a save', () => {
  for (const [trigger, send] of Object.entries(TRIGGERS)) {
    test(`locks the row while a save by ${trigger} that answers 200 is in flight`, async ({
      page,
      seed,
    }) => {
      await page.goto(seed.primitive.path);
      const hold = await holdTranslationRequests(page);
      const form = page.locator(ROW_FORM);
      const input = page.locator(ROW_NAME);
      await input.fill(SENT_TEXT);
      await send(page);

      await expect(form).toHaveAttribute('aria-busy', 'true');
      await expect(input).toHaveAttribute('readonly', '');
      await input.focus();
      await input.pressSequentially(TYPED_WHILE_SAVING);
      await expect(input).toHaveValue(SENT_TEXT);
      await expect(input).toBeFocused();

      hold.release();
      await expect(page.locator(ROW_FORM)).not.toHaveAttribute('aria-busy', /.*/);
      await expectSwapSettled(page);
      await expect(page.locator('#toast .toast')).toBeVisible();
      const saved = page.locator(ROW_NAME);
      await expect(saved).not.toHaveAttribute('readonly', /.*/);
      await expect(saved).not.toHaveAttribute('data-request-lock', /.*/);
      await expect(saved).toHaveValue(SENT_TEXT);

      await saved.fill(TYPED_AFTER_UNLOCK);
      await expect(saved).toHaveValue(TYPED_AFTER_UNLOCK);
    });
  }

  test('locks the row while a save that answers 422 is in flight', async ({ page, seed }) => {
    await page.goto(seed.primitive.path);
    const hold = await holdTranslationRequests(page);
    const form = page.locator(ROW_FORM);
    const name = page.locator(ROW_NAME);
    const description = page.locator(ROW_DESCRIPTION);
    await name.fill(BLANK_NAME);
    await description.fill(SENT_TEXT);
    const answer = page.waitForResponse((response) => response.request().method() === 'POST');
    await page.locator(ROW_SAVE).click();

    await expect(form).toHaveAttribute('aria-busy', 'true');
    await expect(description).toHaveAttribute('readonly', '');
    await description.focus();
    await description.pressSequentially(TYPED_WHILE_SAVING);
    await expect(description).toHaveValue(SENT_TEXT);
    await expect(description).toBeFocused();

    hold.release();
    expect((await answer).status()).toBe(422);
    await expect(page.locator(ROW_FORM)).not.toHaveAttribute('aria-busy', /.*/);
    await expectSwapSettled(page);
    await expect(page.locator('#error-panel .alert-error')).toBeVisible();
    const refusedName = page.locator(ROW_NAME);
    await expect(refusedName).toHaveAttribute('aria-invalid', 'true');
    await expect(refusedName).not.toHaveAttribute('readonly', /.*/);
    const refused = page.locator(ROW_DESCRIPTION);
    await expect(refused).not.toHaveAttribute('readonly', /.*/);
    await expect(refused).not.toHaveAttribute('data-request-lock', /.*/);
    await expect(refused).toHaveValue(SENT_TEXT);

    await refused.fill(TYPED_AFTER_UNLOCK);
    await expect(refused).toHaveValue(TYPED_AFTER_UNLOCK);
  });

  test('locks only the delete form while a removal is in flight', async ({ page, seed }) => {
    await page.goto(seed.primitive.path);
    await page.locator(ROW_NAME).fill(SENT_TEXT);
    const saved = page.waitForResponse((response) => response.request().method() === 'POST');
    await page.locator(ROW_SAVE).click();
    expect((await saved).status()).toBe(200);
    await expectSwapSettled(page);

    const hold = await holdTranslationRequests(page);
    await page.locator(`#translation-${LANG} details.danger > summary`).click();
    await page.locator(`#translation-${LANG}-delete`).click();

    await expect(page.locator(ROW_DELETE_FORM)).toHaveAttribute('aria-busy', 'true');
    await expect(page.locator(ROW_FORM)).not.toHaveAttribute('aria-busy', /.*/);
    await expect(page.locator(ROW_NAME)).not.toHaveAttribute('readonly', /.*/);

    hold.release();
    await expect(page.locator(ROW_DELETE_FORM)).toHaveCount(0);
    await expectSwapSettled(page);
    await expect(page.locator(ROW_NAME)).toHaveValue('');
  });
});

// Scenarios of the requirement "Message of a lost answer" in specs/web-ui/spec.md, design D1, and
// ADR-0158 (e), (f), (h). The vendored HTMX fires `htmx:afterRequest` and then `htmx:sendError`
// after a network error, and `htmx:afterRequest` and then `htmx:timeout` after a timeout.
/** The message of a lost answer inside the error panel. */
const LOST_MESSAGE = '#error-panel [data-transport-error]';
/** The code of the message of a lost answer. */
const LOST_CODE = 'E-WEB-015';
/** The timeout in milliseconds that a test sets for the requests of HTMX. */
const SHORT_TIMEOUT_MS = 300;
/** The time in milliseconds that a held request waits, longer than the short timeout. */
const LONG_WAIT_MS = 1_500;

/** Fills the value input with the text and sends the form by Enter. */
async function sendSave(page: Page, text: string): Promise<void> {
  const input = page.locator(VALUE_INPUT);
  await input.fill(text);
  await input.focus();
  await input.press('Enter');
}

/** Expects exactly one lost-answer message with the code, the alert role, and no reference. */
async function expectLostMessage(page: Page): Promise<void> {
  const message = page.locator(LOST_MESSAGE);
  await expect(message).toHaveCount(1);
  await expect(message.locator('[role="alert"]')).toHaveCount(1);
  await expect(message).toContainText(LOST_CODE);
  await expect(message.locator('.alert-ref')).toHaveCount(0);
  await expect(page.locator('#error-panel')).toBeVisible();
}

test.describe('message of a lost answer', () => {
  test('shows the message after a network error before the send', async ({ page, seed }) => {
    await page.goto(seed.struct.path);
    const hold = await holdRequests(page, '**/definition', (route) =>
      route.abort('connectionfailed'),
    );
    await sendSave(page, ACCEPTED_TEXT);
    await expect(page.locator(FORM)).toHaveAttribute('aria-busy', 'true');
    hold.release();

    await expectLostMessage(page);
    await expect(page.locator(FORM)).not.toHaveAttribute('aria-busy', /.*/);
    await expect(page.locator(VALUE_INPUT)).not.toHaveAttribute('readonly', /.*/);
    await expect(page.locator(VALUE_INPUT)).toHaveValue(ACCEPTED_TEXT);
  });

  test('shows the message when the answer is lost after the server saved', async ({
    page,
    seed,
  }) => {
    await page.goto(seed.struct.path);
    const hold = await holdRequests(page, '**/definition', async (route) => {
      await route.fetch();
      await route.abort('connectionreset');
    });
    await sendSave(page, ACCEPTED_TEXT);
    hold.release();

    await expectLostMessage(page);
    await expect(page.locator(FORM)).not.toHaveAttribute('aria-busy', /.*/);
    await page.reload();
    await expect(page.locator(VALUE_INPUT)).toHaveValue(ACCEPTED_TEXT);
  });

  test('shows the message after a timeout', async ({ page, seed }) => {
    await page.goto(seed.struct.path);
    await page.evaluate((timeout) => {
      (window as unknown as { htmx: { config: { timeout: number } } }).htmx.config.timeout =
        timeout;
    }, SHORT_TIMEOUT_MS);
    await page.route('**/definition', async (route) => {
      if (route.request().method() !== 'POST') {
        await route.continue();
        return;
      }
      await new Promise((resolve) => setTimeout(resolve, LONG_WAIT_MS));
      await route.abort('timedout').catch(() => undefined);
    });
    await sendSave(page, ACCEPTED_TEXT);
    await expect(page.locator(FORM)).toHaveAttribute('aria-busy', 'true');

    await expectLostMessage(page);
    await expect(page.locator(FORM)).not.toHaveAttribute('aria-busy', /.*/);
    await expect(page.locator(VALUE_INPUT)).not.toHaveAttribute('readonly', /.*/);
  });

  test('empties the toast of an earlier success', async ({ page, seed }) => {
    await page.goto(seed.struct.path);
    await sendSave(page, ACCEPTED_TEXT);
    await expect(page.locator('#toast .toast')).toBeVisible();
    await expectSwapSettled(page);

    const hold = await holdRequests(page, '**/definition', (route) =>
      route.abort('connectionfailed'),
    );
    await sendSave(page, TYPED_AFTER_UNLOCK);
    hold.release();

    await expectLostMessage(page);
    await expect(page.locator('#toast .toast')).toHaveCount(0);
    await expect(page.locator('#toast')).toBeEmpty();
  });

  test('deletes the message when the next request starts', async ({ page, seed }) => {
    await page.goto(seed.struct.path);
    let attempts = 0;
    let release: () => void = () => undefined;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    await page.route('**/definition', async (route) => {
      if (route.request().method() !== 'POST') {
        await route.continue();
        return;
      }
      attempts += 1;
      if (attempts === 1) {
        await route.abort('connectionfailed');
        return;
      }
      await gate;
      await route.continue();
    });
    await sendSave(page, ACCEPTED_TEXT);
    await expectLostMessage(page);

    await sendSave(page, TYPED_AFTER_UNLOCK);
    await expect(page.locator(FORM)).toHaveAttribute('aria-busy', 'true');
    await expect(page.locator(LOST_MESSAGE)).toHaveCount(0);
    release();
    await expect(page.locator('#toast .toast')).toBeVisible();
    await expect(page.locator(LOST_MESSAGE)).toHaveCount(0);
  });

  test('scrolls the panel into view', async ({ page, seed }) => {
    await page.goto(seed.struct.path);
    const hold = await holdRequests(page, '**/definition', (route) =>
      route.abort('connectionfailed'),
    );
    await sendSave(page, ACCEPTED_TEXT);
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    hold.release();

    await expectLostMessage(page);
    await expect(page.locator('#error-panel')).toBeInViewport();
  });
});

// ADR-0158 (c) and design D1, last bullet: a busy form also freezes the controls that `readonly`
// cannot lock. Tab and Shift+Tab stay free. The parameter forms hold no check box and no radio
// button, so the test of those two injects them with `page.evaluate`.
/** The BOOL leaf of the seeded struct. Its value input is a `select` that starts at `true`. */
const BOOL_SELECT = `${FORM} select[name="value.enabled"]`;
/** The text leaf that follows the BOOL leaf in the Tab order. Its input is `readonly` while busy. */
const NEXT_INPUT = `${FORM} input[name="value.label"]`;
/** The ids of the check box and the radio button that the test adds to the form. */
const INJECTED_CHECK = 'injected-check';
const INJECTED_RADIO = 'injected-radio';

/** Holds the save, sends the form, and waits until the form is busy. */
async function holdBusySave(
  page: Page,
  answer?: (route: Route) => Promise<void>,
): Promise<{ release: () => void }> {
  const hold = await holdRequests(page, '**/definition', answer);
  await sendSave(page, ACCEPTED_TEXT);
  await expect(page.locator(FORM)).toHaveAttribute('aria-busy', 'true');
  return hold;
}

test.describe('controls that readonly cannot lock', () => {
  test('keeps the value of a select while the form is busy', async ({ page, seed }) => {
    await page.goto(seed.struct.path);
    const select = page.locator(BOOL_SELECT);
    await expect(select).toHaveValue('true');
    const hold = await holdBusySave(page);

    await select.focus();
    await select.press('ArrowDown');
    await expect(select).toHaveValue('true');
    await select.click();
    await select.press('ArrowDown');
    await select.press('Enter');
    await expect(select).toHaveValue('true');
    await expect(page.locator(FORM)).toHaveAttribute('aria-busy', 'true');

    hold.release();
    await expect(page.locator(FORM)).not.toHaveAttribute('aria-busy', /.*/);
    await expectSwapSettled(page);
    const saved = page.locator(BOOL_SELECT);
    await saved.focus();
    await saved.press('ArrowDown');
    await expect(saved).toHaveValue('false');
  });

  test('keeps a check box and a radio button while the form is busy', async ({ page, seed }) => {
    await page.goto(seed.struct.path);
    await page.evaluate(
      ([form, check, radio]) => {
        const target = document.querySelector(form);
        for (const [id, type] of [
          [check, 'checkbox'],
          [radio, 'radio'],
        ]) {
          const control = document.createElement('input');
          control.id = id;
          control.type = type;
          control.name = id;
          target?.append(control);
        }
      },
      [FORM, INJECTED_CHECK, INJECTED_RADIO],
    );
    const hold = await holdBusySave(page);

    for (const id of [INJECTED_CHECK, INJECTED_RADIO]) {
      const control = page.locator(`#${id}`);
      await control.dispatchEvent('click');
      await control.focus();
      await control.press('Space');
      await control.click({ force: true });
      await expect(control).not.toBeChecked();
    }
    hold.release();
    await expect(page.locator(FORM)).not.toHaveAttribute('aria-busy', /.*/);
  });

  test('moves the focus by Tab and Shift+Tab while the form is busy', async ({ page, seed }) => {
    await page.goto(seed.struct.path);
    const hold = await holdBusySave(page);
    const select = page.locator(BOOL_SELECT);

    await select.focus();
    await page.keyboard.press('Tab');
    await expect(page.locator(NEXT_INPUT)).toBeFocused();
    await page.keyboard.press('Shift+Tab');
    await expect(select).toBeFocused();
    await expect(page.locator(FORM)).toHaveAttribute('aria-busy', 'true');
    hold.release();
  });

  test('lets the shortcuts of the browser through while the form is busy', async ({ page, seed }) => {
    await page.goto(seed.struct.path);
    const hold = await holdBusySave(page);
    const select = page.locator(BOOL_SELECT);
    const wasPrevented = (init: KeyboardEventInit) =>
      select.evaluate(
        (element, eventInit) =>
          !element.dispatchEvent(
            new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...eventInit }),
          ),
        init,
      );

    expect(await wasPrevented({ key: 'ArrowDown' })).toBe(true);
    expect(await wasPrevented({ key: 'F5' })).toBe(false);
    expect(await wasPrevented({ key: 'F12' })).toBe(false);
    expect(await wasPrevented({ key: 'r', ctrlKey: true })).toBe(false);
    expect(await wasPrevented({ key: 'ArrowLeft', altKey: true })).toBe(false);
    expect(await wasPrevented({ key: 'r', metaKey: true })).toBe(false);
    expect(await wasPrevented({ key: 'F', shiftKey: true })).toBe(true);
    hold.release();
    await expect(page.locator(FORM)).not.toHaveAttribute('aria-busy', /.*/);
  });

  test('leaves an input that the page made readonly as it was', async ({ page, seed }) => {
    await page.goto(seed.struct.path);
    await page.locator(NEXT_INPUT).evaluate((input: HTMLInputElement) => {
      input.readOnly = true;
    });
    // A network error swaps nothing, so the same inputs stay in the page after the unlock.
    const hold = await holdBusySave(page, (route) => route.abort('connectionfailed'));
    await expect(page.locator(NEXT_INPUT)).toHaveAttribute('readonly', '');
    await expect(page.locator(NEXT_INPUT)).not.toHaveAttribute('data-request-lock', /.*/);

    hold.release();
    await expectLostMessage(page);
    await expect(page.locator(FORM)).not.toHaveAttribute('aria-busy', /.*/);
    await expect(page.locator(VALUE_INPUT)).not.toHaveAttribute('readonly', /.*/);
    await expect(page.locator(NEXT_INPUT)).toHaveAttribute('readonly', '');
    await expect(page.locator(`${FORM} [data-request-lock]`)).toHaveCount(0);
    await expect(page.locator(`${FORM} input[readonly], ${FORM} textarea[readonly]`)).toHaveCount(1);
  });
});

/** The filter form of the audit list. Its link "Clear" stands inside the form (ADR-0158 (b)). */
const AUDIT_FORM = 'form[action="/audit"]';
const AUDIT_FILTER_INPUT = `${AUDIT_FORM} input[name="actor"]`;
const AUDIT_SUBMIT = `${AUDIT_FORM} button[type="submit"]`;
const AUDIT_CLEAR = `${AUDIT_FORM} a.target-link`;

test.describe('count of the open requests of a form', () => {
  test('unlocks the form only when its last open request ends', async ({ page }) => {
    await page.goto('/audit');
    // The route holds each boosted request. The answer 204 swaps nothing, so the form stays.
    const held: Route[] = [];
    await page.route('**/audit*', async (route) => {
      if (route.request().headers()['hx-request'] !== 'true') {
        await route.continue();
        return;
      }
      held.push(route);
    });
    const form = page.locator(AUDIT_FORM);
    const input = page.locator(AUDIT_FILTER_INPUT);

    await page.locator(AUDIT_SUBMIT).click();
    await page.locator(AUDIT_CLEAR).click();
    await expect.poll(() => held.length).toBe(2);
    await expect(form).toHaveAttribute('aria-busy', 'true');
    await expect(input).toHaveAttribute('readonly', '');

    await held[0].fulfill({ status: 204 });
    await expect(form).toHaveAttribute('aria-busy', 'true');
    await expect(input).toHaveAttribute('readonly', '');
    await expect(input).toHaveAttribute('data-request-lock', '');

    await held[1].fulfill({ status: 204 });
    await expect(form).not.toHaveAttribute('aria-busy', /.*/);
    await expect(input).not.toHaveAttribute('readonly', /.*/);
    await expect(input).not.toHaveAttribute('data-request-lock', /.*/);
  });
});
