<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onDestroy, tick, untrack } from 'svelte';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import DatabaseIcon from '@lucide/svelte/icons/database';
	import EllipsisVerticalIcon from '@lucide/svelte/icons/ellipsis-vertical';
	import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
	import LayoutDashboardIcon from '@lucide/svelte/icons/layout-dashboard';
	import LayoutGridIcon from '@lucide/svelte/icons/layout-grid';
	import LinkIcon from '@lucide/svelte/icons/link';
	import MaximizeIcon from '@lucide/svelte/icons/maximize';
	import MinusIcon from '@lucide/svelte/icons/minus';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import TrashIcon from '@lucide/svelte/icons/trash-2';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';
	import UnlinkIcon from '@lucide/svelte/icons/unlink';
	import type { AppFile, DatabaseFile } from '#lib/api/types.js';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as ButtonGroup from '#lib/components/ui/button-group/index.js';
	import * as ContextMenu from '#lib/components/ui/context-menu/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import { Kbd } from '#lib/components/ui/kbd/index.js';
	import { edgeRoute, type Point } from '#lib/logic/relationships.js';
	import { appHref, databaseHref } from '#lib/logic/routes.js';
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
	import { databaseService } from '#lib/services/database.js';
	import { attempt } from '#lib/state/feedback.svelte.js';
	import { workspace } from '#lib/state/workspace.svelte.js';
	import { settings } from '#lib/state/settings.svelte.js';
	import { cn } from '#lib/utils.js';
	import FocusModeSwitch from './FocusModeSwitch.svelte';
	import NameDialog from './NameDialog.svelte';
	import NewAppDialog from './NewAppDialog.svelte';

	const MIN_ZOOM = 0.25;
	const MAX_ZOOM = 2;

	const ACCENT_TEXT: Record<string, string> = {
		blue: 'text-[oklch(0.55_0.2_255)] bg-[oklch(0.55_0.2_255)]/10',
		green: 'text-[oklch(0.6_0.15_150)] bg-[oklch(0.6_0.15_150)]/10',
		amber: 'text-[oklch(0.75_0.16_75)] bg-[oklch(0.75_0.16_75)]/10',
		red: 'text-[oklch(0.6_0.22_25)] bg-[oklch(0.6_0.22_25)]/10',
		violet: 'text-[oklch(0.55_0.22_295)] bg-[oklch(0.55_0.22_295)]/10'
	};
	// The same hues, for an app's connector lines.
	const ACCENT_COLOR: Record<string, string> = {
		blue: 'oklch(0.55 0.2 255)',
		green: 'oklch(0.6 0.15 150)',
		amber: 'oklch(0.75 0.16 75)',
		red: 'oklch(0.6 0.22 25)',
		violet: 'oklch(0.55 0.22 295)'
	};

	const apps = $derived(workspace.apps);
	const databases = $derived(workspace.databases);
	const appByName = $derived(new Map(apps.map((a) => [a.name, a])));
	const links = $derived(
		workspaceLinks(
			apps,
			databases.filter((d) => d.valid).map((d) => d.name)
		)
	);
	const users = $derived(usersByDatabase(links));
	const linkKey = (l: { app: string; database: string }) => `${l.app}\u0000${l.database}`;
	const accentOf = (app: string) => ACCENT_COLOR[appByName.get(app)?.accent ?? ''] ?? 'var(--primary)';

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
		const [appNames, dbNames, ls] = [apps.map((a) => a.name), databases.map((d) => d.name), links];
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
			databases.map((d) => d.name),
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

	/** Wheel pans; pinch or Ctrl/⌘ + wheel zooms around the pointer. Not passive, so it can stop page scroll. */
	function wheel(node: HTMLDivElement) {
		const onWheel = (e: WheelEvent) => {
			e.preventDefault();
			if (e.ctrlKey || e.metaKey) {
				const rect = node.getBoundingClientRect();
				zoomTo(zoom * Math.exp(-e.deltaY * 0.01), { x: e.clientX - rect.left, y: e.clientY - rect.top });
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
		return !!file?.valid && !!databases.find((d) => d.name === db)?.valid && !file.databases.includes(db);
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

	const edges = $derived(
		links.flatMap((l, i) => {
			const a = layout[nodeKey('app', l.app)];
			if (!a) return [];
			const key = linkKey(l);
			const active = touches(l);
			if (l.missing) {
				const stub = missingStub(a);
				return [{ link: l, key, i, active, path: stub.path, start: { x: a.x + NODE_WIDTH, y: stub.end.y }, end: stub.end }];
			}
			const d = layout[nodeKey('db', l.database)];
			if (!d) return [];
			const route = edgeRoute(a, NODE_HEIGHT / 2, d, NODE_HEIGHT / 2);
			return [{ link: l, key, i, active, path: route.path, start: route.start, end: route.end }];
		})
	);

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

	function newDatabase(at: Point | null = null) {
		createAt = at;
		creatingDatabase = true;
	}
	function newApp(at: Point | null = null) {
		createAt = at;
		creatingApp = true;
	}

	/** Puts a just-created card where the menu was opened, or in the middle of the view. */
	function placeCreated(kind: NodeKind, name: string) {
		const key = nodeKey(kind, name);
		const view = viewport ? toWorld(viewport.getBoundingClientRect().left + viewport.clientWidth / 2, viewport.getBoundingClientRect().top + viewport.clientHeight / 2) : { x: 0, y: 0 };
		const at = createAt ?? { x: Math.round(view.x - NODE_WIDTH / 2), y: Math.round(view.y - NODE_HEIGHT / 2) };
		layout[key] = freeSpot(layout, at, key);
		// Databases a template created alongside go next to the app.
		layout = placeMissingNodes(layout, apps.map((a) => a.name), databases.map((d) => d.name), links);
		selection = { kind: 'node', key };
		saveLayoutNow();
	}

	async function createDatabase(name: string): Promise<boolean> {
		if (!(await workspace.create(name))) return false;
		// Creating a database opens it; the home screen keeps nothing open.
		await attempt(() => databaseService.close());
		await workspace.refresh();
		placeCreated('db', name);
		return true;
	}

	function open(kind: NodeKind, name: string) {
		if (kind === 'app') goto(appHref(resolve('/app'), name));
		else goto(databaseHref(resolve('/db'), name));
	}

	function canOpen(kind: NodeKind, name: string): boolean {
		return kind === 'app' ? !!appByName.get(name)?.valid : !!databases.find((d) => d.name === name)?.valid;
	}

	function remove(kind: NodeKind, name: string) {
		return kind === 'app' ? workspace.removeApp(name) : workspace.remove(name);
	}

	async function rename(kind: NodeKind, from: string, to: string): Promise<boolean> {
		const ok = kind === 'app' ? await workspace.renameApp(from, to) : await workspace.rename(from, to);
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

	function onContextMenu(e: MouseEvent) {
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

	function onKey(e: KeyboardEvent) {
		const typing = (e.target as Element | null)?.closest('input, textarea, select, [contenteditable], [role="dialog"], [role="menu"], [role="listbox"]');
		if (typing || !selection) return;
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
	<Item disabled={!canOpen(kind, name)} onclick={() => open(kind, name)}><ExternalLinkIcon /> Open</Item>
	<Item onclick={() => (renaming = { kind, name })}><PencilIcon /> Rename…</Item>
	<Item disabled={!canOpen(kind, name)} onclick={() => startDuplicate(kind, name)}><CopyIcon /> Duplicate…</Item>
{/snippet}

{#snippet handles(kind: NodeKind, name: string)}
	{#each [-1, 1] as const as side (side)}
		<button
			type="button"
			tabindex="-1"
			aria-label={kind === 'app' ? `Connect ${name} to a database` : `Connect ${name} to an app`}
			title={kind === 'app' ? 'Drag onto a database to connect' : 'Drag onto an app to connect'}
			class={cn(
				'absolute top-1/2 size-3 -translate-y-1/2 cursor-crosshair rounded-full border-2 border-primary bg-background opacity-0 transition-opacity group-hover/node:opacity-100 hover:scale-125',
				side === 1 ? '-end-1.5' : '-start-1.5'
			)}
			onpointerdown={(e) => startConnect(e, kind, name, side)}
			onclick={(e) => e.stopPropagation()}
		></button>
	{/each}
{/snippet}

{#snippet card(kind: NodeKind, name: string, valid: boolean, body: import('svelte').Snippet, icon: import('svelte').Snippet)}
	{@const key = nodeKey(kind, name)}
	{@const p = layout[key]}
	{#if p}
		<div
			data-node={key}
			role="group"
			aria-label={`${kind === 'app' ? 'App' : 'Database'} ${name}`}
			class={cn(
				'group/node absolute flex flex-col rounded-lg border bg-card text-card-foreground shadow-md shadow-black/5 transition-[box-shadow,border-color,opacity,translate] select-none',
				!valid && 'opacity-60',
				related.has(key) && 'border-primary/50',
				selection?.kind === 'node' && selection.key === key && 'border-primary ring-2 ring-primary/30',
				dropTarget === key && 'border-primary ring-2 ring-primary/40',
				connecting && !dropTarget && nodeKey(connecting.kind, connecting.name) !== key && !canConnect(connecting.kind, connecting.name, key) && 'opacity-40'
			)}
			style:left="{p.x}px"
			style:top="{p.y}px"
			style:width="{NODE_WIDTH}px"
			style:height="{NODE_HEIGHT}px"
			onpointerenter={() => (hoveredNode = key)}
			onpointerleave={() => hoveredNode === key && (hoveredNode = null)}
		>
			<div
				class="flex h-11 shrink-0 cursor-grab items-center gap-2 border-b ps-2 pe-1.5 active:cursor-grabbing"
				role="presentation"
				onpointerdown={(e) => startNodeDrag(e, key)}
				ondblclick={() => canOpen(kind, name) && open(kind, name)}
			>
				{@render icon()}
				<span class="truncate text-sm font-semibold" title={name}>{name}</span>
				<span class="flex-1"></span>
				<span role="presentation" onpointerdown={(e) => e.stopPropagation()}>
					<DropdownMenu.Root>
						<DropdownMenu.Trigger>
							{#snippet child({ props })}
								<Button {...props} variant="ghost" size="icon-sm" aria-label={`${name} options`}><EllipsisVerticalIcon /></Button>
							{/snippet}
						</DropdownMenu.Trigger>
						<DropdownMenu.Content align="start" side="right">
							<DropdownMenu.Group>{@render nodeMenu(kind, name, DropdownMenu.Item)}</DropdownMenu.Group>
							<DropdownMenu.Separator />
							<DropdownMenu.Group>
								<DropdownMenu.Item variant="destructive" onclick={() => remove(kind, name)}>
									<TrashIcon /> Delete {kind === 'app' ? 'app' : 'database'}
								</DropdownMenu.Item>
							</DropdownMenu.Group>
						</DropdownMenu.Content>
					</DropdownMenu.Root>
				</span>
			</div>
			<div class="flex min-h-0 flex-1 items-center justify-between gap-2 px-3 text-xs text-muted-foreground">
				{@render body()}
			</div>
			{#if valid}{@render handles(kind, name)}{/if}
		</div>
	{/if}
{/snippet}

{#snippet appNode(app: AppFile)}
	{#snippet icon()}
		<span class="flex size-7 shrink-0 items-center justify-center rounded-md {ACCENT_TEXT[app.accent] ?? 'bg-primary/10 text-primary'}">
			<LayoutDashboardIcon class="size-4" />
		</span>
	{/snippet}
	{#snippet body()}
		{#if !app.valid}
			<Badge variant="destructive"><TriangleAlertIcon /> Can't be read</Badge>
		{:else}
			<span class="truncate" title={app.description}>{app.description || `App · edited ${modifiedLabel(app.modified)}`}</span>
			{#if links.some((l) => l.app === app.name && l.missing)}
				<Badge variant="outline" class="border-warning/40 bg-warning/10 text-warning"><TriangleAlertIcon /> Missing</Badge>
			{:else if !app.databases.length}
				<Badge variant="outline">No data</Badge>
			{/if}
		{/if}
	{/snippet}
	{@render card('app', app.name, app.valid, body, icon)}
{/snippet}

{#snippet dbNode(db: DatabaseFile)}
	{@const usedBy = users.get(db.name) ?? []}
	{#snippet icon()}
		<span class="flex size-7 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground vivid:bg-tone-cyan/15 vivid:text-tone-cyan">
			<DatabaseIcon class="size-4" />
		</span>
	{/snippet}
	{#snippet body()}
		{#if !db.valid}
			<Badge variant="destructive"><TriangleAlertIcon /> Not a database file</Badge>
		{:else}
			<span class="truncate" title={usedBy.join(', ')}>
				{usedBy.length === 0 ? 'Not used by an app' : usedBy.length === 1 ? `Used by ${usedBy[0]}` : `Used by ${usedBy.length} apps`}
			</span>
			<Badge variant="secondary" class="vivid:bg-tone-cyan/12 vivid:text-tone-cyan">{formatBytes(db.size)}</Badge>
		{/if}
	{/snippet}
	{@render card('db', db.name, db.valid, body, icon)}
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
				class="relative min-h-0 flex-1 overflow-hidden bg-muted/40 select-none"
				class:cursor-crosshair={!!connecting}
				style:background-image="radial-gradient(var(--border) 1px, transparent 1px)"
				style:background-size="{20 * zoom}px {20 * zoom}px"
				style:background-position="{pan.x}px {pan.y}px"
			>
				{#if !apps.length && !databases.length}
					<Empty.Root class="h-full">
						<Empty.Header>
							<Empty.Media variant="icon"><DatabaseIcon /></Empty.Media>
							<Empty.Title>Nothing here yet</Empty.Title>
							<Empty.Description>Create a database to hold your tables, then an app with screens over it — or start an app from a template.</Empty.Description>
						</Empty.Header>
						<Empty.Content>
							<div class="flex justify-center gap-2">
								<Button onclick={() => newDatabase()}><PlusIcon data-icon="inline-start" /> New database</Button>
								<Button variant="outline" onclick={() => newApp()}><PlusIcon data-icon="inline-start" /> New app</Button>
							</div>
							<FocusModeSwitch class="mt-6" />
						</Empty.Content>
					</Empty.Root>
				{:else}
					<div class="absolute top-0 left-0 origin-top-left" style:transform="translate({pan.x}px, {pan.y}px) scale({zoom})">
						<svg class="pointer-events-none absolute top-0 left-0 overflow-visible" width="1" height="1" aria-hidden="true">
							{#each edges as { link, key, i, active, path, start, end } (key)}
								{@const color = link.missing ? 'var(--warning)' : active || !settings.current.focusMode ? accentOf(link.app) : null}
								<g data-edge class={cn(!color && 'text-muted-foreground')} style:color style:--i={i}>
									<path
										d={path}
										pathLength="1"
										fill="none"
										stroke="currentColor"
										stroke-width={active ? 2.25 : 1.5}
										stroke-opacity={active ? 1 : settings.current.focusMode ? 0.7 : 0.45}
										class="link-draw"
									/>
									{#if link.missing}
										<path d={path} fill="none" stroke="var(--muted)" stroke-width="2" stroke-dasharray="4 4" />
									{:else if active}
										<!-- Gaps that travel from the app to the database. -->
										<path d={path} fill="none" stroke="var(--background)" stroke-width="2.5" stroke-dasharray="2 10" class="link-flow" />
									{/if}
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
									<circle cx={start.x} cy={start.y} r="3.5" fill="currentColor" class="link-end" />
									{#if link.missing}
										<circle cx={end.x} cy={end.y} r="6" fill="var(--background)" stroke="currentColor" stroke-width="1.5" class="link-end" />
										<path
											d="M {end.x - 2.5} {end.y - 2.5} L {end.x + 2.5} {end.y + 2.5} M {end.x + 2.5} {end.y - 2.5} L {end.x - 2.5} {end.y + 2.5}"
											stroke="currentColor"
											stroke-width="1.5"
											class="link-end"
										/>
									{:else}
										<circle cx={end.x} cy={end.y} r="3.5" fill="currentColor" class="link-end" />
									{/if}
								</g>
							{/each}
							{#if connecting}
								<path d={connectorPreview} fill="none" class="stroke-primary" stroke-width="2" stroke-dasharray="6 5" />
								<circle cx={connecting.pointer.x} cy={connecting.pointer.y} r="4" class="fill-primary" />
							{/if}
						</svg>
						{#each apps as app (app.name)}{@render appNode(app)}{/each}
						{#each databases as db (db.file_name)}{@render dbNode(db)}{/each}
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
							Drag a card's <span class="inline-block size-2.5 rounded-full border-2 border-primary"></span> onto another to connect ·
							Double-click to open · <Kbd>⌘</Kbd> + scroll to zoom · Right-click for more
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
					<TrashIcon /> Delete {t.node === 'app' ? 'app' : 'database'}
				</ContextMenu.Item>
			</ContextMenu.Group>
		{/if}
	</ContextMenu.Content>
</ContextMenu.Root>

<NameDialog
	bind:open={creatingDatabase}
	title="New database"
	description="Give it a name you'll recognise. Double-click it on the map to add tables."
	initial={(() => {
		let n = 1;
		while (workspace.nameTaken(`Database ${n}`)) n++;
		return n === 1 && !workspace.nameTaken('My database') ? 'My database' : `Database ${n}`;
	})()}
	submitLabel="Create database"
	validate={(v) => dbNameProblem(v)}
	onSubmit={createDatabase}
/>

<NewAppDialog bind:open={creatingApp} onCreated={(name) => placeCreated('app', name)} />

<NameDialog
	bind:open={() => renaming !== null, (o) => !o && (renaming = null)}
	title={renaming?.kind === 'app' ? 'Rename app' : 'Rename database'}
	description={renaming?.kind === 'db' ? 'The database file in your folder is renamed too, and apps using it follow.' : undefined}
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
	/* Lines draw themselves in from the app, one after another, like a connection being made. */
	.link-draw {
		stroke-dasharray: 1;
		animation: link-draw 0.7s ease-out calc(var(--i) * 70ms) both;
	}
	.link-end {
		animation: link-fade 0.3s ease-out calc(var(--i) * 70ms + 0.5s) both;
	}
	.link-flow {
		animation: link-flow 0.9s linear infinite;
	}
	@keyframes link-draw {
		from {
			stroke-dashoffset: 1;
		}
		to {
			stroke-dashoffset: 0;
		}
	}
	@keyframes link-fade {
		from {
			opacity: 0;
		}
	}
	@keyframes link-flow {
		to {
			stroke-dashoffset: -12;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.link-draw,
		.link-end,
		.link-flow {
			animation: none;
		}
	}
</style>
