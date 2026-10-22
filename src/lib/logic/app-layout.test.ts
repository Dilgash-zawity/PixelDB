/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import { clamp, compact, findSpot, gridHeight, overlaps, place, type Placed } from './app-layout';

const item = (id: string, x: number, y: number, w: number, h: number): Placed => ({ id, layout: { x, y, w, h } });
const at = (items: Placed[], id: string) => items.find((i) => i.id === id)?.layout;
const noOverlaps = (items: Placed[]) =>
	items.every((a, i) => items.every((b, j) => i === j || !overlaps(a.layout, b.layout)));

describe('page grid', () => {
	test('layouts stay inside the grid', () => {
		expect(clamp({ x: 10, y: -2, w: 5, h: 0 })).toEqual({ x: 7, y: 0, w: 5, h: 1 });
		expect(clamp({ x: 0, y: 0, w: 40, h: 2 })).toEqual({ x: 0, y: 0, w: 12, h: 2 });
	});

	test('compacting floats widgets up without overlaps', () => {
		const out = compact([item('a', 0, 5, 6, 2), item('b', 0, 9, 12, 2), item('c', 6, 3, 6, 4)]);
		expect(at(out, 'a')).toEqual({ x: 0, y: 0, w: 6, h: 2 });
		expect(at(out, 'c')).toEqual({ x: 6, y: 0, w: 6, h: 4 });
		expect(at(out, 'b')).toEqual({ x: 0, y: 4, w: 12, h: 2 });
	});

	test('dropping onto a widget pushes it down', () => {
		const items = [item('a', 0, 0, 6, 2), item('b', 6, 0, 6, 2), item('c', 0, 2, 12, 2)];
		const out = place(items, 'c', { x: 0, y: 0, w: 12, h: 2 });
		expect(noOverlaps(out)).toBe(true);
		expect(at(out, 'c')).toEqual({ x: 0, y: 0, w: 12, h: 2 });
		expect(at(out, 'a')?.y).toBe(2);
		expect(at(out, 'b')?.y).toBe(2);

		// Growing a widget pushes its neighbours below it.
		const grown = place(items, 'a', { x: 0, y: 0, w: 12, h: 3 });
		expect(noOverlaps(grown)).toBe(true);
		expect(at(grown, 'b')?.y).toBe(3);
		expect(gridHeight(grown)).toBe(7);
	});

	test('new widgets go in the first free spot', () => {
		const items = [item('a', 0, 0, 6, 2), item('b', 0, 2, 12, 2)];
		expect(findSpot(items, 6, 2)).toEqual({ x: 6, y: 0, w: 6, h: 2 });
		expect(findSpot(items, 12, 1)).toEqual({ x: 0, y: 4, w: 12, h: 1 });
		expect(findSpot([], 3, 3)).toEqual({ x: 0, y: 0, w: 3, h: 3 });
	});
});
