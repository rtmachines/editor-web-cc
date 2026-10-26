import { expect, test } from '@playwright/test';

test('the login page opens and shows its heading', async ({ page }) => {
  await page.goto('/login');
  await expect(page).toHaveTitle(/.+/);
  await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  await expect(page.locator('form[action="/login"]')).toBeVisible();
});
