import { call, on } from '../api/client';
import type { DatabaseInfo, DiagramLayout, JsonValue, ObjectSummary } from '../api/types';

/** Documents kept in the database file (metadata::DOCUMENT_KEYS): saved views (logic/saved-views.ts) and reports (logic/reports.ts). */
export type DocumentKey = 'saved_views' | 'reports';

export const databaseService = {
	close: () => call<boolean>('close_database'),
	info: () => call<DatabaseInfo | null>('get_database_info'),
	listObjects: (database?: string) => call<ObjectSummary[]>('list_objects', on(database)),
	/** Closes databases opened by an app, keeping the database page's one. */
	release: () => call<void>('release_databases'),
	getLayout: () => call<DiagramLayout>('get_diagram_layout'),
	saveLayout: (layout: DiagramLayout) => call<void>('save_diagram_layout', { layout }),
	/** A document kept in the open database's file; an empty object when there is none. */
	getDocument: (key: DocumentKey) => call<JsonValue>('get_document', { key }),
	saveDocument: (key: DocumentKey, value: object) => call<void>('save_document', { key, value })
};
