import { toast } from 'svelte-sonner';
import type { FieldType, JsonValue, ObjectSummary, TableDef } from '../api/types';
import {
	commandProblems,
	defaultSize,
	findCommand,
	findPage,
	findWidget,
	newWidget,
	removeWidget,
	adaptPastedWidget,
	initialVariables,
	newId,
	readApp,
	resolveValue,
	sourceOf,
	widgetProblems,
	type Action,
	type AppCommand,
	type AppDef,
	type CopiedWidget,
	type SchemaLookup,
	type Binding,
	type Variables,
	type Widget,
	type WidgetType
} from '../logic/app-def';
import { generatePages, syncGeneratedPages } from '../logic/app-generate';
import { compact, findSpot, place } from '../logic/app-layout';
import { History } from '../logic/history';
import { jsonClone } from '../logic/values';
import { appService } from '../services/apps';
import { databaseService } from '../services/database';
import { recordService } from '../services/records';
import { tableService } from '../services/tables';
import { workspaceService } from '../services/workspace';
import { attempt, reportError } from './feedback.svelte';

export type AppMode = 'edit' | 'run';

export interface ColumnInfo {
	name: string;
	type: FieldType | null;
}

/** What a widget needs to know about the table or view it reads. */
export interface ObjectSchema {
	columns: ColumnInfo[];
	/** The table's definition; null for saved queries (views). */
	def: TableDef | null;
}

const SAVE_DELAY_MS = 500;
const SIDEBAR_KEY = 'slatedb.appSidebar';

function loadCollapsed(): boolean {
	try {
		return localStorage.getItem(SIDEBAR_KEY) === 'collapsed';
	} catch {
		return false;
	}
}
const schemaKey = (alias: string, table: string) => `${alias}\u0000${table}`;

/** The app open on the app page: its definition, edits and run-time state. */
class AppSession {
	name = $state<string | null>(null);
	def = $state<AppDef | null>(null);
	loadError = $state<string | null>(null);
	mode = $state<AppMode>('run');
	pageId = $state('');
	/** Widget or sidebar command selected in Edit mode. */
	selected = $state<string | null>(null);
	/** Tab of the Edit-mode panel shown while nothing is selected. */
	builderTab = $state<'add' | 'page' | 'data' | 'style'>('add');
	/** The page sidebar shows icons only. */
	sidebarCollapsed = $state(loadCollapsed());
	/** Columns of each query widget's latest result, for its properties. */
	queryColumns = $state<Record<string, string[]>>({});
	/** Current values of the app's variables (Run mode). */
	vars = $state<Variables>({});
	/** Bumped to make every data widget reload. */
	dataRevision = $state(0);
	saveState = $state<'saved' | 'saving' | 'unsaved' | 'error'>('saved');
	canUndo = $state(false);
	canRedo = $state(false);
	/** Names of the databases in the workspace folder. */
	databases = $state<string[]>([]);
	objects = $state<Record<string, ObjectSummary[]>>({});
	schemas = $state<Record<string, ObjectSchema>>({});
	/** Widget copied with ⌘C, pasted with ⌘V on any page (of any app). */
	clipboard = $state<CopiedWidget | null>(null);
	/** One-shot "start a new record" requests for form and table widgets. */
	intent = $state<{ widget: string; n: number }>({ widget: '', n: 0 });

	private history = new History<AppDef>();
	private saveTimer: ReturnType<typeof setTimeout> | null = null;
	private pendingSchema = new Map<string, Promise<ObjectSchema | null>>();
	/** Forms with unsaved edits, asked before leaving the page or changing a variable they read. */
	private guards = new Map<string, { check: () => Promise<boolean>; reads: string[] }>();

	page = $derived(this.def ? (findPage(this.def, this.pageId) ?? this.def.pages[0]) : null);
	selectedWidget = $derived(this.def && this.selected ? (findWidget(this.def, this.selected)?.widget ?? null) : null);
	selectedCommand = $derived(this.def && this.selected ? (findCommand(this.def, this.selected) ?? null) : null);

	toggleSidebar() {
		this.sidebarCollapsed = !this.sidebarCollapsed;
		try {
			localStorage.setItem(SIDEBAR_KEY, this.sidebarCollapsed ? 'collapsed' : 'open');
		} catch {
			// Only a convenience.
		}
	}

