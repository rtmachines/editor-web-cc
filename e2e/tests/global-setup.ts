import { chromium, type FullConfig } from '@playwright/test';
import path from 'node:path';

/** The file that holds the session of the user `admin`. Git ignores it. */
export const ADMIN_STORAGE_STATE = path.join(__dirname, '..', '.auth', 'admin.json');

/**
 * Logs in as `admin` through the HTTP routes of the app and saves the session.
 * Later specs reuse the saved session. The password comes from `E2E_PASSWORD`.
 */
export default async function globalSetup(config: FullConfig): Promise<void> {
  const baseURL = config.projects[0]?.use.baseURL;
  const password = process.env.E2E_PASSWORD;
  if (!baseURL || !password) {
    throw new Error('Global setup needs the baseURL of the config and E2E_PASSWORD.');
  }

  const browser = await chromium.launch();
  try {
    const context = await browser.newContext({ baseURL });
    const page = await context.newPage();
    await page.goto('/login');
    await page.locator('#login').fill('admin');
    await page.locator('#password').fill(password);
    await page.locator('form[action="/login"] button[type="submit"]').click();
    await page.waitForURL((url) => url.pathname !== '/login');
    await context.storageState({ path: ADMIN_STORAGE_STATE });
  } finally {
    await browser.close();
  }
}
