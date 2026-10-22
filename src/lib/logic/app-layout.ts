import { GRID_COLUMNS, type Layout } from './app-def';

/**
 * Placement of widgets on a page's 12-column grid. Widgets never overlap:
 * moving or resizing one pushes the ones it lands on downwards, and then
 * everything floats up as far as it can (like a dashboard grid).
 */

export interface Placed {
	id: string;
	layout: Layout;
}

export function overlaps(a: Layout, b: Layout): boolean {
	return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
}

/** Keeps a layout inside the grid. */
export function clamp(l: Layout): Layout {
	const w = Math.max(1, Math.min(GRID_COLUMNS, Math.round(l.w)));
	return {
		x: Math.max(0, Math.min(GRID_COLUMNS - w, Math.round(l.x))),
		y: Math.max(0, Math.round(l.y)),
		w,
		h: Math.max(1, Math.round(l.h))
	};
}

const byPosition = (a: Placed, b: Placed) => a.layout.y - b.layout.y || a.layout.x - b.layout.x;

/** Moves every widget up as far as it goes without overlapping one above it. */
export function compact(items: Placed[]): Placed[] {
	const placed: Placed[] = [];
	for (const item of [...items].sort(byPosition)) {
		const l = { ...item.layout };
		while (l.y > 0 && !placed.some((p) => overlaps({ ...l, y: l.y - 1 }, p.layout))) l.y--;
		placed.push({ ...item, layout: l });
	}
	return placed;
}

/**
 * Puts widget `id` at `target` (moving or resizing it), pushing whatever it
 * covers below it, then compacts. `id` keeps its place while others float
 * around it.
 */
export function place(items: Placed[], id: string, target: Layout): Placed[] {
	return settle({ id, layout: clamp(target) }, items.filter((i) => i.id !== id));
}

/** Places `rest` in reading order around `anchor`, which stays exactly where it is. */
function settle(anchor: Placed, rest: Placed[]): Placed[] {
	const placed: Placed[] = [anchor];
	for (const item of [...rest].sort(byPosition)) {
		const l = { ...item.layout };
		while (placed.some((p) => overlaps(l, p.layout))) l.y++;
		while (l.y > 0 && !placed.some((p) => overlaps({ ...l, y: l.y - 1 }, p.layout))) l.y--;
		placed.push({ ...item, layout: l });
	}
	return placed;
}

/** The first spot (top to bottom, left to right) where a `w`×`h` widget fits. */
export function findSpot(items: Placed[], w: number, h: number): Layout {
	const width = Math.min(GRID_COLUMNS, Math.max(1, w));
	const bottom = items.reduce((m, i) => Math.max(m, i.layout.y + i.layout.h), 0);
	for (let y = 0; y <= bottom; y++) {
		for (let x = 0; x + width <= GRID_COLUMNS; x++) {
			const l = { x, y, w: width, h };
			if (!items.some((i) => overlaps(l, i.layout))) return l;
		}
	}
	return { x: 0, y: bottom, w: width, h };
}

/** Rows the grid needs to show every widget. */
export function gridHeight(items: Placed[]): number {
	return items.reduce((m, i) => Math.max(m, i.layout.y + i.layout.h), 0);
}

/**
 * Converts a pointer movement in pixels into a whole number of grid cells,
 * given the size of one column and one row (including the gap).
 */
export function cellsMoved(dx: number, dy: number, colWidth: number, rowHeight: number): { dx: number; dy: number } {
	return { dx: Math.round(dx / colWidth), dy: Math.round(dy / rowHeight) };
}