	/** What is known about the databases, for checking bindings. */
	get schema(): SchemaLookup {
		return {
			databases: this.databases,
			objects: (alias) => this.objects[alias],
			columns: (alias, table) => this.schemas[schemaKey(alias, table)]?.columns.map((c) => c.name)
		};
	}

	async open(name: string) {
		await this.flush();
		this.name = name;
		this.def = null;
		this.loadError = null;
		this.selected = null;
		this.queryColumns = {};
		this.objects = {};
		this.schemas = {};
		this.pendingSchema.clear();
		this.guards.clear();
		this.history.clear();
		this.syncHistory();
		try {
			const def = readApp(await appService.load(name));
			this.def = def;
			this.pageId = def.startPage;
			this.vars = initialVariables(def);
			this.mode = def.pages.some((p) => p.widgets.length) ? 'run' : 'edit';
			this.saveState = 'saved';
			await this.syncAuto();
			await this.loadSchema();
		} catch (e) {
			this.loadError = e instanceof Error ? e.message : String(e);
		}
	}

	/**
	 * Catches up pages that update automatically with tables changed while
	 * the app was closed. Not an undo step: it is how the app should look.
	 */
	private async syncAuto() {
		const def = this.def;
		if (!def) return;
		let changed = false;
		for (const s of def.sources.filter((x) => x.auto)) {
			try {
				const { tables, views } = await tableService.listSchema(s.database);
				const group = def.sources.length > 1 ? s.database : '';
				changed = syncGeneratedPages(def, s.alias, s.database, tables, views, group) || changed;
			} catch {
				// A missing or unreadable database shows up as widget problems instead.
			}
		}
		if (!changed) return;
		if (!findPage(def, this.pageId)) this.pageId = def.startPage;
		this.scheduleSave();
	}

	/** Turns automatic page updates for a database on or off; on also catches up now. */
	async setAutoUpdate(alias: string, on: boolean) {
		const database = this.databaseOf(alias);
		if (!database) return;
		if (!on) {
			this.change((d) => {
				const s = sourceOf(d, alias);
				if (s) s.auto = false;
			});
			return;
		}
		const schema = await attempt(() => tableService.listSchema(database));
		if (!schema) return;
		const group = (this.def?.sources.length ?? 0) > 1 ? database : '';
		this.change((d) => {
			const s = sourceOf(d, alias);
			if (s) s.auto = true;
			syncGeneratedPages(d, alias, database, schema.tables, schema.views, group);
		});
		await this.loadSchema();
		toast.success('Pages update automatically', { description: `Changes to the tables of ${database} now show up in this app.` });
	}

	/** Saves pending edits and closes the databases the app opened. */
	async close(): Promise<boolean> {
		if (!(await this.leavePage())) return false;
		await this.flush();
		await attempt(() => databaseService.release());
		this.name = null;
		this.def = null;
		return true;
	}

	/** Reloads the database list and the tables of every source. */
	async loadSchema() {
		const def = this.def;
		if (!def) return;
		await attempt(async () => {
			this.databases = (await workspaceService.listDatabases()).filter((d) => d.valid).map((d) => d.name);
		});
		const objects: Record<string, ObjectSummary[]> = {};
		await Promise.all(
			def.sources.map(async (s) => {
				if (!this.databases.includes(s.database)) return;
				try {
					objects[s.alias] = await databaseService.listObjects(s.database);
				} catch (e) {
					reportError(e);
				}
			})
		);
		this.objects = objects;
		this.schemas = {};
		this.pendingSchema.clear();
		for (const page of def.pages) for (const w of page.widgets) this.loadWidgetSchema(w);
	}

	/** Loads the columns of whatever `w` reads, so its bindings can be checked. */
	loadWidgetSchema(w: Widget) {
		const b = w.type === 'filter' ? w.options.data : 'data' in w ? w.data : null;
		if (b) this.objectSchema(b.source, b.table);
	}

	databaseOf(alias: string): string | undefined {
		return this.def ? sourceOf(this.def, alias)?.database : undefined;
	}

