<script lang="ts">
	import type { AggregateResult } from '#lib/api/types.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { resolveFilters, usableSeries, type StatWidget } from '#lib/logic/app-def.js';
	import { formatTotal, seriesLabel } from '#lib/logic/chart-data.js';
	import { cleanMessage } from '#lib/state/feedback.svelte.js';
	import { recordService } from '#lib/services/records.js';
	import { appSession } from '#lib/state/app.svelte.js';

	let { widget }: { widget: StatWidget } = $props();

	let result = $state<AggregateResult | null>(null);
	let error = $state<string | null>(null);

	function build() {
		const d = widget.data;
		const database = d ? appSession.databaseOf(d.source) : undefined;
		if (!d || !database) return null;
		return {
			database,
			table: d.table,
			query: { series: [usableSeries(widget.series)], filters: resolveFilters(widget.filters, appSession.vars) }
		};
	}

	// Compared as text, so changing an unrelated variable does not reload.
	const requestKey = $derived(JSON.stringify(build()));

	$effect(() => {
		const r = JSON.parse(requestKey) as ReturnType<typeof build>;
		void appSession.dataRevision;
		if (!r) return;
		let stale = false;
		recordService
			.aggregate(r.table, r.query, r.database)
			.then((res) => {
				if (stale) return;
				result = res;
				error = null;
			})
			.catch((e) => !stale && (error = cleanMessage(e?.message ?? String(e))));
		return () => (stale = true);
	});
</script>

<div class="flex h-full flex-col justify-center gap-1">
	{#if error}
		<p class="text-sm text-destructive">{error}</p>
	{:else if !result}
		<Skeleton class="h-9 w-32" />
	{:else}
		<span class="truncate text-3xl font-semibold tracking-tight tabular-nums vivid:text-(--app-accent)">
			{formatTotal(result.rows[0]?.[0] ?? null, result.types[0] ?? null)}
		</span>
		<span class="truncate text-sm text-muted-foreground">{widget.caption || widget.title || seriesLabel(widget.series)}</span>
	{/if}
</div>
