import { expect, test, type BrowserContext, type Locator, type Page, type Request } from '@playwright/test';
import { adminCookies, seedSweepData } from './support/pages';

// ADR-0037 (h): Enter in the form of a new type or of a new revision presses the
// create button. The button "Add a member" comes second, so Enter never sends `add=1`. ADR-0159: a
// revision with the members of the latest revision gets `409 E-PLT-052`. The machine is `M7` of the
// seed step: an S7_300 machine, so a member of the type `INT` or `DINT` is valid.

/** The seed step hashes passwords, so a debug build of the dev server needs time. */
const SEED_TIMEOUT_MS = 180_000;
/** The time limit of a test with one member row. */
const SMALL_TEST_TIMEOUT_MS = 60_000;
/** The time limit of a test with a large form. The dev server is a debug build. */
const LARGE_TEST_TIMEOUT_MS = 240_000;
/** How long the define answer and the type page may take at 1024 rows. */
const DEFINE_TIMEOUT_MS = 90_000;
/** The machine that the seed step makes. */
const MACHINE = 'M7';
/** The type of every member row. */
const MEMBER_TYPE = 'INT';
/** The type that a revision test gives to one row, so the revision differs from the latest one. */
const CHANGED_MEMBER_TYPE = 'DINT';
/** The text of `E-PLT-052` for the revision 1 of a type. The text comes from `reliability.md` section 1.2. */
const UNCHANGED_REVISION_TEXT = (typeName: string): string =>
  `Revision 1 of type ${typeName} has the same members. Change a member before you create the revision.`;
/** The value of `type.max_members` in `reliability.md` section 7.11. */
const MAX_MEMBERS = 1024;
/** The row counts of the scenario: the smallest form, one row before the limit, and the limit. */
const ROW_COUNTS: readonly number[] = [1, MAX_MEMBERS - 1, MAX_MEMBERS];

/** The key that a test presses: Enter in the name input or Enter in the type input of a row. */
type EnterIn = 'type name' | 'member type';

test.describe.configure({ mode: 'default' });

