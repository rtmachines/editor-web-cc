import type { APIRequestContext, APIResponse, Browser, BrowserContext, Cookie } from '@playwright/test';

/** The widths of ADR-0048 (d), in CSS px. */
export const WIDTHS = [320, 768, 1280] as const;

/** One GET page that the sweep opens. */
export interface SweepPage {
  /** The name in the title of a test. */
  readonly name: string;
  /** The page area. The English test opens one page of each area. */
  readonly area: 'auth' | 'machines' | 'recipes' | 'users' | 'audit' | 'own';
  /** True for the page that the English test opens. */
  readonly english: boolean;
  /** Returns the path. Some paths hold a key that the seed step finds at run time. */
  readonly path: () => string;
}

/** The machine, recipe, and type names that the seed step creates. */
const MACHINE = 'M7';
const RECIPE = 'R-500';
const UDT = 'ST_Zone';

/** The values that the seed step finds at run time. */
interface Found {
  parameterPath: string;
  adminCookies: Cookie[];
}

let found: Found | undefined;

function resolved(): Found {
  if (!found) {
    throw new Error('The seed step must run before the sweep resolves a path.');
  }
  return found;
}

/**
 * Returns the cookies of the session that the seed step opened as `admin`.
 * The sweep logs in on its own, so another spec that rewrites the shared session file cannot
 * break it. A test adds these cookies after `beforeAll`, because a stored file would be stale.
 */
export function adminCookies(): Cookie[] {
  return resolved().adminCookies;
}

const page = (
  area: SweepPage['area'],
  name: string,
  path: string | (() => string),
  english = false,
): SweepPage => ({ area, name, english, path: typeof path === 'string' ? () => path : path });

/** The pages that need the session of `admin`. The Administrator holds every permission. */
export const LOGGED_IN_PAGES: readonly SweepPage[] = [
  page('machines', 'machine list', '/machines'),
  page('machines', 'machine form', '/machines/new'),
  page('machines', 'machine page', `/machines/${MACHINE}`, true),
  page('machines', 'unit tree page', `/machines/${MACHINE}/units`),
  page('machines', 'unit form', `/machines/${MACHINE}/units/new?parent_path=U1`),
  page('machines', 'unit page', `/machines/${MACHINE}/units/U1.2`),
  page('machines', 'type list', `/machines/${MACHINE}/types`),
  page('machines', 'type form', `/machines/${MACHINE}/types/new`),
  page('machines', 'type page', `/machines/${MACHINE}/types/${UDT}`),
  page('machines', 'type usage page', `/machines/${MACHINE}/types/${UDT}/usage`),
  page('recipes', 'recipe list', '/recipes'),
  page('recipes', 'recipe form', '/recipes/new'),
  page('recipes', 'recipe page', `/recipes/${RECIPE}`),
  page('recipes', 'version list', `/recipes/${RECIPE}/versions`),
  page('recipes', 'version form', `/recipes/${RECIPE}/versions/new`),
  page('recipes', 'approved version page', `/recipes/${RECIPE}/versions/1`),
  page('recipes', 'draft version page', `/recipes/${RECIPE}/versions/2`, true),
  page('recipes', 'approval preview', `/recipes/${RECIPE}/versions/2/approve`),
  page('recipes', 'version diff', `/recipes/${RECIPE}/versions/2/diff/1`),
  page('recipes', 'parameter table', `/recipes/${RECIPE}/versions/2/parameters`),
  page('recipes', 'parameter form', `/recipes/${RECIPE}/versions/2/parameters/new`),
  page('recipes', 'parameter page', () => resolved().parameterPath),
  page('recipes', 'recipe history', `/recipes/${RECIPE}/history`),
  page('users', 'user list', '/users'),
  page('users', 'user form', '/users/new'),
  page('users', 'user page', '/users/engineer', true),
  page('audit', 'audit log', '/audit', true),
  page('audit', 'audit entry', '/audit/1'),
  page('audit', 'security events', '/security-events'),
  page('own', 'own profile', '/me', true),
  page('own', 'own password form', '/me/password'),
];

/** The page without a session. */
export const LOGIN_PAGE: SweepPage = page('auth', 'login page', '/login', true);

/** The engineer edits the draft, so the Administrator may approve it (separation of duties). */
const ENGINEER = 'engineer';
const ADMIN = 'admin';
const SESSION_PROBE = '/machines';
const CSRF_HEADER_PATTERN = /hx-headers='\{"X-CSRF-Token":"([^"]+)"\}'/;
const ROW_VERSION_PATTERN = /name="row_version" value="(\d+)"/;

