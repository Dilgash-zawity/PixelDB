/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import type { TableDef } from '../api/types';
import { changes, fieldPages, isDirty, missingRequired, newBuffer, setValue, submission, undoAll, undoField } from './record-buffer';

const def: TableDef = {
	name: 'T',
	fields: [
		{ name: 'id', field_type: { type: 'auto_number' }, primary_key: true },
		{ name: 'name', field_type: { type: 'short_text' }, required: true },
		{ name: 'price', field_type: { type: 'currency', scale: 2 } },
		{ name: 'ok', field_type: { type: 'yes_no' }, default: { kind: 'literal', value: true } },
		{ name: 'calc', field_type: { type: 'calculated', expression: 'price * 2' } }
	]
};

describe('record buffer', () => {
	test('tracks and undoes unsaved changes', () => {
		const buf = newBuffer(def, { key: { row_id: 1 }, values: [1, 'A', 1050, 0, 2100] });
		expect(buf.current.price).toBe('10.50');
		expect(isDirty(buf)).toBe(false);
		setValue(buf, 'name', 'B');
		setValue(buf, 'price', '11.00');
		expect(changes(buf)).toEqual({ name: 'B', price: '11.00' });
		undoField(buf, 'name');
		expect(changes(buf)).toEqual({ price: '11.00' });
		undoAll(buf);
		expect(isDirty(buf)).toBe(false);
	});

	test('new records: defaults, required fields and submission', () => {
		const buf = newBuffer(def, null);
		expect(buf.key).toBeNull();
		expect(buf.current.ok).toBe(true);
		expect(missingRequired(buf, def)).toEqual(['name']);
		setValue(buf, 'name', 'X');
		setValue(buf, 'calc', 5);
		setValue(buf, 'price', '');
		expect(missingRequired(buf, def)).toEqual([]);
		expect(submission(buf, def)).toEqual({ name: 'X', ok: true });
	});
});

describe('fieldPages', () => {
	test('splits fields into steps and always has one', () => {
		expect(fieldPages([1, 2, 3, 4, 5, 6, 7], 3)).toEqual([[1, 2, 3], [4, 5, 6], [7]]);
		expect(fieldPages([1, 2], 5)).toEqual([[1, 2]]);
		expect(fieldPages([], 5)).toEqual([[]]);
	});
});
