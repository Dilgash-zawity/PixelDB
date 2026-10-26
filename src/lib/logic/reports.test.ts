/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import type { FieldType } from '../api/types';
import { buildReport, newReport, readReports, renameFieldInReports, renameTableInReports, reportNameProblem, reportSort, total, totalsFor, type ReportDef } from './reports';

const money: FieldType = { type: 'currency', scale: 2 };
const types: (FieldType | null)[] = [{ type: 'auto_number' }, { type: 'short_text' }, money, { type: 'lookup', value_type: 'integer', display_column: 'Name' }];
const data = {
	columns: ['ID', 'Item', 'Price', 'Customer'],
	types,
	rows: [
		[1, 'Pen', 150, 7],
		[2, 'Ink', 1205, 7],
		[3, 'Cup', 300, null],
		[4, 'Mug', 1, 8]
	]
};
const lookups = { Customer: [{ value: 7, label: 'Ada' }, { value: 8, label: 'Bo' }] };
const base: ReportDef = {
	...newReport('Sales', 'Orders', data.columns.map((name, i) => ({ name, type: types[i] }))),
	columns: [
		{ field: 'Item', label: 'What', total: 'count' },
		{ field: 'Price', label: '', total: 'sum' }
	]
};

describe('reports', () => {
	test('new reports list the fields and keep the name rules', () => {
		const r = newReport('R', 'T', [{ name: 'A', type: { type: 'attachment' } }, { name: 'B', type: null }]);
		expect(r.columns).toEqual([{ field: 'B', label: '', total: 'none' }]);
		expect(reportNameProblem({ Sales: base }, 'sales')).toBe('A report with this name already exists');
		expect(reportNameProblem({ Sales: base }, 'Sales', 'Sales')).toBeNull();
	});

	test('read stored JSON, dropping what is malformed', () => {
		expect(readReports({ x: { name: 'X', source: 'T', columns: [{ field: 'A', total: 'bogus' }, 3], sort: 'no' }, y: { name: '' } })).toEqual({
			X: { name: 'X', source: 'T', title: 'X', subtitle: '', columns: [{ field: 'A', label: '', total: 'none' }], groupBy: null, sort: null, filters: [], landscape: false, showCount: true }
		});
	});

	test('totals add money exactly and fit the field type', () => {
		expect(total('sum', [150, 1205, null], money)).toBe('13.55');
		expect(total('avg', [100, 201], money)).toBe('1.51');
		expect(total('count', [1, null, '', 'x'], null)).toBe('2');
		expect(total('max', ['2024-01-02', '2023-05-01'], { type: 'date_time' })).toBe('2024-01-02');
		expect(total('avg', [1, 2], { type: 'number', real: false })).toBe('1.5');
		expect(totalsFor({ type: 'short_text' })).toEqual(['none', 'count']);
	});

	test('groups rows in order with subtotals and a grand total', () => {
		const r = buildReport({ ...base, groupBy: 'Customer' }, data, lookups);
		expect(r.headers).toEqual([
			{ label: 'What', numeric: false },
			{ label: 'Price', numeric: true }
		]);
		expect(r.groups.map((g) => [g.label, g.count, g.totals])).toEqual([
			['Ada', 2, ['2', '13.55']],
			['(none)', 1, ['1', '3.00']],
			['Bo', 1, ['1', '0.01']]
		]);
		expect(r.groups[0].rows[0]).toEqual([
			{ text: 'Pen', numeric: false },
			{ text: '1.50', numeric: true }
		]);
		expect(r.totals).toEqual(['4', '16.56']);
	});

	test('ungrouped, empty and with fields that are gone', () => {
		const r = buildReport({ ...base, columns: [...base.columns, { field: 'Gone', label: '', total: 'none' }] }, { ...data, rows: [] });
		expect(r.groups).toEqual([{ label: null, rows: [], totals: ['0', ''], count: 0 }]);
		expect(r.missing).toEqual(['Gone']);
	});

	test('sorts by the group first and follows renames', () => {
		expect(reportSort({ ...base, groupBy: 'Customer', sort: { column: 'Price', descending: true } })).toEqual([
			{ column: 'Customer', descending: false },
			{ column: 'Price', descending: true }
		]);
		const g = { Sales: { ...base, groupBy: 'Customer', sort: { column: 'Price', descending: false } } };
		const renamed = renameFieldInReports(g, 'Orders', 'Price', 'Cost').Sales;
		expect(renamed.columns[1].field).toBe('Cost');
		expect(renamed.sort?.column).toBe('Cost');
		expect(renameFieldInReports(g, 'Other', 'Price', 'Cost')).toEqual(g);
		expect(renameTableInReports(g, 'Orders', 'Sales').Sales.source).toBe('Sales');
	});
});
