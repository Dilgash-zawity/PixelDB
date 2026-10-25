/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import { isMac, keyLabels, shortcutText, matches, nearestInDirection, nextRow, scopesFor, SHORTCUTS } from './shortcuts';

const press = (key: string, mods: { meta?: boolean; ctrl?: boolean; shift?: boolean; alt?: boolean } = {}) => ({
	key,
	metaKey: !!mods.meta,
	ctrlKey: !!mods.ctrl,
	shiftKey: !!mods.shift,
	altKey: !!mods.alt
});

describe('shortcuts', () => {
	test('mod is either Command or Control', () => {
		expect(matches(press('k', { meta: true }), 'mod+k')).toBe(true);
		expect(matches(press('K', { ctrl: true }), 'mod+k')).toBe(true);
		expect(matches(press('k'), 'mod+k')).toBe(false);
		expect(matches(press('k', { ctrl: true, alt: true }), 'mod+k')).toBe(false);
	});

	test('shift counts for letters and named keys but not for symbols', () => {
		expect(matches(press('N', { shift: true }), 'shift+n')).toBe(true);
		expect(matches(press('N', { shift: true }), 'n')).toBe(false);
		expect(matches(press('Z', { meta: true, shift: true }), 'shift+mod+z')).toBe(true);
		expect(matches(press('?', { shift: true }), '?')).toBe(true);
		expect(matches(press('/', { meta: true }), 'mod+/')).toBe(true);
		expect(matches(press(' '), 'space')).toBe(true);
		expect(matches(press('Backspace'), 'delete')).toBe(true);
		expect(matches(press('ArrowDown', { alt: true }), 'alt+arrowdown')).toBe(true);
		expect(matches(press('ArrowDown'), 'alt+arrowdown')).toBe(false);
	});

	test('labels follow the platform', () => {
		expect(keyLabels('mod+k', true)).toEqual(['⌘', 'K']);
		expect(keyLabels('mod+k', false)).toEqual(['Ctrl', 'K']);
		expect(keyLabels('shift+mod+z', false)).toEqual(['Shift', 'Ctrl', 'Z']);
		expect(keyLabels('alt+arrowup', true)).toEqual(['⌥', '↑']);
		expect(keyLabels('shift+f10', false)).toEqual(['Shift', 'F10']);
		expect(shortcutText('mod+k', false)).toBe('Ctrl+K');
		expect(shortcutText('mod+k', true)).toBe('⌘K');
		expect(isMac('MacIntel')).toBe(true);
		expect(isMac('Linux x86_64')).toBe(false);
	});

	test('each screen shows its own shortcuts, then drawers and global ones', () => {
		expect(scopesFor('/')).toEqual(['home', 'drawer', 'global']);
		expect(scopesFor('/db')).toEqual(['database', 'datasheet', 'drawer', 'global']);
		expect(scopesFor('/app', true)).toContain('app-edit');
		expect(scopesFor('/app')).not.toContain('app-edit');
		expect(scopesFor('/settings')).toEqual(['drawer', 'global']);
		// No combo is listed twice for one scope.
		const seen = new Set(SHORTCUTS.map((s) => `${s.scope} ${s.keys}`));
		expect(seen.size).toBe(SHORTCUTS.length);
	});

	test('rows move within the list', () => {
		expect(nextRow('ArrowDown', -1, 3)).toBe(0);
		expect(nextRow('ArrowDown', 2, 3)).toBe(2);
		expect(nextRow('ArrowUp', 0, 3)).toBe(0);
		expect(nextRow('ArrowUp', -1, 3)).toBe(0);
		expect(nextRow('End', 0, 3)).toBe(2);
		expect(nextRow('PageDown', 0, 30)).toBe(10);
		expect(nextRow('ArrowDown', 0, 0)).toBeNull();
		expect(nextRow('a', 0, 3)).toBeNull();
	});

	test('arrows pick the nearest card ahead, favouring ones in line', () => {
		const cards = [
			{ key: 'right', x: 300, y: 0 },
			{ key: 'diagonal', x: 200, y: 150 },
			{ key: 'below', x: 0, y: 200 }
		];
		expect(nearestInDirection({ x: 0, y: 0 }, 'right', cards)).toBe('right');
		expect(nearestInDirection({ x: 0, y: 0 }, 'down', cards)).toBe('below');
		expect(nearestInDirection({ x: 0, y: 0 }, 'left', cards)).toBeNull();
	});
});
