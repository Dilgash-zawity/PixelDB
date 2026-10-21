/** Record-selector style row selection (click, Ctrl/Cmd+click, Shift+click). */
export interface Selection {
	rows: number[];
	anchor: number | null;
}

export function emptySelection(): Selection {
	return { rows: [], anchor: null };
}

export function select(sel: Selection, index: number, mods: { shift?: boolean; toggle?: boolean } = {}): Selection {
	if (mods.shift && sel.anchor !== null) {
		const [a, b] = sel.anchor < index ? [sel.anchor, index] : [index, sel.anchor];
		return { rows: Array.from({ length: b - a + 1 }, (_, i) => a + i), anchor: sel.anchor };
	}
	if (mods.toggle) {
		const rows = sel.rows.includes(index) ? sel.rows.filter((r) => r !== index) : [...sel.rows, index].sort((x, y) => x - y);
		return { rows, anchor: index };
	}
	return { rows: [index], anchor: index };
}

/** Absolute record position → page offset for a given page size. */
export function pageOffsetFor(position: number, pageSize: number): number {
	return Math.floor(Math.max(0, position) / pageSize) * pageSize;
}
