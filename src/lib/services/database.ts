import { call } from '../api/client';
import type { DatabaseInfo, ObjectSummary } from '../api/types';

export const databaseService = {
	close: () => call<boolean>('close_database'),
	info: () => call<DatabaseInfo | null>('get_database_info'),
	listObjects: () => call<ObjectSummary[]>('list_objects')
};
