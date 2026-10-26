/**
 * Keyboard shortcuts: matching key presses against combos like `mod+k`,
 * showing them for this platform, and the list the shortcuts dialog shows.
 *
 * `mod` is ⌘ or Ctrl; either works everywhere, as the app always accepted.
 */

/** Where a shortcut works; the dialog shows the screen's own and the global ones. */
export type ShortcutScope = 'global' | 'home' | 'database' | 'datasheet' | 'online' | 'rows' | 'app' | 'app-edit' | 'drawer';

export type Shortcut = { keys: string; label: string; scope: ShortcutScope };

export const SCOPE_LABELS: Record<ShortcutScope, string> = {
	global: 'Everywhere',
	home: 'Home',
	database: 'Database',
	datasheet: 'Datasheet',
	online: 'Online database',
	rows: 'Lists of records',
	app: 'App',
	'app-edit': 'App, Edit mode',
	drawer: 'Step-by-step drawers'
};

export const SHORTCUTS: Shortcut[] = [
	{ keys: '?', label: 'Show keyboard shortcuts', scope: 'global' },
	{ keys: 'mod+/', label: 'Show keyboard shortcuts', scope: 'global' },
	{ keys: 'mod+,', label: 'Open Settings', scope: 'global' },
	{ keys: 'tab', label: 'Move to the next control', scope: 'global' },

	{ keys: 'mod+k', label: 'Search apps, databases and actions', scope: 'home' },
	{ keys: 'arrowdown', label: 'Select the next card in that direction (any arrow)', scope: 'home' },
	{ keys: 'arrowright', label: 'Diagram: the next table in that direction (any arrow)', scope: 'online' },
	{ keys: 'enter', label: 'Open the selected card', scope: 'home' },
	{ keys: 'delete', label: 'Delete the selected card or line', scope: 'home' },
	{ keys: 'shift+f10', label: 'Options for the selected card', scope: 'home' },
	{ keys: 'n', label: 'New database', scope: 'home' },
	{ keys: 'shift+n', label: 'New app', scope: 'home' },
	{ keys: 'escape', label: 'Clear the selection', scope: 'home' },

	{ keys: 'mod+k', label: 'Search tables, queries and actions', scope: 'database' },
	{ keys: 'mod+1', label: 'Diagram', scope: 'database' },
	{ keys: 'mod+2', label: 'Datasheet', scope: 'database' },
	{ keys: 'mod+3', label: 'Query', scope: 'database' },
	{ keys: 'mod+4', label: 'Reports', scope: 'database' },
	{ keys: 'mod+enter', label: 'Run the query', scope: 'database' },
	{ keys: 'mod+z', label: 'Diagram: undo moving or tidying tables', scope: 'database' },

	{ keys: 'arrowdown', label: 'Same field in the next row (↑ for the previous)', scope: 'datasheet' },
	{ keys: 'enter', label: 'Next cell', scope: 'datasheet' },
	{ keys: 'shift+enter', label: 'Save the record', scope: 'datasheet' },
	{ keys: 'escape', label: 'Undo the edit', scope: 'datasheet' },
	{ keys: 'mod+s', label: 'Save the record', scope: 'datasheet' },
	{ keys: 'delete', label: 'Delete the selected records', scope: 'datasheet' },
	{ keys: 'mod+c', label: 'Copy the current cell, or the ticked records', scope: 'datasheet' },
	{ keys: 'mod+v', label: 'Paste cells copied from a spreadsheet', scope: 'datasheet' },
	{ keys: 'mod+d', label: 'Fill the ticked records with the current cell’s value', scope: 'datasheet' },
	{ keys: 'mod+h', label: 'Find and replace', scope: 'datasheet' },
	{ keys: 'mod+z', label: 'Undo the last saved change', scope: 'datasheet' },
	{ keys: 'shift+mod+z', label: 'Redo', scope: 'datasheet' },

	{ keys: 'mod+k', label: 'Search tables, tabs and actions', scope: 'online' },
	{ keys: 'mod+1', label: 'Overview', scope: 'online' },
	{ keys: 'mod+2', label: 'Diagram', scope: 'online' },
	{ keys: 'mod+3', label: 'Data', scope: 'online' },
	{ keys: 'mod+4', label: 'Structure', scope: 'online' },
	{ keys: 'mod+5', label: 'Query', scope: 'online' },
	{ keys: 'mod+enter', label: 'Run the query', scope: 'online' },
	{ keys: 'alt+arrowdown', label: 'Next table', scope: 'online' },
	{ keys: 'alt+arrowup', label: 'Previous table', scope: 'online' },

	{ keys: 'arrowdown', label: 'Next row', scope: 'rows' },
	{ keys: 'arrowup', label: 'Previous row', scope: 'rows' },
	{ keys: 'home', label: 'First row', scope: 'rows' },
	{ keys: 'end', label: 'Last row', scope: 'rows' },
	{ keys: 'enter', label: 'Open the row', scope: 'rows' },
	{ keys: 'space', label: 'Select the row', scope: 'rows' },
	{ keys: '/', label: 'Search', scope: 'rows' },
	{ keys: 'n', label: 'New record', scope: 'rows' },
	{ keys: '[', label: 'Previous page', scope: 'rows' },
	{ keys: ']', label: 'Next page', scope: 'rows' },
	{ keys: 'delete', label: 'Delete the selected rows', scope: 'rows' },

	{ keys: 'mod+k', label: 'Search pages and actions', scope: 'app' },
	{ keys: 'mod+b', label: 'Show or hide the sidebar', scope: 'app' },
	{ keys: 'mod+e', label: 'Switch between Edit and Run', scope: 'app' },
	{ keys: 'alt+arrowdown', label: 'Next page', scope: 'app' },
	{ keys: 'alt+arrowup', label: 'Previous page', scope: 'app' },

	{ keys: 'arrowright', label: 'Move the selected widget (arrows)', scope: 'app-edit' },
	{ keys: 'shift+arrowright', label: 'Resize the selected widget (arrows)', scope: 'app-edit' },
	{ keys: 'mod+d', label: 'Duplicate the widget', scope: 'app-edit' },
	{ keys: 'mod+c', label: 'Copy the widget', scope: 'app-edit' },
	{ keys: 'mod+x', label: 'Cut the widget', scope: 'app-edit' },
	{ keys: 'mod+v', label: 'Paste a widget', scope: 'app-edit' },
	{ keys: 'delete', label: 'Delete the selection', scope: 'app-edit' },
	{ keys: 'mod+z', label: 'Undo', scope: 'app-edit' },
	{ keys: 'shift+mod+z', label: 'Redo', scope: 'app-edit' },
	{ keys: 'escape', label: 'Clear the selection', scope: 'app-edit' },

	{ keys: 'enter', label: 'Next step, or finish on the last', scope: 'drawer' },
	{ keys: 'mod+enter', label: 'Next step, or finish (also from a text area)', scope: 'drawer' },
	{ keys: 'alt+arrowright', label: 'Next step', scope: 'drawer' },
	{ keys: 'alt+arrowleft', label: 'Previous step', scope: 'drawer' },
	{ keys: 'escape', label: 'Close', scope: 'drawer' }
];

