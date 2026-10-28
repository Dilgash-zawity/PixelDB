<script lang="ts">
	// Services
	import { tableService } from '#lib/services/tables.js';

	// Types
	import type { DiagramLayout, ObjectSummary, TableDef } from '#lib/api/types.js';

	// Utils
	import { onDestroy, tick } from 'svelte';
	import {
		CARD_WIDTH,
		type Point,
		autoLayout,
		cardHeight,
		describeRelationship,
		describeRules,
		edgeRoute,
		placeMissing,
		relationshipsOf,
		rowCenter
	} from '#lib/logic/relationships.js';
	import { WHEEL_ZOOM_RATE } from '#lib/logic/pinch.js';
	import { nearestInDirection, typingTarget } from '#lib/logic/shortcuts.js';
	import { settings } from '#lib/state/settings.svelte.js';
	import { workspace } from '#lib/state/workspace.svelte.js';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import * as ButtonGroup from '#lib/components/ui/button-group/index.js';
	import * as ContextMenu from '#lib/components/ui/context-menu/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import DiagramCard from '../DiagramCard.svelte';
	import Shortcut from '../Shortcut.svelte';

	// Icons
	import {
		LayoutGrid as LayoutGridIcon,
		ListTree as ListTreeIcon,
		Lock as LockIcon,
		Maximize as MaximizeIcon,
		Minus as MinusIcon,
		Plus as PlusIcon,
		Rows3 as Rows3Icon,
		Table2 as Table2Icon,
		TriangleAlert as TriangleAlertIcon,
		X as XIcon
	} from '@lucide/svelte';

	// Script
	/**
	 * The tables of an online database and the links between them, read from
	 * the server's catalog. Read-only: cards can be moved (remembered in this
	 * browser) but structure is changed with the server's own tools.
	 */
	let {
		database,
		objects,
		onOpen
	}: {
		database: string;
		objects: ObjectSummary[];
		/** Shows a table's rows or structure. */
		onOpen: (table: string, view: 'data' | 'structure') => void;
	} = $props();

	const MIN_ZOOM = 0.2;
	const MAX_ZOOM = 2;
	/** Tables described at once, leaving connections free for other requests. */
	const PARALLEL = 4;

	let defs = $state<TableDef[]>([]);
	let rowCounts = $state<Record<string, number>>({});
	let failed = $state<string[]>([]);
	let layout = $state<DiagramLayout>({});
	let loading = $state(true);
	let progress = $state({ done: 0, total: 0 });
	let zoom = $state(1);
	let pan = $state<Point>({ x: 0, y: 0 });
	let selection = $state<{ kind: 'table'; name: string } | { kind: 'relationship'; id: string } | null>(null);
	let hoveredTable = $state<string | null>(null);
	let hoveredEdge = $state<string | null>(null);
	let viewport = $state<HTMLDivElement | null>(null);
	let menuTable = $state<string | null>(null);

	const rels = $derived(relationshipsOf(defs));
	const byName = $derived(new Map(defs.map((d) => [d.name, d])));
	const selectedRel = $derived.by(() => {
		const s = selection;
		return s?.kind === 'relationship' ? rels.find((r) => r.id === s.id) : undefined;
	});
	const selectedTable = $derived(selection?.kind === 'table' ? byName.get(selection.name) : undefined);
	const focusTable = $derived(selection?.kind === 'table' ? selection.name : hoveredTable);

	function fieldIndex(def: TableDef, name: string): number {
		return Math.max(0, def.fields.findIndex((f) => f.name.toLowerCase() === name.toLowerCase()));
	}

	const edges = $derived(
		rels.flatMap((r) => {
			const parent = byName.get(r.parent);
			const child = byName.get(r.child);
			const [pp, cp] = [layout[r.parent], layout[r.child]];
			if (!parent || !child || !pp || !cp) return [];
			const route = edgeRoute(pp, rowCenter(fieldIndex(parent, r.parentColumns[0] ?? '')), cp, rowCenter(fieldIndex(child, r.childColumns[0])));
			const active = r.id === selectedRel?.id || r.id === hoveredEdge || r.parent === focusTable || r.child === focusTable;
			return [{ rel: r, route, active }];
		})
	);

	const litFields = $derived.by(() => {
		const r = selectedRel ?? rels.find((x) => x.id === hoveredEdge);
		const out: Record<string, string[]> = {};
		if (r) {
			out[r.parent] = [...(out[r.parent] ?? []), ...r.parentColumns];
			out[r.child] = [...(out[r.child] ?? []), ...r.childColumns];
		}
		return out;
	});

	// ── Loading, and card positions remembered in this browser ──

	const storageKey = $derived(`pixeldb.online-diagram:${workspace.info?.path ?? ''}:${database}`);

	function savedLayout(): DiagramLayout {
		try {
			const raw = JSON.parse(localStorage.getItem(storageKey) ?? '{}');
			return raw && typeof raw === 'object' ? (raw as DiagramLayout) : {};
		} catch {
			return {};
		}
	}

	let saveTimer: ReturnType<typeof setTimeout> | undefined;
	function saveLayoutSoon() {
		clearTimeout(saveTimer);
		saveTimer = setTimeout(saveLayoutNow, 400);
	}
	function saveLayoutNow() {
		clearTimeout(saveTimer);
		saveTimer = undefined;
		try {
			localStorage.setItem(storageKey, JSON.stringify(layout));
		} catch {
			// Not remembered this time; the diagram still works.
		}
	}
	onDestroy(() => {
		if (saveTimer !== undefined) saveLayoutNow();
	});

	async function load() {
		const names = objects.filter((o) => o.kind === 'table').map((o) => o.name);
		progress = { done: 0, total: names.length };
		const found: TableDef[] = [];
		const counts: Record<string, number> = {};
		const missed: string[] = [];
		let next = 0;
		async function worker() {
			while (next < names.length) {
				const name = names[next++];
				try {
					const info = await tableService.get(name, database);
					found.push(info.definition);
					// Servers give an estimate; 0 also means "not known yet", so it isn't shown.
					if (info.row_count > 0) counts[info.definition.name] = info.row_count;
				} catch {
					missed.push(name);
				}
				progress.done++;
			}
		}
		await Promise.all(Array.from({ length: Math.min(PARALLEL, names.length) }, worker));
		// In the listing's order, whichever request finished first.
		const order = new Map(names.map((n, i) => [n, i]));
		defs = found.sort((a, b) => (order.get(a.name) ?? 0) - (order.get(b.name) ?? 0));
		rowCounts = counts;
		failed = missed;
		const saved = savedLayout();
		const known = Object.fromEntries(Object.entries(saved).filter(([n, p]) => byName.has(n) && typeof p?.x === 'number' && typeof p?.y === 'number'));
		layout = Object.keys(known).length ? placeMissing(defs, known) : autoLayout(defs, relationshipsOf(defs));
		loading = false;
		await tick();
		fit();
	}
	load();

	// ── Viewport: pan, zoom, fit ──

	function zoomTo(next: number, at?: Point) {
		next = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, next));
		const c = at ?? { x: (viewport?.clientWidth ?? 0) / 2, y: (viewport?.clientHeight ?? 0) / 2 };
		pan = { x: c.x - ((c.x - pan.x) / zoom) * next, y: c.y - ((c.y - pan.y) / zoom) * next };
		zoom = next;
	}

	function fit() {
		if (!viewport || !defs.length) return;
		let [minX, minY, maxX, maxY] = [Infinity, Infinity, -Infinity, -Infinity];
		for (const d of defs) {
			const p = layout[d.name];
			if (!p) continue;
			minX = Math.min(minX, p.x);
			minY = Math.min(minY, p.y);
			maxX = Math.max(maxX, p.x + CARD_WIDTH + 40);
			maxY = Math.max(maxY, p.y + cardHeight(d));
		}
		const { clientWidth: w, clientHeight: h } = viewport;
		const z = Math.min(1.25, Math.max(MIN_ZOOM, Math.min((w - 64) / (maxX - minX), (h - 64) / (maxY - minY))));
		zoom = z;
		pan = { x: (w - (maxX - minX) * z) / 2 - minX * z, y: (h - (maxY - minY) * z) / 2 - minY * z };
	}

	/** Pans just enough to bring table `name`'s header into view. */
	function reveal(name: string) {
		const p = layout[name];
		if (!viewport || !p) return;
		viewport.scrollTop = viewport.scrollLeft = 0;
		const m = 32;
		const { clientWidth: w, clientHeight: h } = viewport;
		const [left, top] = [p.x * zoom + pan.x, p.y * zoom + pan.y];
		const [right, bottom] = [left + CARD_WIDTH * zoom, top + 120 * zoom];
		const dx = left < m ? m - left : right > w - m ? w - m - right : 0;
		const dy = top < m ? m - top : bottom > h - m ? h - m - bottom : 0;
		if (dx || dy) pan = { x: pan.x + dx, y: pan.y + dy };
	}

	function arrange() {
		layout = autoLayout(defs, rels);
		saveLayoutNow();
		tick().then(fit);
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
	function drag(e: PointerEvent, onMove: (dx: number, dy: number) => void, onEnd: (moved: boolean) => void) {
		const [sx, sy] = [e.clientX, e.clientY];
		let moved = false;
		const move = (ev: PointerEvent) => {
			if (Math.abs(ev.clientX - sx) + Math.abs(ev.clientY - sy) > 3) moved = true;
			if (moved) onMove(ev.clientX - sx, ev.clientY - sy);
		};
		const up = () => {
			window.removeEventListener('pointermove', move);
			window.removeEventListener('pointerup', up);
			onEnd(moved);
		};
		window.addEventListener('pointermove', move);
		window.addEventListener('pointerup', up);
	}

	function startPan(e: PointerEvent) {
		if (e.button !== 0 && e.button !== 1) return;
		if ((e.target as Element).closest('[data-card], [data-edge], [data-toolbar]')) return;
		const origin = { ...pan };
		drag(
			e,
			(dx, dy) => (pan = { x: origin.x + dx, y: origin.y + dy }),
			(moved) => {
				if (!moved) selection = null;
			}
		);
	}

	function startCardDrag(e: PointerEvent, name: string) {
		if (e.button !== 0) return;
		const origin = { ...layout[name] };
		drag(
			e,
			(dx, dy) => (layout[name] = { x: Math.round(origin.x + dx / zoom), y: Math.round(origin.y + dy / zoom) }),
			(moved) => {
				if (moved) saveLayoutSoon();
				else selection = { kind: 'table', name };
			}
		);
	}

	// ── Keyboard: Tab or arrows move between tables, Enter opens one ──

	const ARROWS: Record<string, 'up' | 'down' | 'left' | 'right'> = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };

	function focusTableHeader(name: string) {
		viewport?.querySelector<HTMLElement>(`[data-card-header="${CSS.escape(name)}"]`)?.focus({ preventScroll: true });
	}

	function onHeaderKey(e: KeyboardEvent, name: string) {
		const direction = ARROWS[e.key];
		if (!direction || e.altKey || e.metaKey || e.ctrlKey) return;
		const from = layout[name];
		const next = from && nearestInDirection(from, direction, defs.filter((d) => d.name !== name && layout[d.name]).map((d) => ({ key: d.name, ...layout[d.name] })));
		if (next) {
			e.preventDefault();
			focusTableHeader(next);
		}
	}

	function onKey(e: KeyboardEvent) {
		if (e.defaultPrevented || typingTarget(e.target) || (e.target as Element | null)?.closest('[role="dialog"], [role="alertdialog"]')) return;
		if (e.key === 'Escape' && selection) {
			selection = null;
		} else if (ARROWS[e.key] && !selection && !e.altKey && !e.metaKey && !e.ctrlKey && (e.target === document.body || e.target === viewport)) {
			// Nothing focused yet: start from the table nearest the top left.
			const first = defs.filter((d) => layout[d.name]).sort((a, b) => layout[a.name].x + layout[a.name].y - (layout[b.name].x + layout[b.name].y))[0];
			if (first) {
				e.preventDefault();
				focusTableHeader(first.name);
			}
		}
	}

	function onContextMenu(e: MouseEvent) {
		menuTable = (e.target as Element).closest<HTMLElement>('[data-card]')?.dataset.card ?? null;
	}
