<script lang="ts">
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import SearchIcon from '@lucide/svelte/icons/search';
	import type { JsonValue, RecordPage, RecordRow } from '#lib/api/types.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as InputGroup from '#lib/components/ui/input-group/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import * as Table from '#lib/components/ui/table/index.js';
	import { prefillFromFilters, resolveFilters, type TableWidget } from '#lib/logic/app-def.js';
	import { loadLookups, lookupLabel, type LookupOption } from '#lib/logic/lookups.js';
	import { displayValue, toEditorValue } from '#lib/logic/values.js';
	import { recordService } from '#lib/services/records.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import { cleanMessage } from '#lib/state/feedback.svelte.js';
	import { cn } from '#lib/utils.js';
	import RecordDrawer from '../RecordDrawer.svelte';

	let { widget, editing }: { widget: TableWidget; editing: boolean } = $props();

	let page = $state<RecordPage | null>(null);
	let error = $state<string | null>(null);
	let offset = $state(0);
	let search = $state('');
	let searchTerm = $state('');
	let lookups = $state<Record<string, LookupOption[]>>({});
	let drawerOpen = $state(false);
	let drawerRow = $state<RecordRow | null>(null);

	const database = $derived(widget.data ? appSession.databaseOf(widget.data.source) : undefined);
	const schema = $derived(widget.data ? appSession.knownSchema(widget.data.source, widget.data.table) : undefined);
	const def = $derived(schema?.def ?? null);

	$effect(() => {
		if (widget.data) appSession.objectSchema(widget.data.source, widget.data.table);
	});

	$effect(() => {
		const d = def;
		const db = database;
		if (!d || !db) return;
		loadLookups(d, db).then((l) => (lookups = l));
	});

	function build() {
		const d = widget.data;
		if (!d || !database) return null;
		return {
			database,
			table: d.table,
			query: {
				offset,
				limit: widget.pageSize,
				sort: widget.sort ? [widget.sort] : [],
				filters: resolveFilters(widget.filters, appSession.vars),
				search: widget.search ? searchTerm || null : null
			}
		};
	}

	const requestKey = $derived(JSON.stringify(build()));

	// Changing what is shown starts again from the first page.
	const filterKey = $derived(JSON.stringify([widget.data, resolveFilters(widget.filters, appSession.vars), searchTerm, widget.sort]));
	$effect(() => {
		void filterKey;
		offset = 0;
	});

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

	let searchTimer: ReturnType<typeof setTimeout> | null = null;
	$effect(() => {
		const s = search;
		if (searchTimer) clearTimeout(searchTimer);
		searchTimer = setTimeout(() => (searchTerm = s.trim()), 300);
	});

	const shown = $derived.by(() => {
		if (!page) return [];
		const all = page.columns.map((name, i) => ({ name, i, type: page!.types[i] }));
		if (!widget.columns.length) return all;
		return widget.columns.flatMap((c) => all.filter((a) => a.name === c));
	});

	const editable = $derived(!!def && widget.rowAction.do === 'edit-record');
	const clickable = $derived(!editing && widget.rowAction.do !== 'none' && (widget.rowAction.do !== 'edit-record' || editable));
	const canAdd = $derived(!!def && widget.allowNew);

	function rowValues(row: RecordRow): Record<string, JsonValue> {
		return Object.fromEntries((page?.columns ?? []).map((c, i) => [c, row.values[i]]));
	}

	/** The row picked by this table, when its click sets a variable from a column. */
	function isSelected(row: RecordRow): boolean {
		const a = widget.rowAction;
		if (a.do !== 'set-variable' || a.value.kind !== 'column' || !page) return false;
		const col = a.value.kind === 'column' ? a.value.column : '';
		const i = page.columns.indexOf(col);
		const current = appSession.vars[a.name];
		return i >= 0 && current !== null && current !== undefined && JSON.stringify(row.values[i]) === JSON.stringify(current);
	}

	function clickRow(row: RecordRow) {
		if (!clickable) return;
		if (widget.rowAction.do === 'edit-record') {
			drawerRow = row;
			drawerOpen = true;
		} else {
			appSession.run(widget.rowAction, rowValues(row));
		}
	}

	function addRecord() {
		drawerRow = null;
		drawerOpen = true;
	}

	// A button's "new record" action pointed at this table.
	let seenIntent = appSession.intent.n;
	$effect(() => {
		const i = appSession.intent;
		if (i.n === seenIntent) return;
		seenIntent = i.n;
		if (i.widget === widget.id && canAdd) addRecord();
	});

	function cell(value: JsonValue, col: { name: string; type: (typeof shown)[number]['type'] }): string {
		if (col.type?.type === 'lookup') return lookupLabel(lookups[col.name], value) ?? displayValue(value);
		return displayValue(value, col.type ?? undefined);
	}

	const numeric = (t: (typeof shown)[number]['type']) =>
		!!t && ['number', 'large_number', 'currency', 'auto_number'].includes(t.type);
	const prefill = $derived(prefillFromFilters(widget.filters, appSession.vars));
	const editorPrefill = $derived(
		def
			? Object.fromEntries(
					Object.entries(prefill).flatMap(([k, v]) => {
						const f = def!.fields.find((x) => x.name === k);
						return f ? [[k, toEditorValue(v, f.field_type)]] : [];
					})
				)
			: {}
	);
