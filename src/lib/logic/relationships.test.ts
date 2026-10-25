/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import type { TableDef } from '../api/types';
import {
	CARD_WIDTH,
	autoLayout,
	describeRules,
	cardHeight,
	edgeRoute,
	fieldDesign,
	withoutField,
	junctionDef,
	junctionName,
	keyColumns,
	linkDesign,
	placeMissing,
	proposeLink,
	typeMismatch,
	relationshipsOf,
	suggestFieldName,
	unlinkDesign,
	updateLinkDesign
} from './relationships';

const categories: TableDef = {
	name: 'Categories',
	fields: [
		{ name: 'ID', field_type: { type: 'auto_number' }, primary_key: true },
		{ name: 'Name', field_type: { type: 'short_text', max_length: 255 }, unique: true }
	]
};
const products: TableDef = {
	name: 'Products',
	fields: [
		{ name: 'ID', field_type: { type: 'auto_number' }, primary_key: true },
		{ name: 'Title', field_type: { type: 'short_text' } },
		{ name: 'CategoryID', field_type: { type: 'lookup', value_type: 'integer' } }
	],
	foreign_keys: [{ columns: ['CategoryID'], ref_table: 'Categories', ref_columns: ['ID'], on_delete: 'cascade' }]
};
const suppliers: TableDef = {
	name: 'suppliers',
	fields: [
		{ name: 'id', field_type: { type: 'auto_number' }, primary_key: true },
		{ name: 'name', field_type: { type: 'short_text' } }
	]
};

describe('relationships', () => {
	test('are read from foreign keys with their cardinality', () => {
		const [rel] = relationshipsOf([categories, products]);
		expect(rel).toMatchObject({
			parent: 'Categories',
			parentColumns: ['ID'],
			child: 'Products',
			childColumns: ['CategoryID'],
			cardinality: 'one_to_many',
			onDelete: 'cascade',
			onUpdate: 'no_action'
		});
		const unique = { ...products, fields: products.fields.map((f) => ({ ...f, unique: f.name === 'CategoryID' })) };
		expect(relationshipsOf([categories, unique])[0].cardinality).toBe('one_to_one');
		// Links to tables outside the set are skipped.
		expect(relationshipsOf([products])).toEqual([]);
	});

	test('key columns are the single primary key and unique fields', () => {
		expect(keyColumns(categories)).toEqual(['ID', 'Name']);
	});

	test('new field names follow the table style', () => {
		expect(suggestFieldName('Categories', 'ID', products)).toBe('CategoryID2');
		expect(suggestFieldName('suppliers', 'id', products)).toBe('supplier_id');
		expect(suggestFieldName('Stores', 'Code', products)).toBe('StoreCode');
	});

	test('linking adds a dropdown field and the foreign key', () => {
		const { definition, renames } = linkDesign(products, {
			parent: suppliers,
			parentColumn: 'id',
			childColumn: null,
			cardinality: 'one_to_many',
			onUpdate: 'cascade',
			onDelete: 'restrict',
			asDropdown: true
		});
		expect(renames).toEqual([]);
		expect(definition.fields.at(-1)).toEqual({
			name: 'supplier_id',
			field_type: { type: 'lookup', value_type: 'integer', display_column: 'name' },
			unique: false
		});
		expect(definition.foreign_keys?.at(-1)).toEqual({
			columns: ['supplier_id'],
			ref_table: 'suppliers',
			ref_columns: ['id'],
			on_update: 'cascade',
			on_delete: 'restrict'
		});
		// The input definition is not modified.
		expect(products.fields).toHaveLength(3);
	});

	test('linking an existing field replaces its old relationship', () => {
		const { definition } = linkDesign(products, {
			parent: categories,
			parentColumn: 'ID',
			childColumn: 'CategoryID',
			cardinality: 'one_to_one',
			onUpdate: 'no_action',
			onDelete: 'set_null',
			asDropdown: false
		});
		expect(definition.foreign_keys).toHaveLength(1);
		expect(definition.foreign_keys?.[0].on_delete).toBe('set_null');
		expect(definition.fields[2].unique).toBe(true);
	});

	test('unlinking turns a dropdown back into a plain field', () => {
		const [rel] = relationshipsOf([categories, products]);
		const { definition } = unlinkDesign(products, rel);
		expect(definition.foreign_keys).toEqual([]);
		expect(definition.fields[2].field_type).toEqual({ type: 'number', real: false });
	});

	test('updating changes rules and uniqueness', () => {
		const [rel] = relationshipsOf([categories, products]);
		const { definition } = updateLinkDesign(products, rel, {
			cardinality: 'one_to_one',
			onUpdate: 'cascade',
			onDelete: 'restrict'
		});
		expect(definition.foreign_keys?.[0]).toMatchObject({ on_update: 'cascade', on_delete: 'restrict' });
		expect(definition.fields[2].unique).toBe(true);
	});

	test('many-to-many builds a junction table', () => {
		// Taken names are compared without case.
		expect(junctionName('Products', 'suppliers', ['ProductSuppliers'])).toBe('Productsuppliers2');
		const name = junctionName('Products', 'suppliers', []);
		const def = junctionDef(products, suppliers, name);
		expect(def.fields.map((f) => f.name)).toEqual(['ID', 'ProductID', 'supplier_id']);
		expect(def.unique_constraints).toEqual([['ProductID', 'supplier_id']]);
		expect(def.foreign_keys?.map((fk) => [fk.ref_table, fk.on_delete])).toEqual([
			['Products', 'cascade'],
			['suppliers', 'cascade']
		]);
		expect(() => junctionDef({ name: 'X', fields: [{ name: 'a', field_type: { type: 'short_text' } }] }, suppliers, 'J')).toThrow();
	});
});

