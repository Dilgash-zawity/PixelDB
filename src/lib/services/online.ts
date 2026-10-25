import { call } from '../api/client';
import type { ConnectionSettings, ConnectionTest, OnlineDatabase } from '../api/types';

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
		call<ConnectionTest>('test_online_database', { settings, secret, name })
};
