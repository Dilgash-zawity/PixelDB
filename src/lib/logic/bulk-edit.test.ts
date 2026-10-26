/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import type { FieldDef, RecordRow } from '../api/types';
import { cellText, isBlock, parseCell, parseClipboard, planFill, planPaste, planReplace, replaceText, toClipboard } from './bulk-edit';

const fields: FieldDef[] = [
	{ name: 'ID', field_type: { type: 'auto_number' }, primary_key: true },
	{ name: 'Name', field_type: { type: 'short_text', max_length: 255 } },
	{ name: 'Price', field_type: { type: 'currency', scale: 2 } },
	{ name: 'Customer', field_type: { type: 'lookup', value_type: 'integer', display_column: 'Name' } }
];
const columns = fields.map((f) => f.name);
const rows: RecordRow[] = [
	{ key: { row_id: 1 }, values: [1, 'Pen', 150, 7] },
	{ key: { row_id: 2 }, values: [2, 'Ink', 1200, null] }
];
const lookups = { Customer: [{ value: 7, label: 'Ada' }, { value: 8, label: 'Bo' }, { value: 9, label: 'Bo' }] };
const editable = (c: string) => c !== 'ID';

describe('clipboard', () => {
	test('reads spreadsheet cells, quotes included', () => {
		expect(parseClipboard('a\tb\r\nc\t"d\te ""x"""\n')).toEqual([
			['a', 'b'],
			['c', 'd\te "x"']
		]);
		expect(parseClipboard('one')).toEqual([['one']]);
		expect(isBlock('one\n')).toBe(false);
		expect(isBlock('a\tb')).toBe(true);
	});

	test('writes cells that read back the same', () => {
		const cells = [['a', 'b\tc'], ['"q"', 'line\nbreak']];
		expect(parseClipboard(toClipboard(cells))).toEqual(cells);
	});

	test('relations copy and paste by label', () => {
		expect(cellText(7, fields[3], lookups.Customer)).toBe('Ada');
		expect(cellText(150, fields[2])).toBe('1.50');
		expect(parseCell('ada', fields[3], 'dmy', lookups.Customer)).toEqual({ value: 7 });
		expect(parseCell('Bo', fields[3], 'dmy', lookups.Customer)).toEqual({ problem: 'more than one Customer is called “Bo”' });
		expect(parseCell('8', fields[3], 'dmy', lookups.Customer)).toEqual({ value: 8 });
	});
});

describe('paste', () => {
	test('updates rows from the cursor and adds the rest, skipping read-only columns', () => {
		const plan = planPaste({ cells: [['x', 'Pencil', '2.50'], ['y', 'Ink', ''], ['z', 'Cup', '3']], rows, start: 0, columns, startCol: 0, fields, editable, order: 'dmy' });
		expect(plan.skippedColumns).toBe(1);
		expect(plan.ops).toEqual([
			{ op: 'update', key: { row_id: 1 }, values: { Name: 'Pencil', Price: '2.50' } },
			{ op: 'update', key: { row_id: 2 }, values: { Name: 'Ink', Price: null } },
			{ op: 'insert', values: { Name: 'Cup', Price: '3' } }
		]);
		expect([plan.updated, plan.added]).toEqual([2, 1]);
	});

	test('reports values that do not fit', () => {
		const plan = planPaste({ cells: [['abc']], rows, start: 1, columns, startCol: 2, fields, editable, order: 'dmy', lookups });
		expect(plan.problems).toEqual([{ row: 1, column: 'Price', message: '“abc” is not an amount' }]);
	});
});

describe('fill and replace', () => {
	test('fill sets one field on every record', () => {
		expect(planFill([{ row_id: 1 }, { row_id: 2 }], 'Price', '1')).toEqual([
			{ op: 'update', key: { row_id: 1 }, values: { Price: '1' } },
			{ op: 'update', key: { row_id: 2 }, values: { Price: '1' } }
		]);
	});

	test('replaces text, literally, with or without case', () => {
		const o = { find: 'p.n', replace: '$&!', matchCase: false, wholeCell: false };
		expect(replaceText('P.N and p.n', o)).toBe('$&! and $&!');
		expect(replaceText('pen', o)).toBeNull();
		expect(replaceText('Pen', { ...o, find: 'pen', matchCase: true })).toBeNull();
		expect(replaceText('Pen', { ...o, find: 'pen', replace: 'Ink', wholeCell: true })).toBe('Ink');
		expect(replaceText('Pens', { ...o, find: 'pen', wholeCell: true })).toBeNull();
	});

	test('plans updates only where text changes', () => {
		const r = planReplace(rows, columns, ['Name'], { find: 'n', replace: 'N', matchCase: true, wholeCell: false });
		expect(r.ops).toEqual([
			{ op: 'update', key: { row_id: 1 }, values: { Name: 'PeN' } },
			{ op: 'update', key: { row_id: 2 }, values: { Name: 'INk' } }
		]);
		expect(r.cells).toBe(2);
	});
});
