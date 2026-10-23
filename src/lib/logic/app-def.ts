import type { Aggregate, AggregateOrder, DateBucket, FilterOp, FilterSpec, JsonValue, SortSpec } from '../api/types';
import { needsValue } from './filters';
import { jsonClone } from './values';

/**
 * Apps: pages of widgets (tables, forms, charts, number cards, filters,
 * buttons, text, queries) over one or more workspace databases, listed in a
 * sidebar together with the app's commands. The definition is
 * plain JSON stored by the backend in `<workspace>/Apps/<name>.baseapp`.
 *
 * It is declarative only. Widgets name a source, a table or saved query and
 * columns; values typed into filters become bound parameters in the backend.
 */

export const APP_FORMAT_VERSION = 1;
/** Columns of the page grid. */
export const GRID_COLUMNS = 12;

/** A database the app uses. Widgets refer to the alias, so renaming the database only changes this entry. */
export interface AppSource {
	alias: string;
	database: string;
	/** Keep the pages built from this database in step with its tables (see `syncGeneratedPages`). */
	auto: boolean;
}

/**
 * Marks a page built from a table by "Build pages", so automatic updates can
 * find it again. `fields` and `children` are what the last update saw, so a
 * field the user took out of the list isn't put back.
 */
export interface PageOrigin {
	source: string;
	table: string;
	kind: 'table' | 'board';
	fields: string[];
	/** Linking tables shown underneath, as `table.field`. */
	children: string[];
	/** The variable the page's list sets to pick a record; removed with the page. */
	variable: string;
}

/** App-wide value that filter widgets and buttons set and other widgets read. */
export interface AppVariable {
	name: string;
	default: JsonValue;
}

export interface Layout {
	x: number;
	y: number;
	w: number;
	h: number;
}

/** A table or saved query (view) in one of the app's sources. */
export interface Binding {
	source: string;
	table: string;
}

/** Where a value comes from: typed in, an app variable, or a column of the row that was clicked. */
export type ValueRef =
	| { kind: 'literal'; value: JsonValue }
	| { kind: 'variable'; name: string }
	| { kind: 'column'; column: string };

export interface WidgetFilter {
	column: string;
	op: FilterOp;
	value: ValueRef;
}

export type Action =
	| { do: 'none' }
	| { do: 'navigate'; page: string; set: { name: string; value: ValueRef }[] }
	| { do: 'set-variable'; name: string; value: ValueRef }
	/** Opens the clicked row in the record editor (table rows only). */
	| { do: 'edit-record' }
	/** Starts a new record in a form or table widget. */
	| { do: 'new-record'; widget: string }
	| { do: 'refresh' }
	| { do: 'print' };

export type ActionKind = Action['do'];

export type ChartKind = 'bar' | 'horizontal-bar' | 'line' | 'area' | 'pie';

export interface ChartSeries {
	/** Null only for `count` (count of records). */
	column: string | null;
	agg: Aggregate;
}

interface WidgetBase {
	id: string;
	title: string;
	layout: Layout;
}

export interface TableWidget extends WidgetBase {
	type: 'table';
	data: Binding | null;
	/** Shown columns in order; empty shows all. */
	columns: string[];
	filters: WidgetFilter[];
	sort: SortSpec | null;
	pageSize: number;
	search: boolean;
	allowNew: boolean;
	/** The record editor a row click opens can delete the record. */
	allowDelete: boolean;
	rowAction: Action;
	/** Columns shown as colored labels, like a status. */
	badges: string[];
}

export interface FormWidget extends WidgetBase {
	type: 'form';
	data: Binding | null;
	/** Shown fields in order; empty shows all. */
	fields: string[];
	/** Which records the form steps through. */
	filters: WidgetFilter[];
	allowNew: boolean;
	allowEdit: boolean;
	allowDelete: boolean;
	/**
	 * Starts blank; Enter adds the record and clears the form for the next
	 * one. A record is edited when a filter on the key picks it (a click in a
	 * linked list); saving it clears that pick again.
	 */
	quickEntry: boolean;
}

/**
 * Records as cards in a column per value of `groupBy` (a kanban board).
 * Dragging a card to another column sets that field of the record.
 */
export interface BoardWidget extends WidgetBase {
	type: 'board';
	data: Binding | null;
	groupBy: string;
	/** Columns shown first, in this order, even while empty; other values found get columns after them. */
	lanes: string[];
	/** Field shown as each card's heading; the first text field when empty. */
	cardTitle: string;
	/** More fields shown on each card. */
	cardFields: string[];
	filters: WidgetFilter[];
	allowNew: boolean;
	/** The record editor a card click opens can delete the record. */
	allowDelete: boolean;
	cardAction: Action;
}

export interface ChartWidget extends WidgetBase {
	type: 'chart';
	data: Binding | null;
	chart: ChartKind;
	groupBy: { column: string; bucket: DateBucket | null } | null;
	series: ChartSeries[];
	filters: WidgetFilter[];
	order: AggregateOrder;
	limit: number;
}

export interface StatWidget extends WidgetBase {
	type: 'stat';
	data: Binding | null;
	series: ChartSeries;
	filters: WidgetFilter[];
	caption: string;
}

export type FilterControl = 'text' | 'number' | 'select' | 'date' | 'toggle';

export interface FilterWidget extends WidgetBase {
	type: 'filter';
	variable: string;
	control: FilterControl;
	/** Choices for `select`: values of one column, labelled by another. */
	options: { data: Binding | null; valueColumn: string; labelColumn: string };
	placeholder: string;
}

export type ButtonVariant = 'default' | 'outline' | 'secondary' | 'destructive';

export interface ButtonWidget extends WidgetBase {
	type: 'button';
	label: string;
	variant: ButtonVariant;
	action: Action;
}

export interface TextWidget extends WidgetBase {
	type: 'text';
	markdown: string;
}

/**
 * Results of a read-only SQL query. `:name` in the SQL reads the app variable
 * `name`; the backend binds it as a parameter.
 */
export interface QueryWidget extends WidgetBase {
	type: 'query';
	/** Alias of the source the query runs on; empty until chosen. */
	source: string;
	sql: string;
	search: boolean;
	rowAction: Action;
}

export type Widget = TableWidget | FormWidget | BoardWidget | ChartWidget | StatWidget | FilterWidget | ButtonWidget | TextWidget | QueryWidget;
export type WidgetType = Widget['type'];

