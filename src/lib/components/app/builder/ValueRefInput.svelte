<script lang="ts">
	// Types
	import type { ValueRef } from '#lib/logic/app-def.js';

	// Utils
	import { appSession } from '#lib/state/app.svelte.js';

	// Components
	import { Input } from '#lib/components/ui/input/index.js';
	import PropSelect from './PropSelect.svelte';

	// Script
	let {
		value,
		columns,
		label = 'Value',
		onchange,
		ontyped
	}: {
		value: ValueRef;
		/** Fields of the clicked row that can be passed on (row actions only). */
		columns?: string[];
		label?: string;
		onchange: (v: ValueRef) => void;
		/** Typing a value; grouped so it undoes as one step. */
		ontyped?: (v: ValueRef) => void;
	} = $props();

	const options = $derived([
		{ value: 'literal', label: 'A value I type' },
		...(appSession.def?.variables ?? []).map((v) => ({ value: `var:${v.name}`, label: v.name, group: 'Variable' })),
		...(columns ?? []).map((c) => ({ value: `col:${c}`, label: c, group: 'Field of the clicked row' }))
	]);
	const selected = $derived(
		value.kind === 'literal' ? 'literal' : value.kind === 'variable' ? `var:${value.name}` : `col:${value.column}`
	);
	const text = $derived(value.kind === 'literal' && value.value !== null ? String(value.value) : '');
</script>

<div class="flex flex-col gap-1.5">
	<PropSelect
		{label}
		value={selected}
		{options}
		onchange={(v) => {
			if (v === 'literal') onchange({ kind: 'literal', value: null });
			else if (v.startsWith('var:')) onchange({ kind: 'variable', name: v.slice(4) });
			else onchange({ kind: 'column', column: v.slice(4) });
		}}
	/>
	{#if value.kind === 'literal'}
		<Input
			class="h-7"
			aria-label={label}
			placeholder="Value"
			value={text}
			oninput={(e) => (ontyped ?? onchange)({ kind: 'literal', value: e.currentTarget.value })}
		/>
	{/if}
</div>
