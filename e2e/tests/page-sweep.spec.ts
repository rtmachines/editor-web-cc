import { expect, test, type BrowserContext, type Page } from '@playwright/test';
import {
  adminCookies,
  LOGGED_IN_PAGES,
  LOGIN_PAGE,
  seedSweepData,
  WIDTHS,
  type SweepPage,
} from './support/pages';

// ADR-0048 (c): each page gets the layout check and the CSP checks at three widths.
// The tests only read. The seed step in `beforeAll` writes through the HTTP routes.

/** The seed step hashes passwords, so a debug build of the dev server needs time. */
const SEED_TIMEOUT_MS = 180_000;
/** The height of the viewport. Only the width matters for the checks. */
const VIEWPORT_HEIGHT = 900;
/** The texts that a browser writes to the console for a CSP violation. */
const CSP_CONSOLE_TEXTS = ['Content Security Policy', 'Refused to'];
const NO_SESSION = { cookies: [], origins: [] };

// The tests share one dev server and one seed. One worker runs this file, so seeds cannot race.
test.describe.configure({ mode: 'default' });

test.beforeAll(async ({ browser }, testInfo) => {
  test.setTimeout(SEED_TIMEOUT_MS);
  const baseURL = testInfo.project.use.baseURL;
  const password = process.env.E2E_PASSWORD;
  if (!baseURL || !password) {
    throw new Error('The sweep needs the baseURL of the config and E2E_PASSWORD.');
  }
  await seedSweepData(browser, baseURL, password);
});

/**
 * The literal grammar of each `hx-*` attribute (ADR-0037, api.md section 9).
 * An attribute that is not in this table fails. Only `hx-headers` of `<body>` holds a
 * session value: the CSRF token (api.md section 7.3).
 */
