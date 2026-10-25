<script lang="ts">
	// Services
	import { onlineService } from '#lib/services/online.js';
	import { recordService } from '#lib/services/records.js';
	import { tableService } from '#lib/services/tables.js';

	// Types
	import type { FieldType, JsonValue, OnlineTable, RecordKey, RecordPage, RecordRow, SortSpec, TableInfo } from '#lib/api/types.js';

	// Utils
	import { untrack } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { loadLookups, lookupLabel, type LookupOption } from '#lib/logic/lookups.js';
	import { displayValue } from '#lib/logic/values.js';
	import { attempt, cleanMessage, confirm } from '#lib/state/feedback.svelte.js';
	import { cn } from '#lib/utils.js';

	// Components
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Checkbox } from '#lib/components/ui/checkbox/index.js';
	import * as InputGroup from '#lib/components/ui/input-group/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import * as Table from '#lib/components/ui/table/index.js';
	import RecordDrawer from '../RecordDrawer.svelte';

	// Icons
	import {
		ArrowDown as ArrowDownIcon,
		ArrowUp as ArrowUpIcon,
		ChevronLeft as ChevronLeftIcon,
		ChevronRight as ChevronRightIcon,
		Lock as LockIcon,
		Plus as PlusIcon,
		RefreshCw as RefreshIcon,
		Search as SearchIcon,
		Trash2 as TrashIcon
	} from '@lucide/svelte';

	// Script
	/**
	 * Rows of one table or view of an online database, a page at a time,
	 * sorted on the server. Tables with a primary key can be edited.
	 * Remount (`{#key}`) to show another table.
	 */
	let { database, table }: { database: string; table: string } = $props();

	const PAGE_SIZE = 50;

	let described = $state<OnlineTable | null>(null);
	let info = $state<TableInfo | null>(null);
	let lookups = $state<Record<string, LookupOption[]>>({});
	let page = $state<RecordPage | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let offset = $state(0);
	let sort = $state<SortSpec | null>(null);
	let search = $state('');
	let term = $state('');
	let revision = $state(0);
	/** Keys of the selected rows, as JSON. */
	let selected = $state<string[]>([]);
	let drawerOpen = $state(false);
	let drawerRow = $state<RecordRow | null>(null);

	const editable = $derived(!!described?.editable);

	$effect(() => {
		untrack(async () => {
			const [d, i] = await Promise.all([
				attempt(() => onlineService.describe(database, table)),
				attempt(() => tableService.get(table, database))
			]);
			described = d ?? null;
			info = i ?? null;
			if (i) lookups = await loadLookups(i.definition, database);
		});
	});

	// Search as you type, once typing pauses.
	$effect(() => {
		const s = search.trim();
		const timer = setTimeout(() => {
			if (s !== term) {
				term = s;
				offset = 0;
			}
		}, 300);
		return () => clearTimeout(timer);
	});

	const request = $derived(JSON.stringify({ offset, sort, term, revision }));
	$effect(() => {
		const r = request;
		untrack(() => load(r));
	});

	async function load(r: string) {
		loading = true;
		try {
			const p = await recordService.list(table, { offset, limit: PAGE_SIZE, sort: sort ? [sort] : [], filters: [], search: term || null }, database);
			if (r !== request) return;
			page = p;
			error = null;
			selected = [];
		} catch (e) {
			if (r === request) error = cleanMessage(e instanceof Error ? e.message : String(e));
		} finally {
			if (r === request) loading = false;
		}
	}

	function sortBy(column: string) {
		sort = sort?.column !== column ? { column, descending: false } : !sort.descending ? { column, descending: true } : null;
		offset = 0;
	}

	const keyOf = (row: RecordRow) => (row.key ? JSON.stringify(row.key) : '');

	function toggle(row: RecordRow, on: boolean) {
		const k = keyOf(row);
		selected = on ? [...selected, k] : selected.filter((x) => x !== k);
	}

	function openRow(row: RecordRow | null) {
		if (!editable || !info) return;
		drawerRow = row;
		drawerOpen = true;
	}

	async function deleteSelected() {
		const n = selected.length;
		const ok = await confirm({
			title: `Delete ${n} ${n === 1 ? 'record' : 'records'}?`,
			description: `They are removed from ${database} on the server, for everyone using it. This can't be undone.`,
			action: 'Delete',
			destructive: true
		});
		if (!ok) return;
		const keys = selected.map((k) => JSON.parse(k) as RecordKey);
		const done = await attempt(() => recordService.delete(table, keys, true, database));
		if (done === undefined) return;
		toast.success(`Deleted ${done} ${done === 1 ? 'record' : 'records'}`);
		revision++;
	}

	function cell(value: JsonValue, name: string, t: FieldType | null): string {
		if (t?.type === 'lookup') return lookupLabel(lookups[name], value) ?? displayValue(value);
		return displayValue(value, t ?? undefined);
	}

	const numeric = (t: FieldType | null) => !!t && ['number', 'large_number', 'currency', 'auto_number'].includes(t.type);
	const allSelected = $derived(!!page?.rows.length && page.rows.every((r) => selected.includes(keyOf(r))));
