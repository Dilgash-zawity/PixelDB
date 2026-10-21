import { call } from '../api/client';
import type { DatabaseFile, DatabaseInfo, WorkspaceInfo } from '../api/types';

export const workspaceService = {
	get: () => call<WorkspaceInfo | null>('get_workspace'),
	set: (path: string) => call<WorkspaceInfo>('set_workspace', { path }),
	listDatabases: () => call<DatabaseFile[]>('list_databases'),
	createDatabase: (name: string) => call<DatabaseInfo>('create_workspace_database', { name }),
	openDatabase: (name: string) => call<DatabaseInfo>('open_workspace_database', { name })
};
