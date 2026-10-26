import { expect, test, type Page } from '@playwright/test';
import { fakeBackend, type Scenario } from './fake-backend';

const money = { type: 'currency', scale: 2 };
const shop: Scenario = {
	workspace: '/fake',
	databases: {
		Shop: {
			Products: {
				fields: [
					{ name: 'ID', field_type: { type: 'auto_number' }, primary_key: true },
					{ name: 'Name', field_type: { type: 'short_text' } },
					{ name: 'Price', field_type: money }
				],
				rows: [
					[1, 'Pen', 150],
					[2, 'Ink', 1205],
					[3, 'Mug', 900]
				]
			}
		}
	},
	apps: [{ name: 'Till', databases: ['Shop'] }]
};

async function start(page: Page, scenario: Scenario, path = '/') {
	await page.addInitScript(fakeBackend, scenario);
	await page.goto(path);
}

test('first run asks for a workspace folder', async ({ page }) => {
	await start(page, { workspace: null, databases: {} });
	await expect(page.getByRole('button', { name: /choose a folder/i })).toBeVisible();
});

test('the home map shows apps and databases, and search finds them', async ({ page }) => {
	await start(page, shop);
	await expect(page.getByText('Shop', { exact: true }).first()).toBeVisible();
	await expect(page.getByText('Till', { exact: true }).first()).toBeVisible();
	await page.keyboard.press('Control+k');
	const palette = page.getByRole('dialog');
	await palette.getByRole('combobox').fill('till');
	await expect(palette.getByRole('option', { name: /Till/ })).toBeVisible();
});

test('an empty workspace offers a sample', async ({ page }) => {
	await start(page, { workspace: '/fake', databases: {} });
	await expect(page.getByRole('button', { name: /try a sample shop/i })).toBeVisible();
});

test('the Datasheet lists records with money as money', async ({ page }) => {
	await start(page, shop, '/db?name=Shop');
	await page.getByRole('tab', { name: /Datasheet/ }).click();
	const grid = page.getByRole('grid', { name: 'Products records' });
	await expect(grid.getByText('Pen')).toBeVisible();
	await expect(grid.getByText('12.05')).toBeVisible();
});

test('a report groups nothing by default and totals what it is told to', async ({ page }) => {
	await start(page, shop, '/db?name=Shop');
	await page.getByRole('tab', { name: /Reports/ }).click();
	await page.getByRole('button', { name: 'New report' }).first().click();
	const paper = page.locator('article.report-paper');
	await expect(paper.getByRole('heading', { name: 'Products' })).toBeVisible();
	await expect(paper.getByText('Total · 3 records')).toBeVisible();
	await page.getByRole('combobox', { name: 'Total for Price' }).click();
	await page.getByRole('option', { name: 'Sum' }).click();
	await expect(paper.locator('tfoot')).toContainText('22.55');
});

test('Settings switches between sections', async ({ page }) => {
	await start(page, shop, '/settings');
	await page.getByRole('link', { name: /Appearance/ }).or(page.getByRole('button', { name: /Appearance/ })).first().click();
	await expect(page).toHaveURL(/section=appearance/);
});
