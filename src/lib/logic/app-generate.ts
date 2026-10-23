import type { FieldDef, TableDef } from '../api/types';
import {
	addPage,
	addVariable,
	bindingOf,
	GRID_COLUMNS,
	newWidget,
	removePage,
	removeVariable,
	removeWidget,
	uniqueName,
	variablesRead,
	type AppDef,
	type BoardWidget,
	type ChartWidget,
	type FormWidget,
	type Layout,
	type Page,
	type PageOrigin,
	type StatWidget,
	type TableWidget,
	type TextWidget,
	type Widget
} from './app-def';
import { findSpot } from './app-layout';

/**
 * "Build pages": an app made from a database's tables in one go. Each table
 * gets a page with its records on the left and a form for the picked one on
 * the right, with the records of tables that link to it underneath; a table
 * with a status-like field also gets a board. An empty start page becomes an
 * overview with a count per table, a chart and the latest records.
 */

/** Field names that read as a workflow state, shown as colored labels and boards. */
const STATUS_NAME = /^(status|stage|state|phase|priority|progress)$/i;

/**
 * Page icons guessed from the words of table names; the first match wins.
 * Each pattern must start a word, so "Borders" isn't taken for orders.
 */
const ICON_GUESSES: [RegExp, string][] = [
	[/^(people|person|contact|customer|client|user|member|employee|staff|student|author)/, 'users'],
	[/^(compan|organi[sz]ation|vendor|supplier|account|business)/, 'building'],
	[/^(product|item|inventor|stock|asset)/, 'package'],
	[/^(order|sale|purchase|cart)/, 'shopping-cart'],
	[/^(invoice|payment|receipt|bill|expense|transaction)/, 'receipt'],
	[/^(task|todo|issue|ticket|bug)/, 'square-check'],
	[/^(event|meeting|appointment|booking|reservation|schedule)/, 'calendar'],
	[/^(project|job|deal)/, 'briefcase'],
	[/^(shipment|shipping|deliver)/, 'truck'],
	[/^(categor|tag|label|type)/, 'tag'],
	[/^(location|address|place|site|store)/, 'map-pin'],
	[/^(message|email|mail)/, 'mail'],
	[/^(post|article|book|course|lesson|recipe)/, 'book-open'],
	[/^(note|document|page)/, 'file-text']
];

/** The words of a name: split at spaces, underscores, dashes and camelCase humps, lower case. */
function words(name: string): string[] {
	return name
		.replace(/([a-z0-9])([A-Z])/g, '$1 $2')
		.split(/[^A-Za-z0-9]+/)
		.filter(Boolean)
		.map((w) => w.toLowerCase());
}

export function guessIcon(table: string): string {
	const ws = words(table);
	return ICON_GUESSES.find(([re]) => ws.some((w) => re.test(w)))?.[1] ?? 'table';
}

const keyOf = (t: TableDef): string | null => {
	const pk = t.fields.filter((f) => f.primary_key);
	return pk.length === 1 ? pk[0].name : null;
};

const isStatus = (f: FieldDef) => STATUS_NAME.test(f.name) && (f.field_type.type === 'short_text' || f.field_type.type === 'yes_no');

/** A table that only links two others (many-to-many), which doesn't need a page of its own. */
export function isJunction(t: TableDef): boolean {
	const linked = new Set((t.foreign_keys ?? []).flatMap((k) => k.columns));
	return linked.size >= 2 && t.fields.every((f) => linked.has(f.name) || (f.primary_key && f.field_type.type === 'auto_number'));
}

/** Fields worth a column in a list: not the auto-numbered key, long text, files or the ones in `except`. */
export function listColumns(t: TableDef, max = 5, except: string[] = []): string[] {
	return t.fields
		.filter((f) => !except.includes(f.name))
		.filter((f) => !(f.primary_key && f.field_type.type === 'auto_number'))
		.filter((f) => !['long_text', 'attachment'].includes(f.field_type.type))
		.slice(0, max)
		.map((f) => f.name);
}

