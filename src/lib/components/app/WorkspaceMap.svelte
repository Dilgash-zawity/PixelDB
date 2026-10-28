<script lang="ts">
	// Services
	import { databaseService } from '#lib/services/database.js';

	// Types
	import type { AppFile, DatabaseFile, OnlineDatabase } from '#lib/api/types.js';

	// Utils
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onDestroy, tick, untrack } from 'svelte';
	import { edgeRoute, type Point } from '#lib/logic/relationships.js';
	import { appHref, databaseHref } from '#lib/logic/routes.js';
	import { connectionSummary, provider as providerOf } from '#lib/logic/online.js';
	import { WHEEL_ZOOM_RATE } from '#lib/logic/pinch.js';
	import { TEMPLATES } from '#lib/logic/templates.js';
	import { matches, nearestInDirection } from '#lib/logic/shortcuts.js';
	import { databaseNameProblem, formatBytes } from '#lib/logic/values.js';
	import {
		NODE_HEIGHT,
		NODE_WIDTH,
		autoMapLayout,
		freeSpot,
		homeLayoutKey,
		missingStub,
		nodeKey,
		placeMissingNodes,
		usersByDatabase,
		workspaceLinks,
		type MapLayout,
		type NodeKind,
		type WorkspaceLink
	} from '#lib/logic/workspace-map.js';
	import { attempt } from '#lib/state/feedback.svelte.js';
	import { toast } from 'svelte-sonner';
	import { workspace } from '#lib/state/workspace.svelte.js';
	import { settings } from '#lib/state/settings.svelte.js';
	import { cn } from '#lib/utils.js';

	// Components
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import * as ButtonGroup from '#lib/components/ui/button-group/index.js';
	import * as ContextMenu from '#lib/components/ui/context-menu/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import FocusModeSwitch from './FocusModeSwitch.svelte';
	import NameDialog from './NameDialog.svelte';
	import NewDatabaseDrawer from './NewDatabaseDrawer.svelte';
	import NewAppDrawer from './NewAppDrawer.svelte';
	import HomeCommandPalette from './HomeCommandPalette.svelte';
	import Shortcut from './Shortcut.svelte';
	import PixelIcon from './PixelIcon.svelte';
	import ProviderIcon from './ProviderIcon.svelte';

	// Icons
	import {
		Copy as CopyIcon,
		PackageOpen as PackageOpenIcon,
		Share as ShareIcon,
		Database as DatabaseIcon,
		EllipsisVertical as EllipsisVerticalIcon,
		ExternalLink as ExternalLinkIcon,
		LayoutDashboard as LayoutDashboardIcon,
		LayoutGrid as LayoutGridIcon,
		Link as LinkIcon,
		Maximize as MaximizeIcon,
		Minus as MinusIcon,
		Pencil as PencilIcon,
		PlugZap as PlugZapIcon,
		Plus as PlusIcon,
		Settings2 as SettingsIcon,
		TextCursorInput as TextCursorInputIcon,
		Trash2 as TrashIcon,
		TriangleAlert as TriangleAlertIcon,
		Unlink as UnlinkIcon,
		Sparkles as SparklesIcon,
	} from '@lucide/svelte';

	// Script
	const MIN_ZOOM = 0.25;
	const MAX_ZOOM = 2;

	/** Graph paper cell, and the chip drawn on each line. */
	const GRID = 40;
	const CHIP = 22;

	const apps = $derived(workspace.apps);
	const databases = $derived(workspace.databases);
	const online = $derived(workspace.online);
	const onlineByName = $derived(new Map(online.map((o) => [o.name, o])));
	/** Every database card's name, local and online (names are unique across both). */
	const databaseNames = $derived([...databases.map((d) => d.name), ...online.map((o) => o.name)]);
	const appByName = $derived(new Map(apps.map((a) => [a.name, a])));
	const links = $derived(
		workspaceLinks(
			apps,
			[...databases.filter((d) => d.valid).map((d) => d.name), ...online.map((o) => o.name)]
		)
	);
	const users = $derived(usersByDatabase(links));
	const linkKey = (l: { app: string; database: string }) => `${l.app}\u0000${l.database}`;

	let layout = $state<MapLayout>({});
	let zoom = $state(1);
	let pan = $state<Point>({ x: 0, y: 0 });
	let viewport = $state<HTMLDivElement | null>(null);
	/** A selected card (its node key) or line (its link key). */
	let selection = $state<{ kind: 'node' | 'link'; key: string } | null>(null);
	let hoveredNode = $state<string | null>(null);
	let hoveredLink = $state<string | null>(null);
	let connecting = $state<{ kind: NodeKind; name: string; start: Point; pointer: Point } | null>(null);
	let dropTarget = $state<string | null>(null);

	const focusNode = $derived(selection?.kind === 'node' ? selection.key : hoveredNode);
	const touches = (l: WorkspaceLink) =>
		linkKey(l) === hoveredLink ||
		(selection?.kind === 'link' && selection.key === linkKey(l)) ||
		focusNode === nodeKey('app', l.app) ||
		focusNode === nodeKey('db', l.database);
	/** Cards at the other end of a lit line. */
	const related = $derived(new Set(links.filter(touches).flatMap((l) => [nodeKey('app', l.app), nodeKey('db', l.database)])));

	// ── Layout: saved per folder in this browser, cards placed as they appear ──

	const storageKey = $derived(workspace.info ? homeLayoutKey(workspace.info.path) : null);
	let restoredFor: string | null = null;
	$effect(() => {
		const key = storageKey;
		const [appNames, dbNames, ls] = [apps.map((a) => a.name), databaseNames, links];
		untrack(() => {
			if (key !== restoredFor) {
				restoredFor = key;
				layout = key ? readLayout(key) : {};
				placed = false;
			}
			layout = placeMissingNodes(layout, appNames, dbNames, ls);
			if (!placed) {
				placed = true;
				tick().then(fit);
			}
		});
	});
	let placed = false;

	function readLayout(key: string): MapLayout {
		try {
			const raw = JSON.parse(localStorage.getItem(key) ?? '{}');
			return raw && typeof raw === 'object' ? raw : {};
		} catch {
			return {};
		}
	}

	let saveTimer: ReturnType<typeof setTimeout> | undefined;
	function saveLayoutNow() {
		clearTimeout(saveTimer);
		saveTimer = undefined;
		try {
			if (storageKey) localStorage.setItem(storageKey, JSON.stringify(layout));
		} catch {
			// Positions are a convenience; the map still works without them.
		}
	}
	function saveLayoutSoon() {
		clearTimeout(saveTimer);
		saveTimer = setTimeout(saveLayoutNow, 400);
	}
	onDestroy(() => saveTimer !== undefined && saveLayoutNow());

	function moveNode(kind: NodeKind, from: string, to: string) {
		const p = layout[nodeKey(kind, from)];
		if (!p) return;
		delete layout[nodeKey(kind, from)];
		layout[nodeKey(kind, to)] = p;
		saveLayoutNow();
	}

	function arrange() {
		layout = autoMapLayout(
			apps.map((a) => a.name),
			databaseNames,
			links
		);
		saveLayoutNow();
		tick().then(fit);
	}

	// ── Viewport: pan, zoom, fit ──

	function toWorld(clientX: number, clientY: number): Point {
		const rect = viewport?.getBoundingClientRect();
		return { x: (clientX - (rect?.left ?? 0) - pan.x) / zoom, y: (clientY - (rect?.top ?? 0) - pan.y) / zoom };
	}

	function zoomTo(next: number, at?: Point) {
		next = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, next));
		const c = at ?? { x: (viewport?.clientWidth ?? 0) / 2, y: (viewport?.clientHeight ?? 0) / 2 };
		pan = { x: c.x - ((c.x - pan.x) / zoom) * next, y: c.y - ((c.y - pan.y) / zoom) * next };
		zoom = next;
	}

	function fit() {
		const ps = Object.values(layout);
		if (!viewport || !ps.length) return;
		const minX = Math.min(...ps.map((p) => p.x));
		const minY = Math.min(...ps.map((p) => p.y));
		const maxX = Math.max(...ps.map((p) => p.x)) + NODE_WIDTH + 80;
		const maxY = Math.max(...ps.map((p) => p.y)) + NODE_HEIGHT;
		const { clientWidth: w, clientHeight: h } = viewport;
		const z = Math.min(1.25, Math.max(MIN_ZOOM, Math.min((w - 96) / (maxX - minX), (h - 160) / (maxY - minY))));
		zoom = z;
		pan = { x: (w - (maxX - minX) * z) / 2 - minX * z, y: (h - (maxY - minY) * z) / 2 - minY * z };
	}

	/** Pans just enough to bring card `key` into view, with a margin. */
	function reveal(key: string) {
		const p = layout[key];
		if (!viewport || !p) return;
		// Focusing a card can scroll the clipped viewport itself; panning does that job.
		viewport.scrollTop = viewport.scrollLeft = 0;
		const m = 32;
		const { clientWidth: w, clientHeight: h } = viewport;
		const [left, top] = [p.x * zoom + pan.x, p.y * zoom + pan.y];
		const [right, bottom] = [left + NODE_WIDTH * zoom, top + NODE_HEIGHT * zoom];
		const dx = left < m ? m - left : right > w - m ? w - m - right : 0;
		const dy = top < m ? m - top : bottom > h - m ? h - m - bottom : 0;
		if (dx || dy) pan = { x: pan.x + dx, y: pan.y + dy };
	}

	function focusCard(key: string) {
		viewport?.querySelector<HTMLElement>(`[data-node="${CSS.escape(key)}"]`)?.focus({ preventScroll: true });
	}

	/** Wheel pans; pinch or Ctrl/⌘ + wheel zooms around the pointer. Not passive, so it can stop page scroll. */
	function wheel(node: HTMLDivElement) {
		const onWheel = (e: WheelEvent) => {
			e.preventDefault();
			if (e.ctrlKey || e.metaKey) {
				const rect = node.getBoundingClientRect();
				zoomTo(zoom * Math.exp(-e.deltaY * WHEEL_ZOOM_RATE), { x: e.clientX - rect.left, y: e.clientY - rect.top });
			} else {
				pan = { x: pan.x - e.deltaX, y: pan.y - e.deltaY };
			}
		};
		node.addEventListener('wheel', onWheel, { passive: false });
		return () => node.removeEventListener('wheel', onWheel);
	}

	/** Tracks a pointer drag on the window until release. */
	function drag(e: PointerEvent, onMove: (dx: number, dy: number, ev: PointerEvent) => void, onEnd: (moved: boolean, ev: PointerEvent) => void) {
		const [sx, sy] = [e.clientX, e.clientY];
		let moved = false;
		const move = (ev: PointerEvent) => {
			if (Math.abs(ev.clientX - sx) + Math.abs(ev.clientY - sy) > 3) moved = true;
			if (moved) onMove(ev.clientX - sx, ev.clientY - sy, ev);
		};
		const up = (ev: PointerEvent) => {
			window.removeEventListener('pointermove', move);
			window.removeEventListener('pointerup', up);
			onEnd(moved, ev);
		};
		window.addEventListener('pointermove', move);
		window.addEventListener('pointerup', up);
	}

	function startPan(e: PointerEvent) {
		if (e.button !== 0 && e.button !== 1) return;
		if ((e.target as Element).closest('[data-node], [data-link], [data-toolbar]')) return;
		const origin = { ...pan };
		drag(
			e,
			(dx, dy) => (pan = { x: origin.x + dx, y: origin.y + dy }),
			(moved) => {
				if (!moved) selection = null;
			}
		);
	}

	function startNodeDrag(e: PointerEvent, key: string) {
		if (e.button !== 0) return;
		const origin = { ...layout[key] };
		drag(
			e,
			(dx, dy) => (layout[key] = { x: Math.round(origin.x + dx / zoom), y: Math.round(origin.y + dy / zoom) }),
			(moved) => {
				if (moved) saveLayoutSoon();
				else selection = { kind: 'node', key };
			}
		);
	}

	// ── Drag a card's dot onto a card of the other kind to connect ──

	function canConnect(kind: NodeKind, name: string, targetKey: string | null): targetKey is string {
		if (!targetKey) return false;
		const [tKind, ...rest] = targetKey.split(':');
		const tName = rest.join(':');
		if (tKind === kind) return false;
		const [app, db] = kind === 'app' ? [name, tName] : [tName, name];
		const file = appByName.get(app);
		const usable = !!databases.find((d) => d.name === db)?.valid || !!onlineByName.get(db)?.settings;
		return !!file?.valid && usable && !file.databases.includes(db);
	}

	function startConnect(e: PointerEvent, kind: NodeKind, name: string, side: 1 | -1) {
		if (e.button !== 0) return;
		e.stopPropagation();
		const p = layout[nodeKey(kind, name)];
		const start = { x: p.x + (side === 1 ? NODE_WIDTH : 0), y: p.y + NODE_HEIGHT / 2 };
		connecting = { kind, name, start, pointer: start };
		drag(
			e,
			(_dx, _dy, ev) => {
				if (!connecting) return;
				connecting.pointer = toWorld(ev.clientX, ev.clientY);
				const over = document.elementFromPoint(ev.clientX, ev.clientY)?.closest<HTMLElement>('[data-node]')?.dataset.node ?? null;
				dropTarget = canConnect(kind, name, over) ? over : null;
			},
			async () => {
				const target = dropTarget;
				connecting = null;
				dropTarget = null;
				if (!target) return;
				const other = target.slice(target.indexOf(':') + 1);
				const [app, db] = kind === 'app' ? [name, other] : [other, name];
				if (await workspace.connectApp(app, db)) selection = { kind: 'link', key: linkKey({ app, database: db }) };
			}
		);
	}

	const connectorPreview = $derived.by(() => {
		if (!connecting) return '';
		const { start, pointer } = connecting;
		const bend = Math.max(40, Math.abs(pointer.x - start.x) / 2);
		const dir = pointer.x >= start.x ? 1 : -1;
		return `M ${start.x} ${start.y} C ${start.x + bend * dir} ${start.y}, ${pointer.x - bend * dir} ${pointer.y}, ${pointer.x} ${pointer.y}`;
	});

	// ── Lines ──

	const edges = $derived.by(() => {
		// Lines into the same database share their last stretch; one chip there is enough.
		const chips = new Set<string>();
		const chipAt = (p: Point) => {
			const k = `${p.x},${p.y}`;
			if (chips.has(k)) return null;
			chips.add(k);
			return p;
		};
		return links.flatMap((l, i) => {
			const a = layout[nodeKey('app', l.app)];
			if (!a) return [];
			const key = linkKey(l);
			const active = touches(l);
			if (l.missing) {
				const stub = missingStub(a);
				return [{ link: l, key, i, active, path: stub.path, end: stub.end, endDir: -1 as const, chip: null }];
			}
			const d = layout[nodeKey('db', l.database)];
			if (!d) return [];
			const route = edgeRoute(a, NODE_HEIGHT / 2, d, NODE_HEIGHT / 2, NODE_WIDTH);
			const chip = chipAt({ x: (route.mid.x + route.end.x) / 2, y: route.end.y });
			return [{ link: l, key, i, active, path: route.path, end: route.end, endDir: route.endDir, chip }];
		});
	});

	// ── Creating, opening, renaming ──

	let creatingDatabase = $state(false);
	let creatingApp = $state(false);
	/** Where a card created from the right-click menu goes. */
	let createAt = $state<Point | null>(null);
	let renaming = $state<{ kind: NodeKind; name: string } | null>(null);
	let duplicating = $state<{ kind: NodeKind; name: string; suggested: string } | null>(null);

	const appNameProblem = (v: string, except?: string) =>
		databaseNameProblem(v) ?? (workspace.appNameTaken(v, except) ? 'An app with this name already exists' : null);
	const dbNameProblem = (v: string, except?: string) =>
		databaseNameProblem(v) ?? (workspace.nameTaken(v, except) ? 'A database with this name already exists' : null);

	let paletteOpen = $state(false);

	/** Opens the ⌘K search (also from the home screen's top bar). */
	export function openSearch() {
		paletteOpen = true;
	}

	function newDatabase(at: Point | null = null) {
		createAt = at;
		creatingDatabase = true;
	}
	function newApp(at: Point | null = null) {
		createAt = at;
		creatingApp = true;
	}

	async function openShared() {
		const app = await workspace.openSharedApp();
		if (app) open('app', app);
	}

	let makingSample = $state(false);

	/** Makes the shop template (a database with sample records and an app over it) and opens its database. */
	async function trySample() {
		const template = TEMPLATES.find((t) => t.id === 'shop') ?? TEMPLATES[0];
		if (!template || makingSample) return;
		let name = template.suggestedName;
		for (let n = 2; workspace.nameTaken(name) || workspace.appNameTaken(name); n++) name = `${template.suggestedName} ${n}`;
		makingSample = true;
		try {
			if (await workspace.createFromTemplate(template, name)) goto(databaseHref(resolve('/db'), name));
		} finally {
			makingSample = false;
		}
	}

	/** Puts a just-created card where the menu was opened, or in the middle of the view. */
	function placeCreated(kind: NodeKind, name: string) {
		const key = nodeKey(kind, name);
		const view = viewport ? toWorld(viewport.getBoundingClientRect().left + viewport.clientWidth / 2, viewport.getBoundingClientRect().top + viewport.clientHeight / 2) : { x: 0, y: 0 };
		const at = createAt ?? { x: Math.round(view.x - NODE_WIDTH / 2), y: Math.round(view.y - NODE_HEIGHT / 2) };
		layout[key] = freeSpot(layout, at, key);
		// Databases a template created alongside go next to the app.
		layout = placeMissingNodes(layout, apps.map((a) => a.name), databaseNames, links);
		selection = { kind: 'node', key };
		saveLayoutNow();
	}

	async function databaseCreated(name: string, kind: 'local' | 'online') {
		// Creating a database opens it; the home screen keeps nothing open.
		if (kind === 'local') await attempt(() => databaseService.close());
		await workspace.refresh();
		placeCreated('db', name);
	}

	/** The saved online connection being edited. */
	let editingOnline = $state<OnlineDatabase | null>(null);
	let editOpen = $state(false);
	function editOnline(name: string) {
		const o = onlineByName.get(name);
		if (!o?.settings) return;
		editingOnline = o;
		editOpen = true;
	}

	async function testOnline(name: string) {
		const settings = onlineByName.get(name)?.settings;
		if (!settings) return;
		const result = await attempt(() => workspace.testOnline(settings, undefined, name));
		if (result) toast.success(`Connected to “${name}”`, { description: `${result.server} answered in ${result.millis} ms.` });
	}

	function open(kind: NodeKind, name: string) {
		if (kind === 'app') goto(appHref(resolve('/app'), name));
		else if (onlineByName.has(name)) goto(databaseHref(resolve('/online'), name));
		else goto(databaseHref(resolve('/db'), name));
	}

	function canOpen(kind: NodeKind, name: string): boolean {
		if (kind === 'app') return !!appByName.get(name)?.valid;
		return !!databases.find((d) => d.name === name)?.valid || !!onlineByName.get(name)?.settings;
	}

	function remove(kind: NodeKind, name: string) {
		if (kind === 'db' && onlineByName.has(name)) return workspace.removeOnline(name);
		return kind === 'app' ? workspace.removeApp(name) : workspace.remove(name);
	}

	async function rename(kind: NodeKind, from: string, to: string): Promise<boolean> {
		const settings = kind === 'db' ? onlineByName.get(from)?.settings : undefined;
		const ok =
			kind === 'app'
				? await workspace.renameApp(from, to)
				: settings
					? await workspace.updateOnline(from, to, settings)
					: await workspace.rename(from, to);
		if (ok) moveNode(kind, from, to);
		return ok;
	}

	async function duplicate(kind: NodeKind, from: string, to: string): Promise<boolean> {
		const ok = kind === 'app' ? await workspace.duplicateApp(from, to) : await workspace.duplicate(from, to);
		if (ok) {
			const p = layout[nodeKey(kind, from)];
			if (p) createAt = { x: p.x + 24, y: p.y + 24 };
			placeCreated(kind, to);
		}
		return ok;
	}

	function startDuplicate(kind: NodeKind, name: string) {
		const suggested = kind === 'app' ? workspace.copyName(name, (n) => workspace.appNameTaken(n)) : workspace.copyName(name);
		duplicating = { kind, name, suggested };
	}

	// ── Context menu and keys ──

	type MenuTarget = { kind: 'canvas' } | { kind: 'node'; node: NodeKind; name: string } | { kind: 'link'; app: string; database: string; missing: boolean };
	let menu = $state<{ target: MenuTarget; at: Point }>({ target: { kind: 'canvas' }, at: { x: 0, y: 0 } });

	/** When Shift+F10 last opened a card's options, so the browser's own context menu for that key is skipped. */
	let optionsByKeyAt = 0;
	/** The card whose options menu is open. */
	let optionsOpen = $state<string | null>(null);

	function onContextMenu(e: MouseEvent) {
		if (Date.now() - optionsByKeyAt < 500) {
			e.preventDefault();
			return;
		}
		const el = e.target as Element;
		const node = el.closest<HTMLElement>('[data-node]')?.dataset.node;
		const link = el.closest<SVGElement>('[data-link]')?.dataset.link;
		const at = toWorld(e.clientX, e.clientY);
		if (node) {
			const i = node.indexOf(':');
			menu = { target: { kind: 'node', node: node.slice(0, i) as NodeKind, name: node.slice(i + 1) }, at };
		} else if (link) {
			const l = links.find((x) => linkKey(x) === link);
			menu = l ? { target: { kind: 'link', app: l.app, database: l.database, missing: l.missing }, at } : { target: { kind: 'canvas' }, at };
		} else {
			menu = { target: { kind: 'canvas' }, at: { x: Math.round(at.x), y: Math.round(at.y) } };
		}
	}

	const ARROWS: Record<string, 'up' | 'down' | 'left' | 'right'> = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };

	function onKey(e: KeyboardEvent) {
		if (matches(e, 'mod+k') && !e.defaultPrevented) {
			e.preventDefault();
			paletteOpen = !paletteOpen;
			return;
		}
		const typing = (e.target as Element | null)?.closest('input, textarea, select, [contenteditable], [role="dialog"], [role="alertdialog"], [role="menu"], [role="listbox"]');
		if (typing || e.defaultPrevented || creatingDatabase || creatingApp) return;
		if (matches(e, 'n') || matches(e, 'shift+n')) {
			e.preventDefault();
			if (e.shiftKey) newApp();
			else newDatabase();
			return;
		}
		const direction = ARROWS[e.key];
		if (direction && !e.altKey && !e.metaKey && !e.ctrlKey) {
			const keys = Object.keys(layout).filter((k) => viewport?.querySelector(`[data-node="${CSS.escape(k)}"]`));
			const from = selection?.kind === 'node' ? layout[selection.key] : null;
			// With nothing selected, start from the card nearest the top left.
			const next = from
				? nearestInDirection({ x: from.x, y: from.y }, direction, keys.filter((k) => k !== selection?.key).map((k) => ({ key: k, ...layout[k] })))
				: keys.sort((a, b) => layout[a].x + layout[a].y - (layout[b].x + layout[b].y))[0];
			if (next) {
				e.preventDefault();
				focusCard(next);
			}
			return;
		}
		if (!selection) return;
		if ((matches(e, 'shift+f10') || e.key === 'ContextMenu') && selection.kind === 'node') {
			e.preventDefault();
			optionsByKeyAt = Date.now();
			optionsOpen = selection.key;
			return;
		}
		if (e.key === 'Escape') {
			selection = null;
		} else if (e.key === 'Enter' && selection.kind === 'node') {
			const i = selection.key.indexOf(':');
			const [kind, name] = [selection.key.slice(0, i) as NodeKind, selection.key.slice(i + 1)];
			if (canOpen(kind, name)) open(kind, name);
		} else if (e.key === 'Delete' || e.key === 'Backspace') {
			e.preventDefault();
			if (selection.kind === 'link') {
				const l = links.find((x) => linkKey(x) === selection?.key);
				if (l) workspace.disconnectApp(l.app, l.database);
			} else {
				const i = selection.key.indexOf(':');
				remove(selection.key.slice(0, i) as NodeKind, selection.key.slice(i + 1));
			}
		}
	}

	function modifiedLabel(ms: number | null): string {
		if (ms === null) return '';
		const relative = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' });
		const minutes = Math.round((ms - Date.now()) / 60000);
		if (Math.abs(minutes) < 60) return relative.format(minutes, 'minute');
		const hours = Math.round(minutes / 60);
		if (Math.abs(hours) < 24) return relative.format(hours, 'hour');
		const days = Math.round(hours / 24);
		if (Math.abs(days) < 7) return relative.format(days, 'day');
		return new Date(ms).toLocaleDateString(undefined, { dateStyle: 'medium' });
	}
