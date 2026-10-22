/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import {
	addPage,
	addSource,
	addVariable,
	newApp,
	newWidget,
	prefillFromFilters,
	readApp,
	removePage,
	removeSource,
	removeVariable,
	removeWidget,
	renameFieldInApp,
	renameTableInApp,
	renameVariable,
	resolveFilters,
	widgetProblems,
	type AppDef,
	type SchemaLookup
} from './app-def';
import { jsonClone } from './values';

function sample(): AppDef {
	const app = newApp(['Sales', 'Stock']);
	addVariable(app, 'customer', null);
	const page = app.pages[0];
	const orders = newWidget('table', { x: 0, y: 0, w: 12, h: 8 }, { source: 'sales', table: 'Orders' });
	if (orders.type !== 'table') throw new Error();
	orders.columns = ['id', 'customer_id', 'total'];
	orders.filters = [{ column: 'customer_id', op: 'eq', value: { kind: 'variable', name: 'customer' } }];
	orders.rowAction = { do: 'set-variable', name: 'customer', value: { kind: 'column', column: 'customer_id' } };
	orders.id = 'orders';
	const chart = newWidget('chart', { x: 0, y: 8, w: 6, h: 6 }, { source: 'sales', table: 'Orders' });
	if (chart.type !== 'chart') throw new Error();
	chart.groupBy = { column: 'placed', bucket: 'month' };
	chart.series = [{ column: 'total', agg: 'sum' }];
	chart.id = 'chart';
	const stock = newWidget('table', { x: 6, y: 8, w: 6, h: 6 }, { source: 'stock', table: 'Orders' });
	stock.id = 'stock';
	const button = newWidget('button', { x: 0, y: 14, w: 2, h: 1 });
	if (button.type !== 'button') throw new Error();
	button.action = { do: 'new-record', widget: 'orders' };
	button.id = 'button';
	page.widgets.push(orders, chart, stock, button);
	return app;
}

