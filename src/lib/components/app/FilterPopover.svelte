<script lang="ts">
	import FilterIcon from '@lucide/svelte/icons/list-filter';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import XIcon from '@lucide/svelte/icons/x';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as Popover from '#lib/components/ui/popover/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import { FILTER_OPS, needsValue, type FilterRow } from '#lib/logic/filters.js';

	let {
		open = $bindable(false),
		columns,
		active,
		initial,
		onApply
	}: {
		open?: boolean;
		columns: string[];
		/** Number of filters currently applied. */
		active: number;
		initial: FilterRow[];
		onApply: (rows: FilterRow[]) => void;
	} = $props();

	let rows = $state<FilterRow[]>([]);
	$effect(() => {
		if (open) rows = initial.length ? initial.map((r) => ({ ...r })) : [{ column: columns[0] ?? '', op: 'contains', value: '' }];
	});

	const columnItems = $derived(columns.map((c) => ({ value: c, label: c })));
	const opItems = FILTER_OPS.map((o) => ({ value: o.op, label: o.label }));

	function apply() {
		onApply(rows);
		open = false;
	}
</script>

<Popover.Root bind:open>
	<Popover.Trigger>
		{#snippet child({ props })}
			<Button {...props} variant="outline">
				<FilterIcon data-icon="inline-start" />
				Filter
				{#if active}<Badge>{active}</Badge>{/if}
			</Button>
		{/snippet}
	</Popover.Trigger>
	<Popover.Content class="w-[34rem] max-w-[calc(100vw-2rem)]" align="start">
		<form
			class="flex flex-col gap-3"
			onsubmit={(e) => {
				e.preventDefault();
				apply();
			}}
		>
			<p class="text-sm font-medium">Show records where…</p>
			{#each rows as row, i (i)}
				<div class="flex items-center gap-2">
					<Select.Root type="single" items={columnItems} bind:value={row.column}>
						<Select.Trigger class="w-36" aria-label="Field"><Select.Value placeholder="Field" /></Select.Trigger>
						<Select.Content>
							<Select.Group>
								{#each columnItems as c (c.value)}<Select.Item value={c.value} label={c.label}>{c.label}</Select.Item>{/each}
							</Select.Group>
						</Select.Content>
					</Select.Root>
					<Select.Root type="single" items={opItems} bind:value={row.op}>
						<Select.Trigger class="w-36" aria-label="Condition"><Select.Value /></Select.Trigger>
						<Select.Content>
							<Select.Group>
								{#each opItems as o (o.value)}<Select.Item value={o.value} label={o.label}>{o.label}</Select.Item>{/each}
							</Select.Group>
						</Select.Content>
					</Select.Root>
					{#if needsValue(row.op)}
						<Input class="flex-1" bind:value={row.value} placeholder={row.op === 'in' ? 'a, b, c' : 'Value'} aria-label="Value" />
					{:else}
						<span class="flex-1"></span>
					{/if}
					<Button variant="ghost" size="icon" aria-label="Remove condition" onclick={() => rows.splice(i, 1)}><XIcon /></Button>
				</div>
			{/each}
			<div class="flex items-center gap-2">
				<Button variant="ghost" size="sm" onclick={() => rows.push({ column: columns[0] ?? '', op: 'contains', value: '' })}>
					<PlusIcon data-icon="inline-start" /> Add condition
				</Button>
				<span class="flex-1"></span>
				<Button
					variant="outline"
					size="sm"
					onclick={() => {
						rows = [];
						apply();
					}}>Clear all</Button
				>
				<Button size="sm" type="submit">Apply</Button>
			</div>
		</form>
	</Popover.Content>
</Popover.Root>
