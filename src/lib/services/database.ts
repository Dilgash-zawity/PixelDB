import { call, on } from '../api/client';
import type { DatabaseInfo, DiagramLayout, ObjectSummary } from '../api/types';

export const databaseService = {
	close: () => call<boolean>('close_database'),
	info: () => call<DatabaseInfo | null>('get_database_info'),
	listObjects: (database?: string) => call<ObjectSummary[]>('list_objects', on(database)),
	/** Closes databases opened by an app, keeping the database page's one. */
	release: () => call<void>('release_databases'),
	getLayout: () => call<DiagramLayout>('get_diagram_layout'),
	saveLayout: (layout: DiagramLayout) => call<void>('save_diagram_layout', { layout })
};
