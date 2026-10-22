import type { FieldType, FilterOp, JsonValue, TableDef } from '../api/types';
import { needsValue } from './filters';
import { relationshipsOf, valueKind, type Relationship } from './relationships';

/**
 * The visual query designer: tables joined along their relationships and a
 * grid of columns with sort, totals and criteria, turned into one SELECT.
 * Values typed as criteria become SQL literals, so the generated text can be
 * copied into the SQL editor and still mean the same thing.
 */

export type JoinKind = 'inner' | 'left';
export type SortOrder = 'none' | 'asc' | 'desc';
export type Aggregate = 'none' | 'group' | 'count' | 'sum' | 'avg' | 'min' | 'max';

export interface QueryJoin {
	table: string;
	kind: JoinKind;
}

export interface QueryColumn {
	table: string;
	field: string;
	show: boolean;
	sort: SortOrder;
	aggregate: Aggregate;
	op: FilterOp;
	/** Criteria value as typed; ignored when empty (except for "is empty" ops). */
	value: string;
}

export interface VisualQuery {
	from: string | null;
	joins: QueryJoin[];
	columns: QueryColumn[];
}

export interface BuiltQuery {
	sql: string;
	/** Field type behind each output column, for formatting results. */
	types: (FieldType | undefined)[];
	/** Criteria that couldn't be used, e.g. text typed for a number field. */
	problems: string[];
}

export const AGGREGATES: { value: Aggregate; label: string }[] = [
	{ value: 'none', label: '—' },
	{ value: 'group', label: 'Group by' },
	{ value: 'count', label: 'Count' },
	{ value: 'sum', label: 'Sum' },
	{ value: 'avg', label: 'Average' },
	{ value: 'min', label: 'Min' },
	{ value: 'max', label: 'Max' }
];

/** Everything the Query tab keeps between visits. */
export interface QueryDraft {
	mode: 'visual' | 'sql';
	visual: VisualQuery;
	sql: string;
	/** The designer's SQL last copied into the editor, to tell whether it was edited since. */
	copied: string;
	/** Name of the saved query (view) being edited, if any. */
	saved: string | null;
	result: { columns: string[]; rows: JsonValue[][]; truncated: boolean; elapsedMs: number; types: (FieldType | undefined)[] } | null;
}

export function newQueryDraft(): QueryDraft {
	return { mode: 'visual', visual: emptyQuery(), sql: '', copied: '', saved: null, result: null };
}

export function emptyQuery(): VisualQuery {
	return { from: null, joins: [], columns: [] };
}

export function newColumn(table: string, field: string): QueryColumn {
	return { table, field, show: true, sort: 'none', aggregate: 'none', op: 'eq', value: '' };
}

function same(a: string, b: string) {
	return a.toLowerCase() === b.toLowerCase();
}

export function quoteIdent(name: string): string {
	return `"${name.replace(/"/g, '""')}"`;
}

export function quoteLiteral(value: string): string {
	return `'${value.replace(/'/g, "''")}'`;
}

/** Tables currently in the query, FROM table first. */
export function queryTables(q: VisualQuery): string[] {
	return q.from ? [q.from, ...q.joins.map((j) => j.table)] : [];
}

/** The relationship that joins `table` to one of the tables already in `present`. */
export function joinRelationship(rels: Relationship[], present: string[], table: string): Relationship | undefined {
	const has = (t: string) => present.some((p) => same(p, t));
	return rels.find(
		(r) => (same(r.child, table) && has(r.parent)) || (same(r.parent, table) && has(r.child))
	);
}

/** Tables that can be added: linked to the query by a relationship and not in it yet. */
export function joinableTables(defs: TableDef[], q: VisualQuery): string[] {
	const present = queryTables(q);
	if (!present.length) return defs.map((d) => d.name);
	const rels = relationshipsOf(defs);
	return defs
		.map((d) => d.name)
		.filter((t) => !present.some((p) => same(p, t)) && joinRelationship(rels, present, t));
}

