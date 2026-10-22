import { call } from '../api/client';
import type { DatabaseInfo, DiagramLayout, ObjectSummary } from '../api/types';

export const databaseService = {
	close: () => call<boolean>('close_database'),
	info: () => call<DatabaseInfo | null>('get_database_info'),
	listObjects: () => call<ObjectSummary[]>('list_objects'),
	getLayout: () => call<DiagramLayout>('get_diagram_layout'),
	saveLayout: (layout: DiagramLayout) => call<void>('save_diagram_layout', { layout })
};