</script>

<div class="flex h-full min-h-0 flex-col gap-2">
	{#if widget.search || canAdd}
		<div class="flex items-center gap-2">
			{#if widget.search}
				<InputGroup.Root class="h-8 max-w-64">
					<InputGroup.Input placeholder="Search" bind:value={search} aria-label="Search {widget.title || 'records'}" />
					<InputGroup.Addon><SearchIcon /></InputGroup.Addon>
				</InputGroup.Root>
			{/if}
			<span class="flex-1"></span>
			{#if canAdd}
				<Button size="sm" variant="outline" disabled={editing} onclick={addRecord}><PlusIcon data-icon="inline-start" /> New</Button>
			{/if}
		</div>
	{/if}

	<div class="min-h-0 flex-1 overflow-auto border">
		{#if error}
			<p class="p-3 text-sm text-destructive">{error}</p>
		{:else if !page}
			<div class="flex flex-col gap-2 p-3">{#each Array(4) as _, i (i)}<Skeleton class="h-7" />{/each}</div>
		{:else}
			<Table.Root>
				<Table.Header class="sticky top-0 bg-background">
					<Table.Row>
						{#each shown as col (col.name)}
							<Table.Head class={cn(numeric(col.type) && 'text-right')}>{col.name}</Table.Head>
						{/each}
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each page.rows as row, r (r)}
						<Table.Row
							data-state={isSelected(row) ? 'selected' : undefined}
							class={cn(clickable && 'cursor-pointer')}
							onclick={() => clickRow(row)}
						>
							{#each shown as col (col.name)}
								<Table.Cell class={cn('max-w-64 truncate', numeric(col.type) && 'text-right tabular-nums')}>
									{#if col.type?.type === 'yes_no'}
										{row.values[col.i] === 1 || row.values[col.i] === true ? 'Yes' : row.values[col.i] === null ? '' : 'No'}
									{:else}
										{cell(row.values[col.i], col)}
									{/if}
								</Table.Cell>
							{/each}
						</Table.Row>
					{:else}
						<Table.Row>
							<Table.Cell colspan={Math.max(1, shown.length)} class="py-6 text-center text-muted-foreground">No records.</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		{/if}
	</div>

	{#if page && page.total > page.limit}
		<div class="flex items-center justify-end gap-2 text-xs text-muted-foreground">
			<span>{page.offset + 1}–{Math.min(page.total, page.offset + page.rows.length)} of {page.total}</span>
			<Button size="icon-sm" variant="ghost" aria-label="Previous page" disabled={offset === 0} onclick={() => (offset = Math.max(0, offset - widget.pageSize))}>
				<ChevronLeftIcon />
			</Button>
			<Button size="icon-sm" variant="ghost" aria-label="Next page" disabled={offset + widget.pageSize >= page.total} onclick={() => (offset += widget.pageSize)}>
				<ChevronRightIcon />
			</Button>
		</div>
	{/if}
</div>

{#if def && widget.data && page}
	<RecordDrawer
		bind:open={drawerOpen}
		table={widget.data.table}
		{def}
		columns={page.columns}
		row={drawerRow}
		{lookups}
		{database}
		prefill={editorPrefill}
		onSaved={() => appSession.dataRevision++}
	/>
{/if}