export interface Page {
	id: string;
	title: string;
	/** One of `ICONS`. */
	icon: string;
	/** Heading the page is listed under in the sidebar; empty for none. */
	group: string;
	/** Left out of the sidebar; still reachable from buttons and links. */
	hidden: boolean;
	widgets: Widget[];
	/** Set on pages built from a table; null for pages made by hand. */
	origin: PageOrigin | null;
}

/** An action listed in the sidebar, like "New order" or "Print". */
export interface AppCommand {
	id: string;
	label: string;
	icon: string;
	action: Action;
}

export type AppAccent = 'neutral' | 'blue' | 'green' | 'amber' | 'red' | 'violet';

export interface AppDef {
	version: number;
	description: string;
	icon: string;
	accent: AppAccent;
	sources: AppSource[];
	variables: AppVariable[];
	pages: Page[];
	startPage: string;
	commands: AppCommand[];
}

export const WIDGET_TYPES: { type: WidgetType; label: string; description: string }[] = [
	{ type: 'table', label: 'Table', description: 'A list of records' },
	{ type: 'form', label: 'Form', description: 'View, add and edit one record at a time' },
	{ type: 'board', label: 'Board', description: 'Cards in columns, like a status; drag to move' },
	{ type: 'chart', label: 'Chart', description: 'Bar, line, area or pie chart of totals' },
	{ type: 'stat', label: 'Number', description: 'One total, like revenue this month' },
	{ type: 'filter', label: 'Filter', description: 'A control that sets a variable' },
	{ type: 'button', label: 'Button', description: 'Go to a page, add a record…' },
	{ type: 'text', label: 'Text', description: 'Headings, notes and instructions' },
	{ type: 'query', label: 'Query', description: 'Results of a SQL query' }
];

/** Icons for pages and commands (lucide names; see `AppIcon.svelte`). */
export const ICONS = [
	'file', 'house', 'layout-dashboard', 'table', 'file-text', 'list', 'search', 'chart-column', 'chart-pie',
	'users', 'user', 'building', 'shopping-cart', 'package', 'truck', 'receipt', 'wallet', 'credit-card',
	'calendar', 'clock', 'inbox', 'mail', 'phone', 'clipboard-list', 'square-check', 'briefcase', 'book-open',
	'folder', 'tag', 'star', 'heart', 'flag', 'map-pin', 'settings', 'plus', 'printer', 'refresh-cw', 'zap'
] as const;

/** Kinds of page offered by "New page"; each starts with ready-made widgets. */
export type PageKind = 'blank' | 'list' | 'form' | 'list-form' | 'query' | 'dashboard';

export const PAGE_KINDS: { kind: PageKind; label: string; description: string; icon: string; needsData: boolean }[] = [
	{ kind: 'list', label: 'List', description: 'Records of a table with search; click one to edit it', icon: 'table', needsData: true },
	{ kind: 'form', label: 'Form', description: 'Enter and edit records one at a time', icon: 'file-text', needsData: true },
	{ kind: 'list-form', label: 'List and form', description: 'Pick a record on the left, edit it on the right', icon: 'list', needsData: true },
	{ kind: 'query', label: 'Query', description: 'Results of a SQL query', icon: 'search', needsData: true },
	{ kind: 'dashboard', label: 'Dashboard', description: 'A count, a chart and recent records', icon: 'layout-dashboard', needsData: true },
	{ kind: 'blank', label: 'Blank', description: 'An empty page to build yourself', icon: 'file', needsData: false }
];

export const ACCENTS: AppAccent[] = ['neutral', 'blue', 'green', 'amber', 'red', 'violet'];

export function widgetLabel(type: WidgetType): string {
	return WIDGET_TYPES.find((w) => w.type === type)?.label ?? type;
}

/** Short random id, unique enough within one app. */
export function newId(prefix: string): string {
	return `${prefix}_${Math.random().toString(36).slice(2, 10)}`;
}

/** First free `base`, `base2`, `base3`… */
export function uniqueName(base: string, taken: string[]): string {
	const lower = new Set(taken.map((t) => t.toLowerCase()));
	if (!lower.has(base.toLowerCase())) return base;
	let n = 2;
	while (lower.has(`${base}${n}`.toLowerCase())) n++;
	return `${base}${n}`;
}

/** An alias derived from a database name: lower case letters, digits and underscores. */
export function aliasFor(database: string, taken: string[]): string {
	const base = database.toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_+|_+$/g, '') || 'db';
	return uniqueName(base, taken);
}

export function newApp(databases: string[]): AppDef {
	const sources: AppSource[] = [];
	for (const database of databases) {
		sources.push({ alias: aliasFor(database, sources.map((s) => s.alias)), database, auto: false });
	}
	const home: Page = { id: newId('page'), title: 'Home', icon: 'house', group: '', hidden: false, widgets: [], origin: null };
	return {
		version: APP_FORMAT_VERSION,
		description: '',
		icon: 'layout-dashboard',
		accent: 'neutral',
		sources,
		variables: [],
		pages: [home],
		startPage: home.id,
		commands: []
	};
}

// ----- Reading files -----

type Raw = Record<string, unknown>;

const isObj = (v: unknown): v is Raw => typeof v === 'object' && v !== null && !Array.isArray(v);
const str = (v: unknown, fallback = ''): string => (typeof v === 'string' ? v : fallback);
const bool = (v: unknown, fallback: boolean): boolean => (typeof v === 'boolean' ? v : fallback);
const int = (v: unknown, fallback: number, min = 0, max = Number.MAX_SAFE_INTEGER): number =>
	typeof v === 'number' && Number.isFinite(v) ? Math.min(max, Math.max(min, Math.round(v))) : fallback;
const oneOf = <T extends string>(v: unknown, options: readonly T[], fallback: T): T =>
	options.includes(v as T) ? (v as T) : fallback;
const arr = (v: unknown): unknown[] => (Array.isArray(v) ? v : []);
const strings = (v: unknown): string[] => arr(v).filter((c): c is string => typeof c === 'string');

const FILTER_OP_NAMES: FilterOp[] = [
	'eq', 'ne', 'lt', 'le', 'gt', 'ge', 'contains', 'starts_with', 'ends_with', 'is_null', 'is_not_null', 'in'
];
const AGGREGATE_NAMES: Aggregate[] = ['count', 'count_distinct', 'sum', 'avg', 'min', 'max'];
const DATE_BUCKETS: DateBucket[] = ['day', 'month', 'year'];
const CHART_KINDS: ChartKind[] =['bar', 'horizontal-bar', 'line', 'area', 'pie'];
const FILTER_CONTROLS: FilterControl[] = ['text', 'number', 'select', 'date', 'toggle'];
const BUTTON_VARIANTS: ButtonVariant[] = ['default', 'outline', 'secondary', 'destructive'];