	/** Columns (and, for tables, the definition) of a table or view; cached. */
	objectSchema(alias: string, table: string): Promise<ObjectSchema | null> {
		const key = schemaKey(alias, table);
		const known = this.schemas[key];
		if (known) return Promise.resolve(known);
		const pending = this.pendingSchema.get(key);
		if (pending) return pending;
		const database = this.databaseOf(alias);
		const kind = this.objects[alias]?.find((o) => o.name === table)?.kind;
		if (!database || !kind) return Promise.resolve(null);
		const load = (async (): Promise<ObjectSchema | null> => {
			try {
				let schema: ObjectSchema;
				if (kind === 'table') {
					const info = await tableService.get(table, database);
					schema = {
						columns: info.definition.fields.map((f) => ({ name: f.name, type: f.field_type })),
						def: info.definition
					};
				} else {
					const page = await recordService.list(table, { limit: 1 }, database);
					schema = { columns: page.columns.map((name, i) => ({ name, type: page.types[i] })), def: null };
				}
				this.schemas[key] = schema;
				return schema;
			} catch {
				return null;
			} finally {
				this.pendingSchema.delete(key);
			}
		})();
		this.pendingSchema.set(key, load);
		return load;
	}

	knownSchema(alias: string, table: string): ObjectSchema | undefined {
		return this.schemas[schemaKey(alias, table)];
	}

	problems(w: Widget): string[] {
		return this.def ? widgetProblems(this.def, w, this.schema) : [];
	}

	commandProblems(c: AppCommand): string[] {
		return this.def ? commandProblems(this.def, c) : [];
	}

	// ----- Editing -----

	/**
	 * Applies an edit to the definition with undo and autosave. Edits with
	 * the same `group` (typing in one box) undo as one step.
	 */
	change(edit: (def: AppDef) => void, group?: string) {
		if (!this.def) return;
		this.history.record(jsonClone(this.def), group);
		edit(this.def);
		this.syncHistory();
		this.scheduleSave();
	}

	/** A starting table for a new data widget: the first table of the first database. */
	private firstBinding(type: WidgetType): Binding | null {
		if (!this.def || !['table', 'form', 'board', 'chart', 'stat', 'query'].includes(type)) return null;
		for (const s of this.def.sources) {
			const t = (this.objects[s.alias] ?? []).find((o) => o.kind === 'table');
			if (t) return { source: s.alias, table: t.name };
		}
		return null;
	}

	/** Adds a widget to the current page in the first free spot and selects it. */
	addWidget(type: WidgetType) {
		const page = this.page;
		if (!page) return;
		const { w, h } = defaultSize(type);
		const spot = findSpot(page.widgets.map((x) => ({ id: x.id, layout: x.layout })), w, h);
		const widget = newWidget(type, spot, this.firstBinding(type));
		this.change((d) => findPage(d, page.id)?.widgets.push(widget));
		this.selected = widget.id;
		this.loadWidgetSchema(widget);
	}

	/** The widget `id` with what it needs from this app, as the clipboard holds it. */
	private copied(id: string): CopiedWidget | null {
		const found = this.def ? findWidget(this.def, id) : undefined;
		if (!found || !this.def) return null;
		return jsonClone({ widget: found.widget, sources: this.def.sources, variables: this.def.variables });
	}

	/** Puts a copy of `source` in the first free spot of the current page and selects it. */
	private async placeCopy(source: CopiedWidget) {
		const page = this.page;
		if (!page) return;
		const sourcesBefore = this.def?.sources.length ?? 0;
		const spot = findSpot(page.widgets.map((x) => ({ id: x.id, layout: x.layout })), source.widget.layout.w, source.widget.layout.h);
		const copied = jsonClone(source);
		copied.widget = { ...copied.widget, id: newId('w'), layout: spot };
		const out: { widget?: Widget } = {};
		this.change((d) => {
			out.widget = adaptPastedWidget(d, copied);
			findPage(d, page.id)?.widgets.push(out.widget);
		});
		const placed = out.widget;
		if (!placed) return;
		this.selected = placed.id;
		for (const v of this.def?.variables ?? []) if (!(v.name in this.vars)) this.vars[v.name] = jsonClone(v.default);
		// A database the widget needed was added to this app.
		if ((this.def?.sources.length ?? 0) > sourcesBefore) await this.loadSchema();
		else this.loadWidgetSchema(placed);
	}

