import type {
	FieldDef,
	FieldTypeName,
	ForeignKeyDef,
	TableDef,
	TableDesign
} from '../api/types';
import { makeFieldType } from './field-types';
import { jsonClone } from './values';

export interface DraftField {
	/** Stable identity while editing. */
	id: number;
	/** Column name in the saved table; null for fields added in this draft. */
	originalName: string | null;
	field: FieldDef;
}

/** Table designer state. Plain data so it can be wrapped in Svelte `$state`. */
export interface TableDraft {
	isNew: boolean;
	originalName: string | null;
	name: string;
	description: string;
	fields: DraftField[];
	foreign_keys: ForeignKeyDef[];
	unique_constraints: string[][];
	checks: string[];
	nextId: number;
}

function lower(s: string) {
	return s.toLowerCase();
}

export function newDraft(name = ''): TableDraft {
	return {
		isNew: true,
		originalName: null,
		name,
		description: '',
		fields: [
			{
				id: 1,
				originalName: null,
				field: { name: 'ID', field_type: { type: 'auto_number' }, primary_key: true }
			}
		],
		foreign_keys: [],
		unique_constraints: [],
		checks: [],
		nextId: 2
	};
}

export function draftFromDefinition(def: TableDef): TableDraft {
	const copy = jsonClone(def);
	return {
		isNew: false,
		originalName: copy.name,
		name: copy.name,
		description: copy.description ?? '',
		fields: copy.fields.map((field, i) => ({ id: i + 1, originalName: field.name, field })),
		foreign_keys: copy.foreign_keys ?? [],
		unique_constraints: copy.unique_constraints ?? [],
		checks: copy.checks ?? [],
		nextId: copy.fields.length + 1
	};
}

function find(d: TableDraft, id: number): DraftField {
	const f = d.fields.find((x) => x.id === id);
	if (!f) throw new Error(`no field with id ${id}`);
	return f;
}

export function addField(d: TableDraft, typeName: FieldTypeName = 'short_text', name?: string): DraftField {
	let n = d.fields.length + 1;
	let fieldName = name ?? `Field${n}`;
	while (!name && d.fields.some((f) => lower(f.field.name) === lower(fieldName))) fieldName = `Field${++n}`;
	const df: DraftField = {
		id: d.nextId++,
		originalName: null,
		field: { name: fieldName, field_type: makeFieldType(typeName) }
	};
	d.fields.push(df);
	return df;
}

function columnRefs(d: TableDraft): string[][] {
	return [...d.foreign_keys.map((fk) => fk.columns), ...d.unique_constraints];
}

/** Renames a field and every reference to it within the draft. */
export function renameField(d: TableDraft, id: number, newName: string): void {
	const f = find(d, id);
	const old = f.field.name;
	f.field.name = newName;
	for (const cols of columnRefs(d)) {
		cols.forEach((c, i) => {
			if (lower(c) === lower(old)) cols[i] = newName;
		});
	}
	for (const fk of d.foreign_keys) {
		if (lower(fk.ref_table) === lower(d.name)) {
			fk.ref_columns = (fk.ref_columns ?? []).map((c) => (lower(c) === lower(old) ? newName : c));
		}
	}
}

/** Removes a field and the relationships/unique constraints that use it. */
export function removeField(d: TableDraft, id: number): void {
	const name = lower(find(d, id).field.name);
	d.fields = d.fields.filter((f) => f.id !== id);
	d.foreign_keys = d.foreign_keys.filter((fk) => !fk.columns.some((c) => lower(c) === name));
	d.unique_constraints = d.unique_constraints.filter((u) => !u.some((c) => lower(c) === name));
}

export function moveField(d: TableDraft, id: number, delta: number): void {
	const i = d.fields.findIndex((f) => f.id === id);
	const j = i + delta;
	if (i < 0 || j < 0 || j >= d.fields.length) return;
	const [f] = d.fields.splice(i, 1);
	d.fields.splice(j, 0, f);
}

export function setFieldType(d: TableDraft, id: number, typeName: FieldTypeName): void {
	const f = find(d, id);
	f.field.field_type = makeFieldType(typeName);
	if (typeName === 'auto_number') {
		for (const other of d.fields) other.field.primary_key = other.id === id;
		f.field.default = null;
	}
	if (typeName === 'calculated') {
		f.field.primary_key = false;
		f.field.default = null;
	}
}

/** Single-column relationship for a lookup field (replaces any existing one). */
export function setLookupTarget(d: TableDraft, id: number, refTable: string, refColumn: string): void {
	const name = find(d, id).field.name;
	d.foreign_keys = d.foreign_keys.filter(
		(fk) => !(fk.columns.length === 1 && lower(fk.columns[0]) === lower(name))
	);
	if (refTable) {
		d.foreign_keys.push({
			columns: [name],
			ref_table: refTable,
			ref_columns: refColumn ? [refColumn] : [],
			on_delete: 'no_action',
			on_update: 'no_action'
		});
	}
}