/** Tables with a one-field link to `parent`'s key, and that field. */
function childrenOf(parent: TableDef, tables: TableDef[]): { table: TableDef; column: string }[] {
	const key = keyOf(parent);
	if (!key) return [];
	return tables.flatMap((t) =>
		t.name === parent.name
			? []
			: (t.foreign_keys ?? [])
					.filter((k) => k.ref_table === parent.name && k.columns.length === 1 && (k.ref_columns?.[0] ?? key) === key)
					.slice(0, 1)
					.map((k) => ({ table: t, column: k.columns[0] }))
	);
}

function make<T extends Widget>(type: T['type'], layout: Layout, source: string, table: string | null, set?: (w: T) => void): T {
	const w = newWidget(type, layout, table ? { source, table } : null) as T;
	set?.(w);
	return w;
}

/** Splits the grid into `n` equal widths (the last one takes what is left). */
function widths(n: number): number[] {
	const w = Math.floor(GRID_COLUMNS / n);
	return Array.from({ length: n }, (_, i) => (i === n - 1 ? GRID_COLUMNS - w * (n - 1) : w));
}

const childKey = (c: { table: TableDef; column: string }) => `${c.table.name}.${c.column}`;

/** A table of the `child` records that link to the record picked in `variable`. */
function relatedTable(alias: string, c: { table: TableDef; column: string }, variable: string, layout: Layout): TableWidget {
	return make<TableWidget>('table', layout, alias, c.table.name, (tw) => {
		tw.title = `Related ${c.table.name}`;
		tw.columns = listColumns(c.table, layout.w >= 12 ? 6 : 4, [c.column]);
		tw.badges = c.table.fields.filter(isStatus).map((f) => f.name);
		tw.filters = [{ column: c.column, op: 'eq', value: { kind: 'variable', name: variable } }];
		tw.pageSize = 10;
	});
}

function boardPage(app: AppDef, alias: string, t: TableDef, group: string): Page | null {
	const lane = t.fields.find((f) => isStatus(f) && f.field_type.type === 'short_text');
	if (!lane) return null;
	const board = addPage(app, uniqueName(`${t.name} board`, app.pages.map((p) => p.title)), 'clipboard-list', group);
	board.origin = { source: alias, table: t.name, kind: 'board', fields: t.fields.map((f) => f.name), children: [], variable: '' };
	board.widgets.push(
		make<BoardWidget>('board', { x: 0, y: 0, w: 12, h: 12 }, alias, t.name, (w) => {
			w.groupBy = lane.name;
			w.cardFields = listColumns(t, 3, [lane.name, listColumns(t, 1)[0] ?? '']);
		})
	);
	return board;
}

function tablePage(app: AppDef, alias: string, t: TableDef, tables: TableDef[], group: string): Page[] {
	const page = addPage(app, uniqueName(t.name, app.pages.map((p) => p.title)), guessIcon(t.name), group);
	const key = keyOf(t);
	const status = t.fields.filter(isStatus).map((f) => f.name);
	const columns = listColumns(t);
	const children = childrenOf(t, tables);
	const origin: PageOrigin = {
		source: alias,
		table: t.name,
		kind: 'table',
		fields: t.fields.map((f) => f.name),
		children: children.map(childKey),
		variable: ''
	};
	page.origin = origin;
	if (!key) {
		page.widgets.push(
			make<TableWidget>('table', { x: 0, y: 0, w: 12, h: 12 }, alias, t.name, (w) => {
				w.columns = columns;
				w.badges = status;
				w.pageSize = 50;
			})
		);
		return [page];
	}
	const base = t.name.replace(/[^A-Za-z0-9_]+/g, '_').replace(/^[^A-Za-z_]+/, '').toLowerCase() || 'record';
	const variable = uniqueName(`selected_${base}`, app.variables.map((v) => v.name));
	origin.variable = variable;
	addVariable(app, variable, null);
	page.widgets.push(
		make<TableWidget>('table', { x: 0, y: 0, w: 7, h: 10 }, alias, t.name, (w) => {
			w.columns = columns;
			w.badges = status;
			w.allowNew = false;
			w.rowAction = { do: 'set-variable', name: variable, value: { kind: 'column', column: key } };
		}),
		make<FormWidget>('form', { x: 7, y: 0, w: 5, h: 10 }, alias, t.name, (w) => {
			w.title = `${t.name} details`;
			w.filters = [{ column: key, op: 'eq', value: { kind: 'variable', name: variable } }];
			w.allowDelete = true;
		})
	);
	const shown = children.slice(0, 2);
	const childWidths = widths(shown.length);
	let x = 0;
	shown.forEach((c, i) => {
		page.widgets.push(relatedTable(alias, c, variable, { x, y: 10, w: childWidths[i], h: 7 }));
		x += childWidths[i];
	});
	const board = boardPage(app, alias, t, group);
	return board ? [page, board] : [page];
}

