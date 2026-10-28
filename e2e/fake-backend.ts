/**
 * A stand-in for the Rust backend, injected into the page before it loads
 * (`page.addInitScript(fakeBackend, scenario)`). It answers the Tauri
 * commands the screens under test call, from an in-memory workspace; any
 * other command resolves to null and is recorded in `window.__calls`.
 * Runs in the browser, so it must not use anything from its module scope.
 */
export interface FakeTable {
	fields: { name: string; field_type: Record<string, unknown>; primary_key?: boolean; required?: boolean }[];
	rows: unknown[][];
}

export interface Scenario {
	workspace: string | null;
	databases: Record<string, Record<string, FakeTable>>;
	apps?: { name: string; databases: string[] }[];
}

export function fakeBackend(scenario: Scenario) {
	const state = { ...scenario, current: null as string | null, documents: {} as Record<string, unknown>, apps: scenario.apps ?? [] };
	const calls: { cmd: string; args: unknown }[] = [];
	(window as unknown as { __calls: typeof calls }).__calls = calls;
	// Settings start fresh with the tour already seen, so it doesn't cover the screen.
	localStorage.setItem('pixeldb.settings', JSON.stringify({ tourSeen: true }));

	const info = (name: string) => ({
		path: `/fake/DB/${name}.db`,
		file_size: 4096,
		sqlite_version: '3.50.0',
		page_size: 4096,
		page_count: 1,
		encoding: 'UTF-8',
		journal_mode: 'delete',
		user_version: 0,
		application_id: 0,
		foreign_keys: true,
		table_count: Object.keys(state.databases[name] ?? {}).length,
		view_count: 0
	});
	const db = (name?: string | null) => state.databases[(name ?? state.current) as string] ?? {};
	const fail = (kind: string, message: string) => Promise.reject({ kind, message });

	const handlers: Record<string, (a: Record<string, any>) => unknown> = {
		get_workspace: () => (state.workspace ? { path: state.workspace } : null),
		list_databases: () =>
			Object.keys(state.databases).map((name) => ({ name, file_name: `${name}.db`, size: 4096, modified: Date.now(), valid: true })),
		list_online_databases: () => [],
		list_apps: () => state.apps.map((a) => ({ ...a, size: 100, modified: Date.now(), description: '', accent: 'orange', valid: true })),
		create_workspace_database: ({ name }) => {
			if (state.databases[name]) return fail('already_exists', `a database named "${name}" already exists`);
			state.databases[name] = {};
			return info(name);
		},
		open_workspace_database: ({ name }) => {
			if (!state.databases[name]) return fail('not_found', `database "${name}"`);
			state.current = name;
			return info(name);
		},
		get_database_info: () => info(state.current as string),
		list_objects: ({ database }) => Object.keys(db(database)).map((name) => ({ name, kind: 'table', managed: true })),
		get_table: ({ name: table, database }) => {
			const t = db(database)[table];
			if (!t) return fail('not_found', `table "${table}"`);
			return { definition: { name: table, fields: t.fields }, managed: true, unsupported_features: [], schema_sql: '', without_rowid: false, row_count: t.rows.length, indexes: [] };
		},
		list_records: ({ table, query, database }) => {
			const t = db(database)[table];
			if (!t) return fail('not_found', `table "${table}"`);
			const offset = query?.offset ?? 0;
			const limit = query?.limit || 100;
			const search = String(query?.search ?? '').toLowerCase();
			const rows = t.rows.filter((r) => !search || r.some((v) => String(v ?? '').toLowerCase().includes(search)));
			return {
				columns: t.fields.map((f) => f.name),
				types: t.fields.map((f) => f.field_type),
				rows: rows.slice(offset, offset + limit).map((values, i) => ({ key: { row_id: offset + i + 1 }, values })),
				total: rows.length,
				offset,
				limit
			};
		},
		get_diagram_layout: () => ({}),
		get_document: ({ key }) => state.documents[key] ?? {},
		save_document: ({ key, value }) => {
			state.documents[key] = value;
			return null;
		},
		undo_state: () => ({ undo: null, redo: null }),
		'plugin:app|version': () => '0.1.0'
	};

	let callbackId = 0;
	(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
		metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main', windowLabel: 'main' } },
		transformCallback: () => ++callbackId,
		unregisterCallback: () => {},
		convertFileSrc: (p: string) => p,
		invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
			calls.push({ cmd, args });
			const h = handlers[cmd];
			return h ? h(args) : null;
		}
	};
}