/** Removes a table, along with joined tables that only it connected and their columns. */
export function removeTable(defs: TableDef[], q: VisualQuery, table: string): VisualQuery {
	if (q.from && same(q.from, table)) return emptyQuery();
	const rels = relationshipsOf(defs);
	const present = [q.from!];
	const joins: QueryJoin[] = [];
	for (const j of q.joins) {
		if (same(j.table, table) || !joinRelationship(rels, present, j.table)) continue;
		present.push(j.table);
		joins.push(j);
	}
	return {
		from: q.from,
		joins,
		columns: q.columns.filter((c) => present.some((p) => same(p, c.table)))
	};
}

function fieldOf(defs: TableDef[], table: string, field: string) {
	return defs.find((d) => same(d.name, table))?.fields.find((f) => same(f.name, field));
}

/** Parses a decimal into integer minor units without floating-point error. */
function minorUnits(text: string, scale: number): string | null {
	const m = /^([+-]?)(\d*)(?:\.(\d*))?$/.exec(text);
	if (!m || (!m[2] && !m[3])) return null;
	const frac = m[3] ?? '';
	if (frac.length > scale) return null;
	const digits = ((m[2] || '0') + frac.padEnd(scale, '0')).replace(/^0+(?=\d)/, '');
	return (m[1] === '-' && /[1-9]/.test(digits) ? '-' : '') + digits;
}

/** A criteria value as a SQL literal for a field of type `t`, or null when it doesn't fit. */
export function literalFor(text: string, t: FieldType | undefined): string | null {
	const v = text.trim();
	if (t?.type === 'currency') return minorUnits(v, t.scale ?? 2);
	if (t?.type === 'yes_no') {
		if (/^(yes|true|1|on)$/i.test(v)) return '1';
		if (/^(no|false|0|off)$/i.test(v)) return '0';
		return null;
	}
	const kind = t ? valueKind(t) : 'other';
	if (kind === 'integer') return /^[+-]?\d+$/.test(v) ? String(BigInt(v)) : null;
	if (kind === 'real') return /^[+-]?(\d+\.?\d*|\.\d+)(e[+-]?\d+)?$/i.test(v) ? v.replace(/^\+/, '') : null;
	return quoteLiteral(v);
}

function likePattern(v: string, prefix: boolean, suffix: boolean): string {
	const escaped = v.replace(/[\\%_]/g, (c) => `\\${c}`);
	return `${quoteLiteral(`${prefix ? '%' : ''}${escaped}${suffix ? '%' : ''}`)} ESCAPE '\\'`;
}

const COMPARE: Partial<Record<FilterOp, string>> = { eq: '=', ne: '<>', lt: '<', le: '<=', gt: '>', ge: '>=' };

/** One criteria condition, or a problem message. */
function condition(ref: string, c: QueryColumn, t: FieldType | undefined): { sql: string } | { problem: string } | null {
	if (c.op === 'is_null') return { sql: `${ref} IS NULL` };
	if (c.op === 'is_not_null') return { sql: `${ref} IS NOT NULL` };
	const v = c.value.trim();
	if (!needsValue(c.op) || !v) return null;
	const bad = { problem: `“${v}” doesn't fit ${c.table}.${c.field}` };
	switch (c.op) {
		case 'contains':
			return { sql: `${ref} LIKE ${likePattern(v, true, true)}` };
		case 'starts_with':
			return { sql: `${ref} LIKE ${likePattern(v, false, true)}` };
		case 'ends_with':
			return { sql: `${ref} LIKE ${likePattern(v, true, false)}` };
		case 'in': {
			const items = v.split(',').map((s) => s.trim()).filter(Boolean).map((s) => literalFor(s, t));
			if (!items.length || items.some((i) => i === null)) return bad;
			return { sql: `${ref} IN (${items.join(', ')})` };
		}
		default: {
			const lit = literalFor(v, t);
			return lit === null ? bad : { sql: `${ref} ${COMPARE[c.op]} ${lit}` };
		}
	}
}

