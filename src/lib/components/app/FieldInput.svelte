<script lang="ts">
	// Types
	import type { FieldDef, JsonValue } from '#lib/api/types.js';
	import type { LookupOption } from '#lib/logic/lookups.js';

	// Utils
	import { displayValue, fileToAttachment, formatBytes, isBlobSummary } from '#lib/logic/values.js';

	// Components
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as InputGroup from '#lib/components/ui/input-group/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import DateTimeInput from './DateTimeInput.svelte';

	// Icons
	import { Link as LinkIcon, Paperclip as PaperclipIcon, X as XIcon } from '@lucide/svelte';

	// Script
	let {
		id,
		field,
		value,
		invalid = false,
		options,
		onchange
	}: {
		id: string;
		field: FieldDef;
		value: JsonValue;
		invalid?: boolean;
		options?: LookupOption[];
		onchange: (value: JsonValue) => void;
	} = $props();

	const t = $derived(field.field_type);
	const text = $derived(value === null || value === undefined ? '' : String(value));
	const aria = $derived(invalid || undefined);
	const lookupItems = $derived(
		(options ?? []).map((o) => ({ value: JSON.stringify(o.value), label: o.label || String(o.value) }))
	);
</script>

{#if t.type === 'yes_no'}
	<Switch {id} checked={value === true} onCheckedChange={(v) => onchange(v)} aria-invalid={aria} />
{:else if t.type === 'long_text'}
	<Textarea {id} rows={4} value={text} oninput={(e) => onchange(e.currentTarget.value)} aria-invalid={aria} />
{:else if t.type === 'date_time'}
	<DateTimeInput {id} value={typeof value === 'string' && value ? value : null} {invalid} onchange={onchange} />
{:else if t.type === 'lookup' && options}
	<Select.Root
		type="single"
		items={lookupItems}
		value={value === null || value === undefined ? '' : JSON.stringify(value)}
		onValueChange={(v) => onchange(v ? JSON.parse(v) : null)}
	>
		<Select.Trigger {id} class="w-full" aria-invalid={aria}>
			<Select.Value placeholder="Choose a record" />
		</Select.Trigger>
		<Select.Content>
			<Select.Group>
				{#each lookupItems as item (item.value)}
					<Select.Item value={item.value} label={item.label}>{item.label}</Select.Item>
				{:else}
					<Select.Label>The linked table has no records yet</Select.Label>
				{/each}
			</Select.Group>
		</Select.Content>
	</Select.Root>
{:else if t.type === 'attachment'}
	<div class="flex items-center gap-2">
		{#if isBlobSummary(value)}
			<Badge variant="secondary"><PaperclipIcon /> {formatBytes(value.blob.size)}</Badge>
		{:else if value && typeof value === 'object'}
			<Badge variant="secondary"><PaperclipIcon /> New file selected</Badge>
		{/if}
		<Input
			{id}
			type="file"
			class="flex-1"
			aria-invalid={aria}
			onchange={async (e) => {
				const file = e.currentTarget.files?.[0];
				if (file) onchange(await fileToAttachment(file));
			}}
		/>
		{#if value !== null && value !== undefined}
			<Button variant="ghost" size="icon" aria-label="Remove file" onclick={() => onchange(null)}><XIcon /></Button>
		{/if}
	</div>
{:else if t.type === 'calculated' || t.type === 'auto_number'}
	<Input {id} disabled value={displayValue(value, t) || (t.type === 'auto_number' ? 'Assigned automatically' : 'Calculated when saved')} />
{:else if t.type === 'hyperlink'}
	<InputGroup.Root>
		<InputGroup.Input {id} type="url" placeholder="https://" value={text} oninput={(e) => onchange(e.currentTarget.value)} aria-invalid={aria} />
		<InputGroup.Addon><LinkIcon /></InputGroup.Addon>
	</InputGroup.Root>
{:else if t.type === 'currency'}
	<Input {id} inputmode="decimal" placeholder="0.00" value={text} oninput={(e) => onchange(e.currentTarget.value)} aria-invalid={aria} />
{:else if t.type === 'number' || t.type === 'large_number'}
	<Input
		{id}
		inputmode={t.type === 'number' && t.real ? 'decimal' : 'numeric'}
		value={text}
		oninput={(e) => onchange(e.currentTarget.value)}
		aria-invalid={aria}
	/>
{:else}
	<Input
		{id}
		value={text}
		maxlength={t.type === 'short_text' ? (t.max_length ?? undefined) : undefined}
		oninput={(e) => onchange(e.currentTarget.value)}
		aria-invalid={aria}
	/>
{/if}