test.beforeAll(async ({ browser }, testInfo) => {
  test.setTimeout(SEED_TIMEOUT_MS);
  const baseURL = testInfo.project.use.baseURL;
  const password = process.env.E2E_PASSWORD;
  if (!baseURL || !password) {
    throw new Error('The UDT form specs need the baseURL of the config and E2E_PASSWORD.');
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

let typeCounter = 0;

/** Returns a type name that no earlier test of this run used. */
function uniqueTypeName(): string {
  typeCounter += 1;
  return `ST_${Date.now().toString(36)}_${typeCounter}`;
}

/** Returns the name of the member row `index`. The name is valid and unique in the form. */
function memberName(index: number): string {
  return `m${String(index).padStart(4, '0')}`;
}

/** Returns the form fields of `filled` rows, as the browser sends them. */
function filledRows(filled: number): Record<string, string> {
  const fields: Record<string, string> = {};
  for (let index = 0; index < filled; index += 1) {
    fields[`members[${index}].name`] = memberName(index);
    fields[`members[${index}].type`] = MEMBER_TYPE;
  }
  return fields;
}

/**
 * Loads the form of a new type with `rows` member rows and `rows - 1` of them filled.
 *
 * One request `POST ... add=1` with `rows - 1` filled rows returns the page that the server renders:
 * the sent rows and one more empty row. A route serves that page for the next `GET` of the form,
 * so the browser loads the large form from the real server, with the real scripts and the real
 * HTMX attributes. The route ends before the test presses a key, so the key press reaches the server.
 */
async function openFormWithRows(page: Page, typeName: string, rows: number): Promise<void> {
  const formPath = `/machines/${MACHINE}/types/new`;
  await page.goto(formPath);
  if (rows > 1) {
    const csrfToken = await page.locator('#udt-form input[name="csrf_token"]').inputValue();
    const answer = await page.request.post(`/machines/${MACHINE}/types`, {
      headers: { 'Sec-Fetch-Site': 'same-origin' },
      form: { csrf_token: csrfToken, name: typeName, add: '1', ...filledRows(rows - 1) },
    });
    expect(answer.status()).toBe(200);
    const html = await answer.text();
    await page.route(`**${formPath}`, (route) =>
      route.request().method() === 'GET'
        ? route.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: html })
        : route.fallback(),
    );
    await page.goto(formPath);
    await page.unroute(`**${formPath}`);
  }
  await expect(page.locator('fieldset.member-row')).toHaveCount(rows);
}

/** Fills the name of the type and the last member row, so that every one of the `rows` rows is filled. */
async function fillRemainingInputs(page: Page, typeName: string, rows: number): Promise<void> {
  const last = rows - 1;
  await page.locator('#type_name').fill(typeName);
  await page.locator(`#members-${last}-name`).fill(memberName(last));
  await page.locator(`#members-${last}-type`).fill(MEMBER_TYPE);
}

/** Collects the `POST` requests of the page from now on. */
function collectPosts(page: Page): Request[] {
  const posts: Request[] = [];
  page.on('request', (request) => {
    if (request.method() === 'POST') {
      posts.push(request);
    }
  });
  return posts;
}

/** Returns the input that the test presses Enter in. */
function enterTarget(page: Page, enterIn: EnterIn, rows: number): Locator {
  return enterIn === 'type name'
    ? page.locator('#type_name')
    : page.locator(`#members-${Math.floor(rows / 2)}-type`);
}

/**
 * Presses Enter in the form of a new type with `rows` filled member rows.
 * It asserts exactly one `POST` without the field `add` and the type page with `rows` members.
 */
async function defineByEnter(page: Page, enterIn: EnterIn, rows: number): Promise<void> {
  const typeName = uniqueTypeName();
  await openFormWithRows(page, typeName, rows);
  await fillRemainingInputs(page, typeName, rows);
  if (rows === MAX_MEMBERS) {
    await expect(page.locator('#udt-add-member')).toHaveCount(0);
  }
  const posts = collectPosts(page);

  const answered = page.waitForResponse((response) => response.request().method() === 'POST', {
    timeout: DEFINE_TIMEOUT_MS,
  });
  await enterTarget(page, enterIn, rows).press('Enter');
  await answered;

  expect(posts).toHaveLength(1);
  const fields = new URLSearchParams(posts[0]?.postData() ?? '');
  expect(fields.has('add')).toBe(false);
  expect(fields.get('name')).toBe(typeName);

  await expect(page).toHaveURL(new RegExp(`/machines/${MACHINE}/types/${typeName}$`), {
    timeout: DEFINE_TIMEOUT_MS,
  });
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(`Type ${typeName}@1`, {
    timeout: DEFINE_TIMEOUT_MS,
  });
  await expect(page.locator('li.tree-leaf')).toHaveCount(rows);
  await expect(page.locator('#error-panel')).toBeEmpty();
}

test.describe('Enter in the form of a new type, language en', () => {
  test.beforeEach(signInInEnglish);

  for (const rows of ROW_COUNTS) {
    for (const enterIn of ['type name', 'member type'] as const) {
      test(`Enter in the ${enterIn} input with ${rows} member rows sends one POST without add and defines the type`, async ({
        page,
      }) => {
        test.setTimeout(rows === 1 ? SMALL_TEST_TIMEOUT_MS : LARGE_TEST_TIMEOUT_MS);
        await defineByEnter(page, enterIn, rows);
      });
    }
  }
});

/**
 * Defines the type `typeName` with `rows` filled rows. The type has the revision 1.
 *
 * One request `POST` of the define form without `add` makes the type, so the test does not click
 * through 1024 rows. The page that the request follows to is the type page, so the answer is `200`.
 */
async function createTypeWithRows(page: Page, typeName: string, rows: number): Promise<void> {
  await page.goto(`/machines/${MACHINE}/types/new`);
  const csrfToken = await page.locator('#udt-form input[name="csrf_token"]').inputValue();
  const answer = await page.request.post(`/machines/${MACHINE}/types`, {
    headers: { 'Sec-Fetch-Site': 'same-origin' },
    form: { csrf_token: csrfToken, name: typeName, ...filledRows(rows) },
    timeout: DEFINE_TIMEOUT_MS,
  });
  expect(answer.status()).toBe(200);
  expect(answer.url()).toContain(`/types/${typeName}`);
}

/**
 * Opens the revision form of `typeName`. The server fills it with the `rows` members of the latest
 * revision, so no route is needed. The form of a revision has no name input and no empty row.
 */
async function openRevisionForm(page: Page, typeName: string, rows: number): Promise<void> {
  await page.goto(`/machines/${MACHINE}/types/new?from=${typeName}`);
  await expect(page.locator('fieldset.member-row')).toHaveCount(rows);
  await expect(page.locator('#udt-form')).toHaveAttribute(
    'action',
    `/machines/${MACHINE}/types/${typeName}/revisions`,
  );
}

/** Returns the type input of the row that a revision test presses Enter in. */
function revisionTarget(page: Page, rows: number): Locator {
  return page.locator(`#members-${Math.floor(rows / 2)}-type`);
}

/**
 * Changes the type of one row of a revision form with `rows` rows and presses Enter in it.
 * It asserts exactly one `POST` to the revisions route without the field `add` and the type page
 * of the revision 2 with `rows` members.
 */
async function reviseByEnter(page: Page, rows: number): Promise<void> {
  const typeName = uniqueTypeName();
  await createTypeWithRows(page, typeName, rows);
  await openRevisionForm(page, typeName, rows);
  if (rows === MAX_MEMBERS) {
    await expect(page.locator('#udt-add-member')).toHaveCount(0);
  }
  const target = revisionTarget(page, rows);
  await target.fill(CHANGED_MEMBER_TYPE);
  const posts = collectPosts(page);

  const answered = page.waitForResponse((response) => response.request().method() === 'POST', {
    timeout: DEFINE_TIMEOUT_MS,
  });
  await target.press('Enter');
  await answered;

  expect(posts).toHaveLength(1);
  expect(new URL(posts[0]?.url() ?? '').pathname).toBe(
    `/machines/${MACHINE}/types/${typeName}/revisions`,
  );
  const fields = new URLSearchParams(posts[0]?.postData() ?? '');
  expect(fields.has('add')).toBe(false);
  expect(fields.get(`members[${Math.floor(rows / 2)}].type`)).toBe(CHANGED_MEMBER_TYPE);

  await expect(page).toHaveURL(new RegExp(`/machines/${MACHINE}/types/${typeName}\\?rev=2$`), {
    timeout: DEFINE_TIMEOUT_MS,
  });
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(`Type ${typeName}@2`, {
    timeout: DEFINE_TIMEOUT_MS,
  });
  await expect(page.locator('li.tree-leaf')).toHaveCount(rows);
  await expect(page.locator('#error-panel')).toBeEmpty();
}

test.describe('Enter in the form of a new revision, language en', () => {
  test.beforeEach(signInInEnglish);

  for (const rows of ROW_COUNTS) {
    test(`Enter in a type input of a changed revision form with ${rows} member rows sends one POST without add and defines the next revision`, async ({
      page,
    }) => {
      test.setTimeout(rows === 1 ? SMALL_TEST_TIMEOUT_MS : LARGE_TEST_TIMEOUT_MS);
      await reviseByEnter(page, rows);
    });
  }

  test('Enter in an unedited revision form sends one POST and shows E-PLT-052 in the error panel', async ({
    page,
  }) => {
    test.setTimeout(SMALL_TEST_TIMEOUT_MS);
    const typeName = uniqueTypeName();
    await createTypeWithRows(page, typeName, 1);
    await openRevisionForm(page, typeName, 1);
    const formUrl = page.url();
    const posts = collectPosts(page);

    const answered = page.waitForResponse((response) => response.request().method() === 'POST');
    await page.locator('#members-0-type').press('Enter');
    const response = await answered;

    expect(response.status()).toBe(409);
    expect(posts).toHaveLength(1);
    expect(new URL(posts[0]?.url() ?? '').pathname).toBe(
      `/machines/${MACHINE}/types/${typeName}/revisions`,
    );
    expect(new URLSearchParams(posts[0]?.postData() ?? '').has('add')).toBe(false);
    await expect(page.locator('#error-panel')).toContainText('E-PLT-052');
    await expect(page.locator('#error-panel')).toContainText(UNCHANGED_REVISION_TEXT(typeName));
    expect(page.url()).toBe(formUrl);
    await expect(page.locator('fieldset.member-row')).toHaveCount(1);
    await expect(page.locator('#members-0-name')).toHaveValue(memberName(0));
    await expect(page.locator('#members-0-type')).toHaveValue(MEMBER_TYPE);
  });
});
