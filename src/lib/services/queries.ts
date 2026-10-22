import { call, on } from '../api/client';
import type { JsonValue, QueryResult, SavedQuery } from '../api/types';
import type { VisualQuery } from '../logic/query-builder';

/** Read-only queries, saved as views. */
export const queryService = {
	/** `params` binds the query's `:name` parameters. */
	run: (sql: string, database?: string, params?: Record<string, JsonValue>) =>
		call<QueryResult>('run_query', { sql, ...on(database), ...(params ? { params } : {}) }),
	save: (name: string, sql: string, design: VisualQuery | null, replace: boolean) =>
		call<void>('save_query', { name, sql, design, replace }),
	get: (name: string, database?: string) => call<SavedQuery>('get_query', { name, ...on(database) }),
	drop: (name: string, confirm: boolean) => call<void>('drop_query', { name, confirm })
};
