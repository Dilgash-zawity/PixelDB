import { call } from '../api/client';
import type { FieldDef, SchemaChangeResult, TableDef, TableDesign, TableInfo } from '../api/types';

export const tableService = {
	get: (name: string) => call<TableInfo>('get_table', { name }),
	create: (definition: TableDef) => call<TableInfo>('create_table', { definition }),
	rename: (name: string, newName: string) => call<void>('rename_table', { name, newName }),
	drop: (name: string, confirm: boolean) => call<SchemaChangeResult>('drop_table', { name, confirm }),
	saveDesign: (name: string, design: TableDesign, confirmDataLoss: boolean) =>
		call<SchemaChangeResult>('save_table_design', { name, design, confirmDataLoss }),
	addField: (table: string, field: FieldDef, position?: number) =>
		call<SchemaChangeResult>('add_field', { table, field, position: position ?? null }),
	modifyField: (table: string, name: string, field: FieldDef) =>
		call<SchemaChangeResult>('modify_field', { table, name, field }),
	renameField: (table: string, name: string, newName: string) =>
		call<void>('rename_field', { table, name, newName }),
	dropField: (table: string, name: string, confirm: boolean) =>
		call<SchemaChangeResult>('drop_field', { table, name, confirm })
};
