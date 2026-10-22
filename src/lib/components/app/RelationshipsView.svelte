<script lang="ts">
	import { onDestroy, tick } from 'svelte';
	import { toast } from 'svelte-sonner';
	import LayoutGridIcon from '@lucide/svelte/icons/layout-grid';
	import LinkIcon from '@lucide/svelte/icons/link';
	import MaximizeIcon from '@lucide/svelte/icons/maximize';
	import MinusIcon from '@lucide/svelte/icons/minus';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import Rows3Icon from '@lucide/svelte/icons/rows-3';
	import Table2Icon from '@lucide/svelte/icons/table-2';
	import TrashIcon from '@lucide/svelte/icons/trash-2';
	import type { DiagramLayout, FkAction, TableDef, TableInfo } from '#lib/api/types.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as ContextMenu from '#lib/components/ui/context-menu/index.js';
	import * as Drawer from '#lib/components/ui/drawer/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import { Kbd } from '#lib/components/ui/kbd/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import {
		CARD_WIDTH,
		type Cardinality,
		type LinkDraft,
		type Point,
		type Relationship,
		autoLayout,
		cardHeight,
		describeRelationship,
		edgeRoute,
		junctionDef,
		junctionName,
		keyColumns,
		linkDesign,
		placeMissing,
		proposeLink,
		relationshipId,
		relationshipsOf,
		rowCenter,
		suggestFieldName,
		unlinkDesign,
		updateLinkDesign
	} from '#lib/logic/relationships.js';
	import { jsonClone } from '#lib/logic/values.js';
	import { databaseService } from '#lib/services/database.js';
	import { tableService } from '#lib/services/tables.js';
	import { attempt, confirm } from '#lib/state/feedback.svelte.js';
	import { session } from '#lib/state/session.svelte.js';
	import { settings } from '#lib/state/settings.svelte.js';
	import DiagramCard from './DiagramCard.svelte';
	import NameDialog from './NameDialog.svelte';
	import ConfirmHost from './ConfirmHost.svelte';
	import FieldDrawer from './FieldDrawer.svelte';
	import NewTableDrawer from './NewTableDrawer.svelte';
	import type { WizardField } from '#lib/logic/table-wizard.js';
	import RelationshipPanel, { type Selection } from './RelationshipPanel.svelte';

	const MIN_ZOOM = 0.25;
	const MAX_ZOOM = 2;

	let defs = $state<TableDef[]>([]);
	let infos = $state<Record<string, TableInfo>>({});
	let layout = $state<DiagramLayout>({});
	let loading = $state(true);
	let busy = $state(false);
	let zoom = $state(1);
	let pan = $state<Point>({ x: 0, y: 0 });
	let selection = $state<Selection>(null);
	/** The field drawer: which table and field (null for a new one). */
	let fieldDrawer = $state<{ open: boolean; table: string; field: string | null }>({ open: false, table: '', field: null });
	let renamingTable = $state<string | null>(null);
	const SCOPE = 'diagram-drawer';
	let draft = $state<LinkDraft | null>(null);
	let hoveredTable = $state<string | null>(null);
	let hoveredEdge = $state<string | null>(null);
	let connecting = $state<{ table: string; field: string; start: Point; pointer: Point } | null>(null);
	let dropTarget = $state<{ table: string; field: string | null } | null>(null);
	let viewport = $state<HTMLDivElement | null>(null);
	let addingTable = $state(false);
	/** Where a table added from the right-click menu goes; null lets it be placed automatically. */
	let newTableAt = $state<Point | null>(null);
	/** What was right-clicked, and where (in diagram coordinates). */
	type MenuTarget =
		| { kind: 'canvas' }
		| { kind: 'table'; name: string }
		| { kind: 'field'; table: string; field: string }
		| { kind: 'relationship'; id: string };
	let menu = $state<{ target: MenuTarget; at: Point }>({ target: { kind: 'canvas' }, at: { x: 0, y: 0 } });
	const menuRel = $derived.by(() => {
		const t = menu.target;
		return t.kind === 'relationship' ? rels.find((r) => r.id === t.id) : undefined;
	});

	const rels = $derived(relationshipsOf(defs));
	const byName = $derived(new Map(defs.map((d) => [d.name, d])));
	const selectedRel = $derived.by(() => {
		const s = selection;
		return s?.kind === 'relationship' ? rels.find((r) => r.id === s.id) : undefined;
	});
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
			const route = edgeRoute(
				pp,
				rowCenter(fieldIndex(parent, r.parentColumns[0] ?? '')),
				cp,
				rowCenter(fieldIndex(child, r.childColumns[0]))
			);
			const active =
				r.id === selectedRel?.id || r.id === hoveredEdge || r.parent === focusTable || r.child === focusTable;
			return [{ rel: r, route, active }];
		})
	);

	/** Fields to highlight on each card: those of the selected or hovered relationship. */
	const litFields = $derived.by(() => {
		const r = selectedRel ?? rels.find((x) => x.id === hoveredEdge);
		const out: Record<string, string[]> = {};
		if (r) {
			out[r.parent] = [...(out[r.parent] ?? []), ...r.parentColumns];
			out[r.child] = [...(out[r.child] ?? []), ...r.childColumns];
		}
		return out;
	});

	// ── Loading and saving ──

	let loadedOnce = false;
	async function load() {
		const results = await Promise.all(session.tables.map((t) => tableService.get(t.name).catch(() => null)));
		const loaded = results.filter((r): r is TableInfo => r !== null);
		infos = Object.fromEntries(loaded.map((i) => [i.definition.name, i]));
		defs = loaded.map((i) => i.definition);
		if (loadedOnce) {
			layout = placeMissing(defs, layout);
			return;
		}
		const saved = (await attempt(() => databaseService.getLayout())) ?? {};
		const known = Object.fromEntries(Object.entries(saved).filter(([name]) => infos[name]));
		layout = Object.keys(known).length ? placeMissing(defs, known) : autoLayout(defs, relationshipsOf(defs));
		loadedOnce = true;
		loading = false;
		await tick();
		fit();
	}
	load();

	let saveTimer: ReturnType<typeof setTimeout> | undefined;
	function saveLayoutSoon() {
		clearTimeout(saveTimer);
		saveTimer = setTimeout(saveLayoutNow, 400);
	}
	function saveLayoutNow() {
		clearTimeout(saveTimer);
		saveTimer = undefined;
		const snapshot: DiagramLayout = {};
		for (const d of defs) if (layout[d.name]) snapshot[d.name] = jsonClone(layout[d.name]);
		attempt(() => databaseService.saveLayout(snapshot));
	}
	onDestroy(() => {
		if (saveTimer !== undefined) saveLayoutNow();
	});

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
		if ((e.target as Element).closest('[data-card], [data-edge]')) return;
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
				else selectTable(name);
			}
		);
	}

	// ── Drag a field onto another table to link ──

	function targetAt(x: number, y: number): { table: string; field: string | null } | null {
		const el = document.elementFromPoint(x, y);
		const row = el?.closest<HTMLElement>('[data-row]');
		if (row?.dataset.table && row.dataset.field) return { table: row.dataset.table, field: row.dataset.field };
		const card = el?.closest<HTMLElement>('[data-card]');
		return card?.dataset.card ? { table: card.dataset.card, field: null } : null;
	}

	function startConnect(e: PointerEvent, table: string, field: string, side: 1 | -1) {
		if (e.button !== 0) return;
		e.stopPropagation();
		e.preventDefault();
		const def = byName.get(table);
		const p = layout[table];
		if (!def || !p) return;
		const start = { x: p.x + (side === 1 ? CARD_WIDTH : 0), y: p.y + rowCenter(fieldIndex(def, field)) };
		connecting = { table, field, start, pointer: start };
		drag(
			e,
			(_dx, _dy, ev) => {
				if (connecting) connecting.pointer = toWorld(ev.clientX, ev.clientY);
				const t = targetAt(ev.clientX, ev.clientY);
				dropTarget = t && !(t.table === table && (t.field === null || t.field === field)) ? t : null;
			},
			(_moved, ev) => {
				const t = targetAt(ev.clientX, ev.clientY);
				connecting = null;
				dropTarget = null;
				if (t && !(t.table === table && t.field === null)) linkFromDrag(table, field, t.table, t.field);
			}
		);
	}

	function linkFromDrag(fromTable: string, fromField: string, toTable: string, toField: string | null) {
		const from = byName.get(fromTable);
		const to = byName.get(toTable);
		if (!from || !to) return;
		const proposal = proposeLink(from, fromField, to, toField);
		if (typeof proposal === 'string') {
			toast.error("Can't link those", { description: proposal });
			return;
		}
		openDraft(proposal);
	}

	function openDraft(d: LinkDraft) {
		draft = d;
		selection = { kind: 'new' };
	}

	/** Opens an empty "new relationship" form, starting from `parent` if given. */
	function startLink(parent?: string) {
		const p = (parent && byName.get(parent)) || defs.find((d) => keyColumns(d).length) || defs[0];
		if (!p) return;
		const c = defs.find((d) => d.name !== p.name) ?? p;
		const parentColumn = keyColumns(p)[0] ?? '';
		openDraft({
			parent: p.name,
			parentColumn,
			child: c.name,
			childColumn: null,
			newField: suggestFieldName(p.name, parentColumn, c),
			kind: 'one_to_many',
			onUpdate: 'cascade',
			onDelete: 'restrict',
			asDropdown: true
		});
	}

	function selectTable(name: string) {
		selection = { kind: 'table', name };
	}

	function selectRelationship(id: string) {
		selection = { kind: 'relationship', id };
	}

	// ── Applying changes ──

	async function create() {
		const d = draft;
		const parent = d && byName.get(d.parent);
		const child = d && byName.get(d.child);
		if (!d || !parent || !child || busy) return;
		busy = true;
		try {
			if (d.kind === 'many_to_many') {
				let def: TableDef;
				try {
					def = junctionDef(parent, child, junctionName(parent.name, child.name, session.objects.map((o) => o.name)));
				} catch (e) {
					toast.error("Can't link those", { description: e instanceof Error ? e.message : String(e) });
					return;
				}
				const created = await attempt(() => tableService.create(def));
				if (!created) return;
				const name = created.definition.name;
				const [a, b] = [layout[parent.name], layout[child.name]];
				if (a && b) layout[name] = { x: Math.max(a.x, b.x) + CARD_WIDTH + 112, y: Math.min(a.y, b.y) };
				await session.refresh();
				await load();
				saveLayoutSoon();
				selectTable(name);
				toast.success(`Linked ${parent.name} and ${child.name}`, {
					description: `Added the table “${name}”: each of its records pairs one ${parent.name} with one ${child.name}.`
				});
				return;
			}
			const design = linkDesign(child, {
				parent,
				parentColumn: d.parentColumn,
				childColumn: d.childColumn,
				newField: d.newField,
				cardinality: d.kind,
				onUpdate: d.onUpdate,
				onDelete: d.onDelete,
				asDropdown: d.asDropdown
			});
			const result = await attempt(() => tableService.saveDesign(child.name, design, false));
			if (!result) return;
			const columns = design.definition.foreign_keys?.at(-1)?.columns ?? [];
			await load();
			selectRelationship(relationshipId(child.name, columns));
			draft = null;
			toast.success('Relationship created', {
				description: [describeRelationship({ parent: parent.name, child: child.name, cardinality: d.kind }), ...result.warnings].join(' ')
			});
		} finally {
			busy = false;
		}
	}

	async function update(rel: Relationship, change: { cardinality: Cardinality; onUpdate: FkAction; onDelete: FkAction }) {
		const child = byName.get(rel.child);
		if (!child || busy) return;
		busy = true;
		try {
			const result = await attempt(() => tableService.saveDesign(child.name, updateLinkDesign(child, rel, change), false));
			if (!result) return;
			await load();
			toast.success('Relationship updated', { description: result.warnings.join(' ') || undefined });
		} finally {
			busy = false;
		}
	}

	async function remove(rel: Relationship) {
		const child = byName.get(rel.child);
		if (!child || busy) return;
		const ok = await confirm({
			title: `Unlink ${rel.child} from ${rel.parent}?`,
			description: `Records keep the values in “${rel.childColumns.join(', ')}”, but they will no longer be checked against ${rel.parent}.`,
			action: 'Remove relationship',
			destructive: true,
			scope: SCOPE
		});
		if (!ok) return;
		busy = true;
		try {
			const result = await attempt(() => tableService.saveDesign(child.name, unlinkDesign(child, rel), false));
			if (!result) return;
			selection = null;
			await load();
			toast.success('Relationship removed');
		} finally {
			busy = false;
		}
	}

	/** Opens a field of `table` in the field drawer; `name` null starts a new one. */
	function openField(table: string, name: string | null) {
		selection = null;
		fieldDrawer = { open: true, table, field: name };
	}

	async function renameTable(name: string, newName: string): Promise<boolean> {
		if (name === newName) return true;
		if (!(await session.renameTable(name, newName))) return false;
		if (layout[name]) {
			layout[newName] = layout[name];
			delete layout[name];
		}
		await load();
		saveLayoutSoon();
		if (selection?.kind === 'table' && selection.name === name) selection = { kind: 'table', name: newName };
		return true;
	}

	async function deleteTable(name: string) {
		if (!(await session.deleteTable(name, selection ? SCOPE : undefined))) return;
		selection = null;
		delete layout[name];
		await load();
		saveLayoutSoon();
	}

	function arrange() {
		layout = autoLayout(defs, rels);
		saveLayoutNow();
		tick().then(fit);
	}

	async function addTable(name: string, fields: WizardField[]): Promise<boolean> {
		if (!(await session.createTable(name, false, fields))) return false;
		if (newTableAt) layout[name] = newTableAt;
		await load();
		selectTable(name);
		saveLayoutSoon();
		return true;
	}

	function suggestedTableName(): string {
		let n = 1;
		while (session.objects.some((o) => o.name.toLowerCase() === `table ${n}`)) n++;
		return `Table ${n}`;
	}

	function openAddTable(at: Point | null = null) {
		newTableAt = at;
		addingTable = true;
	}

	function onContextMenu(e: MouseEvent) {
		const el = e.target as Element;
		const row = el.closest<HTMLElement>('[data-row]');
		const card = el.closest<HTMLElement>('[data-card]');
		const edge = el.closest<SVGElement>('[data-rel]');
		const at = toWorld(e.clientX, e.clientY);
		if (row?.dataset.table && row.dataset.field) menu = { target: { kind: 'field', table: row.dataset.table, field: row.dataset.field }, at };
		else if (card?.dataset.card) menu = { target: { kind: 'table', name: card.dataset.card }, at };
		else if (edge?.dataset.rel) menu = { target: { kind: 'relationship', id: edge.dataset.rel }, at };
		else menu = { target: { kind: 'canvas' }, at: { x: Math.round(at.x), y: Math.round(at.y) } };
	}

	function onKey(e: KeyboardEvent) {
		const typing = (e.target as Element | null)?.closest('input, textarea, select, [contenteditable], [role="dialog"], [role="menu"], [role="listbox"]');
		if (typing) return;
		if (e.key === 'Escape') {
			selection = null;
		} else if ((e.key === 'Delete' || e.key === 'Backspace') && selectedRel) {
			e.preventDefault();
			remove(selectedRel);
		}
	}

	const connectorPreview = $derived.by(() => {
		if (!connecting) return '';
		const { start, pointer } = connecting;
		const bend = Math.max(40, Math.abs(pointer.x - start.x) / 2);
		const dir = pointer.x >= start.x ? 1 : -1;
		return `M ${start.x} ${start.y} C ${start.x + bend * dir} ${start.y}, ${pointer.x - bend * dir} ${pointer.y}, ${pointer.x} ${pointer.y}`;
	});
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
			aria-label="Relationships diagram"
			class="relative min-h-0 flex-1 overflow-hidden bg-muted/40 select-none"
			class:cursor-crosshair={!!connecting}
			style:background-image="radial-gradient(var(--border) 1px, transparent 1px)"
			style:background-size="{20 * zoom}px {20 * zoom}px"
			style:background-position="{pan.x}px {pan.y}px"
		>
			{#if loading}
				<div class="flex gap-10 p-12">
					{#each [1, 2, 3] as i (i)}<Skeleton class="h-56 w-60" />{/each}
				</div>
			{:else if !defs.length}
				<Empty.Root class="h-full">
					<Empty.Header>
						<Empty.Media variant="icon"><Table2Icon /></Empty.Media>
						<Empty.Title>No tables to link yet</Empty.Title>
						<Empty.Description>Add a couple of tables, like Customers and Orders, then connect them here.</Empty.Description>
					</Empty.Header>
					<Empty.Content>
						<Button onclick={() => openAddTable()}><PlusIcon data-icon="inline-start" /> Add table</Button>
					</Empty.Content>
				</Empty.Root>
			{:else}
				<div class="absolute top-0 left-0 origin-top-left" style:transform="translate({pan.x}px, {pan.y}px) scale({zoom})">
					<svg class="pointer-events-none absolute top-0 left-0 overflow-visible" width="1" height="1" aria-hidden="true">
						{#each edges as { rel, route, active } (rel.id)}
							<g data-edge class="text-muted-foreground" class:text-primary={active}>
								<path d={route.path} fill="none" stroke="currentColor" stroke-width={active ? 2.25 : 1.5} stroke-opacity={active ? 1 : 0.7} />
								<!-- Wide invisible stroke so the line is easy to click. -->
								<path
									role="presentation"
									data-rel={rel.id}
									d={route.path}
									fill="none"
									stroke="transparent"
									stroke-width="14"
									class="pointer-events-auto cursor-pointer"
									onpointerdown={(e) => e.stopPropagation()}
									onclick={() => selectRelationship(rel.id)}
									onpointerenter={() => (hoveredEdge = rel.id)}
									onpointerleave={() => hoveredEdge === rel.id && (hoveredEdge = null)}
								/>
								<circle cx={route.start.x} cy={route.start.y} r="3.5" fill="currentColor" />
								<circle cx={route.end.x} cy={route.end.y} r="3.5" fill="currentColor" />
								<text x={route.start.x + route.startDir * 12} y={route.start.y - 7} text-anchor="middle" class="fill-foreground text-[13px] font-medium">1</text>
								<text x={route.end.x + route.endDir * 14} y={route.end.y - 6} text-anchor="middle" class="fill-foreground font-medium {rel.cardinality === 'one_to_one' ? 'text-[13px]' : 'text-[17px]'}">
									{rel.cardinality === 'one_to_one' ? '1' : '∞'}
								</text>
							</g>
						{/each}
						{#if connecting}
							<path d={connectorPreview} fill="none" class="stroke-primary" stroke-width="2" stroke-dasharray="6 5" />
							<circle cx={connecting.pointer.x} cy={connecting.pointer.y} r="4" class="fill-primary" />
						{/if}
					</svg>
					{#each defs as def (def.name)}
						{#if layout[def.name]}
							<DiagramCard
								{def}
								position={layout[def.name]}
								rows={infos[def.name]?.row_count ?? null}
								selected={selection?.kind === 'table' && selection.name === def.name}
								highlight={litFields[def.name] ?? []}
								dropTarget={dropTarget?.table === def.name ? (dropTarget.field ?? '') : null}
								connecting={!!connecting}
								onHeaderDown={(e) => startCardDrag(e, def.name)}
								onConnectStart={(e, field, side) => startConnect(e, def.name, field, side)}
								onOpen={() => session.select(def.name)}
								onRename={() => (renamingTable = def.name)}
								onDelete={() => deleteTable(def.name)}
								onLink={() => startLink(def.name)}
								selectedField={selection?.kind === 'field' && selection.table === def.name ? selection.field : null}
								onFieldClick={(f) => openField(def.name, f)}
								onAddField={() => openField(def.name, null)}
								onHover={(inside) => (hoveredTable = inside ? def.name : hoveredTable === def.name ? null : hoveredTable)}
							/>
						{/if}
					{/each}
				</div>
				{#if settings.current.canvasHints}
					<p class="pointer-events-none absolute bottom-3 left-4 flex items-center gap-1.5 text-xs text-muted-foreground">
						Drag a field's <span class="inline-block size-2.5 rounded-full border-2 border-primary"></span> onto another table to link ·
						<Kbd>⌘</Kbd> + scroll to zoom · Right-click to add tables and more
					</p>
				{/if}
			{/if}
		</div>
		{/snippet}
		</ContextMenu.Trigger>
		<ContextMenu.Content class="w-56">
			{#if menu.target.kind === 'canvas'}
				{@const at = menu.at}
				<ContextMenu.Group>
					<ContextMenu.Item onclick={() => openAddTable(at)}><PlusIcon /> New table…</ContextMenu.Item>
					<ContextMenu.Item disabled={!defs.length} onclick={() => startLink()}><LinkIcon /> Link tables…</ContextMenu.Item>
				</ContextMenu.Group>
				<ContextMenu.Separator />
				<ContextMenu.Group>
					<ContextMenu.Item disabled={!defs.length} onclick={arrange}><LayoutGridIcon /> Tidy up</ContextMenu.Item>
					<ContextMenu.Item disabled={!defs.length} onclick={fit}><MaximizeIcon /> Fit to screen</ContextMenu.Item>
					<ContextMenu.Item disabled={!defs.length} onclick={() => zoomTo(zoom * 1.2)}><PlusIcon /> Zoom in</ContextMenu.Item>
					<ContextMenu.Item disabled={!defs.length} onclick={() => zoomTo(zoom / 1.2)}><MinusIcon /> Zoom out</ContextMenu.Item>
				</ContextMenu.Group>
			{:else if menu.target.kind === 'relationship'}
				{@const rel = menuRel}
				<ContextMenu.Group>
					<ContextMenu.Item disabled={!rel} onclick={() => rel && selectRelationship(rel.id)}><PencilIcon /> Edit relationship…</ContextMenu.Item>
				</ContextMenu.Group>
				<ContextMenu.Separator />
				<ContextMenu.Group>
					<ContextMenu.Item variant="destructive" disabled={!rel} onclick={() => rel && remove(rel)}><TrashIcon /> Remove relationship</ContextMenu.Item>
				</ContextMenu.Group>
			{:else}
				{@const t = menu.target}
				{@const table = t.kind === 'field' ? t.table : t.name}
				{#if t.kind === 'field'}
					<ContextMenu.Group>
						<ContextMenu.Item onclick={() => openField(table, t.field)}><PencilIcon /> Edit “{t.field}”…</ContextMenu.Item>
					</ContextMenu.Group>
					<ContextMenu.Separator />
				{/if}
				<ContextMenu.Group>
					<ContextMenu.Item onclick={() => session.select(table)}><Rows3Icon /> Open datasheet</ContextMenu.Item>
					<ContextMenu.Item onclick={() => openField(table, null)}><PlusIcon /> Add field…</ContextMenu.Item>
					<ContextMenu.Item onclick={() => startLink(table)}><LinkIcon /> Link to another table…</ContextMenu.Item>
					<ContextMenu.Item onclick={() => (renamingTable = table)}><PencilIcon /> Rename table…</ContextMenu.Item>
				</ContextMenu.Group>
				<ContextMenu.Separator />
				<ContextMenu.Group>
					<ContextMenu.Item variant="destructive" onclick={() => deleteTable(table)}><TrashIcon /> Delete table</ContextMenu.Item>
				</ContextMenu.Group>
			{/if}
		</ContextMenu.Content>
		</ContextMenu.Root>
</div>

<Drawer.Root direction="right" bind:open={() => selection !== null, (open) => !open && (selection = null)}>
	<Drawer.Content>
		<div class="flex min-h-0 flex-1 flex-col">
			<Drawer.Header class="sr-only">
				<Drawer.Title>{selection?.kind === 'table' ? selection.name : 'Relationship'}</Drawer.Title>
				<Drawer.Description>Details of the selected item on the diagram</Drawer.Description>
			</Drawer.Header>
			<RelationshipPanel
				{defs}
				{infos}
				{rels}
				{selection}
				bind:draft
				{busy}
				onSelect={(s) => (s?.kind === 'field' ? openField(s.table, s.field) : (selection = s))}
				onCreate={create}
				onUpdate={update}
				onDelete={remove}
				onOpen={(table) => session.select(table)}
				onStartLink={startLink}
				onRenameTable={(t) => (renamingTable = t)}
				onDeleteTable={deleteTable}
			/>
		</div>
		<ConfirmHost scope={SCOPE} />
	</Drawer.Content>
</Drawer.Root>

<FieldDrawer
	bind:open={fieldDrawer.open}
	table={fieldDrawer.table}
	field={fieldDrawer.field}
	onSaved={() => load()}
/>

<NameDialog
	bind:open={() => renamingTable !== null, (o) => !o && (renamingTable = null)}
	title="Rename table"
	initial={renamingTable ?? ''}
	submitLabel="Rename"
	validate={(v) => session.tableNameProblem(v, renamingTable ?? undefined)}
	onSubmit={(v) => (renamingTable ? renameTable(renamingTable, v) : Promise.resolve(true))}
/>

<NewTableDrawer bind:open={addingTable} initial={suggestedTableName()} onCreate={addTable} />
