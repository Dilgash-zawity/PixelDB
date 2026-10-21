import { call } from '../api/client';
import type {
	BatchOp,
	BatchResult,
	JsonValue,
	RecordKey,
	RecordPage,
	RecordQuery,
	RecordRow,
	RecordValues
} from '../api/types';

export const recordService = {
	list: (table: string, query: RecordQuery) => call<RecordPage>('list_records', { table, query }),
	get: (table: string, key: RecordKey) => call<RecordRow>('get_record', { table, key }),
	getAttachment: (table: string, key: RecordKey, column: string) =>
		call<JsonValue>('get_attachment', { table, key, column }),
	insert: (table: string, values: RecordValues) => call<RecordKey>('insert_record', { table, values }),
	update: (table: string, key: RecordKey, values: RecordValues) =>
		call<RecordKey>('update_record', { table, key, values }),
	delete: (table: string, keys: RecordKey[], confirm: boolean) =>
		call<number>('delete_records', { table, keys, confirm }),
	batch: (table: string, ops: BatchOp[], confirmDeletes: boolean) =>
		call<BatchResult>('batch_records', { table, ops, confirmDeletes })
};
