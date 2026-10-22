/// <reference types="bun" />
import { expect, test } from 'bun:test';
import { History } from './history';

test('undo and redo walk through snapshots, merging grouped edits', () => {
	const h = new History<{ n: number }>(3);
	let doc = { n: 0 };
	const change = (n: number, group?: string) => {
		h.record(doc, group);
		doc = { n };
	};
	change(1);
	change(2, 'typing');
	change(3, 'typing');
	expect((doc = h.undo(doc)!)).toEqual({ n: 1 });
	expect((doc = h.redo(doc)!)).toEqual({ n: 3 });
	expect(h.canRedo).toBe(false);

	h.seal();
	change(4, 'typing');
	expect((doc = h.undo(doc)!)).toEqual({ n: 3 });
	change(5);
	expect(h.canRedo).toBe(false);

	// The oldest steps fall off past the limit.
	change(6);
	change(7);
	let steps = 0;
	while (h.canUndo) {
		doc = h.undo(doc)!;
		steps++;
	}
	expect(steps).toBe(3);
});
