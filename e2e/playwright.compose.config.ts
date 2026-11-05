import { defineConfig, devices } from '@playwright/test';
import os from 'node:os';
import path from 'node:path';

// ADR-0128 (ad) to (af): the smoke suite runs against the Compose stack behind Caddy.
// This config starts no web server and runs no global setup.

/** The password rule of the application. Both smoke users need 12 characters or more. */
const MIN_PASSWORD_LENGTH = 12;
/** The published port of Caddy in the dev stack of deploy/compose.dev.yaml. */
const DEFAULT_BASE_URL = 'https://127.0.0.1:8443';
/** The password variables. The config reads their names only and never prints a value. */
const PASSWORD_VARIABLES = ['E2E_ADMIN_PASSWORD', 'E2E_APPROVER_PASSWORD'] as const;

// Stop before the first request. The message names the variable and never shows its value.
const badVariables = PASSWORD_VARIABLES.filter(
  (name) => (process.env[name] ?? '').length < MIN_PASSWORD_LENGTH,
);
if (badVariables.length > 0) {
  throw new Error(
    `${badVariables.join(' and ')} must be set and hold at least ${MIN_PASSWORD_LENGTH} characters. ` +
      'The smoke suite needs both passwords.',
  );
}

const baseURL = process.env.E2E_BASE_URL || DEFAULT_BASE_URL;

// The session files hold cookies. They stay outside the upload path of the CI job.
// The setup project and the flow project read the same directory through this variable.
const stateDir = process.env.E2E_STATE_DIR || path.join(os.tmpdir(), 'recipe-smoke-state');
process.env.E2E_STATE_DIR = stateDir;

export default defineConfig({
  testDir: './compose',
  fullyParallel: false,
  workers: 1,
  retries: 0,
  forbidOnly: !!process.env.CI,
  reporter: [['html', { open: 'never' }], ['list']],
  use: {
    baseURL,
    // Only this config accepts the certificate of `tls internal` (ADR-0128 (ad)).
    ignoreHTTPSErrors: true,
  },
  projects: [
    {
      // This project types the passwords. No trace, screenshot, or video may record them.
      name: 'smoke-login',
      testMatch: /.*\.setup\.ts/,
      use: {
        ...devices['Desktop Chrome'],
        trace: 'off',
        screenshot: 'off',
        video: 'off',
      },
    },
    {
      name: 'smoke-compose',
      testMatch: /.*\.spec\.ts/,
      dependencies: ['smoke-login'],
      use: {
        ...devices['Desktop Chrome'],
        trace: 'retain-on-failure',
      },
    },
  ],
});