/** The scopes shown on the screen at `path` (a route path, e.g. `/db`), most specific first. */
export function scopesFor(path: string, appEditing = false): ShortcutScope[] {
	const route = '/' + (path.split('/').filter(Boolean)[0] ?? '');
	const own: Record<string, ShortcutScope[]> = {
		'/': ['home'],
		'/db': ['database', 'datasheet'],
		'/online': ['online', 'rows'],
		'/app': appEditing ? ['app', 'app-edit', 'rows'] : ['app', 'rows']
	};
	return [...(own[route] ?? []), 'drawer', 'global'];
}

type Combo = { mod: boolean; shift: boolean; alt: boolean; key: string };

function parse(combo: string): Combo {
	const parts = combo.toLowerCase().split('+');
	// `mod++` would be a plus key; none of ours use it.
	const key = parts.pop() ?? '';
	return { mod: parts.includes('mod'), shift: parts.includes('shift'), alt: parts.includes('alt'), key };
}

/** The key a press is matched by: lower-case, with Space spelled out. */
function pressedKey(e: Pick<KeyboardEvent, 'key'>): string {
	return e.key === ' ' ? 'space' : e.key.toLowerCase();
}

/**
 * Whether the press `e` is `combo`. Shift is only compared when the combo
 * names a letter, arrow or other named key: `?` and `/` sit on different
 * keys across layouts, so their own character is what counts.
 */