describe('drag to link', () => {
	test('from a key, the target becomes the many side', () => {
		const p = proposeLink(suppliers, 'id', products, null);
		expect(p).toMatchObject({ parent: 'suppliers', parentColumn: 'id', child: 'Products', childColumn: null, newField: 'supplier_id', kind: 'one_to_many' });
	});

	test('from a plain field, it points at the target key', () => {
		const p = proposeLink(products, 'Title', categories, 'Name');
		expect(p).toMatchObject({ parent: 'Categories', parentColumn: 'Name', child: 'Products', childColumn: 'Title' });
		const q = proposeLink(products, 'Title', categories, null);
		expect(q).toMatchObject({ parentColumn: 'ID' });
	});

	test('key to key proposes one-to-one; nonsense is refused', () => {
		expect(proposeLink(categories, 'ID', suppliers, 'id')).toMatchObject({ kind: 'one_to_one' });
		expect(proposeLink(categories, 'ID', categories, 'ID')).toBeTypeOf('string');
		const keyless: TableDef = { name: 'Notes', fields: [{ name: 'text', field_type: { type: 'long_text' } }] };
		expect(proposeLink(products, 'Title', keyless, null)).toBeTypeOf('string');
	});

	test('type mismatches are explained', () => {
		expect(typeMismatch(categories, 'ID', products, 'Title')).toContain('whole numbers');
		expect(typeMismatch(categories, 'ID', products, 'CategoryID')).toBeNull();
		expect(typeMismatch(categories, 'ID', products, null)).toBeNull();
	});
});

