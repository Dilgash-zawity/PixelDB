/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import {
	NODE_HEIGHT,
	NODE_WIDTH,
	autoMapLayout,
	freeSpot,
	nodeKey,
	placeMissingNodes,
	usersByDatabase,
	workspaceLinks
} from './workspace-map';

describe('workspaceLinks', () => {
	const apps = [
		{ name: 'Shop', databases: ['Sales', 'Stock', 'Sales'], valid: true },
		{ name: 'Old', databases: ['Archive'], valid: true },
		{ name: 'Broken', databases: ['Sales'], valid: false }
	];

	test('lists each app → database once and flags missing databases', () => {
		expect(workspaceLinks(apps, ['Sales', 'Stock'])).toEqual([
			{ app: 'Shop', database: 'Sales', missing: false },
			{ app: 'Shop', database: 'Stock', missing: false },
			{ app: 'Old', database: 'Archive', missing: true }
		]);
	});

	test('counts users per database', () => {
		const users = usersByDatabase(workspaceLinks([...apps, { name: 'Report', databases: ['Sales'], valid: true }], ['Sales']));
		expect(users.get('Sales')).toEqual(['Shop', 'Report']);
		expect(users.get('Stock')).toEqual(['Shop']);
	});
});

describe('layout', () => {
	const links = workspaceLinks(
		[
			{ name: 'A', databases: ['Z'], valid: true },
			{ name: 'B', databases: ['Y'], valid: true }
		],
		['Y', 'Z', 'Spare']
	);

	test('puts apps left, databases right, level with their apps', () => {
		const l = autoMapLayout(['A', 'B'], ['Y', 'Z', 'Spare'], links);
		expect(l[nodeKey('app', 'A')].x).toBe(0);
		expect(l[nodeKey('db', 'Z')].x).toBeGreaterThan(NODE_WIDTH);
		expect(l[nodeKey('db', 'Z')].y).toBeLessThan(l[nodeKey('db', 'Y')].y);
		expect(l[nodeKey('db', 'Spare')].y).toBeGreaterThan(l[nodeKey('db', 'Y')].y);
	});

	test('freeSpot steps down past occupied cards', () => {
		const l = { a: { x: 0, y: 0 } };
		expect(freeSpot(l, { x: 10, y: 10 }).y).toBeGreaterThanOrEqual(NODE_HEIGHT);
		expect(freeSpot(l, { x: 1000, y: 0 })).toEqual({ x: 1000, y: 0 });
	});

	test('keeps saved positions, forgets removed cards and places new ones', () => {
		const saved = { [nodeKey('app', 'A')]: { x: 500, y: 300 }, [nodeKey('db', 'Gone')]: { x: 0, y: 0 } };
		const l = placeMissingNodes(saved, ['A', 'B'], ['Y', 'Z', 'Spare'], links);
		expect(l[nodeKey('app', 'A')]).toEqual({ x: 500, y: 300 });
		expect(l[nodeKey('db', 'Gone')]).toBeUndefined();
		// Z sits beside A, which uses it.
		expect(l[nodeKey('db', 'Z')]).toEqual({ x: 500 + NODE_WIDTH + 200, y: 300 });
		const all = Object.values(l);
		for (let i = 0; i < all.length; i++)
			for (let j = i + 1; j < all.length; j++)
				expect(Math.abs(all[i].x - all[j].x) >= NODE_WIDTH || Math.abs(all[i].y - all[j].y) >= NODE_HEIGHT).toBe(true);
	});
});
