import { expect, test, type Request } from '@playwright/test';

// Proof of V-15 (task 1.1 of review-design-fixes-2). This spec runs only in the project
// `v15-plain-http`. Its server has `session.secure=false`, and its URL is a plain `http://` URL
// that Chromium does not trust. The spec sends the real login form and prints one proof line.
// It records the facts and passes for class R and class P. It fails only for class I.

/** R is the refusal `403 E-WEB-007`. P is a login that passes. I is a fault of the setup. */
type ProofClass = 'R' | 'P' | 'I';

const NONE = 'none';
const ERROR_CODE_PATTERN = /E-[A-Z]+-\d{3}/;
const REFUSAL_CODE = 'E-WEB-007';
const REFUSAL_STATUS = 403;
const FIRST_ERROR_STATUS = 400;
const SUBMIT_TIMEOUT_MS = 15_000;

/** The facts of the form submit. */
interface Facts {
  status: number | undefined;
  code: string;
  scheme: string;
  origin: string;
  secFetchSite: string;
  cookie: 'present' | 'absent';
  finalPath: string;
}

function classify(facts: Facts, submitSeen: boolean): ProofClass {
  if (!submitSeen || facts.scheme !== 'http' || facts.status === undefined) return 'I';
  if (facts.status === REFUSAL_STATUS && facts.code === REFUSAL_CODE) return 'R';
  if (facts.status < FIRST_ERROR_STATUS && facts.code === NONE) return 'P';
  return 'I';
}

/** Tell whether the Origin check of `csrf.rs` would pass for these headers. */
function originCheckWouldPass(facts: Facts, host: string): boolean {
  if (facts.secFetchSite === 'same-origin') return true;
  return facts.origin === `https://${host}`;
}

async function readFacts(request: Request, finalPath: string): Promise<Facts> {
  const headers = await request.allHeaders();
  const response = await request.response();
  const body = response ? await response.text().catch(() => '') : '';
  return {
    status: response?.status(),
    code: ERROR_CODE_PATTERN.exec(body)?.[0] ?? NONE,
    scheme: new URL(request.url()).protocol.replace(':', ''),
    origin: headers['origin'] ?? NONE,
    secFetchSite: headers['sec-fetch-site'] ?? NONE,
    cookie: headers['cookie'] ? 'present' : 'absent',
    finalPath,
  };
}

test('V-15: the login form over plain http with session.secure=false', async ({ page }) => {
  const password = process.env.E2E_PASSWORD ?? '';
  await page.goto('/login');
  await page.locator('#login').fill('admin');
  await page.locator('#password').fill(password);

  const submitted = page
    .waitForRequest((request) => request.method() === 'POST' && request.url().endsWith('/login'), {
      timeout: SUBMIT_TIMEOUT_MS,
    })
    .catch(() => undefined);
  await page.locator('form[action="/login"] button[type="submit"]').click();
  const request = await submitted;
  // Wait for the answer page, so that the body of a refusal is complete.
  await page.waitForLoadState('load').catch(() => undefined);

  const facts: Facts = request
    ? await readFacts(request, new URL(page.url()).pathname)
    : {
        status: undefined,
        code: NONE,
        scheme: new URL(page.url()).protocol.replace(':', ''),
        origin: NONE,
        secFetchSite: NONE,
        cookie: 'absent',
        finalPath: new URL(page.url()).pathname,
      };
  const proofClass = classify(facts, request !== undefined);
  // The event `csrf_rejected` has no reason field, so the reason stays unknown. The next line
  // shows whether the Origin check alone explains a refusal. The log of the server (the lines
  // that start with `[WebServer]`) shows the logged error of the refusal.
  console.log(
    `V15-PROOF class=${proofClass} status=${facts.status ?? NONE} code=${facts.code} ` +
      `scheme=${facts.scheme} origin=${facts.origin} sec_fetch_site=${facts.secFetchSite} ` +
      `cookie=${facts.cookie} reason=unknown`,
  );
  console.log(
    `V15-HINT origin_check_would_pass=${originCheckWouldPass(facts, new URL(page.url()).host)} ` +
      `final_path=${facts.finalPath}`,
  );

  expect(proofClass, 'class I is a fault of the setup').not.toBe('I');
});