const HX_GRAMMAR: Readonly<Record<string, RegExp>> = {
  'hx-boost': /^(true|false)$/,
  'hx-history': /^false$/,
  'hx-sync': /^closest form:drop$/,
  'hx-push-url': /^(true|false)$/,
  'hx-swap': /^(innerHTML|outerHTML|beforeend)( show:none focus-scroll:true)?$/,
  'hx-swap-oob': /^(true|innerHTML)$/,
  'hx-target': /^(#[a-z][a-z0-9-]*|closest \.[a-z][a-z0-9-]*)$/,
  'hx-post': /^\/[a-z][a-z/-]*$/,
  'hx-headers': /^\{"X-CSRF-Token":"[A-Za-z0-9_.~+/=-]+"\}$/,
};
/** A template tag or an HTML special character never belongs in a literal value. */
const TEMPLATE_MARKS = /\{\{|\{%|<|>/;

interface DomReport {
  styleAttributes: string[];
  styleElements: number;
  inlineScripts: string[];
  eventAttributes: string[];
  hxAttributes: { element: string; name: string; value: string }[];
  language: string;
}

/** Reads the DOM once and returns the facts that the CSP checks need. */
function readDom(target: Page): Promise<DomReport> {
  return target.evaluate(() => {
    const label = (element: Element): string => {
      const id = element.id ? `#${element.id}` : '';
      return `${element.tagName.toLowerCase()}${id}`;
    };
    const everything = Array.from(document.querySelectorAll('*'));
    const attributes = everything.flatMap((element) =>
      Array.from(element.attributes).map((attribute) => ({ element, attribute })),
    );
    return {
      styleAttributes: Array.from(document.querySelectorAll('[style]')).map(label),
      styleElements: document.querySelectorAll('style').length,
      inlineScripts: Array.from(document.querySelectorAll('script:not([src])')).map(label),
      eventAttributes: attributes
        .filter(({ attribute }) => attribute.name.toLowerCase().startsWith('on'))
        .map(({ element, attribute }) => `${label(element)}[${attribute.name}]`),
      hxAttributes: attributes
        .filter(({ attribute }) => attribute.name.toLowerCase().startsWith('hx-'))
        .map(({ element, attribute }) => ({
          element: label(element),
          name: attribute.name.toLowerCase(),
          value: attribute.value,
        })),
      language: document.documentElement.lang,
    };
  });
}

/** Returns the `hx-*` attributes whose value is not a literal of the grammar. */
function hxViolations(report: DomReport): string[] {
  return report.hxAttributes
    .filter(({ name, value }) => {
      const grammar = HX_GRAMMAR[name];
      return !grammar || TEMPLATE_MARKS.test(value) || !grammar.test(value);
    })
    .map(({ element, name, value }) => `${element} ${name}="${value}"`);
}

/** Collects the CSP messages of the console, the page errors, and the policy events. */
function watchCsp(target: Page): () => Promise<string[]> {
  const messages: string[] = [];
  const isCsp = (text: string): boolean => CSP_CONSOLE_TEXTS.some((mark) => text.includes(mark));
  target.on('console', (message) => {
    if (isCsp(message.text())) messages.push(`console: ${message.text()}`);
  });
  target.on('pageerror', (error) => {
    if (isCsp(error.message)) messages.push(`pageerror: ${error.message}`);
  });
  return async () => {
    const events = await target.evaluate(
      () => (window as unknown as { __cspEvents?: string[] }).__cspEvents ?? [],
    );
    return [...messages, ...events.map((event) => `violation: ${event}`)];
  };
}

/** Opens a page, waits for the fonts, and checks that the answer is a full page. */
async function openPage(target: Page, entry: SweepPage): Promise<void> {
  await target.addInitScript(() => {
    const events: string[] = [];
    (window as unknown as { __cspEvents: string[] }).__cspEvents = events;
    document.addEventListener('securitypolicyviolation', (event) => {
      events.push(`${event.violatedDirective} ${event.blockedURI}`);
    });
  });
  const path = entry.path();
  const response = await target.goto(path);
  expect(response?.status(), `status of ${path}`).toBe(200);
  // A page without a session redirects to the login page with the status 200, so check who we are.
  expect(new URL(target.url()).pathname, `final path of ${path}`).toBe(new URL(path, target.url()).pathname);
  if (entry.area === 'auth') {
    await expect(target.locator('form[action="/login"]')).toBeVisible();
  } else {
    await expect(target.locator('.user-login .data')).toHaveText('admin');
  }
  await target.evaluate(() => document.fonts.ready);
}

async function expectNoSidewaysScroll(target: Page, path: string): Promise<void> {
  await expect
    .poll(
      () => target.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth),
      { message: `sideways overflow of ${path} in CSS px` },
    )
    .toBeLessThanOrEqual(0);
}

async function expectCspClean(target: Page, path: string, report: () => Promise<string[]>): Promise<void> {
  const dom = await readDom(target);
  expect.soft(dom.styleAttributes, `style attributes of ${path}`).toEqual([]);
  expect.soft(dom.styleElements, `style elements of ${path}`).toBe(0);
  expect.soft(dom.inlineScripts, `inline scripts of ${path}`).toEqual([]);
  expect.soft(dom.eventAttributes, `on* attributes of ${path}`).toEqual([]);
  expect.soft(hxViolations(dom), `hx-* values of ${path} outside the literal grammar`).toEqual([]);
  expect.soft(await report(), `CSP messages of ${path}`).toEqual([]);
}

/** Registers the two checks for each page at the current width. */
function registerChecks(entries: readonly SweepPage[], language: 'pl' | 'en'): void {
  for (const entry of entries) {
    test(`${entry.name} has no sideways scroll`, async ({ page: tab }) => {
      await openPage(tab, entry);
      await expect(tab.locator('html')).toHaveAttribute('lang', language);
      await expectNoSidewaysScroll(tab, entry.path());
    });
    test(`${entry.name} obeys the CSP rules`, async ({ page: tab }) => {
      const report = watchCsp(tab);
      await openPage(tab, entry);
      await expect(tab.locator('html')).toHaveAttribute('lang', language);
      await expectCspClean(tab, entry.path(), report);
    });
  }
}

/** Adds the session of `admin` that the seed step opened (see `adminCookies`). */
async function signInAsAdmin({ context }: { context: BrowserContext }): Promise<void> {
  await context.addCookies(adminCookies());
}

/** Sets the cookie `recipe_lang` to `en` (api.md section 8). */
async function selectEnglish({ context }: { context: BrowserContext }): Promise<void> {
  await context.addCookies([{ name: 'recipe_lang', value: 'en', domain: '127.0.0.1', path: '/' }]);
}

for (const width of WIDTHS) {
  test.describe(`at ${width} CSS px`, () => {
    test.use({ viewport: { width, height: VIEWPORT_HEIGHT } });

    test.describe('without a session, language pl', () => {
      test.use({ storageState: NO_SESSION });
      registerChecks([LOGIN_PAGE], 'pl');
    });

    test.describe('as admin, language pl', () => {
      test.beforeEach(signInAsAdmin);
      registerChecks(LOGGED_IN_PAGES, 'pl');
    });

    test.describe('without a session, language en', () => {
      test.use({ storageState: NO_SESSION });
      test.beforeEach(selectEnglish);
      registerChecks([LOGIN_PAGE], 'en');
    });

    test.describe('as admin, language en, one page of each area', () => {
      test.beforeEach(signInAsAdmin);
      test.beforeEach(selectEnglish);
      registerChecks(LOGGED_IN_PAGES.filter((entry) => entry.english), 'en');
    });
  });
}