</script>

<svelte:window onkeydown={onKey} />

<div class="flex h-full min-h-0 flex-col">
	<ContextMenu.Root>
		<ContextMenu.Trigger oncontextmenu={onContextMenu} onpointerdown={startPan} disabled={loading}>
			{#snippet child({ props })}
				<div
					{...props}
					bind:this={viewport}
					{@attach wheel}
					role="application"
					aria-label="Diagram of {database}"
					class="paper-grid relative min-h-0 flex-1 overflow-hidden select-none"
					style:--grid="{40 * zoom}px"
					style:background-position="{pan.x}px {pan.y}px"
				>
					{#if loading}
						<div class="flex h-full items-center justify-center">
							<div class="flex items-center gap-3 rounded-lg border bg-card px-4 py-3 text-sm" role="status">
								<Spinner />
								<span>Reading tables from the server… <span class="text-muted-foreground tabular-nums">{progress.done} of {progress.total}</span></span>
							</div>
						</div>
					{:else if !defs.length}
						<Empty.Root class="h-full">
							<Empty.Header>
								<Empty.Media variant="icon"><Table2Icon /></Empty.Media>
								<Empty.Title>No tables to show</Empty.Title>
								<Empty.Description>
									{failed.length ? `None of the ${failed.length} tables could be read with this user's permissions.` : "The server has no tables this user may read. Tables are created with the server's own tools."}
								</Empty.Description>
							</Empty.Header>
						</Empty.Root>
					{:else}
						<div class="absolute top-0 left-0 origin-top-left" style:transform="translate({pan.x}px, {pan.y}px) scale({zoom})">
							<svg class="pointer-events-none absolute top-0 left-0 overflow-visible" width="1" height="1" aria-hidden="true">
								{#each edges as { rel, route, active } (rel.id)}
									<g data-edge style:color={active ? 'var(--map-accent)' : 'var(--map-line)'}>
										<path d={route.path} fill="none" stroke="currentColor" stroke-width={active ? 2 : 1.5} stroke-dasharray="4 4" stroke-opacity={active ? 1 : 0.75} class={active ? 'rel-flow' : ''} />
										<!-- Wide invisible stroke so the line is easy to click. -->
										<path
											role="presentation"
											d={route.path}
											fill="none"
											stroke="transparent"
											stroke-width="14"
											class="pointer-events-auto cursor-pointer"
											onpointerdown={(e) => e.stopPropagation()}
											onclick={() => (selection = { kind: 'relationship', id: rel.id })}
											onpointerenter={() => (hoveredEdge = rel.id)}
											onpointerleave={() => hoveredEdge === rel.id && (hoveredEdge = null)}
										/>
										<rect x={route.start.x - 3} y={route.start.y - 3} width="6" height="6" fill="currentColor" />
										<path
											d="M {route.end.x + route.endDir * 7} {route.end.y - 4.5} L {route.end.x} {route.end.y} L {route.end.x + route.endDir * 7} {route.end.y + 4.5}"
											fill="none"
											stroke="currentColor"
											stroke-width="1.5"
											stroke-linecap="square"
										/>
										<text x={route.start.x + route.startDir * 12} y={route.start.y - 7} text-anchor="middle" class="fill-foreground text-[13px] font-medium">1</text>
										<text x={route.end.x + route.endDir * 14} y={route.end.y - 6} text-anchor="middle" class="fill-foreground font-medium {rel.cardinality === 'one_to_one' ? 'text-[13px]' : 'text-[17px]'}">
											{rel.cardinality === 'one_to_one' ? '1' : '∞'}
										</text>
									</g>
								{/each}
							</svg>
							{#each defs as def (def.name)}
								{#if layout[def.name]}
									<DiagramCard
										{def}
										readonly
										position={layout[def.name]}
										rows={rowCounts[def.name] ?? null}
										selected={selection?.kind === 'table' && selection.name === def.name}
										highlight={litFields[def.name] ?? []}
										onHeaderDown={(e) => startCardDrag(e, def.name)}
										onHeaderFocus={() => {
											selection = { kind: 'table', name: def.name };
											reveal(def.name);
										}}
										onHeaderKey={(e) => onHeaderKey(e, def.name)}
										onOpen={() => onOpen(def.name, 'data')}
										onStructure={() => onOpen(def.name, 'structure')}
										onHover={(inside) => (hoveredTable = inside ? def.name : hoveredTable === def.name ? null : hoveredTable)}
									/>
								{/if}
							{/each}
						</div>

						{#if selectedTable || selectedRel || failed.length}
							<div data-toolbar class="absolute top-3 left-3 flex w-80 max-w-[calc(100%-1.5rem)] flex-col gap-2">
								{#if failed.length}
									<p class="flex items-start gap-2 rounded-lg border border-warning/40 bg-warning/10 p-2.5 text-xs text-warning">
										<TriangleAlertIcon class="mt-px size-3.5 shrink-0" />
										<span>{failed.length === 1 ? `“${failed[0]}” couldn't be read` : `${failed.length} tables couldn't be read`} with this user's permissions, so {failed.length === 1 ? "it isn't" : "they aren't"} shown.</span>
									</p>
								{/if}
								{#if selectedTable}
									{@const links = rels.filter((r) => r.parent === selectedTable.name || r.child === selectedTable.name).length}
									<div class="flex flex-col gap-2 rounded-lg border bg-card p-3 text-sm">
										<div class="flex items-center gap-2">
											<span class="truncate font-medium">{selectedTable.name}</span>
											<span class="flex-1"></span>
											<Button size="icon-xs" variant="ghost" aria-label="Close" onclick={() => (selection = null)}><XIcon /></Button>
										</div>
										<p class="text-xs text-muted-foreground">
											{selectedTable.fields.length} {selectedTable.fields.length === 1 ? 'field' : 'fields'} · {links} {links === 1 ? 'link' : 'links'}{rowCounts[selectedTable.name] ? ` · about ${rowCounts[selectedTable.name].toLocaleString()} rows` : ''}
										</p>
										<div class="flex gap-2">
											<Button size="sm" variant="outline" onclick={() => onOpen(selectedTable.name, 'data')}><Rows3Icon data-icon="inline-start" /> Open data</Button>
											<Button size="sm" variant="outline" onclick={() => onOpen(selectedTable.name, 'structure')}><ListTreeIcon data-icon="inline-start" /> Structure</Button>
										</div>
									</div>
								{:else if selectedRel}
									<div class="flex flex-col gap-1.5 rounded-lg border bg-card p-3 text-sm">
										<div class="flex items-center gap-2">
											<span class="truncate font-medium">{selectedRel.child} → {selectedRel.parent}</span>
											<span class="flex-1"></span>
											<Button size="icon-xs" variant="ghost" aria-label="Close" onclick={() => (selection = null)}><XIcon /></Button>
										</div>
										<p class="font-mono text-xs text-muted-foreground">
											{selectedRel.child}.{selectedRel.childColumns.join(', ')} → {selectedRel.parent}.{selectedRel.parentColumns.join(', ')}
										</p>
										<p class="text-xs">{describeRelationship(selectedRel)}</p>
										{#each describeRules(selectedRel) as line (line)}<p class="text-xs text-muted-foreground">{line}</p>{/each}
									</div>
								{/if}
							</div>
						{/if}

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
								<LockIcon class="size-3" /> Read-only: change tables with the server's own tools · Double-click or Enter to open ·
								<Shortcut keys="mod" /> + scroll to zoom · <Shortcut keys="?" /> shortcuts
							</p>
						{/if}
					{/if}
				</div>
			{/snippet}
		</ContextMenu.Trigger>
		<ContextMenu.Content class="w-56">
			{#if menuTable}
				{@const t = menuTable}
				<ContextMenu.Group>
					<ContextMenu.Item onclick={() => onOpen(t, 'data')}><Rows3Icon /> Open data</ContextMenu.Item>
					<ContextMenu.Item onclick={() => onOpen(t, 'structure')}><ListTreeIcon /> Show structure</ContextMenu.Item>
				</ContextMenu.Group>
				<ContextMenu.Separator />
			{/if}
			<ContextMenu.Group>
				<ContextMenu.Item disabled={!defs.length} onclick={arrange}><LayoutGridIcon /> Tidy up</ContextMenu.Item>
				<ContextMenu.Item disabled={!defs.length} onclick={fit}><MaximizeIcon /> Fit to screen</ContextMenu.Item>
				<ContextMenu.Item disabled={!defs.length} onclick={() => zoomTo(zoom * 1.2)}><PlusIcon /> Zoom in</ContextMenu.Item>
				<ContextMenu.Item disabled={!defs.length} onclick={() => zoomTo(zoom / 1.2)}><MinusIcon /> Zoom out</ContextMenu.Item>
			</ContextMenu.Group>
		</ContextMenu.Content>
	</ContextMenu.Root>
</div>

<style>
	/* A lit relationship's dashes march from the key to the linking field. */
	.rel-flow {
		animation: rel-flow 0.6s linear infinite;
	}
	@keyframes rel-flow {
		to {
			stroke-dashoffset: -8;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.rel-flow {
			animation: none;
		}
	}
</style>
