import type {
	DiagramLayout,
	FieldDef,
	FieldType,
	FkAction,
	ForeignKeyDef,
	TableDef,
	TableDesign
} from '../api/types';
import { jsonClone } from './values';

/**
 * Relationships are foreign keys on the "many" (child) table pointing at a
 * key of the "one" (parent) table. Everything here is pure: it turns table
 * definitions into diagram data, and relationship edits into table designs
 * that the backend saves with `save_table_design`.
 */

export type Cardinality = 'one_to_one' | 'one_to_many';

export interface Relationship {
	/** Stable while the child table and its columns keep their names. */
	id: string;
	parent: string;
	parentColumns: string[];
	child: string;
	childColumns: string[];
	cardinality: Cardinality;
	onUpdate: FkAction;
	onDelete: FkAction;
}

function lower(s: string) {
	return s.toLowerCase();
}

function same(a: string, b: string) {
	return lower(a) === lower(b);
}

function field(def: TableDef, name: string): FieldDef | undefined {
	return def.fields.find((f) => same(f.name, name));
}

export function relationshipId(child: string, columns: string[]): string {
	return `${child}\u0000${columns.join('\u0000')}`;
}

function primaryKey(def: TableDef): string[] {
	return def.fields.filter((f) => f.primary_key).map((f) => f.name);
}

/** True when `columns` can hold each value at most once in `def`. */
function isUniqueSet(def: TableDef, columns: string[]): boolean {
	const wanted = columns.map(lower).sort().join('\u0000');
	const pk = primaryKey(def).map(lower).sort().join('\u0000');
	if (wanted === pk) return true;
	if (columns.length === 1 && field(def, columns[0])?.unique) return true;
	return (def.unique_constraints ?? []).some((u) => u.map(lower).sort().join('\u0000') === wanted);
}

/** Every single- or multi-column relationship between the given tables. */
export function relationshipsOf(defs: TableDef[]): Relationship[] {
	const byName = new Map(defs.map((d) => [lower(d.name), d]));
	const out: Relationship[] = [];
	for (const child of defs) {
		for (const fk of child.foreign_keys ?? []) {
			const parent = byName.get(lower(fk.ref_table));
			if (!parent) continue;
			const parentColumns = fk.ref_columns?.length ? fk.ref_columns : primaryKey(parent);
			out.push({
				id: relationshipId(child.name, fk.columns),
				parent: parent.name,
				parentColumns,
				child: child.name,
				childColumns: fk.columns,
				cardinality: isUniqueSet(child, fk.columns) ? 'one_to_one' : 'one_to_many',
				onUpdate: fk.on_update ?? 'no_action',
				onDelete: fk.on_delete ?? 'no_action'
			});
		}
	}
	return out;
}

/** Columns another table can link to: the single-column key and unique fields. */
export function keyColumns(def: TableDef): string[] {
	const pk = primaryKey(def);
	const keys = pk.length === 1 ? [...pk] : [];
	for (const f of def.fields) if (f.unique && !keys.includes(f.name)) keys.push(f.name);
	return keys;
}

export type ValueKind = 'integer' | 'real' | 'text' | 'other';

/** How a field's values are stored, to warn about links that can never match. */
export function valueKind(t: FieldType): ValueKind {
	switch (t.type) {
		case 'auto_number':
		case 'large_number':
		case 'currency':
		case 'yes_no':
			return 'integer';
		case 'number':
			return t.real ? 'real' : 'integer';
		case 'lookup':
			return t.value_type === 'text' ? 'text' : 'integer';
		case 'short_text':
		case 'long_text':
		case 'hyperlink':
		case 'date_time':
			return 'text';
		default:
			return 'other';
	}
}

/** Whether a field can become a pick-from-a-list (lookup) field. */
export function canBeDropdown(f: FieldDef): boolean {
	return (
		!f.primary_key &&
		['number', 'large_number', 'short_text', 'long_text', 'lookup', 'unknown'].includes(f.field_type.type) &&
		!(f.field_type.type === 'number' && f.field_type.real)
	);
}