</script>

<svelte:window onkeydown={onKey} />

{#snippet nodeMenu(kind: NodeKind, name: string, Item: typeof DropdownMenu.Item | typeof ContextMenu.Item)}
	{#if kind === 'db' && onlineByName.has(name)}
		{@const readable = !!onlineByName.get(name)?.settings}
		<Item disabled={!readable} onclick={() => open(kind, name)}><ExternalLinkIcon /> Open</Item>
		<Item disabled={!readable} onclick={() => editOnline(name)}><SettingsIcon /> Edit connection…</Item>
		<Item disabled={!readable} onclick={() => testOnline(name)}><PlugZapIcon /> Test connection</Item>
		<Item disabled={!readable} onclick={() => (renaming = { kind, name })}><TextCursorInputIcon /> Rename…</Item>
	{:else}
		<Item disabled={!canOpen(kind, name)} onclick={() => open(kind, name)}><ExternalLinkIcon /> Open</Item>
		<Item onclick={() => (renaming = { kind, name })}><PencilIcon /> Rename…</Item>
		<Item disabled={!canOpen(kind, name)} onclick={() => startDuplicate(kind, name)}><CopyIcon /> Duplicate…</Item>
		{#if kind === 'app'}
			<Item disabled={!canOpen(kind, name)} onclick={() => workspace.shareApp(name)}><ShareIcon /> Share as a file…</Item>
		{/if}
	{/if}
{/snippet}

{#snippet handles(kind: NodeKind, name: string)}
	{#each [-1, 1] as const as side (side)}
		<button
			type="button"
			tabindex="-1"
			aria-label={kind === 'app' ? `Connect ${name} to a database` : `Connect ${name} to an app`}
			title={kind === 'app' ? 'Drag onto a database to connect' : 'Drag onto an app to connect'}
			class={cn(
				'absolute top-1/2 size-3 -translate-y-1/2 cursor-crosshair rounded-[2px] border-2 border-(--map-line) bg-(--map-paper) opacity-0 transition-opacity group-hover/node:opacity-100 hover:scale-125',
				side === 1 ? '-end-1.5' : '-start-1.5'
			)}
			onpointerdown={(e) => startConnect(e, kind, name, side)}
			onclick={(e) => e.stopPropagation()}
		></button>
	{/each}
{/snippet}

<!-- A tile: name and a line of detail top-left, a pixel icon bottom-right, badges bottom-left. -->
{#snippet card(
	kind: NodeKind,
	name: string,
	valid: boolean,
	detail: string,
	badges: import('svelte').Snippet,
	icon: import('svelte').Snippet,
	connectable = valid
)}
	{@const key = nodeKey(kind, name)}
	{@const p = layout[key]}
	{@const selected = selection?.kind === 'node' && selection.key === key}
	{#if p}
		<!-- svelte-ignore a11y_no_noninteractive_tabindex (a card is a focus stop: Enter opens it; it holds its own buttons, so it can't be one) -->
		<div
			data-node={key}
			role="group"
			tabindex="0"
			aria-label={`${kind === 'app' ? 'App' : onlineByName.has(name) ? 'Online database' : 'Database'} ${name}`}
			aria-keyshortcuts="Enter Delete Shift+F10"
			onfocus={() => {
				selection = { kind: 'node', key };
				reveal(key);
			}}
			class={cn(
				'group/node absolute flex cursor-grab flex-col rounded-lg border outline-none focus-visible:ring-4 focus-visible:ring-(--map-accent) border-(--map-tile-border) p-3 text-(--map-tile-fg) transition-[box-shadow,background-color,opacity,translate] select-none active:cursor-grabbing',
				kind === 'app' ? 'bg-(--map-app)' : 'bg-(--map-db)',
				!valid && 'opacity-60',
				related.has(key) && !selected && 'ring-2 ring-(--map-accent)',
				selected && 'bg-(--map-accent) text-(--map-accent-fg)',
				dropTarget === key && 'ring-4 ring-(--map-accent)',
				connecting && !dropTarget && nodeKey(connecting.kind, connecting.name) !== key && !canConnect(connecting.kind, connecting.name, key) && 'opacity-40'
			)}
			style:left="{p.x}px"
			style:top="{p.y}px"
			style:width="{NODE_WIDTH}px"
			style:height="{NODE_HEIGHT}px"
			onpointerenter={() => (hoveredNode = key)}
			onpointerleave={() => hoveredNode === key && (hoveredNode = null)}
			onpointerdown={(e) => startNodeDrag(e, key)}
			ondblclick={() => canOpen(kind, name) && open(kind, name)}
		>
			<div class="flex items-start gap-1">
				<span class="line-clamp-2 min-w-0 flex-1 text-[13px] leading-snug font-medium break-words" title={name}>{name}</span>
				<span role="presentation" class="-me-1.5 -mt-1.5" onpointerdown={(e) => e.stopPropagation()}>
					<DropdownMenu.Root open={optionsOpen === key} onOpenChange={(o) => (optionsOpen = o ? key : null)}>
						<DropdownMenu.Trigger>
							{#snippet child({ props })}
								<Button
									{...props}
									variant="ghost"
									size="icon-xs"
									class="text-current opacity-0 group-hover/node:opacity-100 hover:bg-white/15 hover:text-current focus-visible:opacity-100 data-[state=open]:opacity-100"
									aria-label={`${name} options`}><EllipsisVerticalIcon /></Button
								>
							{/snippet}
						</DropdownMenu.Trigger>
						<DropdownMenu.Content align="start" side="right">
							<DropdownMenu.Group>{@render nodeMenu(kind, name, DropdownMenu.Item)}</DropdownMenu.Group>
							<DropdownMenu.Separator />
							<DropdownMenu.Group>
								<DropdownMenu.Item variant="destructive" onclick={() => remove(kind, name)}>
									<TrashIcon /> {kind === 'app' ? 'Delete app' : onlineByName.has(name) ? 'Remove connection' : 'Delete database'}
								</DropdownMenu.Item>
							</DropdownMenu.Group>
						</DropdownMenu.Content>
					</DropdownMenu.Root>
				</span>
			</div>
			{#if detail}<span class="mt-0.5 truncate text-[11px] leading-snug opacity-65" title={detail}>{detail}</span>{/if}
			<div class="mt-auto flex items-end justify-between gap-2">
				<div class="flex min-w-0 flex-wrap gap-1">{@render badges()}</div>
				<span class="shrink-0">{@render icon()}</span>
			</div>
			{#if connectable}{@render handles(kind, name)}{/if}
		</div>
	{/if}
{/snippet}

{#snippet appNode(app: AppFile)}
	{#snippet icon()}<PixelIcon name="app" class="size-7" />{/snippet}
	{#snippet badges()}
		{#if !app.valid}
			<Badge variant="destructive"><TriangleAlertIcon /> Can't be read</Badge>
		{:else if links.some((l) => l.app === app.name && l.missing)}
			<Badge variant="outline" class="border-warning/60 bg-warning/15 text-warning"><TriangleAlertIcon /> Missing</Badge>
		{:else if !app.databases.length}
			<Badge variant="outline" class="border-current/30 text-current">No data</Badge>
		{/if}
	{/snippet}
	{@render card('app', app.name, app.valid, app.valid ? app.description || `Edited ${modifiedLabel(app.modified)}` : '', badges, icon)}
{/snippet}

{#snippet dbNode(db: DatabaseFile)}
	{@const usedBy = users.get(db.name) ?? []}
	{#snippet icon()}<PixelIcon name="database" class="size-7" />{/snippet}
	{#snippet badges()}
		{#if !db.valid}
			<Badge variant="destructive"><TriangleAlertIcon /> Not a database</Badge>
		{:else}
			<Badge variant="outline" class="border-current/30 text-current tabular-nums">{formatBytes(db.size)}</Badge>
		{/if}
	{/snippet}
	{@render card(
		'db',
		db.name,
		db.valid,
		!db.valid ? '' : usedBy.length === 0 ? 'Not used by an app' : usedBy.length === 1 ? `Used by ${usedBy[0]}` : `Used by ${usedBy.length} apps`,
		badges,
		icon
	)}
{/snippet}

{#snippet onlineNode(db: OnlineDatabase)}
	{#snippet icon()}<ProviderIcon provider={db.settings?.provider} class="size-7" />{/snippet}
	{#snippet badges()}
		{#if !db.settings}
			<Badge variant="destructive"><TriangleAlertIcon /> Can't be read</Badge>
		{:else}
			<Badge variant="outline" class="border-current/30 text-current">{providerOf(db.settings.provider)?.name ?? 'Online'}</Badge>
		{/if}
	{/snippet}
	{@render card('db', db.name, !!db.settings, db.settings ? connectionSummary(db.settings) : '', badges, icon)}
{/snippet}

<ContextMenu.Root>
	<ContextMenu.Trigger oncontextmenu={onContextMenu} onpointerdown={startPan}>
		{#snippet child({ props })}
			<div
				{...props}
				bind:this={viewport}
				{@attach wheel}
				role="application"
				aria-label="Apps and databases"
				class="paper-grid relative min-h-0 flex-1 overflow-hidden select-none"
				class:cursor-crosshair={!!connecting}
				style:--grid="{GRID * zoom}px"
				style:background-position="{pan.x}px {pan.y}px"
			>
				{#if !apps.length && !databaseNames.length}
					<Empty.Root class="h-full">
						<Empty.Header>
							<Empty.Media variant="icon"><DatabaseIcon /></Empty.Media>
							<Empty.Title>Nothing here yet</Empty.Title>
							<Empty.Description>Create a database to hold your tables, then an app with screens over it — or start an app from a template.</Empty.Description>
						</Empty.Header>
						<Empty.Content>
							<div class="flex flex-wrap justify-center gap-2">
								<Button onclick={() => newDatabase()}><PlusIcon data-icon="inline-start" /> New database</Button>
								<Button variant="outline" onclick={() => newApp()}><PlusIcon data-icon="inline-start" /> New app</Button>
								<Button variant="secondary" disabled={makingSample} onclick={trySample}>
									{#if makingSample}<Spinner data-icon="inline-start" />{:else}<SparklesIcon data-icon="inline-start" />{/if}
									Try a sample shop
								</Button>
							</div>
							<p class="text-xs text-muted-foreground">The sample has customers, products and orders to explore, and an app over them.</p>
							<FocusModeSwitch class="mt-6" />
						</Empty.Content>
					</Empty.Root>
				{:else}
					<div class="absolute top-0 left-0 origin-top-left" style:transform="translate({pan.x}px, {pan.y}px) scale({zoom})">
						<svg class="pointer-events-none absolute top-0 left-0 overflow-visible" width="1" height="1" aria-hidden="true">
							{#each edges as { link, key, i, active, path, end, endDir, chip } (key)}
								{@const color = link.missing ? 'var(--warning)' : active ? 'var(--map-accent)' : 'var(--map-line)'}
								{@const d = -endDir}
								<g data-edge class="link-in" style:color style:--i={i}>
									<path
										d={path}
										fill="none"
										stroke="currentColor"
										stroke-width={active ? 2 : 1.5}
										stroke-dasharray="4 4"
										stroke-opacity={active || link.missing ? 1 : 0.75}
										class={cn(active && 'link-flow')}
									/>
									<!-- Wide invisible stroke so the line is easy to click. -->
									<path
										role="presentation"
										data-link={key}
										d={path}
										fill="none"
										stroke="transparent"
										stroke-width="14"
										class="pointer-events-auto cursor-pointer"
										onpointerdown={(e) => e.stopPropagation()}
										onclick={() => (selection = { kind: 'link', key })}
										onpointerenter={() => (hoveredLink = key)}
										onpointerleave={() => hoveredLink === key && (hoveredLink = null)}
									/>
									{#if link.missing}
										<circle cx={end.x} cy={end.y} r="6" fill="var(--map-paper)" stroke="currentColor" stroke-width="1.5" />
										<path
											d="M {end.x - 2.5} {end.y - 2.5} L {end.x + 2.5} {end.y + 2.5} M {end.x + 2.5} {end.y - 2.5} L {end.x - 2.5} {end.y + 2.5}"
											stroke="currentColor"
											stroke-width="1.5"
										/>
									{:else}
										<!-- Arrowhead into the database, and a chip on its last stretch for the data that flows. -->
										<path
											d="M {end.x - 7 * d} {end.y - 4.5} L {end.x} {end.y} L {end.x - 7 * d} {end.y + 4.5}"
											fill="none"
											stroke="currentColor"
											stroke-width="1.5"
											stroke-linecap="square"
										/>
										{#if chip}
											<rect
												x={chip.x - CHIP / 2}
												y={chip.y - CHIP / 2}
												width={CHIP}
												height={CHIP}
												rx="3"
												fill="var(--map-chip)"
												stroke="currentColor"
												stroke-width="1"
											/>
											<PixelIcon name="rows" x={chip.x - 7} y={chip.y - 7} width={14} height={14} />
										{/if}
									{/if}
								</g>
							{/each}
							{#if connecting}
								<path d={connectorPreview} fill="none" stroke="var(--map-accent)" stroke-width="2" stroke-dasharray="4 4" />
								<rect x={connecting.pointer.x - 4} y={connecting.pointer.y - 4} width="8" height="8" rx="1" fill="var(--map-accent)" />
							{/if}
						</svg>
						{#each apps as app (app.name)}{@render appNode(app)}{/each}
						{#each databases as db (db.file_name)}{@render dbNode(db)}{/each}
						{#each online as db (db.name)}{@render onlineNode(db)}{/each}
					</div>

					<div data-toolbar class="absolute top-3 left-3 flex items-center gap-2">
						<Button size="sm" onclick={() => newDatabase()}><PlusIcon data-icon="inline-start" /> New database</Button>
						<Button size="sm" variant="outline" class="bg-background" onclick={() => newApp()}><PlusIcon data-icon="inline-start" /> New app</Button>
					</div>
					<div data-toolbar class="absolute right-3 bottom-3">
						<ButtonGroup.Root>
							<Button size="icon-sm" variant="outline" class="bg-background" aria-label="Zoom out" onclick={() => zoomTo(zoom / 1.2)}><MinusIcon /></Button>
							<Button size="sm" variant="outline" class="w-14 bg-background tabular-nums" aria-label="Fit to screen" title="Fit to screen" onclick={fit}>
								{Math.round(zoom * 100)}%
							</Button>
							<Button size="icon-sm" variant="outline" class="bg-background" aria-label="Zoom in" onclick={() => zoomTo(zoom * 1.2)}><PlusIcon /></Button>
						</ButtonGroup.Root>
					</div>
					{#if settings.current.canvasHints}
						<p class="pointer-events-none absolute bottom-3 left-4 flex items-center gap-1.5 text-xs text-muted-foreground">
							Drag a card's <span class="inline-block size-2.5 rounded-[2px] border-2 border-(--map-line)"></span> onto another to connect ·
							Double-click or Enter to open · <Shortcut keys="mod" /> + scroll to zoom · Right-click for more · <Shortcut keys="?" /> shortcuts
						</p>
					{/if}
				{/if}
			</div>
		{/snippet}
	</ContextMenu.Trigger>
	<ContextMenu.Content class="w-56">
		{@const t = menu.target}
		{#if t.kind === 'canvas'}
			<ContextMenu.Group>
				<ContextMenu.Item onclick={() => newDatabase(menu.at)}><DatabaseIcon /> New database here…</ContextMenu.Item>
				<ContextMenu.Item onclick={() => newApp(menu.at)}><LayoutDashboardIcon /> New app here…</ContextMenu.Item>
				<ContextMenu.Item onclick={openShared}><PackageOpenIcon /> Open a shared app…</ContextMenu.Item>
			</ContextMenu.Group>
			<ContextMenu.Separator />
			<ContextMenu.Group>
				<ContextMenu.Item disabled={!Object.keys(layout).length} onclick={arrange}><LayoutGridIcon /> Tidy up</ContextMenu.Item>
				<ContextMenu.Item disabled={!Object.keys(layout).length} onclick={fit}><MaximizeIcon /> Fit to screen</ContextMenu.Item>
				<ContextMenu.Item onclick={() => zoomTo(zoom * 1.2)}><PlusIcon /> Zoom in</ContextMenu.Item>
				<ContextMenu.Item onclick={() => zoomTo(zoom / 1.2)}><MinusIcon /> Zoom out</ContextMenu.Item>
			</ContextMenu.Group>
		{:else if t.kind === 'link'}
			<ContextMenu.Group>
				<ContextMenu.Item onclick={() => open('app', t.app)}><LayoutDashboardIcon /> Open “{t.app}”</ContextMenu.Item>
				<ContextMenu.Item disabled={t.missing} onclick={() => open('db', t.database)}><DatabaseIcon /> Open “{t.database}”</ContextMenu.Item>
			</ContextMenu.Group>
			<ContextMenu.Separator />
			<ContextMenu.Group>
				<ContextMenu.Item variant="destructive" onclick={() => workspace.disconnectApp(t.app, t.database)}><UnlinkIcon /> Disconnect</ContextMenu.Item>
			</ContextMenu.Group>
		{:else}
			{@const choices =
				t.node === 'app'
					? databases.filter((d) => canConnect('app', t.name, nodeKey('db', d.name))).map((d) => d.name)
					: apps.filter((a) => canConnect('db', t.name, nodeKey('app', a.name))).map((a) => a.name)}
			<ContextMenu.Group>
				{@render nodeMenu(t.node, t.name, ContextMenu.Item)}
				<ContextMenu.Sub>
					<ContextMenu.SubTrigger disabled={!choices.length}><LinkIcon /> {t.node === 'app' ? 'Connect a database' : 'Use in an app'}</ContextMenu.SubTrigger>
					<ContextMenu.SubContent class="max-h-72 w-52 overflow-y-auto">
						<ContextMenu.Group>
							{#each choices as other (other)}
								<ContextMenu.Item
									onclick={() => (t.node === 'app' ? workspace.connectApp(t.name, other) : workspace.connectApp(other, t.name))}
								>
									{#if t.node === 'app'}<DatabaseIcon />{:else}<LayoutDashboardIcon />{/if}
									<span class="truncate">{other}</span>
								</ContextMenu.Item>
							{/each}
						</ContextMenu.Group>
					</ContextMenu.SubContent>
				</ContextMenu.Sub>
			</ContextMenu.Group>
			<ContextMenu.Separator />
			<ContextMenu.Group>
				<ContextMenu.Item variant="destructive" onclick={() => remove(t.node, t.name)}>
					<TrashIcon /> {t.node === 'app' ? 'Delete app' : onlineByName.has(t.name) ? 'Remove connection' : 'Delete database'}
				</ContextMenu.Item>
			</ContextMenu.Group>
		{/if}
	</ContextMenu.Content>
</ContextMenu.Root>

<NewDatabaseDrawer bind:open={creatingDatabase} onCreated={databaseCreated} />
<NewDatabaseDrawer bind:open={editOpen} editing={editingOnline} />

<NewAppDrawer bind:open={creatingApp} onCreated={(name) => placeCreated('app', name)} />

<HomeCommandPalette
	bind:open={paletteOpen}
	{canOpen}
	onOpen={open}
	onNewDatabase={() => newDatabase()}
	onOpenShared={openShared}
	onNewApp={() => newApp()}
	onTidy={arrange}
	onFit={fit}
/>

<NameDialog
	bind:open={() => renaming !== null, (o) => !o && (renaming = null)}
	title={renaming?.kind === 'app' ? 'Rename app' : 'Rename database'}
	description={renaming?.kind === 'db' ? (onlineByName.has(renaming.name) ? 'Only the name PixelDB shows changes; the database on the server keeps its own.' : 'The database file in your folder is renamed too, and apps using it follow.') : undefined}
	initial={renaming?.name ?? ''}
	submitLabel="Rename"
	validate={(v) => (renaming?.kind === 'app' ? appNameProblem(v, renaming.name) : dbNameProblem(v, renaming?.name))}
	onSubmit={async (name) => (renaming ? rename(renaming.kind, renaming.name, name) : true)}
/>

<NameDialog
	bind:open={() => duplicating !== null, (o) => !o && (duplicating = null)}
	title={duplicating?.kind === 'app' ? 'Duplicate app' : 'Duplicate database'}
	description={duplicating?.kind === 'app'
		? `Copies the pages and widgets of “${duplicating?.name ?? ''}”. Its databases are shared, not copied.`
		: `Makes a full copy of “${duplicating?.name ?? ''}”, including all tables and records.`}
	initial={duplicating?.suggested ?? ''}
	submitLabel="Duplicate"
	validate={(v) => (duplicating?.kind === 'app' ? appNameProblem(v) : dbNameProblem(v))}
	onSubmit={async (name) => (duplicating ? duplicate(duplicating.kind, duplicating.name, name) : true)}
/>

<style>
	/* Lines fade in one after another; a lit line's dashes march from the app to the database. */
	.link-in {
		animation: link-fade 0.35s ease-out calc(var(--i) * 60ms) both;
	}
	.link-flow {
		animation: link-flow 0.6s linear infinite;
	}
	@keyframes link-fade {
		from {
			opacity: 0;
		}
	}
	@keyframes link-flow {
		to {
			stroke-dashoffset: -8;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.link-in,
		.link-flow {
			animation: none;
		}
	}
</style>
