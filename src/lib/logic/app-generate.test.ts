/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import type { TableDef } from '../api/types';
import { addPage, bindingOf, newApp, newWidget, readApp, renameTableInApp, widgetProblems, type SchemaLookup } from './app-def';
import { generatePages, guessIcon, isJunction, listColumns, syncGeneratedPages } from './app-generate';
import { jsonClone } from './values';

const id = { name: 'ID', field_type: { type: 'auto_number' as const }, primary_key: true };
const companies: TableDef = {
	name: 'Companies',
	fields: [id, { name: 'Name', field_type: { type: 'short_text' } }, { name: 'Notes', field_type: { type: 'long_text' } }]
};
const people: TableDef = {
	name: 'People',
	fields: [
		id,
		{ name: 'Name', field_type: { type: 'short_text' } },
		{ name: 'Company', field_type: { type: 'lookup', value_type: 'integer', display_column: 'Name' } }
	],
	foreign_keys: [{ columns: ['Company'], ref_table: 'Companies', ref_columns: ['ID'] }]
};
const tasks: TableDef = {
	name: 'Tasks',
	fields: [
		id,
		{ name: 'Title', field_type: { type: 'short_text' } },
		{ name: 'Status', field_type: { type: 'short_text' } },
		{ name: 'Due', field_type: { type: 'date_time' } }
	]
};
const tags: TableDef = {
	name: 'PeopleTags',
	fields: [
		{ name: 'Person', field_type: { type: 'number' } },
		{ name: 'Tag', field_type: { type: 'number' } }
	],
	foreign_keys: [
		{ columns: ['Person'], ref_table: 'People', ref_columns: ['ID'] },
		{ columns: ['Tag'], ref_table: 'Tags', ref_columns: ['ID'] }
	]
};
const tables = [companies, people, tasks, tags];

function schemaFor(list: TableDef[]): SchemaLookup {
	return {
		databases: ['CRM'],
		objects: () => list.map((t) => ({ name: t.name, kind: 'table' as const })),
		columns: (_, table) => list.find((t) => t.name === table)?.fields.map((f) => f.name)
	};
}

describe('generatePages', () => {
	test('a page per table, a board for status fields and an overview on the empty start page', () => {
		const app = newApp(['CRM']);
		const added = generatePages(app, 'crm', 'CRM', tables);
		expect(added.map((p) => p.title)).toEqual(['Companies', 'People', 'Tasks', 'Tasks board']);
		expect(app.pages.map((p) => p.title)).toEqual(['Overview', 'Companies', 'People', 'Tasks', 'Tasks board']);
		const overview = app.pages[0];
		expect(overview.widgets.map((w) => w.type)).toEqual(['text', 'stat', 'stat', 'stat', 'chart', 'table']);
		expect(app.pages.find((p) => p.title === 'People')?.icon).toBe('users');
	});

	test('a parent page lists the records that link to the picked one', () => {
		const app = newApp(['CRM']);
		generatePages(app, 'crm', 'CRM', tables);
		const page = app.pages.find((p) => p.title === 'Companies')!;
		const [list, form, related] = page.widgets;
		if (list.type !== 'table' || form.type !== 'form' || related?.type !== 'table') throw new Error('unexpected widgets');
		expect(list.rowAction).toEqual({ do: 'set-variable', name: 'selected_companies', value: { kind: 'column', column: 'ID' } });
		expect(form.filters).toEqual([{ column: 'ID', op: 'eq', value: { kind: 'variable', name: 'selected_companies' } }]);
		expect(related.data?.table).toBe('People');
		expect(related.filters[0]).toEqual({ column: 'Company', op: 'eq', value: { kind: 'variable', name: 'selected_companies' } });
		expect(related.columns).not.toContain('Company');
	});

	test('every generated widget resolves and the app survives a save and load', () => {
		const app = newApp(['CRM']);
		generatePages(app, 'crm', 'CRM', tables);
		const loaded = readApp(jsonClone(app));
		expect(loaded).toEqual(app);
		for (const w of app.pages.flatMap((p) => p.widgets)) expect(widgetProblems(app, w, schemaFor(tables))).toEqual([]);
		const board = app.pages.find((p) => p.title === 'Tasks board')?.widgets[0];
		expect(board?.type === 'board' && board.groupBy).toBe('Status');
	});

	test('building again only adds tables without a page', () => {
		const app = newApp(['CRM']);
		generatePages(app, 'crm', 'CRM', [companies]);
		const count = app.pages.length;
		const added = generatePages(app, 'crm', 'CRM', [companies, people]);
		expect(added.map((p) => p.title)).toEqual(['People']);
		expect(app.pages.length).toBe(count + 1);
		// The start page was already filled the first time.
		expect(app.pages[0].widgets.filter((w) => w.type === 'stat')).toHaveLength(1);
	});
});

test('isJunction, listColumns and guessIcon', () => {
	expect(isJunction(tags)).toBe(true);
	expect(isJunction(people)).toBe(false);
	expect(listColumns(companies)).toEqual(['Name']);
	expect(listColumns(tasks, 2)).toEqual(['Title', 'Status']);
	expect(guessIcon('Invoices')).toBe('receipt');
	expect(guessIcon('Widgets')).toBe('table');
	// Whole words only: not orders, not tags.
	expect(guessIcon('Borders')).toBe('table');
	expect(guessIcon('Prototypes')).toBe('table');
	expect(guessIcon('SalesOrders')).toBe('shopping-cart');
	expect(guessIcon('customer_notes')).toBe('users');
});

