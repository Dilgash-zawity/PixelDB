<script lang="ts">
	// Types
	import type { FieldTypeName } from '#lib/api/types.js';

	// Utils
	import { FIELD_TYPES } from '#lib/logic/field-types.js';
	import { cn } from '#lib/utils.js';

	// Components
	import FieldTypeIcon from './FieldTypeIcon.svelte';

	// Script
	/** Field types as a grid of small cards (icon and name; the hint is the tooltip). */
	let {
		value,
		exclude = [],
		disabled = false,
		onChange
	}: {
		value: FieldTypeName;
		exclude?: FieldTypeName[];
		disabled?: boolean;
		onChange: (v: FieldTypeName) => void;
	} = $props();

	const types = $derived(FIELD_TYPES.filter((t) => !exclude.includes(t.name)));
</script>

<div class="grid grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4" role="radiogroup" aria-label="Field type">
	{#each types as t (t.name)}
		{@const selected = value === t.name}
		<button
			type="button"
			role="radio"
			aria-checked={selected}
			{disabled}
			title={t.hint}
			class={cn(
				'flex min-w-0 items-center gap-2 rounded-lg border bg-card p-2 text-start outline-none transition-colors hover:bg-accent/50 focus-visible:ring-3 focus-visible:ring-ring/40 disabled:pointer-events-none disabled:opacity-50',
				selected && 'border-primary bg-primary/5 ring-1 ring-primary'
			)}
			onclick={() => onChange(t.name)}
		>
			<span class={cn('flex size-7 shrink-0 items-center justify-center rounded-md bg-muted', selected && 'bg-primary text-primary-foreground')}>
				<FieldTypeIcon type={t.name} class="size-4" />
			</span>
			<span class="truncate text-sm font-medium">{t.label}</span>
		</button>
	{/each}
</div>
