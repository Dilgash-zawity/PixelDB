/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import { readApp, widgetProblems, type SchemaLookup } from './app-def';
import { TEMPLATES } from './templates';
import { jsonClone } from './values';

describe('templates', () => {
	for (const t of TEMPLATES) {
		test(`${t.name}: every widget is bound to something that exists`, () => {
			const app = t.app('My DB');
			expect(app.sources).toEqual([{ alias: 'my_db', database: 'My DB', auto: false }]);
			const schema: SchemaLookup = {
				databases: ['My DB'],
				objects: () => t.tables.map((x) => ({ name: x.name, kind: 'table' as const })),
				columns: (_, table) => t.tables.find((x) => x.name === table)?.fields.map((f) => f.name)
			};
			for (const page of app.pages) {
				for (const w of page.widgets) expect({ id: w.title, problems: widgetProblems(app, w, schema) }).toEqual({ id: w.title, problems: [] });
			}
			// Survives a save and reload.
			expect(readApp(jsonClone(app))).toEqual(app);
		});

		test(`${t.name}: sample rows only use the tables' fields`, () => {
			for (const [table, rows] of Object.entries(t.rows)) {
				const def = t.tables.find((x) => x.name === table);
				expect(def).toBeDefined();
				for (const r of rows) for (const k of Object.keys(r)) expect(def!.fields.map((f) => f.name)).toContain(k);
			}
		});
	}
});
