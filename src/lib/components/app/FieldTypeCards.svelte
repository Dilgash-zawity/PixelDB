<script lang="ts">
	// Types
	import type { FieldTypeName } from '#lib/api/types.js';

	// Utils
	import { FIELD_TYPES } from '#lib/logic/field-types.js';
	import { cn } from '#lib/utils.js';

	// Components
	import FieldTypeIcon from './FieldTypeIcon.svelte';

	// Script
	/** Field types as a grid of cards; double-click picks one and moves on. */
	let {
		value,
		exclude = [],
		disabled = false,
		onChange,
		onPick
	}: {
		value: FieldTypeName;
		exclude?: FieldTypeName[];
		disabled?: boolean;
		onChange: (v: FieldTypeName) => void;
		/** Double-click: choose and go to the next step. */
		onPick?: (v: FieldTypeName) => void;
	} = $props();

	const types = $derived(FIELD_TYPES.filter((t) => !exclude.includes(t.name)));
</script>

<div class="grid grid-cols-2 gap-2 sm:grid-cols-3" role="radiogroup" aria-label="Field type">
	{#each types as t (t.name)}
		{@const selected = value === t.name}
		<button
			type="button"
			role="radio"
			aria-checked={selected}
			{disabled}
			class={cn(
				'flex flex-col items-start gap-1.5 rounded-lg border bg-card p-3 text-start outline-none transition-colors hover:bg-accent/50 focus-visible:ring-3 focus-visible:ring-ring/40 disabled:pointer-events-none disabled:opacity-50',
				selected && 'border-primary bg-primary/5 ring-1 ring-primary'
			)}
			onclick={() => onChange(t.name)}
			ondblclick={() => (onChange(t.name), onPick?.(t.name))}
		>
			<span class={cn('flex size-8 items-center justify-center rounded-md bg-muted', selected && 'bg-primary text-primary-foreground')}>
				<FieldTypeIcon type={t.name} class="size-4" />
			</span>
			<span class="text-sm font-medium">{t.label}</span>
			<span class="text-xs text-muted-foreground">{t.hint}</span>
		</button>
	{/each}
</div>
