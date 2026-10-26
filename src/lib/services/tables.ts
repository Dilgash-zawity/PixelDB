import { call, on } from '../api/client';
import { databaseService } from './database';
import type { FieldDef, IndexInfo, SchemaChangeResult, TableDef, TableDesign, TableInfo } from '../api/types';

type Listener = () => void;
const listeners = new Set<Listener>();

/** Tells listeners the open database's tables changed (not for renames, which are carried over by name). */
function changed<T>(result: Promise<T>): Promise<T> {
	return result.then((r) => {
		for (const l of listeners) l();
		return r;
	});
}

export const tableService = {
	/** Runs `listener` after every successful change to the open database's tables or fields; returns an unsubscribe. */
	onStructureChange(listener: Listener): () => void {
		listeners.add(listener);
		return () => listeners.delete(listener);
	},
	get: (name: string, database?: string) => call<TableInfo>('get_table', { name, ...on(database) }),
	create: (definition: TableDef) => changed(call<TableInfo>('create_table', { definition })),
	rename: (name: string, newName: string) => call<void>('rename_table', { name, newName }),
	drop: (name: string, confirm: boolean) => changed(call<SchemaChangeResult>('drop_table', { name, confirm })),
	saveDesign: (name: string, design: TableDesign, confirmDataLoss: boolean) =>
		changed(call<SchemaChangeResult>('save_table_design', { name, design, confirmDataLoss })),
	addField: (table: string, field: FieldDef, position?: number) =>
		changed(call<SchemaChangeResult>('add_field', { table, field, position: position ?? null })),
	modifyField: (table: string, name: string, field: FieldDef) =>
		changed(call<SchemaChangeResult>('modify_field', { table, name, field })),
	renameField: (table: string, name: string, newName: string) =>
		call<void>('rename_field', { table, name, newName }),
	dropField: (table: string, name: string, confirm: boolean) =>
		changed(call<SchemaChangeResult>('drop_field', { table, name, confirm })),
	/** Adds an index on `columns` (a unique one keeps their combination unique). */
	createIndex: (table: string, columns: string[], unique: boolean) =>
		call<IndexInfo>('create_index', { table, columns, unique }),
	/** Drops an index added with `createIndex`; no data is lost. */
	dropIndex: (table: string, name: string) => call<void>('drop_index', { table, name }),
	/** Definitions of every table of a workspace database, in listing order, and the names of its saved queries. */
	async listSchema(database: string): Promise<{ tables: TableDef[]; views: string[] }> {
		const objects = await databaseService.listObjects(database);
		const tables = objects.filter((o) => o.kind === 'table');
		return {
			tables: await Promise.all(tables.map(async (t) => (await tableService.get(t.name, database)).definition)),
			views: objects.filter((o) => o.kind === 'view').map((o) => o.name)
		};
	},
	async listDefinitions(database: string): Promise<TableDef[]> {
		return (await tableService.listSchema(database)).tables;
	}
};
