import { call } from '../api/client';
import type { DatabaseInfo, ObjectSummary } from '../api/types';

export const DATABASE_EXTENSIONS = ['db', 'sqlite', 'sqlite3', 'db3', 's3db', 'sl3'];

export const databaseService = {
	create: (path: string) => call<DatabaseInfo>('create_database', { path }),
	open: (path: string) => call<DatabaseInfo>('open_database', { path }),
	close: () => call<boolean>('close_database'),
	info: () => call<DatabaseInfo | null>('get_database_info'),
	listObjects: () => call<ObjectSummary[]>('list_objects')
};
