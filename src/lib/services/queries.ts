import { call } from '../api/client';
import type { QueryResult } from '../api/types';

/** Read-only queries, saved as views. */
export const queryService = {
	run: (sql: string) => call<QueryResult>('run_query', { sql }),
	save: (name: string, sql: string, replace: boolean) => call<void>('save_query', { name, sql, replace }),
	getSql: (name: string) => call<string>('get_query_sql', { name }),
	drop: (name: string, confirm: boolean) => call<void>('drop_query', { name, confirm })
};