function readLayout(v: unknown): Layout {
	const o = isObj(v) ? v : {};
	const w = int(o.w, 4, 1, GRID_COLUMNS);
	return { x: int(o.x, 0, 0, GRID_COLUMNS - w), y: int(o.y, 0, 0, 10_000), w, h: int(o.h, 4, 1, 200) };
}

function readOrigin(v: unknown): PageOrigin | null {
	if (!isObj(v) || typeof v.source !== 'string' || typeof v.table !== 'string') return null;
	return {
		source: v.source,
		table: v.table,
		kind: oneOf(v.kind, ['table', 'board'] as const, 'table'),
		fields: strings(v.fields),
		children: strings(v.children),
		variable: str(v.variable)
	};
}

function readBinding(v: unknown): Binding | null {
	return isObj(v) && typeof v.source === 'string' && typeof v.table === 'string'
		? { source: v.source, table: v.table }
		: null;
}

function readValueRef(v: unknown): ValueRef {
	if (isObj(v) && v.kind === 'variable' && typeof v.name === 'string') return { kind: 'variable', name: v.name };
	if (isObj(v) && v.kind === 'column' && typeof v.column === 'string') return { kind: 'column', column: v.column };
	return { kind: 'literal', value: isObj(v) && 'value' in v ? ((v.value ?? null) as JsonValue) : null };
}

function readFilters(v: unknown): WidgetFilter[] {
	return arr(v)
		.filter(isObj)
		.map((f) => ({ column: str(f.column), op: oneOf(f.op, FILTER_OP_NAMES, 'eq'), value: readValueRef(f.value) }));
}

function readSeries(v: unknown): ChartSeries {
	const o = isObj(v) ? v : {};
	return { column: typeof o.column === 'string' ? o.column : null, agg: oneOf(o.agg, AGGREGATE_NAMES, 'count') };
}

export function readAction(v: unknown): Action {
	const o = isObj(v) ? v : {};
	switch (o.do) {
		case 'navigate':
			return {
				do: 'navigate',
				page: str(o.page),
				set: arr(o.set)
					.filter(isObj)
					.map((s) => ({ name: str(s.name), value: readValueRef(s.value) }))
			};
		case 'set-variable':
			return { do: 'set-variable', name: str(o.name), value: readValueRef(o.value) };
		case 'new-record':
			return { do: 'new-record', widget: str(o.widget) };
		case 'edit-record':
		case 'refresh':
		case 'print':
			return { do: o.do };
		default:
			return { do: 'none' };
	}
}

function readWidget(v: unknown): Widget | null {
	if (!isObj(v)) return null;
	const base = { id: str(v.id) || newId('w'), title: str(v.title), layout: readLayout(v.layout) };
	switch (v.type) {
		case 'table':
			return {
				...base,
				type: 'table',
				data: readBinding(v.data),
				columns: strings(v.columns),
				filters: readFilters(v.filters),
				sort: isObj(v.sort) && typeof v.sort.column === 'string'
					? { column: v.sort.column, descending: bool(v.sort.descending, false) }
					: null,
				pageSize: int(v.pageSize, 25, 5, 500),
				search: bool(v.search, true),
				allowNew: bool(v.allowNew, true),
				allowDelete: bool(v.allowDelete, false),
				rowAction: readAction(v.rowAction),
				badges: strings(v.badges)
			};
		case 'form':
			return {
				...base,
				type: 'form',
				data: readBinding(v.data),
				fields: strings(v.fields),
				filters: readFilters(v.filters),
				allowNew: bool(v.allowNew, true),
				allowEdit: bool(v.allowEdit, true),
				allowDelete: bool(v.allowDelete, false),
				quickEntry: bool(v.quickEntry, true)
			};
		case 'board':
			return {
				...base,
				type: 'board',
				data: readBinding(v.data),
				groupBy: str(v.groupBy),
				lanes: strings(v.lanes),
				cardTitle: str(v.cardTitle),
				cardFields: strings(v.cardFields),
				filters: readFilters(v.filters),
				allowNew: bool(v.allowNew, true),
				allowDelete: bool(v.allowDelete, false),
				cardAction: isObj(v.cardAction) ? readAction(v.cardAction) : { do: 'edit-record' }
			};
		case 'chart': {
			const g = isObj(v.groupBy) && typeof v.groupBy.column === 'string' ? v.groupBy : null;
			return {
				...base,
				type: 'chart',
				data: readBinding(v.data),
				chart: oneOf(v.chart, CHART_KINDS, 'bar'),
				groupBy: g
					? {
							column: g.column as string,
							bucket: DATE_BUCKETS.includes(g.bucket as DateBucket) ? (g.bucket as DateBucket) : null
						}
					: null,
				series: arr(v.series).map(readSeries),
				filters: readFilters(v.filters),
				order: oneOf(v.order, ['group', 'value_asc', 'value_desc'] as const, 'group'),
				limit: int(v.limit, 50, 1, 1000)
			};
		}
		case 'stat':
			return {
				...base,
				type: 'stat',
				data: readBinding(v.data),
				series: readSeries(v.series),
				filters: readFilters(v.filters),
				caption: str(v.caption)
			};
		case 'filter': {
			const o = isObj(v.options) ? v.options : {};
			return {
				...base,
				type: 'filter',
				variable: str(v.variable),
				control: oneOf(v.control, FILTER_CONTROLS, 'text'),
				options: { data: readBinding(o.data), valueColumn: str(o.valueColumn), labelColumn: str(o.labelColumn) },
				placeholder: str(v.placeholder)
			};
		}
		case 'button':
			return {
				...base,
				type: 'button',
				label: str(v.label, 'Button'),
				variant: oneOf(v.variant, BUTTON_VARIANTS, 'default'),
				action: readAction(v.action)
			};
		case 'text':
			return { ...base, type: 'text', markdown: str(v.markdown) };
		case 'query':
			return {
				...base,
				type: 'query',
				source: str(v.source),
				sql: str(v.sql),
				search: bool(v.search, true),
				rowAction: readAction(v.rowAction)
			};
		default:
			// Unknown widget types (from a newer version) are dropped.
			return null;
	}
}

/**
 * Reads an app definition loaded from disk, filling in defaults so the rest
 * of the app can rely on every field being present. Throws when the file is
 * not an app definition at all.
 */