describe('apps', () => {
	test('new apps get one alias per database and a start page', () => {
		const app = newApp(['Sales 2024', 'sales-2024', '***']);
		expect(app.sources.map((s) => s.alias)).toEqual(['sales_2024', 'sales_20242', 'db']);
		expect(app.startPage).toBe(app.pages[0].id);
		expect(addSource(app, 'Sales 2024')).toBe(app.sources[0]);
		expect(addSource(app, 'HR').alias).toBe('hr');
	});

	test('reading a file fills defaults and drops what it does not understand', () => {
		const app = readApp({
			version: 1,
			pages: [
				{
					id: 'p',
					title: 'P',
					widgets: [
						{ type: 'table', id: 'a', data: { source: 's', table: 't' }, layout: { x: 11, y: -3, w: 4, h: 2 } },
						{ type: 'hologram', id: 'b' },
						{ type: 'button', id: 'c', action: { do: 'teleport' } }
					]
				}
			],
			startPage: 'missing'
		});
		expect(app.startPage).toBe('p');
		expect(app.pages[0].widgets.map((w) => w.type)).toEqual(['table', 'button']);
		const table = app.pages[0].widgets[0];
		expect(table.layout).toEqual({ x: 8, y: 0, w: 4, h: 2 });
		if (table.type === 'table') expect(table.rowAction).toEqual({ do: 'none' });
		const button = app.pages[0].widgets[1];
		if (button.type === 'button') expect(button.action).toEqual({ do: 'none' });
		expect(() => readApp({ pages: [] })).toThrow();
		expect(() => readApp({ version: 9, pages: [] })).toThrow(/newer/);
		// A saved app reads back unchanged.
		const s = sample();
		expect(readApp(jsonClone(s))).toEqual(s);
	});

	test('filters skip empty variables and become typed backend filters', () => {
		const filters = [
			{ column: 'customer_id', op: 'eq' as const, value: { kind: 'variable' as const, name: 'customer' } },
			{ column: 'status', op: 'in' as const, value: { kind: 'literal' as const, value: 'open, held' } },
			{ column: 'note', op: 'is_null' as const, value: { kind: 'literal' as const, value: null } }
		];
		expect(resolveFilters(filters, { customer: '' })).toEqual([
			{ column: 'status', op: 'in', value: ['open', 'held'] },
			{ column: 'note', op: 'is_null', value: null }
		]);
		expect(resolveFilters(filters, { customer: 7 })[0]).toEqual({ column: 'customer_id', op: 'eq', value: 7 });
		expect(prefillFromFilters(filters, { customer: 7 })).toEqual({ customer_id: 7 });
	});

	test('renames in a database follow into the widgets that use it', () => {
		const app = sample();
		expect(renameTableInApp(app, 'Sales', 'Orders', 'Purchases')).toBe(true);
		const [orders, chart, stock] = app.pages[0].widgets;
		expect(orders.type === 'table' && orders.data?.table).toBe('Purchases');
		// Same table name in another database is untouched.
		expect(stock.type === 'table' && stock.data?.table).toBe('Orders');

		expect(renameFieldInApp(app, 'Sales', 'Purchases', 'customer_id', 'buyer')).toBe(true);
		expect(renameFieldInApp(app, 'Sales', 'Purchases', 'total', 'amount')).toBe(true);
		if (orders.type !== 'table' || chart.type !== 'chart') throw new Error();
		expect(orders.columns).toEqual(['id', 'buyer', 'amount']);
		expect(orders.filters[0].column).toBe('buyer');
		expect(orders.rowAction).toEqual({ do: 'set-variable', name: 'customer', value: { kind: 'column', column: 'buyer' } });
		expect(chart.series[0].column).toBe('amount');
		expect(renameFieldInApp(app, 'Sales', 'Purchases', 'nothing', 'x')).toBe(false);
	});

	test('variables rename and remove everywhere they are used', () => {
		const app = sample();
		renameVariable(app, 'customer', 'buyer');
		const orders = app.pages[0].widgets[0];
		if (orders.type !== 'table') throw new Error();
		expect(orders.filters[0].value).toEqual({ kind: 'variable', name: 'buyer' });
		expect(orders.rowAction).toMatchObject({ name: 'buyer' });
		removeVariable(app, 'buyer');
		expect(orders.filters).toEqual([]);
		expect(orders.rowAction).toEqual({ do: 'none' });
	});

	test('removing things leaves no dangling references', () => {
		const app = sample();
		removeWidget(app, 'orders');
		const button = app.pages[0].widgets.find((w) => w.id === 'button');
		expect(button?.type === 'button' && button.action).toEqual({ do: 'none' });

		const second = addPage(app, 'Second');
		if (button?.type === 'button') button.action = { do: 'navigate', page: second.id, set: [] };
		app.startPage = second.id;
		removePage(app, second.id);
		expect(app.startPage).toBe(app.pages[0].id);
		expect(button?.type === 'button' && button.action).toEqual({ do: 'none' });
		removePage(app, app.pages[0].id);
		expect(app.pages).toHaveLength(1);

		removeSource(app, 'stock');
		const stock = app.pages[0].widgets.find((w) => w.id === 'stock');
		expect(stock?.type === 'table' && stock.data).toBeNull();
	});

	test('problems name what is missing', () => {
		const app = sample();
		const schema: SchemaLookup = {
			databases: ['Sales'],
			objects: (alias) => (alias === 'sales' ? [{ name: 'Orders', kind: 'table' }] : undefined),
			columns: () => ['id', 'customer_id', 'placed']
		};
		const [orders, chart, stock, button] = app.pages[0].widgets;
		expect(widgetProblems(app, orders, schema)).toEqual(['Field “total” no longer exists.']);
		expect(widgetProblems(app, chart, schema)).toEqual(['Field “total” no longer exists.']);
		expect(widgetProblems(app, stock, schema)).toEqual(['The database “Stock” is missing from the folder.']);
		expect(widgetProblems(app, button, schema)).toEqual([]);
		removeVariable(app, 'customer');
		addVariable(app, 'other');
		const form = newWidget('form', { x: 0, y: 0, w: 1, h: 1 }, { source: 'sales', table: 'Gone' });
		expect(widgetProblems(app, form, schema)).toEqual(['“Gone” no longer exists in Sales.']);
		expect(widgetProblems(app, newWidget('chart', { x: 0, y: 0, w: 1, h: 1 }), schema)).toEqual([
			'Choose where the data comes from.'
		]);
	});
});