function singular(name: string): string {
	if (/ies$/i.test(name) && name.length > 4) return name.slice(0, -3) + (name.endsWith('IES') ? 'Y' : 'y');
	if (/[^s]s$/i.test(name) && name.length > 3) return name.slice(0, -1);
	return name;
}

function freeName(base: string, taken: string[]): string {
	const used = new Set(taken.map(lower));
	let name = base;
	for (let n = 2; used.has(lower(name)); n++) name = `${base}${n}`;
	return name;
}

/**
 * Name for a new linking field in `child`, in the style of the table's
 * existing names: "CustomerID" next to "ID", "customer_id" next to "id".
 */
export function suggestFieldName(parent: string, parentColumn: string, child: TableDef): string {
	const base = singular(parent).replace(/\s+/g, '');
	let name: string;
	if (parentColumn === 'id') name = `${lower(base)}_id`;
	else if (same(parentColumn, 'id')) name = `${base}ID`;
	else name = `${base}${parentColumn.replace(/\s+/g, '')}`;
	return freeName(name, child.fields.map((f) => f.name));
}

/** The first text field other than the key: what a dropdown should show. */
export function displayColumnFor(parent: TableDef, keyColumn: string): string | null {
	const text = parent.fields.find(
		(f) => !same(f.name, keyColumn) && (f.field_type.type === 'short_text' || f.field_type.type === 'long_text')
	);
	return text?.name ?? null;
}

export interface LinkSpec {
	parent: TableDef;
	parentColumn: string;
	/** An existing field of the child table, or null to add `newField`. */
	childColumn: string | null;
	newField?: string;
	cardinality: Cardinality;
	onUpdate: FkAction;
	onDelete: FkAction;
	/** Turn the linking field into a dropdown of parent records. */
	asDropdown: boolean;
}

function dropdownType(parent: TableDef, keyColumn: string): FieldType {
	const key = field(parent, keyColumn);
	return {
		type: 'lookup',
		value_type: key && valueKind(key.field_type) === 'text' ? 'text' : 'integer',
		display_column: displayColumnFor(parent, keyColumn)
	};
}

function plainType(parent: TableDef, keyColumn: string): FieldType {
	const key = field(parent, keyColumn);
	return key && valueKind(key.field_type) === 'text'
		? { type: 'short_text', max_length: 255 }
		: { type: 'number', real: false };
}

function isSingle(fk: ForeignKeyDef, column: string) {
	return fk.columns.length === 1 && same(fk.columns[0], column);
}

export type LinkKind = Cardinality | 'many_to_many';

/** The "new relationship" form on the Relationships screen. */
export interface LinkDraft {
	parent: string;
	parentColumn: string;
	child: string;
	/** An existing field of the child table, or null to add `newField`. */
	childColumn: string | null;
	newField: string;
	kind: LinkKind;
	onUpdate: FkAction;
	onDelete: FkAction;
	asDropdown: boolean;
}

/**
 * Turns a drag from one field onto another table (or one of its fields)
 * into a relationship proposal. Dragging from a key means "records here can
 * have many over there"; dragging from any other field means "this field
 * points at that table".
 */
export function proposeLink(
	from: TableDef,
	fromField: string,
	to: TableDef,
	toField: string | null
): LinkDraft | string {
	const fromKeys = keyColumns(from);
	const toKeys = keyColumns(to);
	let parent: TableDef, parentColumn: string, child: TableDef, childColumn: string | null;
	if (fromKeys.includes(fromField)) {
		[parent, parentColumn, child, childColumn] = [from, fromField, to, toField];
	} else {
		const key = toField && toKeys.includes(toField) ? toField : toKeys[0];
		if (!key) return `“${to.name}” has no ID or unique field to link to.`;
		[parent, parentColumn, child, childColumn] = [to, key, from, fromField];
	}
	if (same(parent.name, child.name) && childColumn !== null && same(parentColumn, childColumn)) {
		return 'A field cannot link to itself.';
	}
	const childField = childColumn === null ? undefined : field(child, childColumn);
	return {
		parent: parent.name,
		parentColumn,
		child: child.name,
		childColumn,
		newField: suggestFieldName(parent.name, parentColumn, child),
		kind: childField && isUniqueSet(child, [childField.name]) ? 'one_to_one' : 'one_to_many',
		onUpdate: 'cascade',
		onDelete: 'restrict',
		asDropdown: true
	};
}

