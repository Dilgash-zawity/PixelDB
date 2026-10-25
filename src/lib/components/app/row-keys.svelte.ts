import { nextRow } from '#lib/logic/shortcuts.js';

/**
 * Keyboard use of a list of rows that open on click: one Tab stop for the
 * whole list (the last row focused), arrows / Home / End / Page keys move
 * between rows, Enter opens one and Space selects it.
 *
 * Give each row `data-row={i}`, `tabindex={rows.tabindex(i)}` and
 * `onfocus={() => rows.focused(i)}`, and put `onkeydown={rows.keydown}` on
 * the element around them (e.g. the table body).
 */
export class RowKeys {
	active = $state(0);

	constructor(
		private readonly actions: {
			open: (index: number) => void;
			toggle?: (index: number) => void;
		}
	) {}

	tabindex = (index: number): 0 | -1 => (index === this.active ? 0 : -1);

	focused = (index: number) => {
		this.active = index;
	};

	/** Keeps the Tab stop on a row that exists after the rows change. */
	clamp(count: number) {
		if (this.active >= count) this.active = Math.max(0, count - 1);
	}

	keydown = (e: KeyboardEvent) => {
		const target = e.target as HTMLElement;
		const row = target.closest<HTMLElement>('[data-row]');
		// Keys inside a row's own controls (its checkbox) stay theirs.
		if (!row || row !== target || e.metaKey || e.ctrlKey || e.altKey) return;
		const container = e.currentTarget as HTMLElement;
		const all = [...container.querySelectorAll<HTMLElement>('[data-row]')];
		const index = all.indexOf(row);
		if (e.key === 'Enter') {
			e.preventDefault();
			this.actions.open(index);
			return;
		}
		if (e.key === ' ' && this.actions.toggle) {
			e.preventDefault();
			this.actions.toggle(index);
			return;
		}
		const next = nextRow(e.key, index, all.length);
		if (next === null) return;
		e.preventDefault();
		this.active = next;
		all[next]?.focus();
		all[next]?.scrollIntoView({ block: 'nearest' });
	};

	/** Moves focus to the Tab-stop row inside `container`, if there is one. */
	focusIn(container: HTMLElement | null) {
		container?.querySelectorAll<HTMLElement>('[data-row]')[this.active]?.focus();
	}
}
