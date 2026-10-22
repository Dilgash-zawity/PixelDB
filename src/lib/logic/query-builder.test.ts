/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import type { TableDef } from '../api/types';
import { buildSql, joinableTables, literalFor, newColumn, removeTable, type VisualQuery } from './query-builder';

const customers: TableDef = {
	name: 'Customers',
	fields: [
		{ name: 'ID', field_type: { type: 'auto_number' }, primary_key: true },
		{ name: 'Name', field_type: { type: 'short_text' } },
		{ name: 'Active', field_type: { type: 'yes_no' } }
	]
};
const orders: TableDef = {
	name: 'Orders',
	fields: [
		{ name: 'ID', field_type: { type: 'auto_number' }, primary_key: true },
		{ name: 'Customer', field_type: { type: 'number' } },
		{ name: 'Total', field_type: { type: 'currency', scale: 2 } }
	],
	foreign_keys: [{ columns: ['Customer'], ref_table: 'Customers' }]
};
const notes: TableDef = { name: 'Notes', fields: [{ name: 'Text', field_type: { type: 'long_text' } }] };
const defs = [customers, orders, notes];

describe('query builder', () => {
	test('joins along relationships and disambiguates clashing names', () => {
		const q: VisualQuery = {
			from: 'Customers',
			joins: [{ table: 'Orders', kind: 'left' }],
			columns: [newColumn('Customers', 'ID'), newColumn('Customers', 'Name'), newColumn('Orders', 'ID')]
		};
		const built = buildSql(defs, q);
		expect(built.sql).toBe(
			[
				'SELECT "Customers"."ID" AS "Customers.ID",',
				'       "Customers"."Name",',
				'       "Orders"."ID" AS "Orders.ID"',
				'FROM "Customers"',
				'LEFT JOIN "Orders" ON "Orders"."Customer" = "Customers"."ID"'
			].join('\n')
		);
		expect(built.types.map((t) => t?.type)).toEqual(['auto_number', 'short_text', 'auto_number']);
		expect(built.problems).toEqual([]);
	});

	test('criteria become typed literals; bad values are reported, not sent', () => {
		const q: VisualQuery = {
			from: 'Orders',
			joins: [{ table: 'Customers', kind: 'inner' }],
			columns: [
				{ ...newColumn('Customers', 'Name'), op: 'contains', value: "O'50%" },
				{ ...newColumn('Orders', 'Total'), op: 'ge', value: '12.5', sort: 'desc' },
				{ ...newColumn('Customers', 'Active'), show: false, value: 'yes' },
				{ ...newColumn('Orders', 'Customer'), show: false, op: 'in', value: '1, 2,x' }
			]
		};
		const built = buildSql(defs, q);
		expect(built.sql).toContain(`WHERE "Customers"."Name" LIKE '%O''50\\%%' ESCAPE '\\'`);
		expect(built.sql).toContain(`AND "Orders"."Total" >= 1250`);
		expect(built.sql).toContain(`AND "Customers"."Active" = 1`);
		expect(built.sql).toContain(`ORDER BY "Orders"."Total" DESC`);
		expect(built.sql).not.toContain('IN (');
		expect(built.problems).toEqual(['“1, 2,x” doesn\'t fit Orders.Customer']);
	});

	test('totals group the other shown columns', () => {
		const q: VisualQuery = {
			from: 'Customers',
			joins: [{ table: 'Orders', kind: 'inner' }],
			columns: [
				{ ...newColumn('Customers', 'Name'), aggregate: 'group' },
				{ ...newColumn('Orders', 'Total'), aggregate: 'sum', sort: 'desc' },
				{ ...newColumn('Orders', 'ID'), aggregate: 'count' }
			]
		};
		const built = buildSql(defs, q);
		expect(built.sql).toContain('sum("Orders"."Total") AS "Sum of Total"');
		expect(built.sql).toContain('GROUP BY "Customers"."Name"');
		expect(built.sql).toContain('ORDER BY sum("Orders"."Total") DESC');
		expect(built.types.map((t) => t?.type)).toEqual(['short_text', 'currency', undefined]);
	});

	test('only linked tables can be added, and removing one drops what hangs off it', () => {
		expect(joinableTables(defs, { from: null, joins: [], columns: [] })).toEqual(['Customers', 'Orders', 'Notes']);
		const q: VisualQuery = {
			from: 'Customers',
			joins: [{ table: 'Orders', kind: 'inner' }],
			columns: [newColumn('Customers', 'Name'), newColumn('Orders', 'Total')]
		};
		expect(joinableTables(defs, { ...q, joins: [] })).toEqual(['Orders']);
		expect(removeTable(defs, q, 'Orders')).toEqual({ from: 'Customers', joins: [], columns: [q.columns[0]] });
		expect(removeTable(defs, q, 'Customers').from).toBeNull();
	});

	test('literals', () => {
		expect(literalFor('-0.05', { type: 'currency', scale: 2 })).toBe('-5');
		expect(literalFor('1.234', { type: 'currency', scale: 2 })).toBeNull();
		expect(literalFor('1e3', { type: 'number', real: true })).toBe('1e3');
		expect(literalFor('12abc', { type: 'number' })).toBeNull();
		expect(literalFor(' 2024-01-01 ', { type: 'date_time' })).toBe("'2024-01-01'");
	});
});
