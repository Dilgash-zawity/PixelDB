<script lang="ts">
	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import { Checkbox } from '#lib/components/ui/checkbox/index.js';
	import { Label } from '#lib/components/ui/label/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';

	// Icons
	import { ArrowDown as ArrowDownIcon, ArrowUp as ArrowUpIcon } from '@lucide/svelte';

	// Script
	let {
		columns,
		selected,
		allLabel = 'Show every field',
		showAll = true,
		orderable = true,
		onchange
	}: {
		/** Every available column, in table order. */
		columns: string[];
		/** Chosen columns in display order; empty means all. */
		selected: string[];
		allLabel?: string;
		/** Offer the "every field" switch; without it, nothing chosen means none. */
		showAll?: boolean;
		orderable?: boolean;
		onchange: (next: string[]) => void;
	} = $props();

	const id = $props.id();
	const all = $derived(showAll && selected.length === 0);
	// Chosen columns first (in their order), then the rest.
	const ordered = $derived([...selected.filter((c) => columns.includes(c)), ...columns.filter((c) => !selected.includes(c))]);

	function toggle(c: string, on: boolean) {
		onchange(on ? [...selected, c] : selected.filter((x) => x !== c));
	}

	function shift(c: string, by: number) {
		const next = [...selected];
		const i = next.indexOf(c);
		const j = i + by;
		if (i < 0 || j < 0 || j >= next.length) return;
		[next[i], next[j]] = [next[j], next[i]];
		onchange(next);
	}
</script>

<div class="flex flex-col gap-2">
	{#if showAll}
		<div class="flex items-center gap-2">
			<Switch id="{id}-all" checked={all} onCheckedChange={(v) => onchange(v ? [] : [...columns])} />
			<Label for="{id}-all" class="font-normal">{allLabel}</Label>
		</div>
	{/if}
	{#if !all}
		<div class="flex flex-col border">
			{#each ordered as c (c)}
				{@const on = selected.includes(c)}
				<div class="flex items-center gap-2 px-2 py-1 text-sm">
					<Checkbox id="{id}-{c}" checked={on} onCheckedChange={(v) => toggle(c, !!v)} />
					<Label for="{id}-{c}" class="min-w-0 flex-1 truncate font-normal">{c}</Label>
					{#if on && orderable}
						<Button size="icon-xs" variant="ghost" aria-label="Move {c} up" disabled={selected.indexOf(c) === 0} onclick={() => shift(c, -1)}><ArrowUpIcon /></Button>
						<Button size="icon-xs" variant="ghost" aria-label="Move {c} down" disabled={selected.indexOf(c) === selected.length - 1} onclick={() => shift(c, 1)}><ArrowDownIcon /></Button>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>