/** Output type of an aggregate over a field of type `t`. */
function aggregateType(a: Aggregate, t: FieldType | undefined): FieldType | undefined {
	if (a === 'count' || a === 'avg') return undefined;
	if (a === 'sum' && t?.type === 'yes_no') return undefined;
	return t;
}

const AGG_SQL: Partial<Record<Aggregate, string>> = { count: 'count', sum: 'sum', avg: 'avg', min: 'min', max: 'max' };
const AGG_LABEL: Partial<Record<Aggregate, string>> = {
	count: 'Count of',
	sum: 'Sum of',
	avg: 'Average of',
	min: 'Min of',
	max: 'Max of'
};

export function buildSql(defs: TableDef[], q: VisualQuery): BuiltQuery {
	if (!q.from) return { sql: '', types: [], problems: [] };
	const rels = relationshipsOf(defs);
	const lines: string[] = [];
	const types: (FieldType | undefined)[] = [];
	const problems: string[] = [];
	const ref = (c: { table: string; field: string }) => `${quoteIdent(c.table)}.${quoteIdent(c.field)}`;
	const totals = q.columns.some((c) => c.aggregate !== 'none');

	const shown = q.columns.filter((c) => c.show);
	const select = shown.map((c) => {
		const t = fieldOf(defs, c.table, c.field)?.field_type;
		const agg = AGG_SQL[c.aggregate];
		types.push(agg ? aggregateType(c.aggregate, t) : t);
		if (agg) return `${agg}(${ref(c)}) AS ${quoteIdent(`${AGG_LABEL[c.aggregate]} ${c.field}`)}`;
		// Disambiguate fields with the same name in different tables.
		const clash = shown.some((o) => o !== c && !AGG_SQL[o.aggregate] && same(o.field, c.field));
		return clash ? `${ref(c)} AS ${quoteIdent(`${c.table}.${c.field}`)}` : ref(c);
	});
	lines.push(`SELECT ${select.length ? select.join(',\n       ') : `${quoteIdent(q.from)}.*`}`);
	lines.push(`FROM ${quoteIdent(q.from)}`);

	const present = [q.from];
	for (const j of q.joins) {
		const rel = joinRelationship(rels, present, j.table);
		if (!rel) {
			problems.push(`${j.table} isn't linked to the other tables`);
			continue;
		}
		const on = rel.childColumns
			.map((col, i) => `${ref({ table: rel.child, field: col })} = ${ref({ table: rel.parent, field: rel.parentColumns[i] })}`)
			.join(' AND ');
		lines.push(`${j.kind === 'left' ? 'LEFT JOIN' : 'JOIN'} ${quoteIdent(j.table)} ON ${on}`);
		present.push(j.table);
	}

	const where: string[] = [];
	for (const c of q.columns) {
		const r = condition(ref(c), c, fieldOf(defs, c.table, c.field)?.field_type);
		if (!r) continue;
		if ('problem' in r) problems.push(r.problem);
		else where.push(r.sql);
	}
	if (where.length) lines.push(`WHERE ${where.join('\n  AND ')}`);

	if (totals) {
		const group = shown.filter((c) => !AGG_SQL[c.aggregate]).map(ref);
		if (group.length) lines.push(`GROUP BY ${group.join(', ')}`);
	}

	const order = q.columns
		.filter((c) => c.sort !== 'none')
		.map((c) => {
			const agg = AGG_SQL[c.aggregate];
			const expr = agg ? `${agg}(${ref(c)})` : ref(c);
			return `${expr}${c.sort === 'desc' ? ' DESC' : ''}`;
		});
	if (order.length) lines.push(`ORDER BY ${order.join(', ')}`);

	return { sql: lines.join('\n'), types, problems };
}