export function matches(e: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'shiftKey' | 'altKey'>, combo: string): boolean {
	const c = parse(combo);
	const mod = e.metaKey || e.ctrlKey;
	if (mod !== c.mod || e.altKey !== c.alt) return false;
	const named = /^[a-z0-9]$/.test(c.key) || c.key.length > 1;
	if (named && e.shiftKey !== c.shift) return false;
	const key = pressedKey(e);
	// A Mac's delete key sends Backspace.
	return key === c.key || (c.key === 'delete' && key === 'backspace');
}

const NAMES: Record<string, [mac: string, other: string]> = {
	mod: ['⌘', 'Ctrl'],
	shift: ['⇧', 'Shift'],
	alt: ['⌥', 'Alt'],
	enter: ['↵', 'Enter'],
	escape: ['Esc', 'Esc'],
	delete: ['⌫', 'Delete'],
	space: ['Space', 'Space'],
	tab: ['Tab', 'Tab'],
	home: ['Home', 'Home'],
	end: ['End', 'End'],
	arrowup: ['↑', '↑'],
	arrowdown: ['↓', '↓'],
	arrowleft: ['←', '←'],
	arrowright: ['→', '→']
};

/** The keys of `combo` as labels for this platform, e.g. `['Ctrl', 'K']` or `['⌘', 'K']`. */
export function keyLabels(combo: string, mac: boolean): string[] {
	return combo
		.toLowerCase()
		.split('+')
		.map((k) => NAMES[k]?.[mac ? 0 : 1] ?? (k.length === 1 ? k.toUpperCase() : k[0].toUpperCase() + k.slice(1)));
}

/** `combo` as plain text for this platform, for titles and messages: `Ctrl+K` or `⌘K`. */
export function shortcutText(combo: string, mac: boolean = isMac()): string {
	return keyLabels(combo, mac).join(mac ? '' : '+');
}

/** Whether this is a Mac, where shortcuts are shown with ⌘ and ⌥. */
export function isMac(platform: string = typeof navigator === 'undefined' ? '' : navigator.platform || navigator.userAgent): boolean {
	return /mac|iphone|ipad/i.test(platform);
}

/** Whether a key press happens while typing, or inside a menu or list that uses the keys itself. */
export function typingTarget(target: EventTarget | null): boolean {
	const t = target as Element | null;
	if (!t || typeof t.closest !== 'function') return false;
	return !!t.closest('input, textarea, select, [contenteditable=""], [contenteditable="true"], [role="menu"], [role="listbox"], [role="combobox"]');
}

/** The row to move to from `current` (−1 for none) among `count` rows for `key`, or null when the key doesn't move. */
export function nextRow(key: string, current: number, count: number): number | null {
	if (!count) return null;
	switch (key) {
		case 'ArrowDown':
			return Math.min(count - 1, current + 1);
		case 'ArrowUp':
			return current < 0 ? 0 : Math.max(0, current - 1);
		case 'Home':
			return 0;
		case 'End':
			return count - 1;
		case 'PageDown':
			return Math.min(count - 1, Math.max(0, current) + 10);
		case 'PageUp':
			return Math.max(0, current - 10);
		default:
			return null;
	}
}

/** The card to move to from `from` in an arrow's direction: the nearest one ahead, favouring ones in line. */
export function nearestInDirection<K>(
	from: { x: number; y: number },
	direction: 'up' | 'down' | 'left' | 'right',
	candidates: { key: K; x: number; y: number }[]
): K | null {
	let best: { key: K; score: number } | null = null;
	for (const c of candidates) {
		const dx = c.x - from.x;
		const dy = c.y - from.y;
		const ahead = direction === 'right' ? dx : direction === 'left' ? -dx : direction === 'down' ? dy : -dy;
		const aside = direction === 'left' || direction === 'right' ? Math.abs(dy) : Math.abs(dx);
		if (ahead <= 0) continue;
		const score = ahead + aside * 2;
		if (!best || score < best.score) best = { key: c.key, score };
	}
	return best?.key ?? null;
}
