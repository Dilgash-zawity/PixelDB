<script lang="ts">
	// Services
	import { recordService } from '#lib/services/records.js';

	// Types
	import type { JsonValue, RecordPage, RecordRow } from '#lib/api/types.js';

	// Utils
	import { prefillFromFilters, resolveFilters, type BoardWidget } from '#lib/logic/app-def.js';
	import { boardLanes, laneKey, type Lane } from '#lib/logic/board.js';
	import { loadLookups, lookupLabel, type LookupOption } from '#lib/logic/lookups.js';
	import { displayValue, toEditorValue } from '#lib/logic/values.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import { attempt, cleanMessage } from '#lib/state/feedback.svelte.js';
	import { cn } from '#lib/utils.js';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import RecordDrawer from '../RecordDrawer.svelte';
	import ValueBadge from './ValueBadge.svelte';

	// Icons
	import { Ellipsis as EllipsisIcon, Plus as PlusIcon } from '@lucide/svelte';

	// Script
	let { widget, editing }: { widget: BoardWidget; editing: boolean } = $props();

	/** Cards loaded at most; a board is for a working set, not a whole archive. */
	const BOARD_LIMIT = 500;

	let page = $state<RecordPage | null>(null);
	let error = $state<string | null>(null);
	let lookups = $state<Record<string, LookupOption[]>>({});
	let drawerOpen = $state(false);
	let drawerRow = $state<RecordRow | null>(null);
	let drawerLane = $state<Lane | null>(null);
	/** The card being dragged, where the pointer is and the lane under it. */
	let drag = $state<{ row: RecordRow; x: number; y: number; dx: number; dy: number; width: number; moved: boolean; over: string | null } | null>(null);
	let suppressClick = false;

	const database = $derived(widget.data ? appSession.databaseOf(widget.data.source) : undefined);
	const schema = $derived(widget.data ? appSession.knownSchema(widget.data.source, widget.data.table) : undefined);
	const def = $derived(schema?.def ?? null);

	$effect(() => {
		if (widget.data) appSession.objectSchema(widget.data.source, widget.data.table);
	});

	$effect(() => {
		const d = def;
		const db = database;
		if (d && db) loadLookups(d, db).then((l) => (lookups = l));
	});

	function build() {
		const d = widget.data;
		if (!d || !database) return null;
		return { database, table: d.table, query: { limit: BOARD_LIMIT, filters: resolveFilters(widget.filters, appSession.vars) } };
	}

	const requestKey = $derived(JSON.stringify(build()));

	$effect(() => {
		const r = JSON.parse(requestKey) as ReturnType<typeof build>;
		void appSession.dataRevision;
		if (!r) return;
		let stale = false;
		recordService
			.list(r.table, r.query, r.database)
			.then((res) => {
				if (stale) return;
				page = res;
				error = null;
			})
			.catch((e) => !stale && (error = cleanMessage(e?.message ?? String(e))));
		return () => (stale = true);
	});

	const groupIndex = $derived(page ? page.columns.indexOf(widget.groupBy) : -1);
	const groupType = $derived(page && groupIndex >= 0 ? page.types[groupIndex] : null);
	const lanes = $derived(
		page && groupIndex >= 0
			? boardLanes(groupType, widget.lanes, page.rows.map((r) => r.values[groupIndex]), lookups[widget.groupBy])
			: []
	);
	const cards = $derived.by(() => {
		const byLane = new Map<string, RecordRow[]>(lanes.map((l) => [l.key, []]));
		for (const row of page?.rows ?? []) byLane.get(laneKey(row.values[groupIndex], groupType))?.push(row);
		return byLane;
	});

	const titleColumn = $derived(
		widget.cardTitle ||
			def?.fields.find((f) => f.field_type.type === 'short_text' && f.name !== widget.groupBy)?.name ||
			page?.columns.find((c) => c !== widget.groupBy) ||
			''
	);
	const canAdd = $derived(!!def && widget.allowNew);
	const canMove = $derived(!editing && !!def);

	function shown(row: RecordRow, column: string): string {
		if (!page) return '';
		const i = page.columns.indexOf(column);
		if (i < 0) return '';
		const t = page.types[i];
		if (t?.type === 'lookup') return lookupLabel(lookups[column], row.values[i]) ?? displayValue(row.values[i]);
		return displayValue(row.values[i], t ?? undefined);
	}

	function rowValues(row: RecordRow): Record<string, JsonValue> {
		return Object.fromEntries((page?.columns ?? []).map((c, i) => [c, row.values[i]]));
	}

	function openCard(row: RecordRow) {
		if (editing) return;
		if (widget.cardAction.do === 'edit-record') {
			if (!def) return;
			drawerRow = row;
			drawerLane = null;
			drawerOpen = true;
		} else {
			appSession.run(widget.cardAction, rowValues(row));
		}
	}

	function addCard(lane: Lane | null) {
		drawerRow = null;
		drawerLane = lane;
		drawerOpen = true;
	}

	// A button's "new record" action pointed at this board.
	let seenIntent = appSession.intent.n;
	$effect(() => {
		const i = appSession.intent;
		if (i.n === seenIntent) return;
		seenIntent = i.n;
		if (i.widget === widget.id && canAdd) addCard(null);
	});

	const prefill = $derived.by(() => {
		if (!def) return {};
		const values: Record<string, JsonValue> = { ...prefillFromFilters(widget.filters, appSession.vars) };
		if (drawerLane && drawerLane.value !== null) values[widget.groupBy] = drawerLane.value;
		return Object.fromEntries(
			Object.entries(values).flatMap(([k, v]) => {
				const f = def!.fields.find((x) => x.name === k);
				return f ? [[k, toEditorValue(v, f.field_type)]] : [];
			})
		);
	});

	// ----- Dragging cards between lanes -----

	function pointerDown(e: PointerEvent, row: RecordRow) {
		if (!canMove || e.button !== 0 || !row.key) return;
		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
		drag = { row, x: e.clientX, y: e.clientY, dx: e.clientX - rect.left, dy: e.clientY - rect.top, width: rect.width, moved: false, over: null };
	}

	function pointerMove(e: PointerEvent) {
		if (!drag) return;
		if (!drag.moved && Math.hypot(e.clientX - drag.x, e.clientY - drag.y) < 5) return;
		drag.moved = true;
		drag.x = e.clientX;
		drag.y = e.clientY;
		const lane = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>(`[data-board="${widget.id}"] [data-lane]`);
		drag.over = lane?.dataset.lane ?? null;
	}

	function pointerUp() {
		const d = drag;
		drag = null;
		if (!d?.moved) return;
		// The click that ends a drag doesn't open the card.
		suppressClick = true;
		setTimeout(() => (suppressClick = false));
		const lane = lanes.find((l) => l.key === d.over);
		if (lane && lane.key !== laneKey(d.row.values[groupIndex], groupType)) moveCard(d.row, lane);
	}

	/** Alt+← / Alt+→ moves a focused card to the column beside it, for moving cards without a mouse. */
	function cardKey(e: KeyboardEvent, row: RecordRow) {
		if (!canMove || !row.key || !e.altKey || (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight')) return;
		e.preventDefault();
		const i = lanes.findIndex((l) => l.key === laneKey(row.values[groupIndex], groupType));
		const next = lanes[i + (e.key === 'ArrowLeft' ? -1 : 1)];
		if (next) moveCard(row, next);
	}

	async function moveCard(row: RecordRow, lane: Lane) {
		const d = widget.data;
		const key = row.key;
		if (!d || !key || !database) return;
		const i = groupIndex;
		const before = row.values[i];
		// Shown in its new lane straight away; put back if saving fails.
		row.values[i] = lane.value;
		const saved = await attempt(() => recordService.update(d.table, key, { [widget.groupBy]: lane.value }, database));
		if (saved === undefined) row.values[i] = before;
		else appSession.dataRevision++;
	}
</script>

<svelte:window onpointermove={pointerMove} onpointerup={pointerUp} onpointercancel={() => (drag = null)} />

<div class="flex h-full min-h-0 flex-col gap-2" data-board={widget.id}>
	{#if error}
		<p class="text-sm text-destructive">{error}</p>
	{:else if !page}
		<div class="flex gap-3">{#each Array(3) as _, i (i)}<Skeleton class="h-40 w-60" />{/each}</div>
	{:else if groupIndex < 0}
		<p class="text-sm text-muted-foreground">Choose the field the columns come from.</p>
	{:else}
		<div class={cn('flex min-h-0 flex-1 gap-3 overflow-x-auto pb-1', drag?.moved && 'cursor-grabbing select-none')}>
			{#each lanes as lane (lane.key)}
				{@const laneCards = cards.get(lane.key) ?? []}
				<section
					class={cn(
						'flex w-64 shrink-0 flex-col rounded-lg border bg-muted/40',
						drag?.moved && drag.over === lane.key && 'border-ring bg-accent'
					)}
					data-lane={lane.key}
					aria-label={lane.label}
				>
					<header class="flex h-9 shrink-0 items-center gap-2 ps-3 pe-1">
						{#if lane.value === null}
							<span class="truncate text-sm text-muted-foreground">{lane.label}</span>
						{:else}
							<ValueBadge text={lane.label} />
						{/if}
						<span class="text-xs text-muted-foreground tabular-nums">{laneCards.length}</span>
						<span class="flex-1"></span>
						{#if canAdd}
							<Button size="icon-xs" variant="ghost" aria-label="New record in {lane.label}" disabled={editing} onclick={() => addCard(lane)}>
								<PlusIcon />
							</Button>
						{/if}
					</header>
					<div class="flex min-h-12 flex-1 flex-col gap-2 overflow-y-auto px-2 pb-2">
						{#each laneCards as row, r (r)}
							{@const movable = canMove && !!row.key && lanes.length > 1}
							<div class="group/card relative">
								<button
									type="button"
									class={cn(
										'flex w-full touch-none flex-col gap-1.5 rounded-md border bg-card p-2.5 text-start text-sm vivid:shadow-xs',
										canMove && 'cursor-grab hover:border-ring/60',
										movable && 'pe-8',
										drag?.moved && JSON.stringify(drag.row.key) === JSON.stringify(row.key) && 'opacity-40'
									)}
									title={movable ? 'Drag to another column, or Alt+← / Alt+→' : undefined}
									onpointerdown={(e) => pointerDown(e, row)}
									onkeydown={(e) => cardKey(e, row)}
									onclick={() => !suppressClick && openCard(row)}
								>
									<span class="truncate font-medium">{shown(row, titleColumn) || 'Untitled'}</span>
									{#each widget.cardFields.filter((c) => c !== titleColumn) as c (c)}
										{@const v = shown(row, c)}
										{#if v}
											<span class="flex min-w-0 items-baseline gap-1.5 text-xs">
												<span class="shrink-0 text-muted-foreground">{c}</span>
												<span class="truncate">{v}</span>
											</span>
										{/if}
									{/each}
								</button>
								{#if movable}
									{@const here = laneKey(row.values[groupIndex], groupType)}
									<DropdownMenu.Root>
										<DropdownMenu.Trigger>
											{#snippet child({ props })}
												<Button
													{...props}
													size="icon-xs"
													variant="ghost"
													class="absolute end-1 top-1.5 opacity-0 group-hover/card:opacity-100 focus-visible:opacity-100 data-[state=open]:opacity-100"
													aria-label="Move {shown(row, titleColumn) || 'card'}"
												>
													<EllipsisIcon />
												</Button>
											{/snippet}
										</DropdownMenu.Trigger>
										<DropdownMenu.Content align="end">
											<DropdownMenu.Group>
												<DropdownMenu.Label>Move to</DropdownMenu.Label>
												{#each lanes.filter((l) => l.key !== here) as l (l.key)}
													<DropdownMenu.Item onclick={() => moveCard(row, l)}>{l.label}</DropdownMenu.Item>
												{/each}
											</DropdownMenu.Group>
										</DropdownMenu.Content>
									</DropdownMenu.Root>
								{/if}
							</div>
						{:else}
							<p class="px-1 py-3 text-center text-xs text-muted-foreground">{canMove ? 'Drop cards here' : 'No records'}</p>
						{/each}
					</div>
				</section>
			{:else}
				<p class="text-sm text-muted-foreground">No records.</p>
			{/each}
		</div>
		{#if page.total > page.rows.length}
			<p class="text-xs text-muted-foreground">Showing the first {page.rows.length} of {page.total} records. Add a filter to narrow it down.</p>
		{/if}
	{/if}
</div>

{#if drag?.moved}
	<div
		class="pointer-events-none fixed z-50 rounded-md border bg-card p-2.5 text-sm font-medium shadow-lg"
		style="left: {drag.x - drag.dx}px; top: {drag.y - drag.dy}px; width: {drag.width}px;"
		aria-hidden="true"
	>
		<span class="block truncate">{shown(drag.row, titleColumn) || 'Untitled'}</span>
	</div>
{/if}

{#if def && widget.data && page}
	<RecordDrawer
		bind:open={drawerOpen}
		table={widget.data.table}
		{def}
		columns={page.columns}
		row={drawerRow}
		{lookups}
		{database}
		allowDelete={widget.allowDelete}
		{prefill}
		onSaved={() => appSession.dataRevision++}
	/>
{/if}