export function readApp(raw: unknown): AppDef {
	if (!isObj(raw) || typeof raw.version !== 'number' || !Array.isArray(raw.pages)) {
		throw new Error('This file is not an app definition.');
	}
	if (raw.version > APP_FORMAT_VERSION) {
		throw new Error('This app was made by a newer version of SlateDB.');
	}
	const pages: Page[] = arr(raw.pages)
		.filter(isObj)
		.map((p) => ({
			id: str(p.id) || newId('page'),
			title: str(p.title, 'Page'),
			icon: oneOf(p.icon, ICONS, 'file'),
			group: str(p.group).trim(),
			hidden: bool(p.hidden, false),
			widgets: arr(p.widgets).map(readWidget).filter((w): w is Widget => w !== null),
			origin: readOrigin(p.origin)
		}));
	if (!pages.length) pages.push({ id: newId('page'), title: 'Home', icon: 'house', group: '', hidden: false, widgets: [], origin: null });
	const startPage = pages.some((p) => p.id === raw.startPage) ? (raw.startPage as string) : pages[0].id;
	return {
		version: APP_FORMAT_VERSION,
		description: str(raw.description),
		icon: str(raw.icon, 'layout-dashboard'),
		accent: oneOf(raw.accent, ACCENTS, 'neutral'),
		sources: arr(raw.sources)
			.filter(isObj)
			.map((s) => ({ alias: str(s.alias), database: str(s.database), auto: bool(s.auto, false) }))
			.filter((s) => s.alias && s.database),
		variables: arr(raw.variables)
			.filter(isObj)
			.map((v) => ({ name: str(v.name), default: (v.default ?? null) as JsonValue }))
			.filter((v) => v.name),
		pages,
		startPage,
		commands: arr(raw.commands)
			.filter(isObj)
			.map((c) => ({
				id: str(c.id) || newId('cmd'),
				label: str(c.label, 'Action'),
				icon: oneOf(c.icon, ICONS, 'zap'),
				action: readAction(c.action)
			}))
	};
}

// ----- Widgets -----

/** Default size of a new widget, in grid cells. */
export function defaultSize(type: WidgetType): { w: number; h: number } {
	switch (type) {
		case 'table':
			return { w: 12, h: 8 };
		case 'form':
			return { w: 6, h: 9 };
		case 'board':
			return { w: 12, h: 9 };
		case 'chart':
			return { w: 6, h: 7 };
		case 'stat':
			return { w: 3, h: 3 };
		case 'filter':
			return { w: 3, h: 2 };
		case 'button':
			return { w: 2, h: 1 };
		case 'text':
			return { w: 12, h: 2 };
		case 'query':
			return { w: 12, h: 8 };
	}
}

/** A new widget of `type` at `layout`, bound to `data` when it shows data. */
export function newWidget(type: WidgetType, layout: Layout, data: Binding | null = null): Widget {
	const base = { id: newId('w'), title: '', layout };
	switch (type) {
		case 'table':
			return {
				...base,
				type,
				title: data?.table ?? 'Table',
				data,
				columns: [],
				filters: [],
				sort: null,
				pageSize: 25,
				search: true,
				allowNew: true,
				allowDelete: false,
				rowAction: { do: 'edit-record' },
				badges: []
			};
		case 'form':
			return {
				...base,
				type,
				title: data?.table ?? 'Form',
				data,
				fields: [],
				filters: [],
				allowNew: true,
				allowEdit: true,
				allowDelete: false,
				quickEntry: true
			};
		case 'board':
			return {
				...base,
				type,
				title: data?.table ?? 'Board',
				data,
				groupBy: '',
				lanes: [],
				cardTitle: '',
				cardFields: [],
				filters: [],
				allowNew: true,
				allowDelete: false,
				cardAction: { do: 'edit-record' }
			};
		case 'chart':
			return {
				...base,
				type,
				title: 'Chart',
				data,
				chart: 'bar',
				groupBy: null,
				series: [{ column: null, agg: 'count' }],
				filters: [],
				order: 'group',
				limit: 50
			};
		case 'stat':
			return { ...base, type, title: 'Number', data, series: { column: null, agg: 'count' }, filters: [], caption: '' };
		case 'filter':
			return {
				...base,
				type,
				title: 'Filter',
				variable: '',
				control: 'text',
				options: { data: null, valueColumn: '', labelColumn: '' },
				placeholder: ''
			};
		case 'button':
			return { ...base, type, label: 'Button', variant: 'default', action: { do: 'none' } };
		case 'text':
			return { ...base, type, markdown: '## Heading\n\nWrite a note here.' };
		case 'query':
			return {
				...base,
				type,
				title: 'Query',
				source: data?.source ?? '',
				sql: data ? `SELECT *\nFROM "${data.table.replaceAll('"', '""')}"` : '',
				search: true,
				rowAction: { do: 'none' }
			};
	}
}

export function findPage(app: AppDef, pageId: string): Page | undefined {
	return app.pages.find((p) => p.id === pageId);
}

export function findWidget(app: AppDef, widgetId: string): { page: Page; widget: Widget } | undefined {
	for (const page of app.pages) {
		const widget = page.widgets.find((w) => w.id === widgetId);
		if (widget) return { page, widget };
	}
	return undefined;
}

/** The binding a widget reads data from, if any. */
export function bindingOf(w: Widget): Binding | null {
	switch (w.type) {
		case 'table':
		case 'form':
		case 'board':
		case 'chart':
		case 'stat':
			return w.data;
		case 'filter':
			return w.control === 'select' ? w.options.data : null;
		default:
			return null;
	}
}

/** Every action a widget can run. */
function actionsOf(w: Widget): Action[] {
	if (w.type === 'button') return [w.action];
	if (w.type === 'table' || w.type === 'query') return [w.rowAction];
	if (w.type === 'board') return [w.cardAction];
	return [];
}

/** Removes a widget and the "new record" actions that pointed at it. */
export function removeWidget(app: AppDef, widgetId: string): void {
	for (const page of app.pages) page.widgets = page.widgets.filter((w) => w.id !== widgetId);
	forEachAction(app, (a) => (a.do === 'new-record' && a.widget === widgetId ? { do: 'none' } : a));
}

function forEachAction(app: AppDef, change: (a: Action) => Action): void {
	for (const page of app.pages) {
		for (const w of page.widgets) {
			if (w.type === 'button') w.action = change(w.action);
			if (w.type === 'table' || w.type === 'query') w.rowAction = change(w.rowAction);
			if (w.type === 'board') w.cardAction = change(w.cardAction);
		}
	}
	for (const c of app.commands) c.action = change(c.action);
}

/** A widget on the clipboard, with what it needs from the app it was copied from. */
export interface CopiedWidget {
	widget: Widget;
	sources: AppSource[];
	variables: AppVariable[];
}