</script>

<div class="flex h-full min-h-0 flex-col">
	<div class="flex shrink-0 items-center gap-2 border-b px-3 py-2">
		<InputGroup.Root class="h-8 max-w-72">
			<InputGroup.Input placeholder="Search" bind:value={search} aria-label="Search {table}" />
			<InputGroup.Addon><SearchIcon /></InputGroup.Addon>
		</InputGroup.Root>
		{#if described && !described.editable}
			<Badge variant="outline" class="border-warning/40 bg-warning/10 text-warning">
				<LockIcon /> {described.kind === 'view' ? 'Read-only view' : 'Read-only: no primary key'}
			</Badge>
		{/if}
		<span class="flex-1"></span>
		{#if selected.length}
			<Button size="sm" variant="destructive" onclick={deleteSelected}><TrashIcon data-icon="inline-start" /> Delete {selected.length}</Button>
		{/if}
		<Button size="icon-sm" variant="ghost" aria-label="Reload rows" disabled={loading} onclick={() => revision++}><RefreshIcon /></Button>
		{#if editable && info}
			<Button size="sm" onclick={() => openRow(null)}><PlusIcon data-icon="inline-start" /> New record</Button>
		{/if}
	</div>

	<div class="min-h-0 flex-1 overflow-auto">
		{#if error}
			<p class="p-4 text-sm text-destructive">{error}</p>
		{:else if !page}
			<div class="flex flex-col gap-2 p-4">{#each Array(6) as _, i (i)}<Skeleton class="h-8" />{/each}</div>
		{:else}
			<Table.Root class={cn(loading && 'opacity-60')}>
				<Table.Header class="sticky top-0 bg-background">
					<Table.Row>
						{#if editable}
							<Table.Head class="w-10">
								<Checkbox
									checked={allSelected}
									disabled={!page.rows.length}
									onCheckedChange={(v) => (selected = v ? page!.rows.map(keyOf) : [])}
									aria-label="Select all rows on this page"
								/>
							</Table.Head>
						{/if}
						{#each page.columns as col, i (col)}
							{@const t = page.types[i]}
							{@const sorted = sort?.column === col ? sort : null}
							<Table.Head class={cn(numeric(t) && 'text-right')} aria-sort={sorted ? (sorted.descending ? 'descending' : 'ascending') : undefined}>
								<button
									type="button"
									class={cn('inline-flex max-w-full items-center gap-1 hover:text-foreground', numeric(t) && 'flex-row-reverse')}
									onclick={() => sortBy(col)}
								>
									<span class="truncate">{col}</span>
									{#if sorted?.descending}<ArrowDownIcon class="size-3.5 shrink-0" />{:else if sorted}<ArrowUpIcon class="size-3.5 shrink-0" />{/if}
								</button>
							</Table.Head>
						{/each}
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each page.rows as row, r (r)}
						{@const k = keyOf(row)}
						<Table.Row
							data-state={selected.includes(k) ? 'selected' : undefined}
							class={cn(editable && info && 'cursor-pointer')}
							onclick={() => openRow(row)}
						>
							{#if editable}
								<Table.Cell class="w-10" onclick={(e: MouseEvent) => e.stopPropagation()}>
									<Checkbox checked={selected.includes(k)} onCheckedChange={(v) => toggle(row, !!v)} aria-label="Select row" />
								</Table.Cell>
							{/if}
							{#each page.columns as col, i (col)}
								{@const t = page.types[i]}
								<Table.Cell class={cn('max-w-72 truncate', numeric(t) && 'text-right tabular-nums')}>{cell(row.values[i], col, t)}</Table.Cell>
							{/each}
						</Table.Row>
					{:else}
						<Table.Row>
							<Table.Cell colspan={page.columns.length + (editable ? 1 : 0)} class="py-8 text-center text-muted-foreground">
								{term ? 'No records match.' : 'No records yet.'}
							</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		{/if}
	</div>

	{#if page}
		<div class="flex shrink-0 items-center justify-end gap-2 border-t px-3 py-1.5 text-xs text-muted-foreground">
			{#if page.total > 0}
				<span class="tabular-nums">{page.offset + 1}–{page.offset + page.rows.length} of {page.total.toLocaleString()}</span>
			{:else}
				<span>0 records</span>
			{/if}
			<Button size="icon-sm" variant="ghost" aria-label="Previous page" disabled={offset === 0} onclick={() => (offset = Math.max(0, offset - PAGE_SIZE))}>
				<ChevronLeftIcon />
			</Button>
			<Button size="icon-sm" variant="ghost" aria-label="Next page" disabled={offset + PAGE_SIZE >= page.total} onclick={() => (offset += PAGE_SIZE)}>
				<ChevronRightIcon />
			</Button>
		</div>
	{/if}
</div>

{#if info && page}
	<RecordDrawer bind:open={drawerOpen} {table} def={info.definition} columns={page.columns} row={drawerRow} {lookups} {database} onSaved={() => revision++} />
{/if}
