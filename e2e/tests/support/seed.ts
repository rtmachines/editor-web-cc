import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {
  test as base,
  request,
  type APIRequestContext,
  type APIResponse,
  expect,
  type BrowserContext,
  type BrowserContextOptions,
  type Page,
} from '@playwright/test';

// ADR-0048 (c): the seed uses the HTTP routes of the app only, as a user does with the forms.

/** The browser sends this header on a same-origin form post. The CSRF layer checks it (api.md 7.3). */
const SAME_ORIGIN_HEADERS = { 'Sec-Fetch-Site': 'same-origin' };
/** A machine code is M and a number from 1 to 9999999. The seed picks a number above this base. */
const MACHINE_NUMBER_BASE = 1_000_000;
const MACHINE_NUMBER_SPAN = 8_000_000;
/** The path of the main unit that `CreateMachine` makes (api.md 2.2). */
const MAIN_UNIT_PATH = 'U1';
/** The members of the struct type of the seed. The struct holds six leaves of six classes. */
const STRUCT_MEMBERS: ReadonlyArray<readonly [string, string]> = [
  ['setpoint', 'REAL'],
  ['enabled', 'BOOL'],
  ['label', 'STRING(20)'],
  ['count', 'INT'],
  ['delay', 'TIME'],
  ['gain', 'LREAL'],
];
/** The value text of each leaf of the struct, by the field name of its value input. */
const STRUCT_VALUES: Readonly<Record<string, string>> = {
  'value.setpoint': '1.5',
  'value.enabled': 'true',
  'value.label': 'alpha',
  'value.count': '3',
  'value.delay': 'T#1s',
  'value.gain': '2.5',
};

/** One seeded parameter: its tag and the path of its edit page. */
export interface SeededParameter {
  readonly tag: string;
  readonly path: string;
}

/** The data that one run of the seed made. All names are unique per run. */
export interface SeededData {
  readonly machineCode: string;
  readonly recipeReference: string;
  /** The path of the draft page, for example `/recipes/R-123/versions/1`. */
  readonly draftPath: string;
  /** The path of the form of a new parameter, for the main unit. */
  readonly newParameterPath: string;
  /** The parameter with the struct type: six leaf rows. */
  readonly struct: SeededParameter;
  /** The parameter with an array type: element rows and one row `[*]`. */
  readonly array: SeededParameter;
  /** The parameter with a primitive type. */
  readonly primitive: SeededParameter;
}

/** The fields and the texts of a form, as the browser would send them. */
type FormFields = Record<string, string>;

function failure(step: string, response: APIResponse, body: string): Error {
  const codes = [...body.matchAll(/alert-code">([^<]*)</g)].map((match) => match[1]);
  return new Error(
    `Seed step "${step}" failed with status ${response.status()} ${codes.join(' ')}`.trim(),
  );
}

/** A client that logs in once and then posts forms with the CSRF token of its session. */
class SeedClient {
  private constructor(
    private readonly context: APIRequestContext,
    private readonly csrfToken: string,
  ) {}

  static async login(baseURL: string, login: string, password: string): Promise<SeedClient> {
    const context = await request.newContext({ baseURL, extraHTTPHeaders: SAME_ORIGIN_HEADERS });
    const page = await context.get('/login');
    const pre = /name="csrf_pre" value="([^"]*)"/.exec(await page.text())?.[1];
    if (!pre) {
      throw new Error('Seed step "login page" found no csrf_pre field.');
    }
    const done = await context.post('/login', {
      form: { login, password, csrf_pre: pre },
      maxRedirects: 0,
    });
    if (done.status() !== 303) {
      throw failure('login', done, await done.text());
    }
    const form = await context.get('/machines/new');
    const token = /name="csrf_token" value="([^"]*)"/.exec(await form.text())?.[1];
    if (!token) {
      throw new Error('Seed step "login" found no csrf_token after the login.');
    }
    return new SeedClient(context, token);
  }

  /** Returns the cookies of the session, so a browser context can reuse the login. */
  async sessionState(): Promise<SessionState> {
    return this.context.storageState();
  }

  async dispose(): Promise<void> {
    await this.context.dispose();
  }

  async get(path: string): Promise<string> {
    const response = await this.context.get(path);
    const body = await response.text();
    if (!response.ok()) {
      throw failure(`GET ${path}`, response, body);
    }
    return body;
  }

  /** Posts a form and returns the body. A `303` and a `200` count as success. */
  async post(path: string, fields: FormFields): Promise<{ body: string; location?: string }> {
    const response = await this.context.post(path, {
      form: { csrf_token: this.csrfToken, ...fields },
      maxRedirects: 0,
    });
    const body = await response.text();
    if (response.status() !== 303 && response.status() !== 200) {
      throw failure(`POST ${path}`, response, body);
    }
    return { body, location: response.headers()['location'] };
  }
}

