/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import {
	addCommand,
	addPage,
	addPageOfKind,
	addVariable,
	commandProblems,
	dropPage,
	duplicatePage,
	mapSqlParameters,
	newApp,
	newWidget,
	pageGroups,
	readApp,
	removePage,
	removeSource,
	removeVariable,
	renameVariable,
	sqlParameters,
	variablesRead,
	widgetProblems,
	type QueryWidget
} from './app-def';

const schema = { databases: ['Sales'], objects: () => undefined, columns: () => undefined };

describe('sidebar pages and commands', () => {
	test('old files get page icons, no groups and no commands', () => {
		const app = readApp({ version: 1, pages: [{ id: 'p', title: 'Home', widgets: [] }], commands: [{ label: 'Go', icon: 'nope', action: { do: 'print' } }] });
		expect(app.pages[0]).toMatchObject({ icon: 'file', group: '', hidden: false });
		expect(app.commands).toHaveLength(1);
		expect(app.commands[0]).toMatchObject({ label: 'Go', icon: 'zap', action: { do: 'print' } });
		expect(readApp({ version: 1, pages: [] }).commands).toEqual([]);
	});

	test('pages are listed by group, ungrouped first, hidden ones only while editing', () => {
		const app = newApp([]);
		const a = addPage(app, 'A', 'file', 'Shop');
		addPage(app, 'B');
		const c = addPage(app, 'C', 'file', 'shop');
		c.hidden = true;
		expect(pageGroups(app.pages, false).map((g) => [g.group, g.pages.map((p) => p.title)])).toEqual([
			['', ['Home', 'B']],
			['Shop', ['A']]
		]);
		expect(pageGroups(app.pages, true)[1].pages.map((p) => p.title)).toEqual(['A', 'C']);
		dropPage(app, c.id, a.id, '');
		expect(app.pages.map((p) => p.title)).toEqual(['Home', 'C', 'A', 'B']);
		expect(c.group).toBe('');
		dropPage(app, a.id, a.id, 'Other');
		expect(a.group).toBe('Other');
	});

	test('removing a page or widget switches off commands that pointed at it', () => {
		const app = newApp([]);
		const page = addPage(app, 'Orders');
		const go = addCommand(app, 'Orders');
		go.action = { do: 'navigate', page: page.id, set: [] };
		const add = addCommand(app, 'Add');
		add.action = { do: 'set-variable', name: 'x', value: { kind: 'literal', value: 1 } };
		expect(commandProblems(app, add)).toEqual(['Variable “x” doesn\'t exist.']);
		removePage(app, page.id);
		expect(app.commands[0].action).toEqual({ do: 'none' });
		addVariable(app, 'x');
		removeVariable(app, 'x');
		expect(app.commands[1].action).toEqual({ do: 'none' });
	});

	test('duplicated pages get new ids and keep links between their own widgets', () => {
		const app = newApp([]);
		const form = newWidget('form', { x: 0, y: 0, w: 4, h: 4 });
		const button = newWidget('button', { x: 4, y: 0, w: 2, h: 1 });
		if (button.type !== 'button') throw new Error();
		button.action = { do: 'new-record', widget: form.id };
		app.pages[0].widgets.push(form, button);
		const copy = duplicatePage(app, app.pages[0].id, 'Home copy')!;
		expect(app.pages[1]).toBe(copy);
		const [f, b] = copy.widgets;
		expect(f.id).not.toBe(form.id);
		expect(b.type === 'button' && b.action).toEqual({ do: 'new-record', widget: f.id });
	});

	test('page kinds start with their widgets', () => {
		const app = newApp(['Sales']);
		const data = { source: 'sales', table: 'Orders' };
		const lf = addPageOfKind(app, 'list-form', 'Orders', data, { key: 'OrderID', group: 'Shop' });
		expect(lf.group).toBe('Shop');
		expect(lf.widgets.map((w) => w.type)).toEqual(['table', 'form']);
		expect(app.variables.map((v) => v.name)).toEqual(['selected_orders']);
		const [table, form] = lf.widgets;
		expect(table.type === 'table' && table.rowAction).toEqual({
			do: 'set-variable',
			name: 'selected_orders',
			value: { kind: 'column', column: 'OrderID' }
		});
		expect(form.type === 'form' && form.filters[0].column).toBe('OrderID');
		const q = addPageOfKind(app, 'query', 'Q', data).widgets[0] as QueryWidget;
		expect(q).toMatchObject({ type: 'query', source: 'sales', sql: 'SELECT *\nFROM "Orders"' });
		expect(addPageOfKind(app, 'dashboard', 'D', data).widgets).toHaveLength(3);
		expect(addPageOfKind(app, 'blank', 'B', null).widgets).toHaveLength(0);
	});
});

describe('query widgets', () => {
	test('parameters are found outside quotes and comments', () => {
		const sql = "SELECT ':no', \"x:no\" FROM t -- :no\nWHERE a = :yes AND b = @also AND c = $too /* :no */ AND d = :yes AND e = '12:30'";
		expect(sqlParameters(sql)).toEqual(['yes', 'also', 'too']);
		expect(mapSqlParameters('SELECT :a, :ab', (n) => (n === 'a' ? 'b' : n))).toBe('SELECT :b, :ab');
	});

	test('renaming a variable rewrites the SQL; removing a database unbinds it', () => {
		const app = newApp(['Sales']);
		addVariable(app, 'customer');
		const w = newWidget('query', { x: 0, y: 0, w: 12, h: 8 }, { source: 'sales', table: 'Orders' }) as QueryWidget;
		w.sql = "SELECT * FROM Orders WHERE customer = :customer AND note <> ':customer'";
		app.pages[0].widgets.push(w);
		expect(variablesRead(w)).toEqual(['customer']);
		renameVariable(app, 'customer', 'client');
		expect(w.sql).toBe("SELECT * FROM Orders WHERE customer = :client AND note <> ':customer'");
		expect(widgetProblems(app, w, schema)).toEqual([]);
		removeSource(app, 'sales');
		expect(w.source).toBe('');
		expect(widgetProblems(app, w, schema)).toEqual(['Choose the database the query runs on.']);
	});
});