/** A warning when the two sides store different kinds of values. */
export function typeMismatch(parent: TableDef, parentColumn: string, child: TableDef, childColumn: string | null): string | null {
	if (childColumn === null) return null;
	const a = field(parent, parentColumn);
	const b = field(child, childColumn);
	if (!a || !b) return null;
	const [ka, kb] = [valueKind(a.field_type), valueKind(b.field_type)];
	if (ka === kb || ka === 'other' || kb === 'other') return null;
	const word = { integer: 'whole numbers', real: 'decimal numbers', text: 'text' };
	return `“${a.name}” holds ${word[ka]} but “${b.name}” holds ${word[kb]}. Values only link when they match exactly.`;
}

/** The child table's design with a new or changed relationship to `spec.parent`. */
export function linkDesign(child: TableDef, spec: LinkSpec): TableDesign {
	const def = jsonClone(child);
	def.foreign_keys ??= [];
	let column = spec.childColumn;
	if (column === null) {
		column = spec.newField?.trim() || suggestFieldName(spec.parent.name, spec.parentColumn, child);
		def.fields.push({
			name: column,
			field_type: spec.asDropdown
				? dropdownType(spec.parent, spec.parentColumn)
				: plainType(spec.parent, spec.parentColumn)
		});
	}
	const f = field(def, column);
	if (!f) throw new Error(`“${child.name}” has no field “${column}”`);
	if (spec.asDropdown && canBeDropdown(f) && f.field_type.type !== 'lookup') {
		f.field_type = dropdownType(spec.parent, spec.parentColumn);
	}
	if (!f.primary_key) f.unique = spec.cardinality === 'one_to_one';
	def.foreign_keys = def.foreign_keys.filter((fk) => !isSingle(fk, column));
	def.foreign_keys.push({
		columns: [f.name],
		ref_table: spec.parent.name,
		ref_columns: [spec.parentColumn],
		on_update: spec.onUpdate,
		on_delete: spec.onDelete
	});
	return { definition: def, renames: [] };
}

/** The child table's design without `rel`; dropdown fields become plain values again. */
export function unlinkDesign(child: TableDef, rel: Relationship): TableDesign {
	const def = jsonClone(child);
	const key = relationshipId(child.name, rel.childColumns);
	def.foreign_keys = (def.foreign_keys ?? []).filter((fk) => relationshipId(child.name, fk.columns) !== key);
	for (const name of rel.childColumns) {
		const f = field(def, name);
		if (f?.field_type.type === 'lookup') {
			f.field_type =
				f.field_type.value_type === 'text' ? { type: 'short_text', max_length: 255 } : { type: 'number', real: false };
		}
	}
	return { definition: def, renames: [] };
}

/** The child table's design with `rel`'s rules and cardinality changed. */
export function updateLinkDesign(
	child: TableDef,
	rel: Relationship,
	change: { cardinality: Cardinality; onUpdate: FkAction; onDelete: FkAction }
): TableDesign {
	const def = jsonClone(child);
	const key = relationshipId(child.name, rel.childColumns);
	const fk = (def.foreign_keys ?? []).find((k) => relationshipId(child.name, k.columns) === key);
	if (!fk) throw new Error('That relationship no longer exists');
	fk.on_update = change.onUpdate;
	fk.on_delete = change.onDelete;
	if (rel.childColumns.length === 1 && change.cardinality !== rel.cardinality) {
		const f = field(def, rel.childColumns[0]);
		if (f && !f.primary_key) f.unique = change.cardinality === 'one_to_one';
	}
	return { definition: def, renames: [] };
}