describe('syncGeneratedPages', () => {
	const built = () => {
		const app = newApp(['CRM']);
		generatePages(app, 'crm', 'CRM', [companies, people]);
		return app;
	};
	const pageOf = (app: ReturnType<typeof newApp>, title: string) => app.pages.find((p) => p.title === title)!;

	test('nothing changes when the tables are the same', () => {
		const app = built();
		expect(syncGeneratedPages(app, 'crm', 'CRM', [companies, people])).toBe(false);
	});

	test('a new table gets its pages, but the overview is not refilled', () => {
		const app = built();
		app.pages[0].widgets = [];
		expect(syncGeneratedPages(app, 'crm', 'CRM', [companies, people, tasks])).toBe(true);
		expect(app.pages.map((p) => p.title)).toContain('Tasks board');
		expect(app.pages[0].widgets).toEqual([]);
	});

	test('a deleted table loses its page, its variable and widgets elsewhere that read it', () => {
		const app = built();
		syncGeneratedPages(app, 'crm', 'CRM', [companies]);
		expect(app.pages.map((p) => p.title)).not.toContain('People');
		expect(app.variables.map((v) => v.name)).not.toContain('selected_people');
		// The related People list under Companies and the overview's count went too.
		expect(app.pages.flatMap((p) => p.widgets).some((w) => bindingOf(w)?.table === 'People')).toBe(false);
	});

	test('new fields join the list once; deleted fields leave every widget', () => {
		const app = built();
		const withEmail = { ...people, fields: [...people.fields, { name: 'Email', field_type: { type: 'short_text' as const } }] };
		syncGeneratedPages(app, 'crm', 'CRM', [companies, withEmail]);
		const list = pageOf(app, 'People').widgets[0];
		expect(list.type === 'table' && list.columns).toEqual(['Name', 'Company', 'Email']);
		// Taken out by hand, it stays out.
		if (list.type === 'table') list.columns = ['Name', 'Company'];
		syncGeneratedPages(app, 'crm', 'CRM', [companies, withEmail]);
		expect(list.type === 'table' && list.columns).toEqual(['Name', 'Company']);
		const noName = { ...withEmail, fields: withEmail.fields.filter((f) => f.name !== 'Name') };
		syncGeneratedPages(app, 'crm', 'CRM', [companies, noName]);
		expect(list.type === 'table' && list.columns).toEqual(['Company']);
		for (const w of app.pages.flatMap((p) => p.widgets)) expect(widgetProblems(app, w, schemaFor([companies, noName]))).toEqual([]);
	});

	test('a newly linked table is listed under its parent', () => {
		const app = newApp(['CRM']);
		const loose = { ...people, foreign_keys: [] };
		generatePages(app, 'crm', 'CRM', [companies, loose]);
		expect(pageOf(app, 'Companies').widgets).toHaveLength(2);
		syncGeneratedPages(app, 'crm', 'CRM', [companies, people]);
		const related = pageOf(app, 'Companies').widgets[2];
		expect(related?.type === 'table' && related.filters[0].column).toBe('Company');
	});

	test('renaming a table keeps its page and follows the name', () => {
		const app = built();
		renameTableInApp(app, 'CRM', 'People', 'Contacts');
		const renamed = { ...people, name: 'Contacts' };
		syncGeneratedPages(app, 'crm', 'CRM', [companies, renamed]);
		expect(app.pages.map((p) => p.title)).toEqual(['Overview', 'Companies', 'Contacts']);
	});

	test('buttons that added records to a removed widget do nothing instead', () => {
		const app = built();
		const people = pageOf(app, 'People');
		const list = people.widgets[0];
		const button = newWidget('button', { x: 0, y: 20, w: 2, h: 1 });
		if (button.type !== 'button') throw new Error();
		button.action = { do: 'new-record', widget: list.id };
		people.widgets.push(button);
		// People is deleted while a hand-made button elsewhere still points at its list.
		app.pages[0].widgets.push(jsonClone(button));
		syncGeneratedPages(app, 'crm', 'CRM', [companies]);
		const left = app.pages[0].widgets.at(-1);
		expect(left?.type === 'button' && left.action).toEqual({ do: 'none' });
	});

	test('an empty database gets no overview', () => {
		const app = newApp(['CRM']);
		expect(generatePages(app, 'crm', 'CRM', [])).toEqual([]);
		expect(app.pages[0].widgets).toEqual([]);
	});

	test('saved queries are left alone', () => {
		const app = built();
		const page = addPage(app, 'Report');
		page.widgets.push(newWidget('table', { x: 0, y: 0, w: 12, h: 8 }, { source: 'crm', table: 'Big orders' }));
		syncGeneratedPages(app, 'crm', 'CRM', [companies, people], ['Big orders']);
		expect(pageOf(app, 'Report').widgets).toHaveLength(1);
	});
});
