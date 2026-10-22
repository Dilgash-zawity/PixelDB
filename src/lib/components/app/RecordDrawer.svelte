<script lang="ts">
	import { toast } from 'svelte-sonner';
	import TrashIcon from '@lucide/svelte/icons/trash-2';
	import Undo2Icon from '@lucide/svelte/icons/undo-2';
	import type { JsonValue, RecordRow, TableDef } from '#lib/api/types.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Kbd } from '#lib/components/ui/kbd/index.js';
	import * as Drawer from '#lib/components/ui/drawer/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import { fieldTypeLabel } from '#lib/logic/field-types.js';
	import type { LookupOption } from '#lib/logic/lookups.js';
	import * as rb from '#lib/logic/record-buffer.js';
	import { recordService } from '#lib/services/records.js';
	import { attempt, confirm } from '#lib/state/feedback.svelte.js';
	import ConfirmHost from './ConfirmHost.svelte';
	import FieldInput from './FieldInput.svelte';

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

	// Load the record each time the sheet opens.
	$effect(() => {
		if (open) {
			buf = fresh();
			submitted = false;
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
		if (missing.length) return;
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

<Drawer.Root direction="right" bind:open={() => open, requestClose}>
	<Drawer.Content onkeydown={onKey}>
		<div class="flex min-h-0 flex-1 flex-col">
			<Drawer.Header>
				<Drawer.Title>{isNew ? `New record` : 'Edit record'}</Drawer.Title>
				<Drawer.Description>{isNew ? `Add a record to ${table}.` : `Make changes to this ${table} record.`}</Drawer.Description>
			</Drawer.Header>

			{#if buf}
				{@const b = buf}
				<form
					class="flex-1 overflow-y-auto px-4 pb-4"
					onsubmit={(e) => {
						e.preventDefault();
						save();
					}}
				>
					<Field.Group>
						{#each def.fields as f (f.name)}
							{@const id = `record-field-${f.name}`}
							{@const invalid = submitted && missing.includes(f.name)}
							{@const changed = JSON.stringify(b.current[f.name]) !== JSON.stringify(b.original[f.name])}
							<Field.Field
								data-invalid={invalid || undefined}
								orientation={f.field_type.type === 'yes_no' ? 'horizontal' : 'vertical'}
							>
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
						{/each}
					</Field.Group>
					<button type="submit" hidden aria-hidden="true"></button>
				</form>
			{/if}

			<Drawer.Footer class="flex-row items-center gap-2">
				{#if !isNew && allowDelete}
					<Button variant="destructive" size="icon" aria-label="Delete record" onclick={remove}><TrashIcon /></Button>
				{/if}
				<span class="flex-1 text-xs text-muted-foreground">
					<Kbd>⌘</Kbd><Kbd>Enter</Kbd> to save
				</span>
				{#if isNew}
					<Button variant="outline" disabled={saving} onclick={() => save(true)}>Save & add another</Button>
				{:else}
					<Button variant="outline" disabled={!dirty} onclick={() => buf && rb.undoAll(buf)}>Undo all</Button>
				{/if}
				<Button disabled={saving || (!isNew && !dirty)} onclick={() => save()}>
					{#if saving}<Spinner data-icon="inline-start" />{/if}
					{isNew ? 'Add record' : 'Save'}
				</Button>
			</Drawer.Footer>
		</div>
		<ConfirmHost scope="record-drawer" />
	</Drawer.Content>
</Drawer.Root>
