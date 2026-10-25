import { call } from '../api/client';
import type { ConnectionInfo, ConnectionInput, RemoteResult, RemoteTable, ServerInfo } from '../api/types';

/** Online databases. `previous` names the saved connection being edited; blank secrets keep its own. */
export const connectionService = {
	list: () => call<ConnectionInfo[]>('list_connections'),
	test: (connection: ConnectionInput, previous?: string) => call<ServerInfo>('test_connection', { connection, previous: previous ?? null }),
	save: (connection: ConnectionInput, previous?: string) => call<ConnectionInfo>('save_connection', { connection, previous: previous ?? null }),
	delete: (name: string, confirm: boolean) => call<void>('delete_connection', { name, confirm }),
	tables: (name: string) => call<RemoteTable[]>('remote_tables', { name }),
	preview: (name: string, table: RemoteTable) =>
		call<RemoteResult>('remote_preview', { name, schema: table.schema, table: table.name }),
	query: (name: string, sql: string) => call<RemoteResult>('remote_query', { name, sql })
};
