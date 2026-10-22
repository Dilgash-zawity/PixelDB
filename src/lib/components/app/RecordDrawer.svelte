<script lang="ts">
	// Services
	import { recordService } from '#lib/services/records.js';

	// Types
	import type { FieldDef, JsonValue, RecordRow, TableDef } from '#lib/api/types.js';
	import type { LookupOption } from '#lib/logic/lookups.js';

	// Utils
	import { toast } from 'svelte-sonner';
	import { fieldTypeLabel } from '#lib/logic/field-types.js';
	import { lookupLabel } from '#lib/logic/lookups.js';
	import * as rb from '#lib/logic/record-buffer.js';
	import { displayValue } from '#lib/logic/values.js';
	import { attempt, confirm } from '#lib/state/feedback.svelte.js';
	import { cn } from '#lib/utils.js';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import FieldInput from './FieldInput.svelte';
	import StepDrawer from './StepDrawer.svelte';

	// Icons
	import {
		CheckCheck as CheckCheckIcon,
		ListTodo as ListTodoIcon,
		Trash2 as TrashIcon,
		Undo2 as Undo2Icon,
	} from '@lucide/svelte';

	// Script
	/** Adds or edits one record, a few fields per step, then a last look before saving. */
	let {
		open = $bindable(false),
		table,
		def,
		columns,
		row,
		lookups,
		database,
		prefill = {},
		allowDelete = true,
		onSaved
	}: {
		open?: boolean;
		table: string;
		def: TableDef;
		columns: string[];
		/** The record to edit, or null for a new record. */
		row: RecordRow | null;
		lookups: Record<string, LookupOption[]>;
		/** Workspace database holding `table` (apps); the open database when omitted. */
		database?: string;
		/** Starting values for a new record. */
		prefill?: Record<string, JsonValue>;
		allowDelete?: boolean;
		onSaved: () => void;
	} = $props();

	let buf = $state<rb.RecordBuffer | null>(null);
	let submitted = $state(false);
	let saving = $state(false);
	let step = $state(0);

	// Load the record each time the drawer opens.
	$effect(() => {
		if (open) {
			buf = fresh();
			submitted = false;
			step = 0;
		}
	});

	function fresh(): rb.RecordBuffer {
		const b = rb.newBuffer(def, row, columns);
		if (!row) for (const [k, v] of Object.entries(prefill)) if (k in b.current) b.current[k] = v;
		return b;
	}

	const isNew = $derived(row === null);
	const dirty = $derived(buf ? rb.isDirty(buf) : false);
	const missing = $derived(buf ? rb.missingRequired(buf, def) : []);
	const pages = $derived(rb.fieldPages(def.fields));
	const steps = $derived([
		...pages.map((p, i) => ({ label: pages.length === 1 ? 'Details' : (p[0]?.name ?? `Part ${i + 1}`), icon: ListTodoIcon })),
		{ label: 'Review', icon: CheckCheckIcon }
	]);

	function shown(f: FieldDef, v: JsonValue | undefined): string {
		return (f.field_type.type === 'lookup' ? lookupLabel(lookups[f.name], v ?? null) : null) ?? displayValue(v, f.field_type);
	}

	async function requestClose(next: boolean) {
		if (next) {
			open = true;
			return;
		}
		if (
			dirty &&
			!(await confirm({
				title: 'Discard changes?',
				description: 'Your changes to this record have not been saved.',
				action: 'Discard',
				destructive: true,
				scope: 'record-drawer'
			}))
		) {
			return;
		}
		open = false;
	}

	async function save(addAnother = false) {
		if (!buf || saving) return;
		submitted = true;
		if (missing.length) {
			// Go back to the step with the first missing field.
			step = Math.max(0, pages.findIndex((p) => p.some((f) => f.name === missing[0])));
			return;
		}
		const b = buf;
		const values = rb.submission(b, def);
		saving = true;
		const ok = await attempt(async () => {
			if (b.key === null) await recordService.insert(table, values, database);
			else if (Object.keys(values).length) await recordService.update(table, b.key, values, database);
			return true;
		});
		saving = false;
		if (!ok) return;
		toast.success(isNew ? 'Record added' : 'Changes saved');
		onSaved();
		if (addAnother) {
			buf = fresh();
			submitted = false;
			step = 0;
		} else {
			open = false;
		}
	}

	async function remove() {
		if (!buf?.key) return;
		const key = buf.key;
		const ok = await confirm({
			title: 'Delete this record?',
			description: 'It will be permanently deleted. This cannot be undone.',
			action: 'Delete record',
			destructive: true,
			scope: 'record-drawer'
		});
		if (!ok) return;
		if ((await attempt(() => recordService.delete(table, [key], true, database))) === undefined) return;
		toast.success('Record deleted');
		open = false;
		onSaved();
	}

	function onKey(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
			e.preventDefault();
			save();
		}
	}