describe('editing fields on the diagram', () => {
	test('adds a field with its rules', () => {
		const design = fieldDesign(suppliers, null, {
			name: 'rating',
			field_type: { type: 'number', real: false },
			required: true,
			check: 'rating BETWEEN 1 AND 5'
		});
		if (Array.isArray(design)) throw new Error(design.join());
		expect(design.definition.fields.at(-1)).toEqual({
			name: 'rating',
			field_type: { type: 'number', real: false },
			required: true,
			check: 'rating BETWEEN 1 AND 5'
		});
		expect(design.renames).toEqual([]);
	});

	test('renaming keeps the relationship and reports the rename', () => {
		const design = fieldDesign(products, 'CategoryID', { ...products.fields[2], name: 'Category' });
		if (Array.isArray(design)) throw new Error(design.join());
		expect(design.renames).toEqual([{ from: 'CategoryID', to: 'Category' }]);
		expect(design.definition.foreign_keys?.[0].columns).toEqual(['Category']);
	});

	test('problems are returned instead of a design', () => {
		expect(fieldDesign(products, null, { name: 'Title', field_type: { type: 'short_text' } })).toEqual([
			'Duplicate field name "Title"'
		]);
		expect(fieldDesign(products, 'Nope', products.fields[1])).toEqual(['“Products” has no field “Nope”']);
	});

	test('removing a field drops its relationship', () => {
		const { definition } = withoutField(products, 'CategoryID');
		expect(definition.fields.map((f) => f.name)).toEqual(['ID', 'Title']);
		expect(definition.foreign_keys).toEqual([]);
	});
});

describe('diagram geometry', () => {
	test('auto layout puts children right of their parents without overlap', () => {
		const rels = relationshipsOf([categories, products, suppliers]);
		const layout = autoLayout([categories, products, suppliers], rels);
		expect(layout.Products.x).toBeGreaterThan(layout.Categories.x + CARD_WIDTH);
		expect(layout.suppliers.x).toBe(layout.Categories.x);
		expect(layout.suppliers.y).toBeGreaterThanOrEqual(layout.Categories.y + cardHeight(categories));
	});

	test('auto layout wraps a tall stack of unlinked tables into more columns', () => {
		const many = Array.from({ length: 20 }, (_, i) => ({ ...suppliers, name: `t${i}` }));
		const layout = autoLayout(many, []);
		const xs = new Set(Object.values(layout).map((p) => p.x));
		expect(xs.size).toBeGreaterThan(1);
		expect(Math.max(...Object.values(layout).map((p) => p.y))).toBeLessThan(1600);
	});

	test('link rules read as sentences', () => {
		const rel = { parent: 'Customers', child: 'Orders', onDelete: 'cascade' as const, onUpdate: 'cascade' as const };
		expect(describeRules(rel)).toEqual([
			'Deleting a Customers record deletes its Orders records too.',
			'Changing a Customers key updates its Orders records too.'
		]);
		expect(describeRules({ ...rel, onDelete: 'restrict', onUpdate: 'no_action' })).toEqual([
			"A Customers record can't be deleted while Orders records link to it."
		]);
	});

	test('auto layout survives cycles', () => {
		const a: TableDef = { ...suppliers, name: 'A', foreign_keys: [{ columns: ['id'], ref_table: 'B' }] };
		const b: TableDef = { ...suppliers, name: 'B', foreign_keys: [{ columns: ['id'], ref_table: 'A' }] };
		const layout = autoLayout([a, b], relationshipsOf([a, b]));
		expect(Object.keys(layout).sort()).toEqual(['A', 'B']);
	});

	test('missing tables are placed below the rest', () => {
		const layout = placeMissing([categories, suppliers], { Categories: { x: 10, y: 20 } });
		expect(layout.Categories).toEqual({ x: 10, y: 20 });
		expect(layout.suppliers.y).toBeGreaterThan(20 + cardHeight(categories));
	});

	test('connectors join facing edges or loop round the right', () => {
		const side = edgeRoute({ x: 0, y: 0 }, 50, { x: 400, y: 100 }, 80);
		expect(side.start).toEqual({ x: CARD_WIDTH, y: 50 });
		expect(side.end).toEqual({ x: 400, y: 180 });
		expect([side.startDir, side.endDir]).toEqual([1, -1]);
		const back = edgeRoute({ x: 400, y: 0 }, 50, { x: 0, y: 0 }, 50);
		expect([back.start.x, back.end.x]).toEqual([400, CARD_WIDTH]);
		const stacked = edgeRoute({ x: 0, y: 0 }, 50, { x: 10, y: 300 }, 50);
		expect([stacked.startDir, stacked.endDir]).toEqual([1, 1]);
		expect(stacked.path.startsWith(`M ${CARD_WIDTH} 50`)).toBe(true);
	});
});
