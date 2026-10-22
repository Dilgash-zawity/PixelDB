<script lang="ts">
	// Types
	import type { FieldTypeName } from '#lib/api/types.js';

	// Utils
	import { FIELD_TYPES } from '#lib/logic/field-types.js';

	// Components
	import * as Select from '#lib/components/ui/select/index.js';
	import FieldTypeIcon from './FieldTypeIcon.svelte';

	// Script
	let {
		value,
		onChange,
		id,
		disabled = false,
		/** Types not offered (the current value is always shown). */
		exclude = [],
		onfocus
	}: {
		value: FieldTypeName;
		onChange: (v: FieldTypeName) => void;
		id?: string;
		disabled?: boolean;
		exclude?: FieldTypeName[];
		onfocus?: () => void;
	} = $props();

	const offered = $derived(FIELD_TYPES.filter((t) => t.name === value || !exclude.includes(t.name)));
	const items = $derived(offered.map((t) => ({ value: t.name, label: t.label })));
</script>

<Select.Root type="single" {items} {value} {disabled} onValueChange={(v) => onChange(v as FieldTypeName)}>
	<Select.Trigger {id} size="sm" class="w-full" aria-label="Data type" {onfocus}>
		<span class="flex items-center gap-2">
			<FieldTypeIcon type={value} />
			<Select.Value placeholder={value === 'unknown' ? 'Other' : ''} />
		</span>
	</Select.Trigger>
	<Select.Content>
		{#each [['basic', 'Common'], ['advanced', 'Advanced']] as [group, heading] (group)}
			<Select.Group>
				<Select.Label>{heading}</Select.Label>
				{#each offered.filter((t) => t.group === group) as t (t.name)}
					<Select.Item value={t.name} label={t.label}>
						<FieldTypeIcon type={t.name} />
						<div class="flex flex-col">
							<span>{t.label}</span>
							<span class="text-xs text-muted-foreground">{t.hint}</span>
						</div>
					</Select.Item>
				{/each}
			</Select.Group>
		{/each}
	</Select.Content>
</Select.Root>
