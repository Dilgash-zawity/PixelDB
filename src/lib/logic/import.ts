/** Importing rows from a CSV or spreadsheet: field names, type guesses and turning cell text into record values. */
import type { FieldDef, FieldType, FieldTypeName, JsonValue, RecordValues } from '../api/types';
import { makeFieldType } from './field-types';
import { identifierProblem } from './values';

/** Which part of a date like 03/04/2024 is the day. */
export type DateOrder = 'dmy' | 'mdy';

/** Where one column of the file goes: a field's name, or null to skip it. */
export type ColumnTarget = string | null;

export interface CellProblem {
	/** The row in the file, counting from 1 (headers included). */
	row: number;
	column: string;
	message: string;
}

/** Types an import can create or fill. */
export const IMPORT_TYPES: FieldTypeName[] = ['short_text', 'long_text', 'number', 'currency', 'date_time', 'yes_no', 'hyperlink', 'large_number'];

const YES = ['yes', 'y', 'true', '1', 'x', '✓', '✔', 'on'];
const NO = ['no', 'n', 'false', '0', '', 'off'];
const CURRENCY_SIGNS = /[$€£¥₹₩₺₽¢]|\b(USD|EUR|GBP|JPY|IQD|CHF|CAD|AUD)\b/gi;
const HAS_CURRENCY = /[$€£¥₹₩₺₽¢]|\b(USD|EUR|GBP|JPY|IQD|CHF|CAD|AUD)\b/i;
const INT32 = 2_147_483_647;

const squash = (s: string) => s.toLowerCase().replace(/[\s_-]+/g, '');

/** Field names for the file's columns: its header row (cleaned, made unique) or Column 1, Column 2, …; never `ID`, which the new table uses. */
export function columnNames(header: string[] | null, count: number, taken: string[] = ['ID']): string[] {
	const used = new Set(taken.map((t) => t.toLowerCase()));
	const out: string[] = [];
	for (let i = 0; i < count; i++) {
		let base = (header?.[i] ?? '')
			.replace(/[\u0000-\u001f]/g, ' ')
			.replace(/\s+/g, ' ')
			.trim()
			.slice(0, 120);
		if (!base || identifierProblem(base)) base = base.replace(/^(sqlite_|__app_)/i, '') || `Column ${i + 1}`;
		let name = base;
		for (let n = 2; used.has(name.toLowerCase()); n++) name = `${base} ${n}`;
		used.add(name.toLowerCase());
		out.push(name);
	}
	return out;
}

/** The number of columns: the longest row. */
export function columnCount(rows: string[][]): number {
	return rows.reduce((n, r) => Math.max(n, r.length), 0);
}

/** Guesses whether the first row holds names rather than data: it has text where the rest has numbers or dates, or no blanks and no repeats. */
export function looksLikeHeader(rows: string[][]): boolean {
	if (rows.length < 2) return true;
	const [first, ...rest] = rows;
	const sample = rest.slice(0, 50);
	const nonText = (s: string) => parseNumber(s) !== null || parseDate(s, 'dmy') !== null;
	for (let c = 0; c < first.length; c++) {
		const below = sample.map((r) => r[c] ?? '').filter((s) => s.trim());
		if (below.length && below.every(nonText) && first[c]?.trim() && !nonText(first[c])) return true;
	}
	const cells = first.map((s) => s.trim().toLowerCase());
	return cells.every((s) => s && !nonText(s)) && new Set(cells).size === cells.length;
}

