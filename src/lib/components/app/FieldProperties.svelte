<script lang="ts">
	import type { DefaultValue, FieldDef } from '#lib/api/types.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';

	/** Edits `field` in place: its type options and its rules. */
	let { field: f, disabled = false }: { field: FieldDef; disabled?: boolean } = $props();

	const DEFAULT_KINDS = [
		{ value: 'none', label: 'None' },
		{ value: 'literal', label: 'A fixed value' },
		{ value: 'current_date', label: 'Today' },
		{ value: 'current_timestamp', label: 'Now (date and time)' },
		{ value: 'expression', label: 'An expression' }
	];

	const t = $derived(f.field_type);
	const fixed = $derived(t.type === 'auto_number' || t.type === 'calculated');
	const defaultKinds = $derived(t.type === 'date_time' ? DEFAULT_KINDS : DEFAULT_KINDS.filter((k) => !k.value.startsWith('current')));

	function setDefaultKind(kind: string) {
		if (kind === 'none') f.default = null;
		else if (kind === 'literal' || kind === 'expression') f.default = { kind, value: '' };
		else f.default = { kind } as DefaultValue;
	}
	function defaultText() {
		return f.default && 'value' in f.default ? String(f.default.value ?? '') : '';
	}
	function setDefaultText(text: string) {
		if (f.default?.kind === 'literal') f.default = { kind: 'literal', value: text };
		else if (f.default?.kind === 'expression') f.default = { kind: 'expression', value: text };
	}
</script>

{#snippet select(id: string, items: { value: string; label: string }[], value: string, onChange: (v: string) => void)}
	<Select.Root type="single" {items} {value} onValueChange={onChange} {disabled}>
		<Select.Trigger {id} size="sm" class="w-full"><Select.Value /></Select.Trigger>
		<Select.Content>
			<Select.Group>
				{#each items as item (item.value)}<Select.Item value={item.value} label={item.label}>{item.label}</Select.Item>{/each}
			</Select.Group>
		</Select.Content>
	</Select.Root>
{/snippet}

{#snippet toggle(id: string, label: string, help: string, checked: boolean, onChange: (v: boolean) => void, off = false)}
	<Field.Field orientation="horizontal">
		<Field.Content>
			<Field.Label for={id}>{label}</Field.Label>
			<Field.Description>{help}</Field.Description>
		</Field.Content>
		<Switch {id} {checked} onCheckedChange={onChange} disabled={disabled || off} />
	</Field.Field>
{/snippet}

<Field.Group class="gap-5">
	{#if t.type === 'short_text'}
		<Field.Field>
			<Field.Label for="fp-size">Maximum length</Field.Label>
			<Input id="fp-size" type="number" min="1" class="h-8" placeholder="No limit" value={t.max_length ?? ''} oninput={(e) => (t.max_length = e.currentTarget.value === '' ? null : Number(e.currentTarget.value))} {disabled} />
		</Field.Field>
	{:else if t.type === 'number'}
		<Field.Field>
			<Field.Label for="fp-size">Numbers</Field.Label>
			{@render select('fp-size', [{ value: 'long', label: 'Whole numbers' }, { value: 'double', label: 'With decimals' }], t.real ? 'double' : 'long', (v) => (t.real = v === 'double'))}
		</Field.Field>
	{:else if t.type === 'currency'}
		<Field.Field>
			<Field.Label for="fp-scale">Decimal places</Field.Label>
			<Input id="fp-scale" type="number" min="0" max="6" class="h-8" value={t.scale ?? 2} oninput={(e) => (t.scale = Number(e.currentTarget.value))} {disabled} />
		</Field.Field>
	{:else if t.type === 'calculated'}
		<Field.Field>
			<Field.Label for="fp-expr">Formula</Field.Label>
			<Input id="fp-expr" class="h-8 font-mono" placeholder="Quantity * Price" bind:value={t.expression} {disabled} />
			<Field.Description>Computed from other fields of the same record.</Field.Description>
		</Field.Field>
		<Field.Field>
			<Field.Label for="fp-result">Result</Field.Label>
			{@render select('fp-result', [{ value: 'any', label: 'Automatic' }, { value: 'text', label: 'Text' }, { value: 'integer', label: 'Whole number' }, { value: 'real', label: 'Decimal' }], t.result ?? 'any', (v) => (t.result = v as never))}
		</Field.Field>
	{:else if t.type === 'unknown'}
		<Field.Field>
			<Field.Label for="fp-decl">Declared type</Field.Label>
			<Input id="fp-decl" class="h-8 font-mono" bind:value={t.declared} {disabled} />
		</Field.Field>
	{/if}

	{#if !fixed}
		{@render toggle('fp-required', 'Required', 'Every record must fill this in.', !!f.required, (v) => (f.required = v))}
	{/if}
	{@render toggle('fp-unique', 'No duplicates', 'Each record needs a different value.', !!f.unique || !!f.primary_key, (v) => (f.unique = v), !!f.primary_key)}

	{#if !fixed}
		<Field.Field>
			<Field.Label for="fp-default">Default value</Field.Label>
			<div class="flex gap-2">
				<div class={f.default?.kind === 'literal' || f.default?.kind === 'expression' ? 'w-36 shrink-0' : 'w-full'}>
					{@render select('fp-default', defaultKinds, f.default?.kind ?? 'none', setDefaultKind)}
				</div>
				{#if f.default?.kind === 'literal' || f.default?.kind === 'expression'}
					<Input class={f.default.kind === 'expression' ? 'h-8 font-mono' : 'h-8'} aria-label="Default value" value={defaultText()} oninput={(e) => setDefaultText(e.currentTarget.value)} {disabled} />
				{/if}
			</div>
		</Field.Field>
	{/if}

	<Field.Field>
		<Field.Label for="fp-check">Validation rule</Field.Label>
		<Input id="fp-check" class="h-8 font-mono" placeholder="e.g. {f.name || 'Price'} >= 0" value={f.check ?? ''} oninput={(e) => (f.check = e.currentTarget.value || null)} {disabled} />
		<Field.Description>A condition every value must meet.</Field.Description>
	</Field.Field>

	<Field.Field>
		<Field.Label for="fp-desc">Description</Field.Label>
		<Textarea id="fp-desc" rows={2} placeholder="What goes in this field" value={f.description ?? ''} oninput={(e) => (f.description = e.currentTarget.value || null)} {disabled} />
	</Field.Field>
</Field.Group>