/**
 * A junction table for a many-to-many relationship: one row per pair, each
 * side a dropdown, the pair unique, and rows removed with either side.
 */
export function junctionDef(a: TableDef, b: TableDef, name: string): TableDef {
	const aKey = primaryKey(a);
	const bKey = primaryKey(b);
	if (aKey.length !== 1 || bKey.length !== 1) {
		throw new Error('Both tables need a single-field primary key to be linked many-to-many');
	}
	const shell: TableDef = { name, fields: [{ name: 'ID', field_type: { type: 'auto_number' }, primary_key: true }] };
	const aField = suggestFieldName(a.name, aKey[0], shell);
	shell.fields.push({ name: aField, field_type: dropdownType(a, aKey[0]), required: true });
	const bField = suggestFieldName(b.name, bKey[0], shell);
	shell.fields.push({ name: bField, field_type: dropdownType(b, bKey[0]), required: true });
	return {
		...shell,
		foreign_keys: [
			{ columns: [aField], ref_table: a.name, ref_columns: aKey, on_update: 'cascade', on_delete: 'cascade' },
			{ columns: [bField], ref_table: b.name, ref_columns: bKey, on_update: 'cascade', on_delete: 'cascade' }
		],
		unique_constraints: [[aField, bField]],
		checks: [],
		description: `Links ${a.name} and ${b.name} (many-to-many).`
	};
}

/** A free name for the junction between `a` and `b`, like "ProductSuppliers". */
export function junctionName(a: string, b: string, taken: string[]): string {
	return freeName(`${singular(a).replace(/\s+/g, '')}${b.replace(/\s+/g, '')}`, taken);
}

/** Plain-language sentence describing a relationship. */
export function describeRelationship(rel: Pick<Relationship, 'parent' | 'child' | 'cardinality'>): string {
	return rel.cardinality === 'one_to_one'
		? `Each ${rel.parent} record has at most one matching ${rel.child} record.`
		: `Each ${rel.parent} record can have many ${rel.child} records; each of those belongs to one ${rel.parent} record.`;
}

// ── Diagram geometry ────────────────────────────────────────────────────────

export const CARD_WIDTH = 248;
export const CARD_HEADER = 44;
export const CARD_ROW = 30;
export const CARD_PADDING = 6;

export function cardHeight(def: TableDef): number {
	return CARD_HEADER + CARD_PADDING * 2 + Math.max(1, def.fields.length) * CARD_ROW;
}

/** Vertical centre of field `index` relative to the card's top. */
export function rowCenter(index: number): number {
	return CARD_HEADER + CARD_PADDING + index * CARD_ROW + CARD_ROW / 2;
}

/**
 * Lays tables out left to right by dependency: tables nothing points at
 * come first, and every child sits in a column after its parents.
 */
export function autoLayout(defs: TableDef[], rels: Relationship[]): DiagramLayout {
	const rank = new Map(defs.map((d) => [d.name, 0]));
	// Longest-path ranking; bounded so a cycle of links cannot loop forever.
	for (let pass = 0; pass < defs.length; pass++) {
		let changed = false;
		for (const r of rels) {
			if (r.parent === r.child) continue;
			const want = (rank.get(r.parent) ?? 0) + 1;
			if ((rank.get(r.child) ?? 0) < want) {
				rank.set(r.child, want);
				changed = true;
			}
		}
		if (!changed) break;
	}
	const columns = new Map<number, TableDef[]>();
	for (const d of defs) {
		const r = rank.get(d.name) ?? 0;
		columns.set(r, [...(columns.get(r) ?? []), d]);
	}
	const layout: DiagramLayout = {};
	for (const [r, tables] of columns) {
		let y = 48;
		for (const d of tables) {
			layout[d.name] = { x: 48 + r * (CARD_WIDTH + 112), y };
			y += cardHeight(d) + 48;
		}
	}
	return layout;
}