/** Fills an empty page with a count per table, a chart of the first table that has something to chart, and its latest records. */
function fillOverview(page: Page, alias: string, database: string, tables: TableDef[]) {
	page.widgets.push(
		make<TextWidget>('text', { x: 0, y: 0, w: 12, h: 2 }, alias, null, (w) => {
			w.markdown = `# ${database}\nAn overview of your data. Pick a page on the left to see and edit records.`;
		})
	);
	let y = 2;
	const counted = tables.slice(0, 4);
	let x = 0;
	widths(counted.length).forEach((w, i) => {
		const t = counted[i];
		page.widgets.push(make<StatWidget>('stat', { x, y, w, h: 2 }, alias, t.name, (s) => (s.title = s.caption = t.name)));
		x += w;
	});
	if (counted.length) y += 2;
	for (const t of tables) {
		const status = t.fields.find(isStatus);
		const date = t.fields.find((f) => f.field_type.type === 'date_time');
		if (!status && !date) continue;
		page.widgets.push(
			make<ChartWidget>('chart', { x: 0, y, w: 12, h: 7 }, alias, t.name, (c) => {
				const by = (status ?? date)!;
				c.title = `${t.name} by ${by.name.toLowerCase()}`;
				c.chart = status ? 'bar' : 'area';
				c.groupBy = { column: by.name, bucket: status ? null : 'month' };
			})
		);
		y += 7;
		break;
	}
	const first = tables[0];
	if (first) {
		const key = keyOf(first);
		page.widgets.push(
			make<TableWidget>('table', { x: 0, y, w: 12, h: 8 }, alias, first.name, (w) => {
				w.title = `Latest ${first.name}`;
				w.columns = listColumns(first, 6);
				w.badges = first.fields.filter(isStatus).map((f) => f.name);
				w.pageSize = 10;
				w.search = false;
				if (key && first.fields.find((f) => f.name === key)?.field_type.type === 'auto_number') w.sort = { column: key, descending: true };
			})
		);
	}
}

/**
 * Adds pages for the tables of source `alias` (database `database`) that
 * don't have a page of the same name yet, under sidebar heading `group`.
 * Fills the start page with an overview while it is empty. Returns the
 * pages added.
 */
export function generatePages(app: AppDef, alias: string, database: string, tables: TableDef[], group = '', overview = true): Page[] {
	const taken = new Set(app.pages.map((p) => p.title.toLowerCase()));
	const wanted = tables.filter((t) => !isJunction(t));
	const built = new Set(app.pages.flatMap((p) => (p.origin?.source === alias && p.origin.kind === 'table' ? [p.origin.table] : [])));
	const start = app.pages.find((p) => p.id === app.startPage);
	// Without tables an overview would only be a heading.
	if (overview && start && !start.widgets.length && wanted.length) {
		fillOverview(start, alias, database, wanted);
		if (start.title === 'Home' && !taken.has('overview')) start.title = 'Overview';
		start.icon = 'layout-dashboard';
	}
	return wanted.filter((t) => !built.has(t.name) && !taken.has(t.name.toLowerCase())).flatMap((t) => tablePage(app, alias, t, wanted, group));
}

// ----- Automatic updates -----

const listable = (f: FieldDef) => !(f.primary_key && f.field_type.type === 'auto_number') && !['long_text', 'attachment'].includes(f.field_type.type);

/**
 * Drops whatever a widget names that its table no longer has. Returns false
 * when the widget can't work without it (a board whose columns field went).
 */