/**
 * Makes a copied widget work in `app` (perhaps another app): its data is
 * read from the same database, added to `app` when it isn't there yet; the
 * variables it reads or sets are created when missing; and actions that go
 * to a page or widget `app` doesn't have do nothing. Changes `app` and the
 * widget, which should be a fresh copy.
 */
export function adaptPastedWidget(app: AppDef, copied: CopiedWidget): Widget {
	const w = copied.widget;
	const alias = (from: string): string => {
		const database = copied.sources.find((s) => s.alias === from)?.database;
		return database ? addSource(app, database).alias : from;
	};
	const rebind = (b: Binding | null) => b && { ...b, source: alias(b.source) };
	if (w.type === 'filter') w.options.data = rebind(w.options.data);
	else if (w.type === 'query') w.source = w.source && alias(w.source);
	else if ('data' in w) w.data = rebind(w.data);

	const used = [...variablesRead(w), ...(w.type === 'filter' && w.variable ? [w.variable] : []), ...actionsOf(w).flatMap(variablesSet)];
	for (const name of new Set(used)) {
		if (!name || app.variables.some((v) => v.name === name)) continue;
		addVariable(app, name, jsonClone(copied.variables.find((v) => v.name === name)?.default ?? null));
	}

	const fix = (a: Action): Action =>
		(a.do === 'navigate' && !findPage(app, a.page)) || (a.do === 'new-record' && !findWidget(app, a.widget)) ? { do: 'none' } : a;
	if (w.type === 'button') w.action = fix(w.action);
	if (w.type === 'table' || w.type === 'query') w.rowAction = fix(w.rowAction);
	if (w.type === 'board') w.cardAction = fix(w.cardAction);
	return w;
}

// ----- Pages -----

export function addPage(app: AppDef, title: string, icon = 'file', group = ''): Page {
	const page: Page = { id: newId('page'), title, icon, group, hidden: false, widgets: [], origin: null };
	app.pages.push(page);
	return page;
}

/**
 * Adds a page of `kind` whose starting widgets read `data` (for a query page,
 * the table or saved query the SQL starts from). A list-and-form page links
 * its table to its form through the table's `key` column.
 */
export function addPageOfKind(
	app: AppDef,
	kind: PageKind,
	title: string,
	data: Binding | null,
	options: { group?: string; key?: string } = {}
): Page {
	const icon = PAGE_KINDS.find((k) => k.kind === kind)?.icon ?? 'file';
	const page = addPage(app, title, icon, options.group ?? '');
	if (!data) return page;
	const put = <T extends Widget>(type: T['type'], layout: Layout, set?: (w: T) => void) => {
		const w = newWidget(type, layout, data) as T;
		set?.(w);
		page.widgets.push(w);
	};
	switch (kind) {
		case 'list':
			put<TableWidget>('table', { x: 0, y: 0, w: 12, h: 12 }, (w) => (w.pageSize = 50));
			break;
		case 'form':
			put<FormWidget>('form', { x: 0, y: 0, w: 8, h: 12 });
			break;
		case 'list-form': {
			const key = options.key ?? 'id';
			const base = data.table.replace(/[^A-Za-z0-9_]+/g, '_').replace(/^[^A-Za-z_]+/, '').toLowerCase() || 'record';
			const variable = uniqueName(`selected_${base}`, app.variables.map((v) => v.name));
			addVariable(app, variable, null);
			put<TableWidget>('table', { x: 0, y: 0, w: 7, h: 12 }, (w) => {
				w.allowNew = false;
				w.rowAction = { do: 'set-variable', name: variable, value: { kind: 'column', column: key } };
			});
			put<FormWidget>('form', { x: 7, y: 0, w: 5, h: 12 }, (w) => {
				w.filters = [{ column: key, op: 'eq', value: { kind: 'variable', name: variable } }];
			});
			break;
		}
		case 'query':
			put<QueryWidget>('query', { x: 0, y: 0, w: 12, h: 12 });
			break;
		case 'dashboard':
			put<StatWidget>('stat', { x: 0, y: 0, w: 3, h: 3 }, (w) => (w.title = w.caption = data.table));
			put<ChartWidget>('chart', { x: 3, y: 0, w: 9, h: 7 });
			put<TableWidget>('table', { x: 0, y: 7, w: 12, h: 8 }, (w) => {
				w.title = `Latest ${data.table}`;
				w.pageSize = 10;
			});
			break;
	}
	return page;
}

/** A copy of a page with new ids, placed after it. Widget links inside the copy point at the copy's widgets. */
export function duplicatePage(app: AppDef, pageId: string, title: string): Page | undefined {
	const index = app.pages.findIndex((p) => p.id === pageId);
	if (index < 0) return undefined;
	// A copy is the user's own page: automatic updates leave it alone.
	const copy: Page = { ...jsonClone(app.pages[index]), id: newId('page'), title, origin: null };
	const ids = new Map<string, string>();
	for (const w of copy.widgets) {
		const id = newId('w');
		ids.set(w.id, id);
		w.id = id;
	}
	for (const w of copy.widgets) {
		for (const key of ['action', 'rowAction', 'cardAction'] as const) {
			const a = (w as unknown as Record<string, Action | undefined>)[key];
			if (a?.do === 'new-record' && ids.has(a.widget)) (w as unknown as Record<string, Action>)[key] = { ...a, widget: ids.get(a.widget)! };
		}
	}
	app.pages.splice(index + 1, 0, copy);
	return copy;
}

/** Sidebar groups in order of first appearance, each with its pages; ungrouped pages come first under "". */
export function pageGroups(pages: Page[], includeHidden: boolean): { group: string; pages: Page[] }[] {
	const groups: { group: string; pages: Page[] }[] = [{ group: '', pages: [] }];
	for (const p of pages) {
		if (p.hidden && !includeHidden) continue;
		let g = groups.find((x) => x.group.toLowerCase() === p.group.toLowerCase());
		if (!g) groups.push((g = { group: p.group, pages: [] }));
		g.pages.push(p);
	}
	return groups.filter((g) => g.pages.length || g.group === '');
}

/** Moves page `pageId` before `beforeId` (or to the end) and into that spot's group. */
export function dropPage(app: AppDef, pageId: string, beforeId: string | null, group: string): void {
	const from = app.pages.findIndex((p) => p.id === pageId);
	if (from < 0) return;
	app.pages[from].group = group;
	if (pageId === beforeId) return;
	const [page] = app.pages.splice(from, 1);
	const to = beforeId ? app.pages.findIndex((p) => p.id === beforeId) : -1;
	app.pages.splice(to < 0 ? app.pages.length : to, 0, page);
}