/** A plain number from text such as `1,234.5`, `-3`, `1 000`, or null. */
export function parseNumber(text: string): number | null {
	let s = text.trim().replace(/[\s ']/g, '');
	if (!s) return null;
	if (/^\(.*\)$/.test(s)) s = '-' + s.slice(1, -1);
	// 1.234,56 (comma decimals) vs 1,234.56.
	if (/^-?\d{1,3}(\.\d{3})+,\d+$/.test(s) || /^-?\d+,\d{1,2}$/.test(s)) s = s.replace(/\./g, '').replace(',', '.');
	else if (/^-?\d{1,3}(,\d{3})+(\.\d+)?$/.test(s)) s = s.replace(/,/g, '');
	if (!/^[-+]?(\d+\.?\d*|\.\d+)(e[-+]?\d+)?$/i.test(s)) return null;
	const n = Number(s);
	return Number.isFinite(n) ? n : null;
}

/** A money amount as an exact decimal string (`-1234.50`) from text like `$1,234.50`, or null. */
export function parseMoney(text: string): string | null {
	const stripped = text.replace(CURRENCY_SIGNS, '').trim();
	const n = parseNumber(stripped);
	if (n === null) return null;
	// Keep the digits as written rather than going through floating point.
	let s = stripped.replace(/[\s ']/g, '');
	const negative = s.startsWith('-') || /^\(.*\)$/.test(s);
	s = s.replace(/^[-+(]|\)$/g, '');
	if (/^\d{1,3}(\.\d{3})+,\d+$/.test(s) || /^\d+,\d{1,2}$/.test(s)) s = s.replace(/\./g, '').replace(',', '.');
	else s = s.replace(/,/g, '');
	if (/e/i.test(s)) s = String(Math.abs(n));
	return (negative && Number(s) !== 0 ? '-' : '') + s;
}

/** Yes/No from text, or null when it is neither. */
export function parseYesNo(text: string): boolean | null {
	const s = text.trim().toLowerCase();
	if (YES.includes(s)) return true;
	if (NO.includes(s)) return false;
	return null;
}

const pad = (n: number) => String(n).padStart(2, '0');

function validDate(y: number, m: number, d: number): boolean {
	if (m < 1 || m > 12 || d < 1 || y < 1 || y > 9999) return false;
	return d <= new Date(Date.UTC(y, m, 0)).getUTCDate();
}

/** An ISO 8601 date or date-time from text like `2024-03-05`, `05/03/2024 14:30` or `2024-03-05T10:00:00Z`, or null. */
export function parseDate(text: string, order: DateOrder): string | null {
	const s = text.trim();
	let m = /^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::(\d{2})(\.\d+)?)?(Z|[+-]\d{2}:?\d{2})?)?$/.exec(s);
	if (m) {
		const [, y, mo, d, h, mi, sec, frac, zone] = m;
		if (!validDate(+y, +mo, +d)) return null;
		if (h === undefined) return `${y}-${mo}-${d}`;
		if (+h > 23 || +mi > 59 || (sec && +sec > 59)) return null;
		return `${y}-${mo}-${d}T${h}:${mi}:${sec ?? '00'}${frac ?? ''}${zone ? (zone === 'Z' ? 'Z' : zone.length === 5 ? `${zone.slice(0, 3)}:${zone.slice(3)}` : zone) : ''}`;
	}
	m = /^(\d{1,2})[/.-](\d{1,2})[/.-](\d{2}|\d{4})(?:[ T](\d{1,2}):(\d{2})(?::(\d{2}))?\s*([ap]m)?)?$/i.exec(s);
	if (!m) return null;
	const [, a, b, yy, h, mi, sec, ampm] = m;
	const [d, mo] = order === 'dmy' ? [+a, +b] : [+b, +a];
	const y = yy.length === 2 ? 2000 + +yy - (+yy > 68 ? 100 : 0) : +yy;
	if (!validDate(y, mo, d)) return null;
	const date = `${String(y).padStart(4, '0')}-${pad(mo)}-${pad(d)}`;
	if (h === undefined) return date;
	let hour = +h;
	if (ampm) {
		if (hour < 1 || hour > 12) return null;
		hour = (hour % 12) + (ampm.toLowerCase() === 'pm' ? 12 : 0);
	}
	if (hour > 23 || +mi > 59 || (sec && +sec > 59)) return null;
	return `${date}T${pad(hour)}:${mi}:${sec ?? '00'}`;
}

/** How this computer's language writes dates: month first in the US, day first elsewhere. */
export function localDateOrder(language = typeof navigator === 'undefined' ? '' : navigator.language): DateOrder {
	return /^en-(US|PH)$|^en$/i.test(language) ? 'mdy' : 'dmy';
}

/** Day-first or month-first, from dates in the column where only one reading works; `fallback` when every date reads both ways. */
export function guessDateOrder(values: string[], fallback: DateOrder = 'dmy'): DateOrder {
	for (const v of values) {
		const m = /^(\d{1,2})[/.-](\d{1,2})[/.-]\d{2,4}/.exec(v.trim());
		if (!m) continue;
		if (+m[1] > 12) return 'dmy';
		if (+m[2] > 12) return 'mdy';
	}
	return fallback;
}

/** The type that fits every non-blank value of a column. */
export function guessType(values: string[], order: DateOrder): FieldType {
	const filled = values.map((v) => v.trim()).filter(Boolean);
	if (!filled.length) return makeFieldType('short_text');
	if (filled.every((v) => parseYesNo(v) !== null) && filled.some((v) => !/^\d+$/.test(v))) return { type: 'yes_no' };
	// Codes like 00123 or phone numbers keep their leading zeros as text.
	const leadingZero = filled.some((v) => /^0\d/.test(v));
	const numbers = filled.map(parseNumber);
	if (!leadingZero && numbers.every((n) => n !== null)) {
		if (numbers.every((n) => Number.isInteger(n))) {
			return numbers.every((n) => Math.abs(n!) <= INT32) ? { type: 'number', real: false } : { type: 'large_number' };
		}
		return { type: 'number', real: true };
	}
	const money = filled.filter((v) => HAS_CURRENCY.test(v));
	if (money.length && filled.every((v) => parseMoney(v) !== null)) {
		const scale = Math.max(2, ...filled.map((v) => (parseMoney(v)!.split('.')[1] ?? '').length));
		return { type: 'currency', scale: Math.min(scale, 4) };
	}
	if (filled.every((v) => parseDate(v, order) !== null)) return { type: 'date_time' };
	if (filled.every((v) => /^https?:\/\/\S+$/i.test(v))) return { type: 'hyperlink' };
	const longest = Math.max(...filled.map((v) => [...v].length));
	if (longest > 255 || filled.some((v) => v.includes('\n'))) return { type: 'long_text' };
	return makeFieldType('short_text');
}

/** One cell's text as the value to store in `field`, or a problem. Blank cells are empty (null). */
export function cellValue(text: string, field: FieldDef, order: DateOrder): { value: JsonValue } | { problem: string } {
	const t = field.field_type;
	const s = text.trim();
	if (!s) return { value: null };
	switch (t.type) {
		case 'short_text':
			if (t.max_length && [...text].length > t.max_length) return { problem: `longer than ${t.max_length} characters` };
			return { value: text };
		case 'long_text':
		case 'hyperlink':
			return { value: text };
		case 'number':
		case 'large_number':
		case 'auto_number': {
			const n = parseNumber(s);
			if (n === null) return { problem: `“${s}” is not a number` };
			if ((t.type !== 'number' || !t.real) && !Number.isInteger(n)) return { problem: `“${s}” is not a whole number` };
			return { value: n };
		}
		case 'currency': {
			const m = parseMoney(s);
			return m === null ? { problem: `“${s}” is not an amount` } : { value: m };
		}
		case 'date_time': {
			const d = parseDate(s, order);
			return d === null ? { problem: `“${s}” is not a date` } : { value: d };
		}
		case 'yes_no': {
			const b = parseYesNo(s);
			return b === null ? { problem: `“${s}” is not yes or no` } : { value: b };
		}
		case 'lookup':
			if (t.value_type === 'integer') {
				const n = parseNumber(s);
				return n !== null && Number.isInteger(n) ? { value: n } : { problem: `“${s}” is not a record ID` };
			}
			return { value: text };
		case 'unknown':
			return { value: text };
		default:
			return { problem: `${field.name} can't be imported into` };
	}
}

/** Fields a file column can be imported into: not formulas, files or Auto IDs. */
export function importableFields(fields: FieldDef[]): FieldDef[] {
	return fields.filter((f) => !['calculated', 'attachment', 'auto_number'].includes(f.field_type.type));
}

/** For each file column, the field with the same name (ignoring case, spaces and underscores), each used once. */
export function matchColumns(names: string[], fields: FieldDef[]): ColumnTarget[] {
	const free = importableFields(fields);
	const taken = new Set<string>();
	return names.map((n) => {
		const f = free.find((x) => !taken.has(x.name) && squash(x.name) === squash(n));
		if (!f) return null;
		taken.add(f.name);
		return f.name;
	});
}

/**
 * The records to insert from the data rows (after the header, if any) and
 * every cell that doesn't fit its field. `first` is the file row number of
 * `rows[0]`, for messages. Rows with every mapped cell blank are skipped;
 * `lines` gives each record's file row.
 */
export function buildRecords(
	rows: string[][],
	targets: ColumnTarget[],
	fields: FieldDef[],
	order: DateOrder,
	first = 1
): { records: RecordValues[]; lines: number[]; problems: CellProblem[]; skipped: number } {
	const byName = new Map(fields.map((f) => [f.name, f]));
	const records: RecordValues[] = [];
	const lines: number[] = [];
	const problems: CellProblem[] = [];
	let skipped = 0;
	rows.forEach((row, i) => {
		const rec: RecordValues = {};
		let any = false;
		targets.forEach((target, c) => {
			const field = target ? byName.get(target) : undefined;
			if (!field) return;
			const text = row[c] ?? '';
			if (text.trim()) any = true;
			const r = cellValue(text, field, order);
			if ('problem' in r) problems.push({ row: first + i, column: field.name, message: r.problem });
			else if (r.value !== null) rec[field.name] = r.value;
		});
		if (any) {
			records.push(rec);
			lines.push(first + i);
		} else skipped++;
	});
	return { records, lines, problems, skipped };
}

/** Required fields of an existing table that no column fills and that have no default. */
export function missingRequired(fields: FieldDef[], targets: ColumnTarget[]): string[] {
	return importableFields(fields)
		.filter((f) => f.required && !f.default && !f.primary_key && !targets.includes(f.name))
		.map((f) => f.name);
}

/** A backend error naming `row N` of the records sent, rewritten to the row of the file. */
export function fileRowMessage(message: string, lines: number[]): string {
	return message.replace(/\brow (\d+)\b/, (all, n) => {
		const line = lines[Number(n) - 1];
		return line ? `row ${line} of the file` : all;
	});
}
