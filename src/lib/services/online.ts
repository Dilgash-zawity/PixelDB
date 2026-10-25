import { call } from '../api/client';
import type { ConnectionSettings, ConnectionTest, OnlineDatabase, OnlineInfo, OnlineRows, OnlineTable } from '../api/types';

/** Saved connections to online databases. A `secret` is the password or token. */
export const onlineService = {
	list: () => call<OnlineDatabase[]>('list_online_databases'),
	create: (name: string, settings: ConnectionSettings, secret?: string) =>
		call<void>('create_online_database', { name, settings, secret }),
	/** Without `secret` the saved one is kept; an empty string forgets it. */
	update: (name: string, newName: string, settings: ConnectionSettings, secret?: string) =>
		call<void>('update_online_database', { name, newName, settings, secret }),
	delete: (name: string, confirm: boolean) => call<void>('delete_online_database', { name, confirm }),
	/** Without `secret`, the one saved for `name` is used. */
	test: (settings: ConnectionSettings, secret?: string, name?: string) =>
		call<ConnectionTest>('test_online_database', { settings, secret, name }),
	/** Server version, size and how many tables and views it has. Records, tables and queries of an online database go through the usual services with its name as `database`. */
	info: (name: string) => call<OnlineInfo>('online_database_info', { name }),
	/** A table's columns with their server types, keys and links. */
	describe: (name: string, table: string) => call<OnlineTable>('describe_online_table', { name, table }),
	/** How the server would run `sql`; `analyze` (PostgreSQL) also runs it, read-only, to time it. */
	explain: (name: string, sql: string, analyze = false) => call<OnlineRows>('explain_online_query', { name, sql, analyze })
};
