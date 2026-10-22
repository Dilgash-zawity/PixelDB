<script lang="ts">
	import PlusIcon from '@lucide/svelte/icons/plus';
	import XIcon from '@lucide/svelte/icons/x';
	import type { FilterOp } from '#lib/api/types.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import type { ValueRef, WidgetFilter } from '#lib/logic/app-def.js';
	import { FILTER_OPS, needsValue } from '#lib/logic/filters.js';
	import PropSelect from './PropSelect.svelte';
	import ValueRefInput from './ValueRefInput.svelte';

	let {
		filters,
		columns,
		onchange
	}: {
		filters: WidgetFilter[];
		columns: string[];
		/** `group` merges typing into one undo step. */
		onchange: (edit: (filters: WidgetFilter[]) => void, group?: string) => void;
	} = $props();

	const columnOptions = $derived(columns.map((c) => ({ value: c, label: c })));
	const opOptions = FILTER_OPS.map((o) => ({ value: o.op, label: o.label }));
</script>

<div class="flex flex-col gap-2">
	{#each filters as f, i (i)}
		<div class="flex flex-col gap-1.5 border p-2">
			<div class="flex items-center gap-1">
				<div class="min-w-0 flex-1">
					<PropSelect label="Field" value={f.column} options={columnOptions} placeholder="Field" onchange={(v) => onchange((fs) => (fs[i].column = v))} />
				</div>
				<Button size="icon-xs" variant="ghost" aria-label="Remove filter" onclick={() => onchange((fs) => fs.splice(i, 1))}><XIcon /></Button>
			</div>
			<PropSelect label="Condition" value={f.op} options={opOptions} onchange={(v) => onchange((fs) => (fs[i].op = v as FilterOp))} />
			{#if needsValue(f.op)}
				<ValueRefInput
					value={f.value}
					onchange={(v: ValueRef) => onchange((fs) => (fs[i].value = v))}
					ontyped={(v: ValueRef) => onchange((fs) => (fs[i].value = v), `filter-${i}`)}
				/>
			{/if}
		</div>
	{:else}
		<p class="text-xs text-muted-foreground">Shows every record.</p>
	{/each}
	<Button
		size="sm"
		variant="outline"
		disabled={!columns.length}
		onclick={() => onchange((fs) => fs.push({ column: columns[0] ?? '', op: 'eq', value: { kind: 'literal', value: null } }))}
	>
		<PlusIcon data-icon="inline-start" /> Add filter
	</Button>
</div>
