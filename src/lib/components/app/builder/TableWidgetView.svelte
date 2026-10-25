<script lang="ts">
	// Services
	import { recordService } from '#lib/services/records.js';

	// Types
	import type { JsonValue, RecordPage, RecordRow, SortSpec } from '#lib/api/types.js';

	// Utils
	import { prefillFromFilters, resolveFilters, type TableWidget } from '#lib/logic/app-def.js';
	import { loadLookups, lookupLabel, type LookupOption } from '#lib/logic/lookups.js';
	import { matches, typingTarget } from '#lib/logic/shortcuts.js';
	import { displayValue, toEditorValue } from '#lib/logic/values.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import { cleanMessage } from '#lib/state/feedback.svelte.js';
	import { cn } from '#lib/utils.js';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import * as InputGroup from '#lib/components/ui/input-group/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import * as Table from '#lib/components/ui/table/index.js';
	import RecordDrawer from '../RecordDrawer.svelte';
	import { RowKeys } from '../row-keys.svelte.js';
	import ValueBadge from './ValueBadge.svelte';

	// Icons
	import {
		ArrowDown as ArrowDownIcon,
		ArrowUp as ArrowUpIcon,
		ChevronLeft as ChevronLeftIcon,
		ChevronRight as ChevronRightIcon,
		Plus as PlusIcon,
		Search as SearchIcon,
	} from '@lucide/svelte';

	// Script
	let { widget, editing }: { widget: TableWidget; editing: boolean } = $props();

	let page = $state<RecordPage | null>(null);
	let error = $state<string | null>(null);
	let offset = $state(0);
	let search = $state('');
	let searchTerm = $state('');
	let lookups = $state<Record<string, LookupOption[]>>({});
	let drawerOpen = $state(false);
	let drawerRow = $state<RecordRow | null>(null);
	/** Sort picked by clicking a column heading; undefined keeps the widget's own. */
	let sortOverride = $state<SortSpec | null | undefined>(undefined);
	const sort = $derived(sortOverride === undefined ? widget.sort : sortOverride);

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
				sort: sort ? [sort] : [],
				filters: resolveFilters(widget.filters, appSession.vars),
				search: widget.search ? searchTerm || null : null
			}
		};
	}

	const requestKey = $derived(JSON.stringify(build()));

	// Changing what is shown starts again from the first page.
	const filterKey = $derived(JSON.stringify([widget.data, resolveFilters(widget.filters, appSession.vars), searchTerm, sort]));

	// A new sort set while editing replaces whatever was clicked.
	$effect(() => {
		void widget.sort;
		sortOverride = undefined;
	});

	/** Heading clicks go A → Z, Z → A, then back to record order. */
	function sortBy(column: string) {
		if (sort?.column !== column) sortOverride = { column, descending: false };
		else if (!sort.descending) sortOverride = { column, descending: true };
		else sortOverride = null;
	}
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
				rows.clamp(res.rows.length);
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

	let searchInput = $state<HTMLInputElement | null>(null);
	let body = $state<HTMLElement | null>(null);
	const rows = new RowKeys({ open: (i) => page && clickRow(page.rows[i]) });
	const lastPage = () => !page || offset + widget.pageSize >= page.total;

	/** Keys while focus is in this table (a page can show several): / search, N new, [ ] pages. */
	function onKey(e: KeyboardEvent) {
		if (e.defaultPrevented || editing || typingTarget(e.target)) return;
		if (matches(e, '/') && widget.search) {
			e.preventDefault();
			searchInput?.focus();
		} else if (matches(e, 'n') && canAdd) {
			e.preventDefault();
			addRecord();
		} else if (matches(e, '[') && offset > 0) {
			e.preventDefault();
			offset = Math.max(0, offset - widget.pageSize);
		} else if (matches(e, ']') && !lastPage()) {
			e.preventDefault();
			offset += widget.pageSize;
		}
	}

	/** In the search box, ↓ goes to the rows and Esc clears it, then leaves it. */
	function onSearchKey(e: KeyboardEvent) {
		if (e.key === 'ArrowDown' || (e.key === 'Escape' && !search)) {
			e.preventDefault();
			rows.focusIn(body);
		} else if (e.key === 'Escape') {
			e.preventDefault();
			search = '';
		}
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

<!-- svelte-ignore a11y_no_static_element_interactions (keys from the controls inside) -->
<div class="flex h-full min-h-0 flex-col gap-2" onkeydown={onKey}>
	{#if widget.search || canAdd}
		<div class="flex items-center gap-2">
			{#if widget.search}
				<InputGroup.Root class="h-8 max-w-64">
					<InputGroup.Input bind:ref={searchInput} placeholder="Search" bind:value={search} aria-label="Search {widget.title || 'records'}" onkeydown={onSearchKey} />
					<InputGroup.Addon><SearchIcon /></InputGroup.Addon>
				</InputGroup.Root>
			{/if}
			<span class="flex-1"></span>
			{#if canAdd}
				<Button size="sm" variant="outline" disabled={editing} title="New record (N)" onclick={addRecord}><PlusIcon data-icon="inline-start" /> New</Button>
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
							{@const sorted = sort?.column === col.name ? sort : null}
							<Table.Head class={cn(numeric(col.type) && 'text-right')} aria-sort={sorted ? (sorted.descending ? 'descending' : 'ascending') : undefined}>
								{#if editing}
									{col.name}
								{:else}
									<button
										type="button"
										class={cn('inline-flex max-w-full items-center gap-1 hover:text-foreground', numeric(col.type) && 'flex-row-reverse')}
										onclick={() => sortBy(col.name)}
									>
										<span class="truncate">{col.name}</span>
										{#if sorted?.descending}<ArrowDownIcon class="size-3.5 shrink-0" />{:else if sorted}<ArrowUpIcon class="size-3.5 shrink-0" />{/if}
									</button>
								{/if}
							</Table.Head>
						{/each}
					</Table.Row>
				</Table.Header>
				<Table.Body bind:ref={body} onkeydown={rows.keydown}>
					{#each page.rows as row, r (r)}
						<Table.Row
							data-row={r}
							tabindex={clickable ? rows.tabindex(r) : undefined}
							onfocus={() => rows.focused(r)}
							data-state={isSelected(row) ? 'selected' : undefined}
							class={cn(clickable && 'cursor-pointer focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring')}
							onclick={() => clickRow(row)}
						>
							{#each shown as col (col.name)}
								<Table.Cell class={cn('max-w-64 truncate', numeric(col.type) && 'text-right tabular-nums')}>
									{#if widget.badges.includes(col.name)}
										<ValueBadge text={col.type?.type === 'yes_no' ? displayValue(row.values[col.i], col.type) : cell(row.values[col.i], col)} />
									{:else if col.type?.type === 'yes_no'}
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

	{#if page && page.total > 0 && page.total <= page.limit}
		<p class="text-xs text-muted-foreground tabular-nums">{page.total} {page.total === 1 ? 'record' : 'records'}</p>
	{:else if page && page.total > page.limit}
		<div class="flex items-center justify-end gap-2 text-xs text-muted-foreground">
			<span class="tabular-nums">{page.offset + 1}–{Math.min(page.total, page.offset + page.rows.length)} of {page.total}</span>
			<Button size="icon-sm" variant="ghost" aria-label="Previous page" title="Previous page ([)" disabled={offset === 0} onclick={() => (offset = Math.max(0, offset - widget.pageSize))}>
				<ChevronLeftIcon />
			</Button>
			<Button size="icon-sm" variant="ghost" aria-label="Next page" title="Next page (])" disabled={lastPage()} onclick={() => (offset += widget.pageSize)}>
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
		allowDelete={widget.allowDelete}
		prefill={editorPrefill}
		onSaved={() => appSession.dataRevision++}
	/>
{/if}
