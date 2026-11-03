import { defineConfig, devices } from '@playwright/test';
import path from 'node:path';
import { READ_ONLY_FILE } from './tests/support/read-only';

// ADR-0048: the regression suite runs against examples/dev_server.rs with its in-memory store.

/** The port of the dev server. A fixed default avoids a conflict with the port 8080. */
const DEFAULT_PORT = '18080';
/** The dev server accepts passwords of 12 to 128 characters. */
const MIN_PASSWORD_LENGTH = 12;
/** The first build of the example can take several minutes. */
const SERVER_START_TIMEOUT_MS = 600_000;

const port = process.env.RECIPE_DEV_PORT ?? DEFAULT_PORT;
/** The second instance runs the read-only test (ADR-0113 (h)). It uses the next port. */
const readOnlyPort = String(Number(port) + 1);
const readOnlySpec = /read-only\.spec\.ts/;
const password = process.env.E2E_PASSWORD ?? '';

if (password.length < MIN_PASSWORD_LENGTH) {
  throw new Error(
    `E2E_PASSWORD must be set and hold at least ${MIN_PASSWORD_LENGTH} characters. ` +
      'Example: E2E_PASSWORD=$(openssl rand -hex 16) npx playwright test',
  );
}

const repoRoot = path.resolve(__dirname, '..');

export default defineConfig({
  testDir: './tests',
  globalSetup: './tests/global-setup.ts',
  fullyParallel: false,
  // The in-memory store of the dev server loses concurrent writes. One worker runs the suite.
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: [['html', { open: 'never' }], ['list']],
  use: {
    baseURL: `http://127.0.0.1:${port}`,
    trace: 'retain-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      testIgnore: readOnlySpec,
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'read-only',
      testMatch: readOnlySpec,
      use: { ...devices['Desktop Chrome'], baseURL: `http://127.0.0.1:${readOnlyPort}` },
    },
  ],
  webServer: [
    {
      command: 'cargo run -p recipe-web --example dev_server',
      cwd: repoRoot,
      url: `http://127.0.0.1:${port}/login`,
      reuseExistingServer: !process.env.CI,
      timeout: SERVER_START_TIMEOUT_MS,
      env: {
        RECIPE_DEV_PASSWORD: password,
        RECIPE_DEV_PORT: port,
      },
    },
    {
      command: 'cargo run -p recipe-web --example dev_server',
      cwd: repoRoot,
      url: `http://127.0.0.1:${readOnlyPort}/login`,
      reuseExistingServer: !process.env.CI,
      timeout: SERVER_START_TIMEOUT_MS,
      env: {
        RECIPE_DEV_PASSWORD: password,
        RECIPE_DEV_PORT: readOnlyPort,
        RECIPE_DEV_READ_ONLY_FILE: READ_ONLY_FILE,
      },
    },
  ],
});
