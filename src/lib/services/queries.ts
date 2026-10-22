import { call } from '../api/client';
import type { QueryResult, SavedQuery } from '../api/types';
import type { VisualQuery } from '../logic/query-builder';

/** Read-only queries, saved as views. */
export const queryService = {
	run: (sql: string) => call<QueryResult>('run_query', { sql }),
	save: (name: string, sql: string, design: VisualQuery | null, replace: boolean) =>
		call<void>('save_query', { name, sql, design, replace }),
	get: (name: string) => call<SavedQuery>('get_query', { name }),
	drop: (name: string, confirm: boolean) => call<void>('drop_query', { name, confirm })
};