function prune(w: Widget, columns: Set<string>): boolean {
	const keep = (c: string) => columns.has(c);
	if ('filters' in w) w.filters = w.filters.filter((f) => !f.column || keep(f.column));
	switch (w.type) {
		case 'table':
			w.columns = w.columns.filter(keep);
			w.badges = w.badges.filter(keep);
			if (w.sort && !keep(w.sort.column)) w.sort = null;
			break;
		case 'form':
			w.fields = w.fields.filter(keep);
			break;
		case 'board':
			if (!keep(w.groupBy)) return false;
			w.cardFields = w.cardFields.filter(keep);
			if (w.cardTitle && !keep(w.cardTitle)) w.cardTitle = '';
			break;
		case 'chart':
			if (w.groupBy && !keep(w.groupBy.column)) w.groupBy = null;
			for (const s of w.series) if (s.column && !keep(s.column)) Object.assign(s, { column: null, agg: 'count' });
			break;
		case 'stat':
			if (w.series.column && !keep(w.series.column)) w.series = { column: null, agg: 'count' };
			break;
		case 'filter':
			if (!keep(w.options.valueColumn)) w.options = { data: null, valueColumn: '', labelColumn: '' };
			else if (w.options.labelColumn && !keep(w.options.labelColumn)) w.options.labelColumn = '';
			break;
	}
	return true;
}

/**
 * Brings an app in step with the tables of source `alias` after they
 * changed: pages for new tables, no pages for deleted ones, new fields added
 * to the lists of their pages, deleted fields taken out of every widget, and
 * newly linked tables listed under their parent. `views` are the source's
 * saved queries, which widgets may also read. Returns true if anything
 * changed.
 */
export function syncGeneratedPages(app: AppDef, alias: string, database: string, tables: TableDef[], views: string[] = [], group = ''): boolean {
	const before = JSON.stringify(app);
	const defs = new Map(tables.map((t) => [t.name, t]));
	const wanted = tables.filter((t) => !isJunction(t));

	// Deleted tables: their pages go, and so do widgets elsewhere that read them.
	for (const p of [...app.pages]) {
		const o = p.origin;
		if (o?.source !== alias || wanted.some((t) => t.name === o.table)) continue;
		removePage(app, p.id);
		const stillRead = app.pages.some((x) => x.widgets.some((w) => variablesRead(w).includes(o.variable)));
		if (o.variable && !stillRead) removeVariable(app, o.variable);
	}
	const gone = app.pages.flatMap((p) =>
		p.widgets.filter((w) => {
			const b = w.type === 'filter' ? w.options.data : bindingOf(w);
			if (!b || b.source !== alias || views.includes(b.table)) return false;
			const def = defs.get(b.table);
			return !def || !prune(w, new Set(def.fields.map((f) => f.name)));
		})
	);
	// Through removeWidget, so buttons that added records to them do nothing instead of pointing nowhere.
	for (const w of gone) removeWidget(app, w.id);

	// Pages already built: new fields and new linking tables.
	for (const p of app.pages) {
		const o = p.origin;
		const t = o?.source === alias ? defs.get(o.table) : undefined;
		if (!o || !t) continue;
		const added = t.fields.filter((f) => !o.fields.includes(f.name));
		o.fields = t.fields.map((f) => f.name);
		if (o.kind !== 'table') continue;
		const list = p.widgets.find((w): w is TableWidget => w.type === 'table' && w.data?.source === alias && w.data.table === t.name);
		if (list && list.columns.length) list.columns.push(...added.filter(listable).map((f) => f.name));
		for (const form of p.widgets) {
			if (form.type === 'form' && form.data?.source === alias && form.data.table === t.name && form.fields.length) {
				form.fields.push(...added.map((f) => f.name));
			}
		}
		const children = childrenOf(t, wanted);
		if (o.variable) {
			for (const c of children.filter((c) => !o.children.includes(childKey(c)))) {
				const spot = findSpot(p.widgets.map((w) => ({ id: w.id, layout: w.layout })), 12, 7);
				p.widgets.push(relatedTable(alias, c, o.variable, spot));
			}
		}
		o.children = children.map(childKey);
	}

	// New tables get pages of their own.
	generatePages(app, alias, database, tables, group, false);
	return JSON.stringify(app) !== before;
}
