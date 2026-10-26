/**
 * Saved views of the Datasheet: a name for a filter, search and sort on one
 * table, kept in the database file so they travel with it. Stored as
 * `{ [table]: SavedView[] }`; the backend keeps the JSON without reading it.
 */
import type { JsonValue, SortSpec } from '../api/types';
import type { FilterRow } from './filters';

export interface SavedView {
	name: string;
	sort: SortSpec | null;
	filters: FilterRow[];
	search: string;
}

export type SavedViews = Record<string, SavedView[]>;

export const MAX_VIEWS_PER_TABLE = 50;

const isObject = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);
const sameName = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase();

/** Saved views from stored JSON, dropping anything malformed. */
export function readViews(json: JsonValue | unknown): SavedViews {
	const out: SavedViews = {};
	if (!isObject(json)) return out;
	for (const [table, list] of Object.entries(json)) {
		if (!Array.isArray(list)) continue;
		const views = list.filter(isObject).flatMap((v): SavedView[] => {
			if (typeof v.name !== 'string' || !v.name.trim()) return [];
			const sort = isObject(v.sort) && typeof v.sort.column === 'string' ? { column: v.sort.column, descending: !!v.sort.descending } : null;
			const filters = Array.isArray(v.filters)
				? v.filters.filter(isObject).flatMap((f) =>
						typeof f.column === 'string' && typeof f.op === 'string'
							? [{ column: f.column, op: f.op as FilterRow['op'], value: typeof f.value === 'string' ? f.value : '' }]
							: []
					)
				: [];
			return [{ name: v.name.trim(), sort, filters, search: typeof v.search === 'string' ? v.search : '' }];
		});
		if (views.length) out[table] = views;
	}
	return out;
}

/** Problem with a name for a new view of `table`, or null. */
export function viewNameProblem(views: SavedViews, table: string, name: string, except?: string): string | null {
	if (!name.trim()) return 'Enter a name';
	if (name.trim().length > 80) return 'Use at most 80 characters';
	if ((views[table] ?? []).some((v) => sameName(v.name, name) && !(except && sameName(v.name, except)))) return 'A view with this name already exists';
	if (!except && (views[table]?.length ?? 0) >= MAX_VIEWS_PER_TABLE) return `A table can have at most ${MAX_VIEWS_PER_TABLE} saved views`;
	return null;
}

/** Adds `view`, replacing one of the same name. */
export function putView(views: SavedViews, table: string, view: SavedView): SavedViews {
	const list = (views[table] ?? []).filter((v) => !sameName(v.name, view.name));
	return { ...views, [table]: [...list, { ...view, name: view.name.trim() }].sort((a, b) => a.name.localeCompare(b.name)) };
}

export function removeView(views: SavedViews, table: string, name: string): SavedViews {
	const list = (views[table] ?? []).filter((v) => !sameName(v.name, name));
	const out = { ...views };
	if (list.length) out[table] = list;
	else delete out[table];
	return out;
}

export function renameTableInViews(views: SavedViews, from: string, to: string): SavedViews {
	if (!views[from]) return views;
	const out = { ...views, [to]: views[from] };
	delete out[from];
	return out;
}

export function renameFieldInViews(views: SavedViews, table: string, from: string, to: string): SavedViews {
	if (!views[table]) return views;
	return {
		...views,
		[table]: views[table].map((v) => ({
			...v,
			sort: v.sort?.column === from ? { ...v.sort, column: to } : v.sort,
			filters: v.filters.map((f) => (f.column === from ? { ...f, column: to } : f))
		}))
	};
}

/** Views of `table` without conditions or sorting on fields it no longer has; tables that are gone lose their views. */
export function pruneViews(views: SavedViews, tables: Record<string, string[]>): SavedViews {
	const out: SavedViews = {};
	for (const [table, list] of Object.entries(views)) {
		const fields = tables[table];
		if (!fields) continue;
		out[table] = list.map((v) => ({
			...v,
			sort: v.sort && fields.includes(v.sort.column) ? v.sort : null,
			filters: v.filters.filter((f) => fields.includes(f.column))
		}));
	}
	return out;
}

/** Whether the datasheet shows exactly `view`. */
export function showsView(view: SavedView, current: Omit<SavedView, 'name'>): boolean {
	return JSON.stringify([view.sort, view.filters, view.search.trim()]) === JSON.stringify([current.sort, current.filters, current.search.trim()]);
}
