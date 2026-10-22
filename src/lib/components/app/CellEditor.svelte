<script lang="ts">
	// Types
	import type { FieldDef, JsonValue } from '#lib/api/types.js';
	import type { LookupOption } from '#lib/logic/lookups.js';

	// Utils
	import { NUMERIC_TYPES } from '#lib/logic/field-types.js';
	import { fileToAttachment, formatBytes, isBlobSummary, placeholderFor } from '#lib/logic/values.js';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import { Checkbox } from '#lib/components/ui/checkbox/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import DateTimeInput from './DateTimeInput.svelte';

	// Icons
	import { Paperclip as PaperclipIcon, X as XIcon } from '@lucide/svelte';

	// Script
	/**
	 * The in-place editor for one datasheet cell. Every focusable control
	 * carries `data-col` so the datasheet can move focus between cells.
	 */
	let {
		field,
		value,
		options,
		onchange,
		onfocus
	}: {
		field: FieldDef;
		value: JsonValue;
		options?: LookupOption[];
		onchange: (value: JsonValue) => void;
		onfocus: () => void;
	} = $props();

	let fileInput = $state<HTMLInputElement | null>(null);
	const t = $derived(field.field_type);
	const col = $derived(field.name);
	const items = $derived((options ?? []).map((o) => ({ value: JSON.stringify(o.value), label: o.label || String(o.value) })));
</script>

{#if t.type === 'yes_no'}
	<div class="flex justify-center">
		<Checkbox data-col={col} checked={value === true} onCheckedChange={(v) => onchange(v)} {onfocus} aria-label={col} />
	</div>
{:else if t.type === 'lookup' && options}
	<Select.Root type="single" {items} value={value === null || value === undefined ? '' : JSON.stringify(value)} onValueChange={(v) => onchange(v ? JSON.parse(v) : null)}>
		<Select.Trigger data-col={col} size="sm" class="w-full" {onfocus} aria-label={col}>
			<Select.Value placeholder="Choose…" />
		</Select.Trigger>
		<Select.Content>
			<Select.Group>
				{#each items as item (item.value)}<Select.Item value={item.value} label={item.label}>{item.label}</Select.Item>{/each}
			</Select.Group>
		</Select.Content>
	</Select.Root>
{:else if t.type === 'date_time'}
	<DateTimeInput compact data-col={col} value={typeof value === 'string' && value ? value : null} {onchange} />
{:else if t.type === 'attachment'}
	<div class="flex items-center gap-1">
		<Button data-col={col} variant="outline" size="xs" {onfocus} onclick={() => fileInput?.click()}>
			<PaperclipIcon data-icon="inline-start" />
			{#if isBlobSummary(value)}{formatBytes(value.blob.size)}{:else if value && typeof value === 'object'}New file{:else}Attach…{/if}
		</Button>
		{#if value !== null && value !== undefined}
			<Button variant="ghost" size="icon-xs" aria-label="Remove file" onclick={() => onchange(null)}><XIcon /></Button>
		{/if}
		<input
			bind:this={fileInput}
			type="file"
			hidden
			onchange={async (e) => {
				const file = e.currentTarget.files?.[0];
				if (file) onchange(await fileToAttachment(file));
				e.currentTarget.value = '';
			}}
		/>
	</div>
{:else}
	<Input
		data-col={col}
		class={NUMERIC_TYPES.includes(t.type) ? 'text-right tabular-nums' : ''}
		value={value === null || value === undefined ? '' : String(value)}
		placeholder={placeholderFor(t)}
		inputmode={t.type === 'currency' || (t.type === 'number' && t.real) ? 'decimal' : t.type === 'number' || t.type === 'large_number' ? 'numeric' : undefined}
		maxlength={t.type === 'short_text' ? (t.max_length ?? undefined) : undefined}
		oninput={(e) => onchange(e.currentTarget.value)}
		{onfocus}
		aria-label={col}
	/>
{/if}