/** Positions for tables missing from `layout`, placed below everything else. */
export function placeMissing(defs: TableDef[], layout: DiagramLayout): DiagramLayout {
	const next = { ...layout };
	const missing = defs.filter((d) => !next[d.name]);
	if (!missing.length) return next;
	const bottom = defs
		.filter((d) => next[d.name])
		.reduce((max, d) => Math.max(max, next[d.name].y + cardHeight(d)), 0);
	let x = 48;
	let rowHeight = 0;
	let y = bottom ? bottom + 64 : 48;
	missing.forEach((d, i) => {
		if (i > 0 && i % 4 === 0) {
			x = 48;
			y += rowHeight + 48;
			rowHeight = 0;
		}
		next[d.name] = { x, y };
		x += CARD_WIDTH + 64;
		rowHeight = Math.max(rowHeight, cardHeight(d));
	});
	return next;
}

export interface Point {
	x: number;
	y: number;
}

export interface EdgeRoute {
	path: string;
	start: Point;
	end: Point;
	/** Which way the line leaves the parent and enters the child: 1 right, -1 left. */
	startDir: 1 | -1;
	endDir: 1 | -1;
}

const GAP = 40;
const LOOP = 32;
const RADIUS = 10;

/**
 * An orthogonal connector with rounded corners from the parent's key row to
 * the child's linking row. Cards side by side connect facing edges; cards
 * stacked in the same column (or a table linked to itself) loop round the
 * right-hand side.
 */
export function edgeRoute(parent: Point, parentY: number, child: Point, childY: number): EdgeRoute {
	let x1: number, x2: number, startDir: 1 | -1, endDir: 1 | -1;
	if (child.x >= parent.x + CARD_WIDTH + GAP) {
		[x1, x2, startDir, endDir] = [parent.x + CARD_WIDTH, child.x, 1, -1];
	} else if (child.x + CARD_WIDTH + GAP <= parent.x) {
		[x1, x2, startDir, endDir] = [parent.x, child.x + CARD_WIDTH, -1, 1];
	} else {
		[x1, x2, startDir, endDir] = [parent.x + CARD_WIDTH, child.x + CARD_WIDTH, 1, 1];
	}
	const y1 = parent.y + parentY;
	const y2 = child.y + childY;
	const turn = startDir === -endDir ? (x1 + x2) / 2 : Math.max(x1, x2) + LOOP;
	const points: Point[] = [
		{ x: x1, y: y1 },
		{ x: turn, y: y1 },
		{ x: turn, y: y2 },
		{ x: x2, y: y2 }
	];
	return { path: roundedPath(points), start: points[0], end: points[3], startDir, endDir };
}

function roundedPath(points: Point[]): string {
	let d = `M ${points[0].x} ${points[0].y}`;
	for (let i = 1; i < points.length - 1; i++) {
		const [prev, cur, next] = [points[i - 1], points[i], points[i + 1]];
		const r = Math.min(
			RADIUS,
			Math.hypot(cur.x - prev.x, cur.y - prev.y) / 2,
			Math.hypot(next.x - cur.x, next.y - cur.y) / 2
		);
		if (r < 0.5) {
			d += ` L ${cur.x} ${cur.y}`;
			continue;
		}
		const a = toward(cur, prev, r);
		const b = toward(cur, next, r);
		d += ` L ${a.x} ${a.y} Q ${cur.x} ${cur.y} ${b.x} ${b.y}`;
	}
	const last = points[points.length - 1];
	return `${d} L ${last.x} ${last.y}`;
}

function toward(from: Point, to: Point, distance: number): Point {
	const len = Math.hypot(to.x - from.x, to.y - from.y) || 1;
	return { x: from.x + ((to.x - from.x) / len) * distance, y: from.y + ((to.y - from.y) / len) * distance };
}
