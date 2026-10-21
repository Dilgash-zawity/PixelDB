/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import { emptySelection, pageOffsetFor, select } from './selection';

describe('row selection', () => {
	test('click, toggle and range', () => {
		let s = select(emptySelection(), 2);
		expect(s.rows).toEqual([2]);
		s = select(s, 5, { toggle: true });
		expect(s.rows).toEqual([2, 5]);
		s = select(s, 5, { toggle: true });
		expect(s.rows).toEqual([2]);
		s = select(select(emptySelection(), 6), 3, { shift: true });
		expect(s.rows).toEqual([3, 4, 5, 6]);
	});

	test('page offsets', () => {
		expect(pageOffsetFor(0, 100)).toBe(0);
		expect(pageOffsetFor(99, 100)).toBe(0);
		expect(pageOffsetFor(100, 100)).toBe(100);
		expect(pageOffsetFor(-3, 100)).toBe(0);
	});
});