/**
 * Removes a page (never the last one); buttons that went there, or added
 * records to one of its widgets, do nothing instead.
 */
export function removePage(app: AppDef, pageId: string): void {
	if (app.pages.length <= 1) return;
	const widgets = new Set(findPage(app, pageId)?.widgets.map((w) => w.id) ?? []);
	app.pages = app.pages.filter((p) => p.id !== pageId);
	if (app.startPage === pageId) app.startPage = app.pages[0].id;
	forEachAction(app, (a) =>
		(a.do === 'navigate' && a.page === pageId) || (a.do === 'new-record' && widgets.has(a.widget)) ? { do: 'none' } : a
	);
}

export function movePage(app: AppDef, pageId: string, toIndex: number): void {
	const from = app.pages.findIndex((p) => p.id === pageId);
	if (from < 0) return;
	const [page] = app.pages.splice(from, 1);
	app.pages.splice(Math.max(0, Math.min(app.pages.length, toIndex)), 0, page);
}

// ----- Commands -----

export function addCommand(app: AppDef, label: string): AppCommand {
	const command: AppCommand = { id: newId('cmd'), label, icon: 'zap', action: { do: 'none' } };
	app.commands.push(command);
	return command;
}

export function findCommand(app: AppDef, id: string): AppCommand | undefined {
	return app.commands.find((c) => c.id === id);
}

export function removeCommand(app: AppDef, id: string): void {
	app.commands = app.commands.filter((c) => c.id !== id);
}

export function moveCommand(app: AppDef, id: string, toIndex: number): void {
	const from = app.commands.findIndex((c) => c.id === id);
	if (from < 0) return;
	const [c] = app.commands.splice(from, 1);
	app.commands.splice(Math.max(0, Math.min(app.commands.length, toIndex)), 0, c);
}

// ----- Query parameters -----

/**
 * Rewrites the `:name` / `@name` / `$name` parameters of `sql`, skipping
 * quoted text and comments. `change` returns the new name, or the same one.
 */
export function mapSqlParameters(sql: string, change: (name: string) => string): string {
	let out = '';
	let i = 0;
	while (i < sql.length) {
		const c = sql[i];
		if (c === "'" || c === '"' || c === '`' || c === '[') {
			const close = c === '[' ? ']' : c;
			let j = i + 1;
			while (j < sql.length) {
				if (sql[j] === close) {
					if (close !== ']' && sql[j + 1] === close) j += 2;
					else break;
				} else j++;
			}
			out += sql.slice(i, j + 1);
			i = j + 1;
		} else if (sql.startsWith('--', i)) {
			const end = sql.indexOf('\n', i);
			const j = end < 0 ? sql.length : end;
			out += sql.slice(i, j);
			i = j;
		} else if (sql.startsWith('/*', i)) {
			const end = sql.indexOf('*/', i + 2);
			const j = end < 0 ? sql.length : end + 2;
			out += sql.slice(i, j);
			i = j;
		} else if ((c === ':' || c === '@' || c === '$') && /[A-Za-z_]/.test(sql[i + 1] ?? '')) {
			let j = i + 1;
			while (j < sql.length && /[A-Za-z0-9_]/.test(sql[j])) j++;
			out += c + change(sql.slice(i + 1, j));
			i = j;
		} else {
			out += c;
			i++;
		}
	}
	return out;
}

/** Names of the parameters a query reads. */
export function sqlParameters(sql: string): string[] {
	const names = new Set<string>();
	mapSqlParameters(sql, (n) => (names.add(n), n));
	return [...names];
}

// ----- Sources -----

export function addSource(app: AppDef, database: string): AppSource {
	const existing = app.sources.find((s) => s.database === database);
	if (existing) return existing;
	const source = { alias: aliasFor(database, app.sources.map((s) => s.alias)), database, auto: false };
	app.sources.push(source);
	return source;
}

/** Widgets reading from `alias`. */
export function widgetsUsingSource(app: AppDef, alias: string): Widget[] {
	return app.pages.flatMap((p) => p.widgets).filter((w) => (w.type === 'query' ? w.source : bindingOf(w)?.source) === alias);
}

/** Removes a source; widgets that read from it lose their data binding. */
export function removeSource(app: AppDef, alias: string): void {
	app.sources = app.sources.filter((s) => s.alias !== alias);
	for (const page of app.pages) {
		for (const w of page.widgets) {
			if (w.type === 'filter' && w.options.data?.source === alias) w.options.data = null;
			else if (w.type === 'query' && w.source === alias) w.source = '';
			else if ('data' in w && w.data?.source === alias) w.data = null;
		}
	}
}

export function sourceOf(app: AppDef, alias: string): AppSource | undefined {
	return app.sources.find((s) => s.alias === alias);
}

export function renameDatabaseInApp(app: AppDef, from: string, to: string): boolean {
	let changed = false;
	for (const s of app.sources) {
		if (s.database === from) {
			s.database = to;
			changed = true;
		}
	}
	return changed;
}

// ----- Variables -----

export function variableProblem(app: AppDef, name: string, except?: string): string | null {
	if (!name.trim()) return 'Enter a name';
	if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) return 'Use letters, digits and underscores, starting with a letter';
	if (app.variables.some((v) => v.name !== except && v.name.toLowerCase() === name.toLowerCase())) {
		return 'A variable with this name already exists';
	}
	return null;
}

export function addVariable(app: AppDef, name: string, value: JsonValue = null): AppVariable {
	const variable = { name, default: value };
	app.variables.push(variable);
	return variable;
}

function mapRefs(app: AppDef, change: (r: ValueRef) => ValueRef): void {
	for (const page of app.pages) {
		for (const w of page.widgets) {
			if ('filters' in w) for (const f of w.filters) f.value = change(f.value);
		}
	}
	forEachAction(app, (a) => {
		if (a.do === 'set-variable') return { ...a, value: change(a.value) };
		if (a.do === 'navigate') return { ...a, set: a.set.map((s) => ({ ...s, value: change(s.value) })) };
		return a;
	});
}

/** Renames a variable everywhere it is set or read. */
export function renameVariable(app: AppDef, from: string, to: string): void {
	for (const v of app.variables) if (v.name === from) v.name = to;
	for (const page of app.pages) {
		for (const w of page.widgets) {
			if (w.type === 'filter' && w.variable === from) w.variable = to;
			if (w.type === 'query') w.sql = mapSqlParameters(w.sql, (n) => (n === from ? to : n));
		}
	}
	forEachAction(app, (a) => {
		if (a.do === 'set-variable' && a.name === from) return { ...a, name: to };
		if (a.do === 'navigate') return { ...a, set: a.set.map((s) => (s.name === from ? { ...s, name: to } : s)) };
		return a;
	});
	mapRefs(app, (r) => (r.kind === 'variable' && r.name === from ? { kind: 'variable', name: to } : r));
}

