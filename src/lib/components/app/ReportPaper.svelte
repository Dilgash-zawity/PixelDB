<script lang="ts">
	// Types
	import type { BuiltReport, ReportDef } from '#lib/logic/reports.js';

	// Utils
	import { t } from '#lib/i18n/index.js';
	import { hasTotals } from '#lib/logic/reports.js';
	import { cn } from '#lib/utils.js';

	// Script
	/** A report as printed: title, then each group's rows with its totals, then the grand total. Plain black on white when printing. */
	let {
		def,
		built,
		database,
		/** More rows matched than were read. */
		truncated = false,
		class: className
	}: { def: ReportDef; built: BuiltReport; database: string; truncated?: boolean; class?: string } = $props();

	const today = new Date().toLocaleDateString(undefined, { year: 'numeric', month: 'long', day: 'numeric' });
	const totals = $derived(hasTotals(def));
	const grouped = $derived(built.groups.length > 1 || built.groups[0]?.label !== null);
	const plural = (n: number) => t('common.records', { count: n });
</script>

{#snippet totalRow(values: (string | null)[], label: string, strong = false)}
	<tr class={cn('border-t', strong ? 'border-t-2 border-foreground/60 font-semibold print:border-black' : 'border-foreground/20 text-muted-foreground print:text-neutral-600')}>
		{#each values as v, i (i)}
			<td class={cn('px-2 py-1.5 tabular-nums', built.headers[i]?.numeric && 'text-right')}>
				{#if i === 0 && label}
					<span class="flex justify-between gap-3"><span>{label}</span><span>{v ?? ''}</span></span>
				{:else}
					{v ?? ''}
				{/if}
			</td>
		{/each}
	</tr>
{/snippet}

<article class={cn('report-paper flex flex-col gap-5 bg-background p-8 text-sm text-foreground print:bg-white print:p-0 print:text-black', className)}>
	<header class="flex items-end justify-between gap-6 border-b-2 border-foreground pb-3 print:border-black">
		<div class="flex min-w-0 flex-col gap-1">
			<h1 class="text-2xl font-semibold">{def.title || def.name}</h1>
			{#if def.subtitle}<p class="text-muted-foreground print:text-neutral-600">{def.subtitle}</p>{/if}
		</div>
		<div class="shrink-0 text-right text-xs text-muted-foreground print:text-neutral-600">
			<div>{database} · {def.source}</div>
			<div>{today}</div>
		</div>
	</header>

	{#if built.missing.length}
		<p class="text-xs text-warning print:hidden">{t('reports.leftOut', { fields: built.missing.join(', '), source: def.source })}</p>
	{/if}

	{#if !built.headers.length}
		<p class="text-muted-foreground">{t('reports.chooseFields')}</p>
	{:else}
		<table class="w-full border-collapse">
			<thead class="print:table-header-group">
				<tr class="border-b border-foreground/40 print:border-black">
					{#each built.headers as h, i (i)}
						<th class={cn('px-2 py-1.5 text-start text-xs font-semibold tracking-wide uppercase', h.numeric && 'text-right')}>{h.label}</th>
					{/each}
				</tr>
			</thead>
			{#each built.groups as g, gi (gi)}
				<tbody class="break-inside-avoid-page">
					{#if grouped}
						<tr>
							<td colspan={built.headers.length} class="px-2 pt-4 pb-1 text-base font-semibold">
								{def.groupBy}: {g.label}
							</td>
						</tr>
					{/if}
					{#each g.rows as row, ri (ri)}
						<tr class="border-b border-foreground/10 print:border-neutral-300">
							{#each row as cell, ci (ci)}
								<td class={cn('px-2 py-1 align-top', cell.numeric && 'text-right tabular-nums')}>{cell.text}</td>
							{/each}
						</tr>
					{/each}
					{#if grouped && (totals || def.showCount)}
						{@render totalRow(totals ? g.totals : built.headers.map(() => null), def.showCount ? plural(g.count) : '')}
					{/if}
				</tbody>
			{/each}
			{#if totals || def.showCount}
				<tfoot>
					{@render totalRow(totals ? built.totals : built.headers.map(() => null), def.showCount ? t('reports.totalCount', { records: plural(built.count) }) : t('reports.total'), true)}
				</tfoot>
			{/if}
		</table>
		{#if !built.count}<p class="text-muted-foreground">{t('reports.noMatch')}</p>{/if}
		{#if truncated}<p class="text-xs text-warning">{t('reports.truncated', { count: built.count.toLocaleString() })}</p>{/if}
	{/if}
</article>
