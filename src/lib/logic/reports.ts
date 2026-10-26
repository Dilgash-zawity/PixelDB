/**
 * Reports: a printable listing of a table or saved query, optionally grouped
 * by one field with totals per group and overall. Definitions are kept in
 * the database file as `{ [name]: ReportDef }` (the backend stores the JSON
 * without reading it); `buildReport` turns rows into what is printed.
 */
import type { FieldType, JsonValue, SortSpec } from '../api/types';
import type { FilterRow } from './filters';
import type { LookupOption } from './lookups';
import { displayValue, formatMinorUnits } from './values';

export type TotalKind = 'none' | 'count' | 'sum' | 'avg' | 'min' | 'max';

export interface ReportColumn {
	field: string;
	/** The heading printed; the field name when empty. */
	label: string;
	total: TotalKind;
}

export interface ReportDef {
	name: string;
	/** A table or saved query of the database. */
	source: string;
	title: string;
	subtitle: string;
	columns: ReportColumn[];
	groupBy: string | null;
	sort: SortSpec | null;
	filters: FilterRow[];
	landscape: boolean;
	/** "12 records" under each group and at the end. */
	showCount: boolean;
}

export type Reports = Record<string, ReportDef>;

/** Rows read for one report; more are left out with a note. */
export const MAX_REPORT_ROWS = 5000;

export const TOTALS: { value: TotalKind; label: string }[] = [
	{ value: 'none', label: 'No total' },
	{ value: 'count', label: 'Count' },
	{ value: 'sum', label: 'Sum' },
	{ value: 'avg', label: 'Average' },
	{ value: 'min', label: 'Smallest' },
	{ value: 'max', label: 'Largest' }
];

const NUMERIC = ['number', 'large_number', 'currency', 'auto_number'];
const ORDERED = [...NUMERIC, 'date_time'];

/** The totals that make sense for a field of type `t`. */
export function totalsFor(t: FieldType | null | undefined): TotalKind[] {
	if (t && NUMERIC.includes(t.type)) return ['none', 'count', 'sum', 'avg', 'min', 'max'];
	if (t && ORDERED.includes(t.type)) return ['none', 'count', 'min', 'max'];
	return ['none', 'count'];
}

const isObject = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);
const str = (v: unknown, d = '') => (typeof v === 'string' ? v : d);

/** Reports from stored JSON, dropping anything malformed. */
export function readReports(json: JsonValue | unknown): Reports {
	const out: Reports = {};
	if (!isObject(json)) return out;
	for (const r of Object.values(json)) {
		if (!isObject(r) || !str(r.name).trim() || !str(r.source)) continue;
		const columns = (Array.isArray(r.columns) ? r.columns : []).filter(isObject).flatMap((c): ReportColumn[] =>
			typeof c.field === 'string' && c.field
				? [{ field: c.field, label: str(c.label), total: TOTALS.some((t) => t.value === c.total) ? (c.total as TotalKind) : 'none' }]
				: []
		);
		const sort = isObject(r.sort) && typeof r.sort.column === 'string' ? { column: r.sort.column, descending: !!r.sort.descending } : null;
		const filters = (Array.isArray(r.filters) ? r.filters : [])
			.filter(isObject)
			.flatMap((f) => (typeof f.column === 'string' && typeof f.op === 'string' ? [{ column: f.column, op: f.op as FilterRow['op'], value: str(f.value) }] : []));
		const name = str(r.name).trim();
		out[name] = {
			name,
			source: str(r.source),
			title: str(r.title, name),
			subtitle: str(r.subtitle),
			columns,
			groupBy: typeof r.groupBy === 'string' && r.groupBy ? r.groupBy : null,
			sort,
			filters,
			landscape: r.landscape === true,
			showCount: r.showCount !== false
		};
	}
	return out;
}

/** A new report listing every field of `source` (up to eight). */
export function newReport(name: string, source: string, fields: { name: string; type: FieldType | null }[]): ReportDef {
	return {
		name,
		source,
		title: source,
		subtitle: '',
		columns: fields
			.filter((f) => f.type?.type !== 'attachment')
			.slice(0, 8)
			.map((f) => ({ field: f.name, label: '', total: 'none' })),
		groupBy: null,
		sort: null,
		filters: [],
		landscape: fields.length > 6,
		showCount: true
	};
}

export function reportNameProblem(reports: Reports, name: string, except?: string): string | null {
	const n = name.trim();
	if (!n) return 'Enter a name';
	if (n.length > 80) return 'Use at most 80 characters';
	if (Object.keys(reports).some((k) => k.toLowerCase() === n.toLowerCase() && k !== except)) return 'A report with this name already exists';
	return null;
}

export function renameTableInReports(reports: Reports, from: string, to: string): Reports {
	return Object.fromEntries(Object.entries(reports).map(([k, r]) => [k, r.source === from ? { ...r, source: to, title: r.title === from ? to : r.title } : r]));
}

