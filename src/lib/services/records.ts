import { call, on } from '../api/client';
import type {
	AggregateQuery,
	AggregateResult,
	BatchOp,
	BatchResult,
	JsonValue,
	RecordKey,
	RecordPage,
	RecordQuery,
	RecordRow,
	RecordValues,
	UndoState
} from '../api/types';

export const recordService = {
	list: (table: string, query: RecordQuery, database?: string) =>
		call<RecordPage>('list_records', { table, query, ...on(database) }),
	aggregate: (table: string, query: AggregateQuery, database?: string) =>
		call<AggregateResult>('aggregate_records', { table, query, ...on(database) }),
	get: (table: string, key: RecordKey, database?: string) =>
		call<RecordRow>('get_record', { table, key, ...on(database) }),
	getAttachment: (table: string, key: RecordKey, column: string, database?: string) =>
		call<JsonValue>('get_attachment', { table, key, column, ...on(database) }),
	insert: (table: string, values: RecordValues, database?: string) =>
		call<RecordKey>('insert_record', { table, values, ...on(database) }),
	update: (table: string, key: RecordKey, values: RecordValues, database?: string) =>
		call<RecordKey>('update_record', { table, key, values, ...on(database) }),
	delete: (table: string, keys: RecordKey[], confirm: boolean, database?: string) =>
		call<number>('delete_records', { table, keys, confirm, ...on(database) }),
	/** `label` names the change for Undo ("Paste 12 rows"). */
	batch: (table: string, ops: BatchOp[], confirmDeletes: boolean, database?: string, label?: string) =>
		call<BatchResult>('batch_records', { table, ops, confirmDeletes, ...on(database), ...(label ? { label } : {}) }),
	/** What the database page's database can undo and redo (record changes since its structure last changed). */
	undoState: () => call<UndoState>('undo_state'),
	/** Undoes the latest record change; returns what it was, or null when there was none. */
	undo: () => call<string | null>('undo_records'),
	redo: () => call<string | null>('redo_records')
};
