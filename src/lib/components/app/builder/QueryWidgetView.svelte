<script lang="ts">
	import { untrack } from 'svelte';
	import SearchIcon from '@lucide/svelte/icons/search';
	import type { FieldType, JsonValue, QueryResult } from '#lib/api/types.js';
	import * as InputGroup from '#lib/components/ui/input-group/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import * as Table from '#lib/components/ui/table/index.js';
	import { sqlParameters, type QueryWidget } from '#lib/logic/app-def.js';
	import { displayValue } from '#lib/logic/values.js';
	import { queryService } from '#lib/services/queries.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import { cleanMessage } from '#lib/state/feedback.svelte.js';
	import { cn } from '#lib/utils.js';

	let { widget, editing }: { widget: QueryWidget; editing: boolean } = $props();

	let result = $state<QueryResult | null>(null);
	let error = $state<string | null>(null);
	let search = $state('');
	// Typing SQL in the properties panel waits for a pause before running it.
	let sql = $state(untrack(() => widget.sql));
	$effect(() => {
		const next = widget.sql;
		const t = setTimeout(() => (sql = next), 400);
		return () => clearTimeout(t);
	});

	const database = $derived(widget.source ? appSession.databaseOf(widget.source) : undefined);

	function build() {
		if (!database || !sql.trim()) return null;
		const declared = new Set((appSession.def?.variables ?? []).map((v) => v.name));
		const params = Object.fromEntries(
			sqlParameters(sql)
				.filter((n) => declared.has(n))
				.map((n) => [n, appSession.vars[n] ?? null])
		);
		return { database, sql, params };
	}

	const requestKey = $derived(JSON.stringify(build()));

	$effect(() => {
		const r = JSON.parse(requestKey) as ReturnType<typeof build>;
		void appSession.dataRevision;
		if (!r) {
			result = null;
			return;
		}
		let stale = false;
		queryService
			.run(r.sql, r.database, r.params)
			.then((res) => {
				if (stale) return;
				result = res;
				error = null;
				appSession.queryColumns[widget.id] = res.columns;
			})
			.catch((e) => !stale && (error = cleanMessage(e?.message ?? String(e))));
		return () => (stale = true);
	});

	const rows = $derived.by(() => {
		if (!result) return [];
		const term = search.trim().toLowerCase();
		const all = result.rows.map((values, i) => ({ values, i }));
		if (!term) return all;
		return all.filter((r) => r.values.some((v, c) => cell(v, result!.types[c]).toLowerCase().includes(term)));
	});

	function cell(value: JsonValue, type: FieldType | null): string {
		if (type?.type === 'yes_no') return value === 1 || value === true ? 'Yes' : value === null ? '' : 'No';
		return displayValue(value, type ?? undefined);
	}

	const numeric = (t: FieldType | null) => !!t && ['number', 'large_number', 'currency', 'auto_number'].includes(t.type);
	const clickable = $derived(!editing && widget.rowAction.do !== 'none' && widget.rowAction.do !== 'edit-record');

	function rowValues(values: JsonValue[]): Record<string, JsonValue> {
		return Object.fromEntries((result?.columns ?? []).map((c, i) => [c, values[i]]));
	}

	function isSelected(values: JsonValue[]): boolean {
		const a = widget.rowAction;
		if (a.do !== 'set-variable' || a.value.kind !== 'column' || !result) return false;
		const i = result.columns.indexOf(a.value.column);
		const current = appSession.vars[a.name];
		return i >= 0 && current !== null && current !== undefined && JSON.stringify(values[i]) === JSON.stringify(current);
	}
</script>

<div class="flex h-full min-h-0 flex-col gap-2">
	{#if widget.search}
		<div class="flex items-center gap-2">
			<InputGroup.Root class="h-8 max-w-64">
				<InputGroup.Input placeholder="Search" bind:value={search} aria-label="Search {widget.title || 'results'}" />
				<InputGroup.Addon><SearchIcon /></InputGroup.Addon>
			</InputGroup.Root>
		</div>
	{/if}

	<div class="min-h-0 flex-1 overflow-auto border">
		{#if error}
			<p class="p-3 font-mono text-xs whitespace-pre-wrap text-destructive">{error}</p>
		{:else if !database || !widget.sql.trim()}
			<p class="p-3 text-sm text-muted-foreground">Choose a database and write the query in the properties.</p>
		{:else if !result}
			<div class="flex flex-col gap-2 p-3">{#each Array(4) as _, i (i)}<Skeleton class="h-7" />{/each}</div>
		{:else}
			<Table.Root>
				<Table.Header class="sticky top-0 bg-background">
					<Table.Row>
						{#each result.columns as col, c (c)}
							<Table.Head class={cn(numeric(result.types[c]) && 'text-right')}>{col}</Table.Head>
						{/each}
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each rows as row (row.i)}
						<Table.Row
							data-state={isSelected(row.values) ? 'selected' : undefined}
							class={cn(clickable && 'cursor-pointer')}
							onclick={() => clickable && appSession.run(widget.rowAction, rowValues(row.values))}
						>
							{#each row.values as value, c (c)}
								<Table.Cell class={cn('max-w-64 truncate', numeric(result.types[c]) && 'text-right tabular-nums')}>
									{cell(value, result.types[c])}
								</Table.Cell>
							{/each}
						</Table.Row>
					{:else}
						<Table.Row>
							<Table.Cell colspan={Math.max(1, result.columns.length)} class="py-6 text-center text-muted-foreground">No results.</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		{/if}
	</div>

	{#if result}
		<div class="text-end text-xs text-muted-foreground">
			{rows.length} row{rows.length === 1 ? '' : 's'}{#if result.truncated} · first {result.rows.length} shown{/if}
		</div>
	{/if}
</div>
