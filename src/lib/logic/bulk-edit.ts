/**
 * Editing many records at once in the Datasheet: pasting a block of cells
 * from a spreadsheet, copying rows out, filling one value into many records,
 * and find & replace. Everything becomes one atomic `batch_records` call.
 */
import type { BatchOp, FieldDef, JsonValue, RecordKey, RecordRow, RecordValues } from '../api/types';
import { cellValue, type DateOrder } from './import';
import type { LookupOption } from './lookups';
import { displayValue } from './values';

/** The backend's limit on operations in one batch. */
export const MAX_BATCH = 10_000;

/** Cells from clipboard text: tab-separated rows, as spreadsheets copy them (quoted cells may hold tabs and line breaks). */
export function parseClipboard(text: string): string[][] {
	const rows: string[][] = [];
	let row: string[] = [];
	let cell = '';
	let quoted = false;
	const s = text.replace(/\r\n?/g, '\n').replace(/\n$/, '');
	for (let i = 0; i < s.length; i++) {
		const c = s[i];
		if (quoted) {
			if (c === '"' && s[i + 1] === '"') {
				cell += '"';
				i++;
			} else if (c === '"') quoted = false;
			else cell += c;
		} else if (c === '"' && cell === '') quoted = true;
		else if (c === '\t') {
			row.push(cell);
			cell = '';
		} else if (c === '\n') {
			row.push(cell);
			rows.push(row);
			row = [];
			cell = '';
		} else cell += c;
	}
	row.push(cell);
	rows.push(row);
	return rows;
}

/** Whether pasted text is more than one cell. */
export function isBlock(text: string): boolean {
	return /[\t\n]/.test(text.replace(/\r?\n$/, ''));
}

/** Rows as tab-separated text spreadsheets understand. */
export function toClipboard(rows: string[][]): string {
	const quote = (c: string) => (/[\t\n"]/.test(c) ? `"${c.replace(/"/g, '""')}"` : c);
	return rows.map((r) => r.map(quote).join('\t')).join('\n');
}

/** A cell as text, the way the Datasheet shows it (relations by their label). */
export function cellText(v: JsonValue | undefined, field: FieldDef | undefined, options?: LookupOption[]): string {
	if (field?.field_type.type === 'lookup' && options) {
		const o = options.find((x) => JSON.stringify(x.value) === JSON.stringify(v ?? null));
		if (o) return o.label;
	}
	return displayValue(v, field?.field_type);
}

/** Text typed or pasted into a field as its value: a relation also accepts the linked record's label. */
export function parseCell(text: string, field: FieldDef, order: DateOrder, options?: LookupOption[]): { value: JsonValue } | { problem: string } {
	if (field.field_type.type === 'lookup' && options && text.trim()) {
		const t = text.trim().toLowerCase();
		const byLabel = options.filter((o) => o.label.trim().toLowerCase() === t);
		if (byLabel.length === 1) return { value: byLabel[0].value };
		if (byLabel.length > 1) return { problem: `more than one ${field.name} is called “${text.trim()}”` };
	}
	return cellValue(text, field, order);
}

export interface PastePlan {
	ops: BatchOp[];
	updated: number;
	added: number;
	/** Pasted columns that fell on read-only fields or past the last column. */
	skippedColumns: number;
	problems: { row: number; column: string; message: string }[];
}

/**
 * The batch that pastes `cells` with its top-left corner on row `start` (an
 * index into `rows`, or `rows.length` for the new-record row) and column
 * `startCol`. Rows below the page's last become new records.
 */
export function planPaste(opts: {
	cells: string[][];
	rows: RecordRow[];
	start: number;
	columns: string[];
	startCol: number;
	fields: FieldDef[];
	editable: (col: string) => boolean;
	order: DateOrder;
	lookups?: Record<string, LookupOption[]>;
}): PastePlan {
	const { cells, rows, start, columns, startCol, fields, editable, order, lookups = {} } = opts;
	const byName = new Map(fields.map((f) => [f.name, f]));
	const width = Math.max(...cells.map((r) => r.length));
	const targets = Array.from({ length: width }, (_, j) => columns[startCol + j]).map((c) => (c && editable(c) ? c : null));
	const plan: PastePlan = { ops: [], updated: 0, added: 0, skippedColumns: targets.filter((t) => t === null).length, problems: [] };
	cells.forEach((line, r) => {
		const values: RecordValues = {};
		targets.forEach((col, j) => {
			if (!col) return;
			const field = byName.get(col);
			if (!field) return;
			const parsed = parseCell(line[j] ?? '', field, order, lookups[col]);
			if ('problem' in parsed) plan.problems.push({ row: r + 1, column: col, message: parsed.problem });
			else values[col] = parsed.value;
		});
		if (!Object.keys(values).length) return;
		const target = rows[start + r];
		if (target?.key) {
			plan.ops.push({ op: 'update', key: target.key, values });
			plan.updated++;
		} else if (!target) {
			plan.ops.push({ op: 'insert', values });
			plan.added++;
		}
	});
	return plan;
}

/** Updates setting `column` to `value` on every record in `keys`. */
export function planFill(keys: RecordKey[], column: string, value: JsonValue): BatchOp[] {
	return keys.map((key) => ({ op: 'update', key, values: { [column]: value } }));
}

export interface ReplaceOptions {
	find: string;
	replace: string;
	matchCase: boolean;
	/** Only cells whose whole text is `find`. */
	wholeCell: boolean;
}

const escapeRegExp = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/** The text after replacing, or null when nothing matched. */
export function replaceText(text: string, o: ReplaceOptions): string | null {
	if (!o.find) return null;
	if (o.wholeCell) {
		const same = o.matchCase ? text === o.find : text.toLowerCase() === o.find.toLowerCase();
		return same ? o.replace : null;
	}
	const re = new RegExp(escapeRegExp(o.find), o.matchCase ? 'g' : 'gi');
	if (!re.test(text)) return null;
	re.lastIndex = 0;
	return text.replace(re, () => o.replace);
}

/** Fields whose text find & replace can change. */
export function replaceableFields(fields: FieldDef[]): FieldDef[] {
	return fields.filter((f) => ['short_text', 'long_text', 'hyperlink'].includes(f.field_type.type) && !f.primary_key);
}

/** The updates replacing text in `columns` of `rows` (rows hold `allColumns` in order), and how many cells change. */
export function planReplace(rows: RecordRow[], allColumns: string[], columns: string[], o: ReplaceOptions): { ops: BatchOp[]; cells: number } {
	const ops: BatchOp[] = [];
	let cells = 0;
	for (const row of rows) {
		if (!row.key) continue;
		const values: RecordValues = {};
		for (const col of columns) {
			const v = row.values[allColumns.indexOf(col)];
			if (typeof v !== 'string') continue;
			const next = replaceText(v, o);
			if (next !== null && next !== v) {
				values[col] = next;
				cells++;
			}
		}
		if (Object.keys(values).length) ops.push({ op: 'update', key: row.key, values });
	}
	return { ops, cells };
}
