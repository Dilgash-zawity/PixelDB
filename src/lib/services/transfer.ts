import { call, on } from '../api/client';
import type { ExportFormat, ExportResult, ImportFile, JsonValue, RecordQuery, RecordValues } from '../api/types';

/** Import from and export to CSV and Excel files, chosen with native dialogs (null when cancelled). */
export const transferService = {
	pickImportFile: () => call<ImportFile | null>('pick_import_file'),
	/** Inserts the rows into a table of the open database, all or none; returns how many. */
	importRecords: (table: string, rows: RecordValues[]) => call<number>('import_records', { table, rows }),
	/** The records of a table or view matching `query`'s filters, search and sort (not its paging). */
	exportRecords: (table: string, query: RecordQuery, format: ExportFormat, database?: string) =>
		call<ExportResult | null>('export_records', { table, query, format, ...on(database) }),
	exportQuery: (sql: string, name: string, format: ExportFormat, database?: string, params?: Record<string, JsonValue>) =>
		call<ExportResult | null>('export_query', { sql, name, format, ...on(database), ...(params ? { params } : {}) })
};