export function renameFieldInReports(reports: Reports, table: string, from: string, to: string): Reports {
	const rename = (f: string) => (f === from ? to : f);
	return Object.fromEntries(
		Object.entries(reports).map(([k, r]) => [
			k,
			r.source !== table
				? r
				: {
						...r,
						columns: r.columns.map((c) => ({ ...c, field: rename(c.field) })),
						groupBy: r.groupBy && rename(r.groupBy),
						sort: r.sort && { ...r.sort, column: rename(r.sort.column) },
						filters: r.filters.map((f) => ({ ...f, column: rename(f.column) }))
					}
		])
	);
}

/** The sorting to ask for: by the group first, then the report's own order. */
export function reportSort(r: ReportDef): SortSpec[] {
	const out: SortSpec[] = [];
	if (r.groupBy) out.push({ column: r.groupBy, descending: false });
	if (r.sort && r.sort.column !== r.groupBy) out.push(r.sort);
	return out;
}

export interface ReportCell {
	text: string;
	numeric: boolean;
}

export interface ReportGroup {
	/** The group's value as shown; null when the report isn't grouped. */
	label: string | null;
	rows: ReportCell[][];
	totals: (string | null)[];
	count: number;
}

export interface BuiltReport {
	headers: { label: string; numeric: boolean }[];
	groups: ReportGroup[];
	totals: (string | null)[];
	count: number;
	/** Columns of the definition the source no longer has. */
	missing: string[];
}

export interface ReportData {
	columns: string[];
	types: (FieldType | null)[];
	rows: JsonValue[][];
}

function shown(v: JsonValue | undefined, t: FieldType | null | undefined, options?: LookupOption[]): string {
	if (t?.type === 'lookup' && options) {
		const o = options.find((x) => JSON.stringify(x.value) === JSON.stringify(v ?? null));
		if (o) return o.label;
	}
	return displayValue(v, t ?? undefined);
}

function formatNumber(n: number, t: FieldType | null | undefined): string {
	if (t?.type === 'currency') return formatMinorUnits(Math.round(n), t.scale ?? 2);
	return Number.isInteger(n) ? String(n) : n.toFixed(2).replace(/\.?0+$/, '');
}

/** A total over the values of one column. */
export function total(kind: TotalKind, values: JsonValue[], t: FieldType | null | undefined, options?: LookupOption[]): string | null {
	if (kind === 'none') return null;
	const present = values.filter((v) => v !== null && v !== undefined && v !== '');
	if (kind === 'count') return String(present.length);
	if (!present.length) return '';
	if (t && NUMERIC.includes(t.type)) {
		const nums = present.filter((v): v is number => typeof v === 'number');
		if (!nums.length) return '';
		// Currency is in whole minor units, so sums stay exact.
		const sum = nums.reduce((a, b) => a + b, 0);
		const n = kind === 'sum' ? sum : kind === 'avg' ? sum / nums.length : kind === 'min' ? Math.min(...nums) : Math.max(...nums);
		return formatNumber(n, t);
	}
	if (kind === 'min' || kind === 'max') {
		const sorted = [...present].map(String).sort();
		return shown(kind === 'min' ? sorted[0] : sorted[sorted.length - 1], t, options);
	}
	return '';
}

/** What is printed: rows as shown text, split into groups (in the order the rows came), with totals. */
export function buildReport(r: ReportDef, data: ReportData, lookups: Record<string, LookupOption[]> = {}): BuiltReport {
	const index = (f: string) => data.columns.indexOf(f);
	const cols = r.columns.filter((c) => index(c.field) >= 0);
	const missing = r.columns.filter((c) => index(c.field) < 0).map((c) => c.field);
	const typeOf = (f: string) => data.types[index(f)] ?? null;
	const headers = cols.map((c) => ({ label: c.label.trim() || c.field, numeric: NUMERIC.includes(typeOf(c.field)?.type ?? '') }));
	const g = r.groupBy && index(r.groupBy) >= 0 ? index(r.groupBy) : -1;

	const buckets: { label: string | null; rows: JsonValue[][] }[] = [];
	for (const row of data.rows) {
		const label = g < 0 ? null : shown(row[g], data.types[g], lookups[r.groupBy!]) || '(none)';
		const last = buckets.at(-1);
		if (last && last.label === label) last.rows.push(row);
		else buckets.push({ label, rows: [row] });
	}
	if (!buckets.length) buckets.push({ label: null, rows: [] });

	const totalsOf = (rows: JsonValue[][]) =>
		cols.map((c) => total(c.total, rows.map((row) => row[index(c.field)]), typeOf(c.field), lookups[c.field]));
	const groups = buckets.map((b) => ({
		label: b.label,
		rows: b.rows.map((row) =>
			cols.map((c, i) => ({ text: shown(row[index(c.field)], typeOf(c.field), lookups[c.field]), numeric: headers[i].numeric }))
		),
		totals: totalsOf(b.rows),
		count: b.rows.length
	}));
	return { headers, groups, totals: totalsOf(data.rows), count: data.rows.length, missing };
}

/** Whether any column has a total (so total rows are printed). */
export function hasTotals(r: ReportDef): boolean {
	return r.columns.some((c) => c.total !== 'none');
}