/** A session that posts the forms of the app through the HTTP routes. */
interface Poster {
  readonly context: BrowserContext;
  readonly csrf: string;
}

async function openPoster(context: BrowserContext): Promise<Poster> {
  const reply = await context.request.get(SESSION_PROBE);
  const match = CSRF_HEADER_PATTERN.exec(await reply.text());
  if (!match?.[1]) {
    throw new Error('The seed step found no CSRF token. The session may have expired.');
  }
  return { context, csrf: match[1] };
}

async function loginAs(
  browser: Browser,
  baseURL: string,
  login: string,
  password: string,
): Promise<BrowserContext> {
  const context = await browser.newContext({ baseURL });
  const tab = await context.newPage();
  await tab.goto('/login');
  await tab.locator('#login').fill(login);
  await tab.locator('#password').fill(password);
  await tab.locator('form[action="/login"] button[type="submit"]').click();
  await tab.waitForURL((url) => url.pathname !== '/login');
  await tab.close();
  return context;
}

async function post(
  poster: Poster,
  path: string,
  form: Record<string, string>,
): Promise<APIResponse> {
  const api: APIRequestContext = poster.context.request;
  return api.post(path, { form, headers: { 'X-CSRF-Token': poster.csrf, 'Sec-Fetch-Site': 'same-origin' } });
}

async function rowVersionOf(poster: Poster, path: string): Promise<string> {
  const reply = await poster.context.request.get(path);
  return ROW_VERSION_PATTERN.exec(await reply.text())?.[1] ?? '1';
}

/**
 * Seeds one machine, one type, and one recipe with an approved version and a draft.
 * The call is idempotent, because the dev server keeps its store while the suite runs.
 * A refused form means the data exists already, so the step ignores a refusal.
 */
export async function seedSweepData(browser: Browser, baseURL: string, password: string): Promise<void> {
  const adminContext = await loginAs(browser, baseURL, ADMIN, password);
  const admin = await openPoster(adminContext);
  const engineer = await openPoster(await loginAs(browser, baseURL, ENGINEER, password));
  try {
    await post(admin, '/machines', {
      code: MACHINE,
      name: 'Sweep line',
      plc_platform: 'S7_300',
      main_unit_name: 'Infeed',
    });
    await post(admin, `/machines/${MACHINE}/units`, {
      parent_path: 'U1',
      segment: '2',
      name: 'Second zone',
    });
    await post(admin, `/machines/${MACHINE}/types`, {
      name: UDT,
      'members[0].name': 'hold',
      'members[0].type': 'INT',
    });
    await seedRecipe(admin, engineer);
    found = {
      parameterPath: await findParameterPath(engineer),
      adminCookies: (await adminContext.storageState()).cookies,
    };
  } finally {
    await admin.context.close();
    await engineer.context.close();
  }
}

async function seedRecipe(admin: Poster, engineer: Poster): Promise<void> {
  const versions = `/recipes/${RECIPE}/versions`;
  await post(engineer, '/recipes', {
    machine_code: MACHINE,
    reference_number: RECIPE,
    name: 'Sweep recipe',
    change_note: 'First version',
  });
  await post(engineer, `${versions}/1/parameters`, {
    row_version: await rowVersionOf(engineer, `${versions}/1/parameters/new`),
    unit_path: 'U1',
    tag: 'SPEED',
    name_en: 'Speed',
    description_en: 'Line speed',
    type: 'INT',
    value_type: 'INT',
    s7_address: 'DB2.DBW0',
    ordinal: '0',
    value: '1200',
    'limits[0].leaf_path': '',
    'limits[0].min': '0',
    'limits[0].max': '3000',
  });
  await post(admin, `${versions}/1/approve`, {
    row_version: await rowVersionOf(admin, `${versions}/1/approve`),
    allow_outdated: 'false',
  });
  await post(engineer, versions, { based_on: '1', change_note: 'Second version' });
  const path = await findParameterPath(engineer);
  await post(engineer, `${path}/value`, {
    row_version: await rowVersionOf(engineer, `${versions}/2/parameters/new`),
    value: '1300',
  });
}

async function findParameterPath(poster: Poster): Promise<string> {
  const reply = await poster.context.request.get(`/recipes/${RECIPE}/versions/2`);
  const pattern = new RegExp(`href="(/recipes/${RECIPE}/versions/2/parameters/[0-9a-f-]{36})"`);
  const match = pattern.exec(await reply.text());
  if (!match?.[1]) {
    throw new Error(`The seed step found no parameter in ${RECIPE} version 2.`);
  }
  return match[1];
}
