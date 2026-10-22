import { call } from '../api/client';
import type { DatabaseFile, DatabaseInfo, WorkspaceInfo } from '../api/types';

export const workspaceService = {
	get: () => call<WorkspaceInfo | null>('get_workspace'),
	set: (path: string) => call<WorkspaceInfo>('set_workspace', { path }),
	listDatabases: () => call<DatabaseFile[]>('list_databases'),
	createDatabase: (name: string) => call<DatabaseInfo>('create_workspace_database', { name }),
	openDatabase: (name: string) => call<DatabaseInfo>('open_workspace_database', { name }),
	renameDatabase: (name: string, newName: string) => call<void>('rename_workspace_database', { name, newName }),
	deleteDatabase: (name: string, confirm: boolean) => call<void>('delete_workspace_database', { name, confirm }),
	duplicateDatabase: (name: string, newName: string) =>
		call<void>('duplicate_workspace_database', { name, newName })
};