/** Removes a variable; filters that read it are removed, buttons that set it no longer do. */
export function removeVariable(app: AppDef, name: string): void {
	app.variables = app.variables.filter((v) => v.name !== name);
	for (const page of app.pages) {
		for (const w of page.widgets) {
			if (w.type === 'filter' && w.variable === name) w.variable = '';
			if ('filters' in w) w.filters = w.filters.filter((f) => !(f.value.kind === 'variable' && f.value.name === name));
		}
	}
	forEachAction(app, (a) => {
		if (a.do === 'set-variable' && a.name === name) return { do: 'none' };
		if (a.do === 'navigate') return { ...a, set: a.set.filter((s) => s.name !== name) };
		return a;
	});
}

// ----- Structure changes in databases -----

function aliasesOf(app: AppDef, database: string): Set<string> {
	return new Set(app.sources.filter((s) => s.database === database).map((s) => s.alias));
}

/** Points widgets at table `to` after table `from` of `database` was renamed. Returns true if anything changed. */
export function renameTableInApp(app: AppDef, database: string, from: string, to: string): boolean {
	const aliases = aliasesOf(app, database);
	let changed = false;
	const fix = (b: Binding | null) => {
		if (b && aliases.has(b.source) && b.table === from) {
			b.table = to;
			changed = true;
		}
	};
	for (const page of app.pages) {
		for (const w of page.widgets) {
			if (w.type === 'filter') fix(w.options.data);
			else if ('data' in w) fix(w.data);
		}
		const o = page.origin;
		if (o && aliases.has(o.source)) {
			if (o.table === from) {
				o.table = to;
				// A page still named after its table follows the new name.
				if (page.title === from) page.title = to;
				else if (page.title === `${from} board`) page.title = `${to} board`;
				changed = true;
			}
			o.children = o.children.map((c) => (c.startsWith(`${from}.`) ? `${to}${c.slice(from.length)}` : c));
		}
	}
	return changed;
}

/** Renames field `from` of `table` in `database` wherever widgets use it. Returns true if anything changed. */
export function renameFieldInApp(app: AppDef, database: string, table: string, from: string, to: string): boolean {
	const aliases = aliasesOf(app, database);
	let changed = false;
	const on = (b: Binding | null) => !!b && aliases.has(b.source) && b.table === table;
	const name = (c: string) => {
		if (c !== from) return c;
		changed = true;
		return to;
	};
	const ref = (r: ValueRef): ValueRef => (r.kind === 'column' ? { kind: 'column', column: name(r.column) } : r);
	const action = (a: Action): Action => {
		if (a.do === 'set-variable') return { ...a, value: ref(a.value) };
		if (a.do === 'navigate') return { ...a, set: a.set.map((s) => ({ ...s, value: ref(s.value) })) };
		return a;
	};
	for (const page of app.pages) {
		const o = page.origin;
		if (o && aliases.has(o.source)) {
			if (o.table === table) o.fields = o.fields.map(name);
			o.children = o.children.map((c) => (c === `${table}.${from}` ? `${table}.${to}` : c));
		}
		for (const w of page.widgets) {
			if (w.type === 'filter') {
				if (on(w.options.data)) {
					w.options.valueColumn = name(w.options.valueColumn);
					w.options.labelColumn = name(w.options.labelColumn);
				}
				continue;
			}
			if (!('data' in w) || !on(w.data)) continue;
			for (const f of w.filters) f.column = name(f.column);
			switch (w.type) {
				case 'table':
					w.columns = w.columns.map(name);
					w.badges = w.badges.map(name);
					if (w.sort) w.sort.column = name(w.sort.column);
					// Column values passed on by a row click come from this table.
					w.rowAction = action(w.rowAction);
					break;
				case 'board':
					w.groupBy = name(w.groupBy);
					w.cardTitle = name(w.cardTitle);
					w.cardFields = w.cardFields.map(name);
					w.cardAction = action(w.cardAction);
					break;
				case 'form':
					w.fields = w.fields.map(name);
					break;
				case 'chart':
					if (w.groupBy) w.groupBy.column = name(w.groupBy.column);
					for (const s of w.series) if (s.column) s.column = name(s.column);
					break;
				case 'stat':
					if (w.series.column) w.series.column = name(w.series.column);
					break;
			}
		}
	}
	return changed;
}

// ----- Running -----

export type Variables = Record<string, JsonValue>;

export function initialVariables(app: AppDef): Variables {
	return Object.fromEntries(app.variables.map((v) => [v.name, jsonClone(v.default)]));
}

/** True for values that mean "no choice made", which switch a filter off. */
export function isBlank(v: JsonValue | undefined): boolean {
	return v === null || v === undefined || v === '' || (Array.isArray(v) && v.length === 0);
}

/**
 * The value a reference stands for. `row` maps column names to the values of
 * the clicked row. Undefined when it can't be known (e.g. no row).
 */
export function resolveValue(ref: ValueRef, vars: Variables, row?: Record<string, JsonValue>): JsonValue | undefined {
	switch (ref.kind) {
		case 'literal':
			return ref.value;
		case 'variable':
			return vars[ref.name];
		case 'column':
			return row ? (row[ref.column] ?? null) : undefined;
	}
}

/**
 * Backend filters for a widget. A filter whose value comes from an empty
 * variable is left out, so an unset filter control shows everything.
 */
export function resolveFilters(filters: WidgetFilter[], vars: Variables): FilterSpec[] {
	const out: FilterSpec[] = [];
	for (const f of filters) {
		if (!f.column) continue;
		if (!needsValue(f.op)) {
			out.push({ column: f.column, op: f.op, value: null });
			continue;
		}
		const value = resolveValue(f.value, vars);
		if (f.value.kind === 'variable' ? isBlank(value) : value === undefined) continue;
		if (f.op === 'in' && typeof value === 'string') {
			out.push({ column: f.column, op: 'in', value: value.split(',').map((s) => s.trim()).filter(Boolean) });
		} else {
			out.push({ column: f.column, op: f.op, value: value ?? null });
		}
	}
	return out;
}

/**
 * Values a new record starts with: the fields an "is" filter pins down, so a
 * record added under a selected customer belongs to that customer.
 */