/** Reads the value of the first `row_version` field of a page. */
function rowVersionOf(html: string): string {
  const version = /name="row_version" value="([^"]*)"/.exec(html)?.[1];
  if (!version) {
    throw new Error('Seed step found no row_version field.');
  }
  return version;
}

/** Reads the field names of the value inputs of a grid, for example `value.setpoint`. */
function valueFieldNames(html: string): string[] {
  return [...html.matchAll(/<(?:input|select)[^>]*\bname="(value[^"]*)"/g)].map(
    (match) => match[1] ?? '',
  );
}

/** Reads every hidden field `limits[i].leaf_path` of a grid, so the save keeps its limit rows. */
function hiddenLimitPaths(html: string): FormFields {
  const fields: FormFields = {};
  for (const match of html.matchAll(
    /<input type="hidden" name="(limits\[\d+\]\.leaf_path)" value="([^"]*)"/g,
  )) {
    if (match[1] !== undefined && match[2] !== undefined) {
      fields[match[1]] = match[2];
    }
  }
  return fields;
}

/** Adds one parameter in the two steps of the form: show the value fields, then save. */
async function addParameter(
  client: SeedClient,
  draftPath: string,
  tag: string,
  type: string,
  valueOf: (fieldName: string) => string,
  known: ReadonlySet<string>,
): Promise<SeededParameter> {
  const form = await client.get(`${draftPath}/parameters/new?unit=${MAIN_UNIT_PATH}`);
  const definition: FormFields = {
    row_version: rowVersionOf(form),
    unit_path: MAIN_UNIT_PATH,
    tag,
    name_en: `Parameter ${tag}`,
    description_en: `Seeded parameter ${tag}`,
    type,
    ordinal: '0',
  };
  const shown = await client.post(`${draftPath}/parameters`, { ...definition, fields: '1' });
  const values: FormFields = {};
  for (const name of valueFieldNames(shown.body)) {
    values[name] = valueOf(name);
  }
  await client.post(`${draftPath}/parameters`, {
    ...definition,
    ...values,
    ...hiddenLimitPaths(shown.body),
    value_type: type,
  });
  const page = await client.get(draftPath);
  const links = [...page.matchAll(new RegExp(`href="(${draftPath}/parameters/[0-9a-f-]{36})"`, 'g'))];
  const added = links.map((link) => link[1] ?? '').find((href) => href && !known.has(href));
  if (!added) {
    throw new Error(`Seed step found no new link for the parameter ${tag}.`);
  }
  return { tag, path: added };
}

/**
 * Makes a machine, a UDT, a recipe with a draft, and three parameters through the forms.
 * The names hold a random number, so two runs never collide in the in-memory store.
 */
async function seedWithClient(client: SeedClient): Promise<SeededData> {
  const number = MACHINE_NUMBER_BASE + Math.floor(Math.random() * MACHINE_NUMBER_SPAN);
  const machineCode = `M${number}`;
  const recipeReference = `R-${number}`;
  const structName = `ST_Zone_${number}`;
  await client.post('/machines', {
    code: machineCode,
    name: `Machine ${machineCode}`,
    plc_platform: 'CodesysIec',
    main_unit_name: 'Line',
  });
  const members: FormFields = {};
  STRUCT_MEMBERS.forEach(([name, type], index) => {
    members[`members[${index}].name`] = name;
    members[`members[${index}].type`] = type;
  });
  // The type catalog is shared by the machines of one platform, so the name is unique per run.
  await client.post(`/machines/${machineCode}/types`, { name: structName, ...members });
  const created = await client.post('/recipes', {
    machine_code: machineCode,
    reference_number: recipeReference,
    name: `Recipe ${machineCode}`,
    change_note: 'Seed of the browser suite',
  });
  const draftPath = created.location;
  if (!draftPath) {
    throw new Error('Seed step "create recipe" found no Location header.');
  }
  const struct = await addParameter(
    client,
    draftPath,
    'ZONE_A',
    `${structName}@1`,
    (name) => STRUCT_VALUES[name] ?? '1',
    new Set(),
  );
  const array = await addParameter(
    client,
    draftPath,
    'RAMP',
    'ARRAY[1..3] OF REAL',
    () => '2.5',
    new Set([struct.path]),
  );
  const primitive = await addParameter(
    client,
    draftPath,
    'LIMIT',
    'INT',
    () => '7',
    new Set([struct.path, array.path]),
  );
  return {
    machineCode,
    recipeReference,
    draftPath,
    newParameterPath: `${draftPath}/parameters/new?unit=${MAIN_UNIT_PATH}`,
    struct,
    array,
    primitive,
  };
}

/** The state of the browser session of the seeded user, saved once for each worker. */
type SessionState = Exclude<BrowserContextOptions['storageState'], string | undefined>;

interface World {
  readonly data: SeededData;
  readonly session: SessionState;
}