	duplicateWidget(id: string) {
		const c = this.copied(id);
		if (c) this.placeCopy(c);
	}

	copyWidget(id: string): boolean {
		const c = this.copied(id);
		if (c) this.clipboard = c;
		return !!c;
	}

	pasteWidget() {
		if (this.clipboard) this.placeCopy(this.clipboard);
	}

	/**
	 * Moves a widget one cell (or resizes it by one cell) with the arrow keys.
	 * A move that the grid would undo by floating it back is stretched until
	 * it lands somewhere new, so a widget can hop over its neighbours.
	 */
	nudgeWidget(id: string, dx: number, dy: number, resize: boolean) {
		const found = this.def ? findWidget(this.def, id) : undefined;
		if (!found) return;
		const pageId = found.page.id;
		const items = found.page.widgets.map((w) => ({ id: w.id, layout: { ...w.layout } }));
		const o = found.widget.layout;
		let final: ReturnType<typeof compact> | null = null;
		if (resize) {
			final = compact(place(items, id, { ...o, w: Math.min(Math.max(1, o.w + dx), 12 - o.x), h: Math.max(1, o.h + dy) }));
		} else {
			const limit = items.reduce((m, i) => Math.max(m, i.layout.y + i.layout.h), 0) + 1;
			for (let step = 1; step <= limit; step++) {
				const next = compact(place(items, id, { ...o, x: o.x + dx * step, y: o.y + dy * step }));
				const l = next.find((n) => n.id === id)?.layout;
				if (l && (l.x !== o.x || l.y !== o.y)) {
					final = next;
					break;
				}
				if (dx) break;
			}
		}
		if (!final) return;
		const layouts = final;
		this.change((d) => {
			const p = findPage(d, pageId);
			if (!p) return;
			for (const w of p.widgets) w.layout = layouts.find((f) => f.id === w.id)?.layout ?? w.layout;
		}, `nudge:${id}`);
	}

	/**
	 * Adds a page for each table of source `alias` that has none yet (and an
	 * overview on an empty start page), then shows the first one.
	 */
	async buildPages(alias: string) {
		const database = this.databaseOf(alias);
		if (!database || !this.def) return;
		const tables = await attempt(() => tableService.listDefinitions(database));
		if (!tables) return;
		if (!tables.length) {
			toast.info(`${database} has no tables yet`, { description: 'Add tables on its Diagram, then build pages.' });
			return;
		}
		const group = this.def.sources.length > 1 ? database : '';
		let added: string[] = [];
		const filledStart = !findPage(this.def, this.def.startPage)?.widgets.length;
		this.change((d) => {
			added = generatePages(d, alias, database, tables, group).map((p) => p.id);
			const s = sourceOf(d, alias);
			if (s) s.auto = true;
		});
		if (!added.length && !filledStart) {
			toast.info('Every table already has a page');
			return;
		}
		await this.loadSchema();
		this.selected = null;
		await this.showPage(filledStart ? this.def.startPage : added[0]);
		toast.success(added.length ? `Added ${added.length} ${added.length === 1 ? 'page' : 'pages'}` : 'Filled in the start page', {
			description: 'Undo brings back the app as it was.'
		});
	}

	/** Removes a widget and closes the gap it leaves. */
	deleteWidget(id: string) {
		this.change((d) => {
			const page = findWidget(d, id)?.page;
			removeWidget(d, id);
			if (!page) return;
			const final = compact(page.widgets.map((w) => ({ id: w.id, layout: w.layout })));
			for (const w of page.widgets) w.layout = final.find((f) => f.id === w.id)?.layout ?? w.layout;
		});
		if (this.selected === id) this.selected = null;
	}

	/** Ends an edit group (e.g. when a text box loses focus). */
	seal() {
		this.history.seal();
	}

	undo() {
		if (!this.def) return;
		const prev = this.history.undo(jsonClone(this.def));
		if (prev) this.restore(prev);
	}

	redo() {
		if (!this.def) return;
		const next = this.history.redo(jsonClone(this.def));
		if (next) this.restore(next);
	}

