<script lang="ts" module>
	export interface PropOption {
		value: string;
		label: string;
		/** Options with the same group are listed under that heading. */
		group?: string;
		disabled?: boolean;
	}
</script>

<script lang="ts">
	import * as Select from '#lib/components/ui/select/index.js';

	let {
		id,
		value,
		options,
		placeholder = 'Choose…',
		label,
		disabled = false,
		onchange
	}: {
		id?: string;
		value: string;
		options: PropOption[];
		placeholder?: string;
		/** Accessible name when there is no visible label. */
		label?: string;
		disabled?: boolean;
		onchange: (value: string) => void;
	} = $props();

	// The select treats "" as "nothing chosen", so an option meaning "none" gets a stand-in value.
	const NONE = '__none__';
	const encode = (v: string) => (v === '' && options.some((o) => o.value === '') ? NONE : v);
	const items = $derived(options.map((o) => ({ ...o, value: encode(o.value) })));

	const groups = $derived.by(() => {
		const out: { name: string | undefined; options: PropOption[] }[] = [];
		for (const o of items) {
			const g = out.find((x) => x.name === o.group);
			if (g) g.options.push(o);
			else out.push({ name: o.group, options: [o] });
		}
		return out;
	});
</script>

<Select.Root
	type="single"
	{items}
	value={encode(value)}
	{disabled}
	onValueChange={(v) => v !== undefined && onchange(v === NONE ? '' : v)}
>
	<Select.Trigger {id} size="sm" class="w-full min-w-0" aria-label={label}>
		<Select.Value {placeholder} />
	</Select.Trigger>
	<Select.Content>
		{#each groups as g, i (g.name ?? i)}
			<Select.Group>
				{#if g.name}<Select.Label>{g.name}</Select.Label>{/if}
				{#each g.options as o (o.value)}
					<Select.Item value={o.value} label={o.label} disabled={o.disabled}>{o.label}</Select.Item>
				{/each}
			</Select.Group>
		{:else}
			<Select.Group><Select.Label>Nothing to choose from</Select.Label></Select.Group>
		{/each}
	</Select.Content>
</Select.Root>