export function prefillFromFilters(filters: WidgetFilter[], vars: Variables): Record<string, JsonValue> {
	const out: Record<string, JsonValue> = {};
	for (const f of resolveFilters(filters, vars)) {
		if (f.op === 'eq' && f.value !== null && f.value !== undefined) out[f.column] = f.value;
	}
	return out;
}

/** Variables a widget reads; it reloads when one of them changes. */
export function variablesRead(w: Widget): string[] {
	if (w.type === 'query') return sqlParameters(w.sql);
	const names = 'filters' in w ? w.filters.flatMap((f) => (f.value.kind === 'variable' ? [f.value.name] : [])) : [];
	return [...new Set(names)];
}

/** Variables an action sets. */
export function variablesSet(a: Action): string[] {
	if (a.do === 'set-variable') return [a.name];
	if (a.do === 'navigate') return a.set.map((s) => s.name);
	return [];
}

/** A series the backend accepts: without a field, only counting records makes sense. */
export function usableSeries(s: ChartSeries): ChartSeries {
	return s.column ? s : { column: null, agg: 'count' };
}

// ----- Checking bindings -----

/** What is known about the app's databases; undefined means not loaded (yet). */
export interface SchemaLookup {
	/** Names of the databases in the workspace. */
	databases: string[];
	/** Tables and views of a source. */
	objects(alias: string): { name: string; kind: 'table' | 'view' }[] | undefined;
	/** Column names of a table or view of a source. */
	columns(alias: string, table: string): string[] | undefined;
}

/**
 * Problems with a widget's bindings: a missing database, table or column,
 * a variable that no longer exists, a button pointing at a removed page.
 * Each problem is a sentence for the user.
 */
export function widgetProblems(app: AppDef, w: Widget, schema: SchemaLookup): string[] {
	const problems: string[] = [];
	if (w.type === 'query') {
		const source = w.source ? sourceOf(app, w.source) : undefined;
		if (!w.source) problems.push('Choose the database the query runs on.');
		else if (!source) problems.push('Its database was removed from the app.');
		else if (!schema.databases.includes(source.database)) problems.push(`The database “${source.database}” is missing from the folder.`);
		if (!w.sql.trim()) problems.push('Write the query.');
	}
	const binding = bindingOf(w);
	const needsData = w.type === 'table' || w.type === 'form' || w.type === 'board' || w.type === 'chart' || w.type === 'stat';
	if (needsData && !binding) problems.push('Choose where the data comes from.');
	if (w.type === 'board' && binding && !w.groupBy) problems.push('Choose the field the columns come from.');
	if (w.type === 'filter' && w.control === 'select' && !binding) problems.push('Choose where the choices come from.');
	if (binding) {
		const source = sourceOf(app, binding.source);
		if (!source) {
			problems.push('Its database was removed from the app.');
		} else if (!schema.databases.includes(source.database)) {
			problems.push(`The database “${source.database}” is missing from the folder.`);
		} else {
			const objects = schema.objects(binding.source);
			const object = objects?.find((o) => o.name === binding.table);
			if (objects && !object) {
				problems.push(`“${binding.table}” no longer exists in ${source.database}.`);
			} else if ((w.type === 'form' || w.type === 'board') && object?.kind === 'view') {
				problems.push(`“${binding.table}” is a saved query; ${w.type === 'form' ? 'forms' : 'boards'} need a table.`);
			} else {
				const columns = schema.columns(binding.source, binding.table);
				if (columns) {
					const missing = columnsUsed(w).filter((c) => !columns.includes(c));
					if (missing.length) {
						problems.push(
							`${missing.length === 1 ? 'Field' : 'Fields'} ${missing.map((m) => `“${m}”`).join(', ')} no longer ${missing.length === 1 ? 'exists' : 'exist'}.`
						);
					}
				}
			}
		}
	}
	const vars = new Set(app.variables.map((v) => v.name));
	const unknownVars = [
		...variablesRead(w),
		...(w.type === 'filter' && w.variable ? [w.variable] : []),
		...actionsOf(w).flatMap(variablesSet)
	].filter((v) => v && !vars.has(v));
	if (unknownVars.length) problems.push(`Variable ${[...new Set(unknownVars)].map((v) => `“${v}”`).join(', ')} doesn't exist.`);
	if (w.type === 'filter' && !w.variable) problems.push('Choose the variable this filter sets.');
	for (const a of actionsOf(w)) problems.push(...actionProblems(app, a));
	return problems;
}

function actionProblems(app: AppDef, a: Action): string[] {
	const problems: string[] = [];
	if (a.do === 'navigate' && !findPage(app, a.page)) problems.push('The page this goes to was removed.');
	if (a.do === 'new-record' && !findWidget(app, a.widget)) problems.push('The widget this adds records to was removed.');
	return problems;
}

/** Problems with a sidebar command, as sentences. */
export function commandProblems(app: AppDef, c: AppCommand): string[] {
	const vars = new Set(app.variables.map((v) => v.name));
	const unknown = variablesSet(c.action).filter((v) => v && !vars.has(v));
	return [
		...actionProblems(app, c.action),
		...(unknown.length ? [`Variable ${unknown.map((v) => `“${v}”`).join(', ')} doesn't exist.`] : [])
	];
}

/** Columns of the clicked row an action passes on. */
function columnRefs(a: Action): string[] {
	const refs = a.do === 'set-variable' ? [a.value] : a.do === 'navigate' ? a.set.map((s) => s.value) : [];
	return refs.flatMap((r) => (r.kind === 'column' ? [r.column] : []));
}

/** Columns of its bound table a widget refers to. */
export function columnsUsed(w: Widget): string[] {
	const cols: string[] = [];
	if ('filters' in w) cols.push(...w.filters.map((f) => f.column).filter(Boolean));
	switch (w.type) {
		case 'table':
			cols.push(...w.columns, ...w.badges);
			if (w.sort) cols.push(w.sort.column);
			cols.push(...columnRefs(w.rowAction));
			break;
		case 'board':
			cols.push(...[w.groupBy, w.cardTitle].filter(Boolean), ...w.cardFields, ...columnRefs(w.cardAction));
			break;
		case 'form':
			cols.push(...w.fields);
			break;
		case 'chart':
			if (w.groupBy) cols.push(w.groupBy.column);
			for (const s of w.series) if (s.column) cols.push(s.column);
			break;
		case 'stat':
			if (w.series.column) cols.push(w.series.column);
			break;
		case 'filter':
			if (w.control === 'select') cols.push(...[w.options.valueColumn, w.options.labelColumn].filter(Boolean));
			break;
	}
	return [...new Set(cols)];
}