</script>

{#snippet fieldRow(b: rb.RecordBuffer, f: FieldDef)}
	{@const id = `record-field-${f.name}`}
	{@const invalid = submitted && missing.includes(f.name)}
	{@const changed = JSON.stringify(b.current[f.name]) !== JSON.stringify(b.original[f.name])}
	<Field.Field data-invalid={invalid || undefined} orientation={f.field_type.type === 'yes_no' ? 'horizontal' : 'vertical'}>
		{#if f.field_type.type === 'yes_no'}
			<FieldInput {id} field={f} value={b.current[f.name]} onchange={(v) => rb.setValue(b, f.name, v)} />
			<Field.Content>
				<Field.Label for={id}>{f.name}</Field.Label>
				{#if f.description}<Field.Description>{f.description}</Field.Description>{/if}
			</Field.Content>
		{:else}
			<div class="flex items-center justify-between gap-2">
				<Field.Label for={id}>
					{f.name}
					{#if f.required || (f.primary_key && f.field_type.type !== 'auto_number')}<span class="text-destructive">*</span>{/if}
				</Field.Label>
				{#if changed && !isNew}
					<Button variant="ghost" size="xs" onclick={() => rb.undoField(b, f.name)}>
						<Undo2Icon data-icon="inline-start" /> Undo
					</Button>
				{:else}
					<span class="text-xs text-muted-foreground">{fieldTypeLabel(f.field_type)}</span>
				{/if}
			</div>
			<FieldInput {id} field={f} value={b.current[f.name]} {invalid} options={lookups[f.name]} onchange={(v) => rb.setValue(b, f.name, v)} />
			{#if invalid}
				<Field.Error>This field is required.</Field.Error>
			{:else if f.description}
				<Field.Description>{f.description}</Field.Description>
			{/if}
		{/if}
	</Field.Field>
{/snippet}

<StepDrawer
	bind:open={() => open, requestClose}
	bind:step
	title={isNew ? 'New record' : 'Edit record'}
	description={isNew ? `Add a record to ${table}.` : `Make changes to this ${table} record.`}
	{steps}
	finishLabel={isNew ? 'Add record' : 'Save'}
	busy={saving}
	disabled={!buf || (step === steps.length - 1 && !isNew && !dirty)}
	onFinish={() => save()}
	scope="record-drawer"
	onkeydown={onKey}
>
	{#snippet actions()}
		{#if !isNew && allowDelete}
			<Button variant="destructive" size="icon" aria-label="Delete record" onclick={remove}><TrashIcon /></Button>
		{/if}
	{/snippet}
	{#snippet secondary()}
		{#if isNew}
			<Button variant="outline" disabled={saving} onclick={() => save(true)}>Save & add another</Button>
		{:else}
			<Button variant="outline" disabled={!dirty} onclick={() => buf && rb.undoAll(buf)}>Undo all</Button>
		{/if}
	{/snippet}
	{#if buf}
		{@const b = buf}
		{#if step < pages.length}
			<Field.Group>
				{#each pages[step] as f (f.name)}{@render fieldRow(b, f)}{/each}
			</Field.Group>
		{:else}
			<ul class="flex flex-col divide-y rounded-lg border text-sm">
				{#each def.fields as f, i (f.name)}
					{@const isMissing = submitted && missing.includes(f.name)}
					<li>
						<button
							type="button"
							class="flex w-full items-center gap-3 px-3 py-2 text-start hover:bg-accent"
							onclick={() => (step = Math.floor(i / (pages[0]?.length || 1)))}
						>
							<span class="w-40 shrink-0 truncate text-muted-foreground">{f.name}</span>
							<span class={cn('min-w-0 flex-1 truncate', isMissing && 'text-destructive')}>
								{#if isMissing}Required{:else if isNew && f.field_type.type === 'auto_number'}Assigned automatically{:else}{shown(f, b.current[f.name]) || '—'}{/if}
							</span>
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	{/if}
</StepDrawer>