export function lookupTarget(d: TableDraft, fieldName: string): ForeignKeyDef | undefined {
	return d.foreign_keys.find((fk) => fk.columns.length === 1 && lower(fk.columns[0]) === lower(fieldName));
}

export function toDefinition(d: TableDraft): TableDef {
	return {
		name: d.name.trim(),
		description: d.description.trim() || null,
		fields: d.fields.map((f) => jsonClone(f.field)),
		foreign_keys: jsonClone(d.foreign_keys),
		unique_constraints: jsonClone(d.unique_constraints),
		checks: d.checks.filter((c) => c.trim() !== '')
	};
}

export function toDesign(d: TableDraft): TableDesign {
	return {
		definition: toDefinition(d),
		renames: d.fields
			.filter((f) => f.originalName !== null && f.originalName !== f.field.name)
			.map((f) => ({ from: f.originalName as string, to: f.field.name }))
	};
}

/** Saved fields that this draft deletes (their data is lost on save). */
export function removedFields(d: TableDraft, saved: TableDef): string[] {
	const kept = new Set(d.fields.map((f) => f.originalName).filter((n) => n !== null));
	return saved.fields.map((f) => f.name).filter((n) => !kept.has(n));
}

const RESERVED = /^(sqlite_|__app_)/i;

function nameProblem(kind: string, name: string): string | null {
	if (name.trim() === '') return `${kind} name is empty`;
	if (name.trim() !== name) return `${kind} name ${JSON.stringify(name)} has surrounding spaces`;
	if (RESERVED.test(name)) return `${kind} name ${JSON.stringify(name)} is reserved`;
	return null;
}

/** Quick client-side checks; the backend performs full validation. */
export function validateDraft(d: TableDraft): string[] {
	const problems: string[] = [];
	const tableProblem = nameProblem('Table', d.name);
	if (tableProblem) problems.push(tableProblem);
	if (d.fields.length === 0) problems.push('A table needs at least one field');
	const seen = new Set<string>();
	for (const { field } of d.fields) {
		const p = nameProblem('Field', field.name);
		if (p) problems.push(p);
		if (seen.has(lower(field.name))) problems.push(`Duplicate field name ${JSON.stringify(field.name)}`);
		seen.add(lower(field.name));
		const t = field.field_type;
		if (t.type === 'calculated' && !t.expression.trim()) {
			problems.push(`Calculated field ${JSON.stringify(field.name)} needs an expression`);
		}
		if (t.type === 'lookup' && !lookupTarget(d, field.name)) {
			problems.push(`Lookup field ${JSON.stringify(field.name)} needs a related table`);
		}
	}
	const autos = d.fields.filter((f) => f.field.field_type.type === 'auto_number');
	const pks = d.fields.filter((f) => f.field.primary_key);
	if (autos.length > 1) problems.push('Only one AutoNumber field is allowed');
	if (autos.length === 1 && (pks.length !== 1 || !autos[0].field.primary_key)) {
		problems.push('An AutoNumber field must be the only primary key');
	}
	return problems;
}

/** Inserts a new field before position `index` (Design View "Insert Rows"). */
export function insertField(d: TableDraft, index: number, typeName: FieldTypeName = 'short_text'): DraftField {
	const f = addField(d, typeName);
	d.fields.pop();
	d.fields.splice(Math.max(0, Math.min(index, d.fields.length)), 0, f);
	return f;
}

/**
 * Design View "Primary Key" button: makes the field the only primary key, or
 * removes the key when it already is the key. An AutoNumber key can only be
 * moved, not removed, because AutoNumber requires it.
 */
export function togglePrimaryKey(d: TableDraft, id: number): void {
	const f = find(d, id);
	const isSoleKey = f.field.primary_key && d.fields.filter((x) => x.field.primary_key).length === 1;
	if (isSoleKey) {
		if (f.field.field_type.type !== 'auto_number') f.field.primary_key = false;
		return;
	}
	for (const other of d.fields) {
		other.field.primary_key = other.id === id;
		if (other.id !== id && other.field.field_type.type === 'auto_number') {
			other.field.field_type = { type: 'large_number' };
		}
	}
	if (f.field.field_type.type === 'calculated') f.field.field_type = makeFieldType('short_text');
}

/** A name like "Table1" not used by any of `taken`. */
export function nextFreeName(prefix: string, taken: string[]): string {
	const lowerTaken = new Set(taken.map(lower));
	let n = 1;
	while (lowerTaken.has(lower(`${prefix}${n}`))) n++;
	return `${prefix}${n}`;
}