	private restore(def: AppDef) {
		this.def = def;
		if (!findPage(def, this.pageId)) this.pageId = def.pages[0].id;
		if (this.selected && !findWidget(def, this.selected) && !findCommand(def, this.selected)) this.selected = null;
		for (const v of def.variables) if (!(v.name in this.vars)) this.vars[v.name] = v.default;
		this.syncHistory();
		this.scheduleSave();
	}

	private syncHistory() {
		this.canUndo = this.history.canUndo;
		this.canRedo = this.history.canRedo;
	}

	private scheduleSave() {
		this.saveState = 'unsaved';
		if (this.saveTimer) clearTimeout(this.saveTimer);
		this.saveTimer = setTimeout(() => this.flush(), SAVE_DELAY_MS);
	}

	/** Saves now if there are unsaved edits. */
	async flush() {
		if (this.saveTimer) clearTimeout(this.saveTimer);
		this.saveTimer = null;
		if (this.saveState !== 'unsaved' && this.saveState !== 'error') return;
		const name = this.name;
		const def = this.def;
		if (!name || !def) return;
		this.saveState = 'saving';
		try {
			await appService.save(name, jsonClone(def));
			if (this.saveState === 'saving') this.saveState = 'saved';
		} catch (e) {
			this.saveState = 'error';
			reportError(e);
		}
	}

	async setMode(mode: AppMode) {
		if (mode === this.mode) return;
		if (mode === 'edit' && !(await this.leavePage())) return;
		this.mode = mode;
		if (mode === 'run' && this.def) {
			// Fresh run: variables start from their defaults.
			this.vars = initialVariables(this.def);
			this.dataRevision++;
		}
	}

	// ----- Running -----

	/**
	 * Registers a check run before leaving the page (forms with unsaved
	 * edits), and before an action changes one of the variables in `reads`.
	 */
	guard(widgetId: string, check: (() => Promise<boolean>) | null, reads: string[] = []) {
		if (check) this.guards.set(widgetId, { check, reads });
		else this.guards.delete(widgetId);
	}

	private async leavePage(): Promise<boolean> {
		for (const g of [...this.guards.values()]) if (!(await g.check())) return false;
		return true;
	}

	/** Asks the forms that read any of `names` whether their unsaved edits may go. */
	private async allowChange(names: string[]): Promise<boolean> {
		for (const g of [...this.guards.values()]) {
			if (g.reads.some((r) => names.includes(r)) && !(await g.check())) return false;
		}
		return true;
	}

	async showPage(pageId: string): Promise<boolean> {
		if (pageId === this.pageId) return true;
		if (this.mode === 'run' && !(await this.leavePage())) return false;
		this.pageId = pageId;
		this.selected = null;
		return true;
	}

	setVariable(name: string, value: JsonValue) {
		this.vars[name] = value;
	}

	/** Runs a button's or a row's action. `row` holds the clicked row's values by column. */
	async run(action: Action, row?: Record<string, JsonValue>) {
		switch (action.do) {
			case 'none':
			case 'edit-record':
				return;
			case 'set-variable': {
				const v = resolveValue(action.value, this.vars, row);
				if (v === undefined || JSON.stringify(v) === JSON.stringify(this.vars[action.name] ?? null)) return;
				if (!(await this.allowChange([action.name]))) return;
				this.setVariable(action.name, v);
				return;
			}
			case 'navigate': {
				if (!this.def || !findPage(this.def, action.page)) {
					toast.error('That page no longer exists');
					return;
				}
				const values = action.set.map((s) => [s.name, resolveValue(s.value, this.vars, row)] as const);
				// Leaving the page asks anyway; staying on it, only forms reading these variables need to.
				if (action.page === this.pageId && !(await this.allowChange(action.set.map((s) => s.name)))) return;
				if (!(await this.showPage(action.page))) return;
				for (const [name, v] of values) if (v !== undefined) this.setVariable(name, v);
				return;
			}
			case 'new-record': {
				const target = this.def ? findWidget(this.def, action.widget) : undefined;
				if (!target) {
					toast.error('That widget no longer exists');
					return;
				}
				if (target.page.id !== this.pageId && !(await this.showPage(target.page.id))) return;
				this.intent = { widget: action.widget, n: this.intent.n + 1 };
				return;
			}
			case 'refresh':
				this.dataRevision++;
				return;
			case 'print':
				window.print();
				return;
		}
	}
}

export const appSession = new AppSession();
