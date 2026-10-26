/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import type { FieldDef } from '../api/types';
import {
	buildRecords,
	cellValue,
	columnNames,
	fileRowMessage,
	guessDateOrder,
	guessType,
	looksLikeHeader,
	matchColumns,
	missingRequired,
	parseDate,
	parseMoney,
	parseNumber
} from './import';

describe('column names', () => {
	test('cleans headers, fills blanks and keeps names unique', () => {
		expect(columnNames(['Name', ' Name ', '', 'id', 'sqlite_x', 'a\tb'], 7)).toEqual([
			'Name',
			'Name 2',
			'Column 3',
			'id 2',
			'x',
			'a b',
			'Column 7'
		]);
		expect(columnNames(null, 2)).toEqual(['Column 1', 'Column 2']);
	});

	test('spots a header row', () => {
		expect(looksLikeHeader([['Name', 'Age'], ['Ada', '36']])).toBe(true);
		expect(looksLikeHeader([['1', '2'], ['3', '4']])).toBe(false);
		expect(looksLikeHeader([['Ada', 'Ada'], ['Bo', 'Cy']])).toBe(false);
	});
});

describe('parsing', () => {
	test('numbers in common notations', () => {
		expect(parseNumber('1,234.5')).toBe(1234.5);
		expect(parseNumber('1.234,5')).toBe(1234.5);
		expect(parseNumber('12,5')).toBe(12.5);
		expect(parseNumber('1 000')).toBe(1000);
		expect(parseNumber('(42)')).toBe(-42);
		expect(parseNumber('1e3')).toBe(1000);
		expect(parseNumber('12a')).toBeNull();
		expect(parseNumber('')).toBeNull();
	});

	test('money stays exact', () => {
		expect(parseMoney('$1,234.50')).toBe('1234.50');
		expect(parseMoney('-€0.10')).toBe('-0.10');
		expect(parseMoney('(5.25 USD)')).toBe('-5.25');
		expect(parseMoney('12,99 €')).toBe('12.99');
		expect(parseMoney('abc')).toBeNull();
	});

	test('dates in ISO and local orders', () => {
		expect(parseDate('2024-02-29', 'dmy')).toBe('2024-02-29');
		expect(parseDate('2023-02-29', 'dmy')).toBeNull();
		expect(parseDate('2024-03-05 14:30', 'dmy')).toBe('2024-03-05T14:30:00');
		expect(parseDate('2024-03-05T10:00:00+0300', 'dmy')).toBe('2024-03-05T10:00:00+03:00');
		expect(parseDate('05/03/2024', 'dmy')).toBe('2024-03-05');
		expect(parseDate('05/03/2024', 'mdy')).toBe('2024-05-03');
		expect(parseDate('5.3.24 2:05 pm', 'dmy')).toBe('2024-03-05T14:05:00');
		expect(parseDate('31/31/2024', 'dmy')).toBeNull();
		expect(guessDateOrder(['01/02/2024', '13/02/2024'])).toBe('dmy');
		expect(guessDateOrder(['02/13/2024'], 'dmy')).toBe('mdy');
		expect(guessDateOrder(['01/02/2024'], 'mdy')).toBe('mdy');
	});
});

describe('type guesses', () => {
	const g = (v: string[]) => guessType(v, 'dmy');
	test('picks the narrowest type that fits every value', () => {
		expect(g(['1', '2', ''])).toEqual({ type: 'number', real: false });
		expect(g(['1', '2.5'])).toEqual({ type: 'number', real: true });
		expect(g(['3000000000'])).toEqual({ type: 'large_number' });
		expect(g(['00123', '456'])).toEqual({ type: 'short_text', max_length: 255 });
		expect(g(['$1.50', '2'])).toEqual({ type: 'currency', scale: 2 });
		expect(g(['Yes', 'no'])).toEqual({ type: 'yes_no' });
		expect(g(['1', '0'])).toEqual({ type: 'number', real: false });
		expect(g(['2024-01-01', '02/03/2024'])).toEqual({ type: 'date_time' });
		expect(g(['https://a.com', 'http://b.org/x'])).toEqual({ type: 'hyperlink' });
		expect(g(['x'.repeat(300)])).toEqual({ type: 'long_text' });
		expect(g([])).toEqual({ type: 'short_text', max_length: 255 });
	});
});

describe('building records', () => {
	const fields: FieldDef[] = [
		{ name: 'ID', field_type: { type: 'auto_number' }, primary_key: true },
		{ name: 'Name', field_type: { type: 'short_text', max_length: 5 }, required: true },
		{ name: 'Price', field_type: { type: 'currency', scale: 2 } },
		{ name: 'Paid', field_type: { type: 'yes_no' } },
		{ name: 'Due', field_type: { type: 'date_time' } },
		{ name: 'Total', field_type: { type: 'calculated', expression: '1', stored: false, result: 'any' } }
	];

	test('matches columns by name once each, never into formulas or IDs', () => {
		expect(matchColumns(['name', 'PRICE', 'price', 'paid_', 'id', 'total'], fields)).toEqual(['Name', 'Price', null, 'Paid', null, null]);
	});

	test('converts cells, reports problems by file row and skips blank rows', () => {
		const rows = [
			['Ada', '$1.50', 'yes', '01/02/2024'],
			['', '', '', ''],
			['Bartholomew', 'abc', 'maybe', 'soon']
		];
		const r = buildRecords(rows, ['Name', 'Price', 'Paid', 'Due'], fields, 'dmy', 2);
		expect(r.records[0]).toEqual({ Name: 'Ada', Price: '1.50', Paid: true, Due: '2024-02-01' });
		expect(r.lines).toEqual([2, 4]);
		expect(r.skipped).toBe(1);
		expect(r.problems.map((p) => [p.row, p.column])).toEqual([
			[4, 'Name'],
			[4, 'Price'],
			[4, 'Paid'],
			[4, 'Due']
		]);
	});

	test('whole numbers only where decimals are not allowed', () => {
		expect(cellValue('2.5', { name: 'n', field_type: { type: 'number', real: false } }, 'dmy')).toEqual({ problem: '“2.5” is not a whole number' });
		expect(cellValue('2.5', { name: 'n', field_type: { type: 'number', real: true } }, 'dmy')).toEqual({ value: 2.5 });
	});

	test('required fields left out, and backend rows mapped to file rows', () => {
		expect(missingRequired(fields, ['Price'])).toEqual(['Name']);
		expect(missingRequired(fields, ['Name'])).toEqual([]);
		expect(fileRowMessage('row 2: Name: required', [2, 4])).toBe('row 4 of the file: Name: required');
	});
});