/** The number of tries of a seed. A lost update of the in-memory store fails one try. */
const SEED_ATTEMPTS = 3;
/** The time that a worker waits between two checks of the lock of the seed. */
const LOCK_POLL_MS = 100;
/** A lock older than this belongs to a process that died. The next worker takes it. */
const LOCK_STALE_MS = 120_000;

/**
 * Runs `work` while this process holds the seed lock of the server at `baseURL`.
 *
 * The function solves one problem: the in-memory store of `dev_server.rs` copies its state at the
 * start of a transaction and replaces it at the end. Two seeds at the same time can lose a write
 * of each other, and the login throttle answers `429 E-IDN-005` to many logins at once. The lock
 * is a directory that one process creates. It lives in the temporary directory, named by the port.
 */
async function withSeedLock<T>(baseURL: string, work: () => Promise<T>): Promise<T> {
  const port = new URL(baseURL).port || '80';
  const lock = path.join(os.tmpdir(), `recipe-e2e-seed-${port}.lock`);
  for (;;) {
    try {
      fs.mkdirSync(lock);
      break;
    } catch (error: unknown) {
      if ((error as NodeJS.ErrnoException).code !== 'EEXIST') {
        throw error;
      }
      const age = Date.now() - fs.statSync(lock, { throwIfNoEntry: false })?.mtimeMs;
      if (age > LOCK_STALE_MS) {
        fs.rmSync(lock, { recursive: true, force: true });
      }
      await new Promise((resolve) => setTimeout(resolve, LOCK_POLL_MS));
    }
  }
  try {
    return await work();
  } finally {
    fs.rmSync(lock, { recursive: true, force: true });
  }
}

/**
 * Logs in as `admin`, seeds one draft, and returns the data with the session of the login.
 * The browser contexts of the tests reuse that session, so a worker logs in only once.
 */
async function seedWorld(baseURL: string, password: string): Promise<World> {
  return withSeedLock(baseURL, async () => {
    let lastError: unknown;
    for (let attempt = 1; attempt <= SEED_ATTEMPTS; attempt += 1) {
      const client = await SeedClient.login(baseURL, 'admin', password);
      try {
        const data = await seedWithClient(client);
        return { data, session: await client.sessionState() };
      } catch (error: unknown) {
        lastError = error;
      } finally {
        await client.dispose();
      }
    }
    throw lastError;
  });
}

/** The password of the test users. The config checks that it is set. */
function passwordOfTests(): string {
  const password = process.env.E2E_PASSWORD;
  if (!password) {
    throw new Error('E2E_PASSWORD must be set.');
  }
  return password;
}

/**
 * The test object of the leaf-grid and focus specs.
 * Each worker seeds its own data once. Each test gets a fresh browser context with the session of
 * that seed. Data and session never leak between workers. The design does not use
 * `.auth/admin.json`, because two runs on two ports would overwrite that file.
 */
export const test = base.extend<{ seed: SeededData }, { world: World }>({
  world: [
    async ({}, use, workerInfo) => {
      const baseURL = workerInfo.project.use.baseURL;
      if (!baseURL) {
        throw new Error('The project needs a baseURL.');
      }
      await use(await seedWorld(baseURL, passwordOfTests()));
    },
    { scope: 'worker', timeout: 180_000 },
  ],
  context: async ({ browser, world, baseURL }, use) => {
    const context = await browser.newContext({ baseURL, storageState: world.session });
    await use(context);
    await context.close();
  },
  seed: async ({ world }, use) => {
    await use(world.data);
  },
});

export { expect };

/** The two UI languages. The cookie `recipe_lang` holds the choice (api.md section 8). */
export type Language = 'pl' | 'en';

/** Every control of the leaf grid in DOM order. The DOM order is the Tab order. */
export const GRID_CONTROLS =
  '.leaf-grid input:not([type="hidden"]), .leaf-grid select, .leaf-grid button';

/** Sets the cookie `recipe_lang` of a context. The next page load uses that language. */
export async function selectLanguage(
  context: BrowserContext,
  baseURL: string | undefined,
  language: Language,
): Promise<void> {
  if (!baseURL) {
    throw new Error('The project needs a baseURL.');
  }
  await context.addCookies([{ name: 'recipe_lang', value: language, url: baseURL }]);
}

/** The classes that HTMX sets on an element while a request or a swap runs. */
const HTMX_BUSY = '.htmx-request, .htmx-swapping, .htmx-settling, .htmx-added';

/**
 * Waits until HTMX has swapped and settled an answer.
 * A caller starts the wait for the response before the click, then awaits the response and
 * calls this function. The check reads the classes of HTMX only and changes nothing.
 */
export async function expectSwapSettled(page: Page): Promise<void> {
  await expect(page.locator(HTMX_BUSY)).toHaveCount(0);
}
