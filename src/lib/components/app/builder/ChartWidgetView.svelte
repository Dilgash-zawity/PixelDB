<script lang="ts">
	import { AreaChart, BarChart, LineChart, PieChart } from 'layerchart';
	import ChartColumnIcon from '@lucide/svelte/icons/chart-column';
	import * as Chart from '#lib/components/ui/chart/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { resolveFilters, usableSeries, type ChartWidget } from '#lib/logic/app-def.js';
	import { toChartData, type ChartData } from '#lib/logic/chart-data.js';
	import { recordService } from '#lib/services/records.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import { cleanMessage } from '#lib/state/feedback.svelte.js';

	let { widget }: { widget: ChartWidget } = $props();

	let data = $state<ChartData | null>(null);
	let error = $state<string | null>(null);

	const series = $derived(widget.series.length ? widget.series.map(usableSeries) : [usableSeries({ column: null, agg: 'count' })]);
	const pie = $derived(widget.chart === 'pie');

	function build() {
		const d = widget.data;
		const database = d ? appSession.databaseOf(d.source) : undefined;
		if (!d || !database) return null;
		return {
			database,
			table: d.table,
			grouped: !!widget.groupBy,
			query: {
				group_by: widget.groupBy ? { column: widget.groupBy.column, bucket: widget.groupBy.bucket } : null,
				// A pie shows one value per slice.
				series: pie ? series.slice(0, 1) : series,
				filters: resolveFilters(widget.filters, appSession.vars),
				order: widget.order,
				limit: widget.limit
			}
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
				data = toChartData(res, r.query.series, r.grouped);
				error = null;
			})
			.catch((e) => !stale && (error = cleanMessage(e?.message ?? String(e))));
		return () => (stale = true);
	});

	const color = (i: number) => (i === 0 ? 'var(--app-accent)' : `var(--chart-${((i - 1) % 5) + 1})`);

	const config = $derived.by((): Chart.ChartConfig => {
		const out: Chart.ChartConfig = {};
		if (!data) return out;
		if (pie) data.rows.forEach((r, i) => (out[String(r.label)] = { label: String(r.label), color: color(i) }));
		data.series.forEach((s, i) => (out[s.key] = { label: s.label, color: color(i) }));
		return out;
	});

	const seriesProps = $derived(data ? data.series.map((s, i) => ({ key: s.key, label: s.label, color: color(i) })) : []);
	const pieRows = $derived(data ? data.rows.map((r, i) => ({ ...r, color: color(i) })) : []);
</script>

{#snippet tooltipSnippet()}
	<Chart.Tooltip>
		{#snippet formatter({ value, name, item })}
			<div class="flex w-full items-center gap-2">
				<div class="size-2.5 shrink-0" style="background: {item.color}"></div>
				<span class="flex-1 text-muted-foreground">{name}</span>
				<span class="font-mono font-medium text-foreground tabular-nums">
					{typeof value === 'number' && data ? data.format(pie ? 's0' : item.key, value) : String(value)}
				</span>
			</div>
		{/snippet}
	</Chart.Tooltip>
{/snippet}

<div class="flex h-full min-h-0 flex-col">
	{#if error}
		<p class="text-sm text-destructive">{error}</p>
	{:else if !data}
		<Skeleton class="h-full w-full" />
	{:else if !data.rows.length}
		<Empty.Root class="h-full p-2">
			<Empty.Header>
				<Empty.Media variant="icon"><ChartColumnIcon /></Empty.Media>
				<Empty.Description>No records to chart.</Empty.Description>
			</Empty.Header>
		</Empty.Root>
	{:else}
		<Chart.Container {config} class="aspect-auto min-h-0 w-full flex-1">
			{#if pie}
				<PieChart data={pieRows} key="label" value="s0" c="color" innerRadius={0.5} padAngle={0.01} legend>
					{#snippet tooltip()}{@render tooltipSnippet()}{/snippet}
				</PieChart>
			{:else if widget.chart === 'line'}
				<LineChart data={data.rows} x="label" series={seriesProps} legend={seriesProps.length > 1} points>
					{#snippet tooltip()}{@render tooltipSnippet()}{/snippet}
				</LineChart>
			{:else if widget.chart === 'area'}
				<AreaChart data={data.rows} x="label" series={seriesProps} legend={seriesProps.length > 1}>
					{#snippet tooltip()}{@render tooltipSnippet()}{/snippet}
				</AreaChart>
			{:else}
				<BarChart
					data={data.rows}
					x={widget.chart === 'horizontal-bar' ? undefined : 'label'}
					y={widget.chart === 'horizontal-bar' ? 'label' : undefined}
					orientation={widget.chart === 'horizontal-bar' ? 'horizontal' : 'vertical'}
					series={seriesProps}
					seriesLayout={seriesProps.length > 1 ? 'group' : 'auto'}
					legend={seriesProps.length > 1}
					props={{ bars: { radius: 0 } }}
				>
					{#snippet tooltip()}{@render tooltipSnippet()}{/snippet}
				</BarChart>
			{/if}
		</Chart.Container>
		{#if data.truncated}
			<p class="pt-1 text-xs text-muted-foreground">Showing the first {data.rows.length} groups.</p>
		{/if}
	{/if}
</div>
