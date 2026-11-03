import { expect, selectLanguage, test } from './support/seed';

// Design D5 of web-ui-review-quick-wins: each row of the type usage table links to the recipe page,
// the version page, and the parameter page. The seed makes the draft version 1 of one recipe with
// the parameter ZONE_A. That parameter uses revision 1 of the seeded struct.

/** The time that a link of the row may take to appear. The page is server-rendered, so it is short. */
const LINK_TIMEOUT_MS = 2_000;
/** The number of links in one row of the usage table. */
const LINKS_PER_ROW = 3;

test.describe('type usage page, language en', () => {
  test.beforeEach(async ({ context, baseURL }) => {
    await selectLanguage(context, baseURL, 'en');
  });

  test('row of ZONE_A links to the recipe, the version, and the parameter page', async ({ page, seed }) => {
    const usagePath = `/machines/${seed.machineCode}/types/${seed.structName}/usage?rev=1`;
    const links = [
      { name: 'recipe', href: `/recipes/${seed.recipeReference}`, heading: `Recipe ${seed.recipeReference}` },
      {
        name: 'version',
        href: `/recipes/${seed.recipeReference}/versions/1`,
        heading: `Version 1 of recipe ${seed.recipeReference}`,
      },
      {
        name: 'parameter',
        href: seed.struct.path,
        heading: `Parameter ${seed.struct.tag}, version 1 of recipe ${seed.recipeReference}`,
      },
    ] as const;

    await page.goto(usagePath);
    const row = page.locator('tbody tr', { hasText: seed.struct.tag });
    await expect(row, 'the usage table has one row for ZONE_A').toHaveCount(1);
    await expect(row.getByRole('link'), 'the row holds three links').toHaveCount(LINKS_PER_ROW, {
      timeout: LINK_TIMEOUT_MS,
    });

    for (const link of links) {
      await page.goto(usagePath);
      const anchor = page.locator('tbody tr', { hasText: seed.struct.tag }).locator(`a[href="${link.href}"]`);
      await expect(anchor, `the row links to the ${link.name} page`).toHaveCount(1, {
        timeout: LINK_TIMEOUT_MS,
      });
      await anchor.click();
      await expect(page.locator('h1'), `the ${link.name} page has its heading`).toHaveText(link.heading);
    }
  });
});
