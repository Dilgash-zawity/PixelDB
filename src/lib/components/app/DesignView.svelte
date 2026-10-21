<script lang="ts">
	import { onDestroy, tick } from 'svelte';
	import { toast } from 'svelte-sonner';
	import InfoIcon from '@lucide/svelte/icons/info';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import ListPlusIcon from '@lucide/svelte/icons/list-plus';
	import ListXIcon from '@lucide/svelte/icons/list-x';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import SaveIcon from '@lucide/svelte/icons/save';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';
	import Undo2Icon from '@lucide/svelte/icons/undo-2';
	import XIcon from '@lucide/svelte/icons/x';
	import type { DefaultValue, FieldDef, FieldTypeName, FkAction, TableDef, TableInfo } from '#lib/api/types.js';
	import * as Alert from '#lib/components/ui/alert/index.js';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as ButtonGroup from '#lib/components/ui/button-group/index.js';
	import * as ContextMenu from '#lib/components/ui/context-menu/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import * as Table from '#lib/components/ui/table/index.js';
	import * as Tabs from '#lib/components/ui/tabs/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import { FIELD_TYPES } from '#lib/logic/field-types.js';
	import * as d from '#lib/logic/table-draft.js';
	import { identifierProblem } from '#lib/logic/values.js';
	import { tableService } from '#lib/services/tables.js';
	import { attempt, confirm } from '#lib/state/feedback.svelte.js';
	import { session } from '#lib/state/session.svelte.js';
	import { cn } from '#lib/utils.js';
	import FieldTypeIcon from './FieldTypeIcon.svelte';

	let { table }: { table: string } = $props();

	const TYPE_ITEMS = FIELD_TYPES.map((t) => ({ value: t.name, label: t.label }));
	const FK_ACTIONS: { value: FkAction; label: string }[] = [
		{ value: 'restrict', label: 'Prevent it' },
		{ value: 'cascade', label: 'Apply to linked records too' },
		{ value: 'set_null', label: 'Clear the link' },
		{ value: 'no_action', label: 'Check at the end (default)' }
	];
	const DEFAULT_KINDS = [
		{ value: 'none', label: 'None' },
		{ value: 'literal', label: 'A fixed value' },
		{ value: 'current_date', label: 'Today' },
		{ value: 'current_timestamp', label: 'Now (date and time)' },
		{ value: 'expression', label: 'An expression' }
	];
	const YES_NO = [
		{ value: 'no', label: 'No' },
		{ value: 'yes', label: 'Yes' }
	];

	let info = $state<TableInfo | null>(null);
	let draft = $state(d.newDraft());
	let baseline = $state('');
	let selectedId = $state<number | null>(null);
	let propTab = $state('general');
	let newFieldName = $state('');
	let newRule = $state('');
	let newUnique = $state('');
	let saving = $state(false);
	let targets = $state<Record<string, TableDef>>({});
	let gridEl = $state<HTMLDivElement | null>(null);

	const dirty = $derived(baseline !== '' && JSON.stringify(d.toDesign(draft)) !== baseline);
	const blocked = $derived(!!info?.unsupported_features.length);
	const selected = $derived(draft.fields.find((f) => f.id === selectedId) ?? null);
	const problems = $derived(d.validateDraft(draft));

	async function load() {
		const loaded = await attempt(() => tableService.get(table));
		if (!loaded) return;
		info = loaded;
		draft = d.draftFromDefinition(loaded.definition);
		baseline = JSON.stringify(d.toDesign(draft));
		if (!draft.fields.some((f) => f.id === selectedId)) selectedId = draft.fields[0]?.id ?? null;
	}

	// Load related tables for the Lookup properties.
	$effect(() => {
		for (const fk of draft.foreign_keys) {
			const t = fk.ref_table;
			if (t && !targets[t] && t !== table) {
				tableService.get(t).then((i) => (targets[t] = i.definition), () => {});
			}
		}
	});

	function targetFields(t: string | undefined): string[] {
		if (!t) return [];
		const fields = t === table ? draft.fields.map((f) => f.field) : (targets[t]?.fields ?? []);
		return fields.map((f) => f.name);
	}

	async function focusRow(id: number) {
		selectedId = id;
		await tick();
		gridEl?.querySelector<HTMLInputElement>(`[data-field-id="${id}"] input`)?.focus();
	}

	function addNamedField() {
		const name = newFieldName.trim();
		if (!name) return;
		const f = d.addField(draft, 'short_text', name);
		newFieldName = '';
		selectedId = f.id;
	}

	function insertRow() {
		const index = selected ? draft.fields.indexOf(selected) : draft.fields.length;
		focusRow(d.insertField(draft, index).id);
	}

	function deleteRow(id = selectedId) {
		if (id === null) return;
		const index = draft.fields.findIndex((f) => f.id === id);
		d.removeField(draft, id);
		selectedId = draft.fields[Math.min(index, draft.fields.length - 1)]?.id ?? null;
	}

	async function save(): Promise<boolean> {
		if (!info || saving) return false;
		if (problems.length) {
			toast.error('Fix the design first', { description: problems.join(' · ') });
			return false;
		}
		const removed = d.removedFields(draft, info.definition);
		let confirmed = false;
		if (removed.length) {
			confirmed = await confirm({
				title: `Delete ${removed.length === 1 ? 'a field' : `${removed.length} fields`}?`,
				description: `Saving permanently deletes ${removed.map((r) => `“${r}”`).join(', ')} and all the data stored in ${removed.length === 1 ? 'it' : 'them'}.`,
				action: 'Delete and save',
				destructive: true
			});
			if (!confirmed) return false;
		}
		saving = true;
		const result = await attempt(() => tableService.saveDesign(table, d.toDesign(draft), confirmed));
		saving = false;
		if (!result) return false;
		const keep = selected?.field.name;
		await load();
		selectedId = draft.fields.find((f) => f.field.name === keep)?.id ?? selectedId;
		await session.refresh();
		toast.success('Design saved', {
			description: [result.rebuilt ? 'Existing records were kept.' : '', ...result.warnings].filter(Boolean).join(' ') || undefined
		});
		return true;
	}

	async function discard() {
		if (!dirty) return;
		const ok = await confirm({
			title: 'Discard design changes?',
			description: 'The table goes back to how it was when you last saved.',
			action: 'Discard',
			destructive: true
		});
		if (ok) await load();
	}

	/** Leaving with unsaved changes: save them or stay. */
	async function guard(): Promise<boolean> {
		if (!dirty) return true;
		const ok = await confirm({
			title: 'Save design changes?',
			description: `You changed the design of ${table}. Save before leaving? Cancel to keep editing.`,
			action: 'Save changes'
		});
		return ok && (await save());
	}

	session.leaveGuard = guard;
	onDestroy(() => {
		if (session.leaveGuard === guard) session.leaveGuard = null;
	});

	let lastIntent = session.intent.n;
	$effect(() => {
		const { kind, n } = session.intent;
		if (n === lastIntent) return;
		lastIntent = n;
		if (kind === 'new-field') {
			const f = d.addField(draft, 'short_text');
			focusRow(f.id);
		}
	});

	function onWindowKey(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 's') {
			e.preventDefault();
			save();
		}
	}

	// Default value helpers.
	function defaultKind(f: FieldDef) {
		return f.default?.kind ?? 'none';
	}
	function setDefaultKind(f: FieldDef, kind: string) {
		if (kind === 'none') f.default = null;
		else if (kind === 'literal' || kind === 'expression') f.default = { kind, value: '' };
		else f.default = { kind } as DefaultValue;
	}
	function defaultText(f: FieldDef) {
		return f.default && 'value' in f.default ? String(f.default.value ?? '') : '';
	}
	function setDefaultText(f: FieldDef, text: string) {
		if (f.default?.kind === 'literal') f.default = { kind: 'literal', value: text };
		else if (f.default?.kind === 'expression') f.default = { kind: 'expression', value: text };
	}

	load();
</script>

<svelte:window onkeydown={onWindowKey} />

{#snippet select(id: string, items: { value: string; label: string }[], value: string, onChange: (v: string) => void, placeholder = '', disabled = false)}
	<Select.Root type="single" {items} {value} onValueChange={onChange} {disabled}>
		<Select.Trigger {id} size="sm" class="w-full"><Select.Value {placeholder} /></Select.Trigger>
		<Select.Content>
			<Select.Group>
				{#each items as item (item.value)}<Select.Item value={item.value} label={item.label}>{item.label}</Select.Item>{/each}
			</Select.Group>
		</Select.Content>
	</Select.Root>
{/snippet}

{#snippet property(id: string, label: string, help: string)}
	<Field.Label for={id}>{label}</Field.Label>
	<Field.Description>{help}</Field.Description>
{/snippet}

<div class="flex h-full flex-col">
	<!-- Toolbar -->
	<div class="flex flex-wrap items-center gap-2 border-b px-4 py-2">
		<ButtonGroup.Root>
			<Button size="sm" disabled={!dirty || saving || blocked} onclick={save}>
				{#if saving}<Spinner data-icon="inline-start" />{:else}<SaveIcon data-icon="inline-start" />{/if}
				Save
			</Button>
			<Button variant="outline" size="sm" disabled={!dirty || saving} onclick={discard}><Undo2Icon data-icon="inline-start" /> Discard</Button>
		</ButtonGroup.Root>
		<ButtonGroup.Root>
			<Button variant="outline" size="sm" disabled={!selected || blocked} onclick={() => selected && d.togglePrimaryKey(draft, selected.id)}>
				<KeyRoundIcon data-icon="inline-start" /> Primary key
			</Button>
			<Button variant="outline" size="sm" disabled={blocked} onclick={insertRow}><ListPlusIcon data-icon="inline-start" /> Insert row</Button>
			<Button variant="outline" size="sm" disabled={!selected || blocked || draft.fields.length <= 1} onclick={() => deleteRow()}>
				<ListXIcon data-icon="inline-start" /> Delete row
			</Button>
		</ButtonGroup.Root>
		<span class="flex-1"></span>
		{#if dirty}<Badge variant="secondary">Unsaved changes</Badge>{/if}
	</div>

	{#if !info}
		<div class="flex flex-col gap-2 p-4">
			{#each Array(5) as _, i (i)}<Skeleton class="h-9" />{/each}
		</div>
	{:else}
		{#if blocked}
			<Alert.Root variant="destructive" class="rounded-none border-x-0 border-t-0">
				<TriangleAlertIcon />
				<Alert.Title>This table can't be redesigned here</Alert.Title>
				<Alert.Description>
					It was created by another program and uses features ({info.unsupported_features.join(', ')}) that would be lost. You can still rename fields from the Datasheet.
				</Alert.Description>
			</Alert.Root>
		{/if}

		<!-- Field grid -->
		<div class="min-h-40 flex-1 overflow-auto" bind:this={gridEl}>
			<ContextMenu.Root>
				<ContextMenu.Trigger>
					{#snippet child({ props })}
						<div {...props}>
							<Table.Root>
								<Table.Header class="sticky top-0 z-10 bg-background">
									<Table.Row>
										<Table.Head class="w-10 border-e"></Table.Head>
										<Table.Head class="w-1/4 border-e">Field Name</Table.Head>
										<Table.Head class="w-56 border-e">Data Type</Table.Head>
										<Table.Head>Description (optional)</Table.Head>
									</Table.Row>
								</Table.Header>
								<Table.Body>
									{#each draft.fields as df (df.id)}
										{@const f = df.field}
										{@const nameBad = !!identifierProblem(f.name) || draft.fields.filter((x) => x.field.name.toLowerCase() === f.name.toLowerCase()).length > 1}
										<Table.Row
											data-field-id={df.id}
											data-state={selectedId === df.id ? 'selected' : undefined}
											onclick={() => (selectedId = df.id)}
											oncontextmenu={() => (selectedId = df.id)}
										>
											<Table.Cell class="w-10 border-e text-center">
												{#if f.primary_key}<KeyRoundIcon class="mx-auto size-4 text-primary" />{/if}
											</Table.Cell>
											<Table.Cell class="border-e py-1">
												<Input
													value={f.name}
													disabled={blocked}
													aria-invalid={nameBad || undefined}
													aria-label="Field name"
													oninput={(e) => d.renameField(draft, df.id, e.currentTarget.value)}
													onfocus={() => (selectedId = df.id)}
												/>
											</Table.Cell>
											<Table.Cell class="border-e py-1">
												<Select.Root
													type="single"
													items={TYPE_ITEMS}
													value={f.field_type.type}
													disabled={blocked}
													onValueChange={(v) => {
														d.setFieldType(draft, df.id, v as FieldTypeName);
														if (v === 'lookup') propTab = 'lookup';
													}}
												>
													<Select.Trigger size="sm" class="w-full" aria-label="Data type" onfocus={() => (selectedId = df.id)}>
														<span class="flex items-center gap-2">
															<FieldTypeIcon type={f.field_type.type} />
															<Select.Value placeholder={f.field_type.type === 'unknown' ? 'Other' : ''} />
														</span>
													</Select.Trigger>
													<Select.Content>
														{#each [['basic', 'Common'], ['advanced', 'Advanced']] as [group, heading] (group)}
															<Select.Group>
																<Select.Label>{heading}</Select.Label>
																{#each FIELD_TYPES.filter((t) => t.group === group) as t (t.name)}
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
											</Table.Cell>
											<Table.Cell class="py-1">
												<Input
													value={f.description ?? ''}
													disabled={blocked}
													aria-label="Description"
													oninput={(e) => (f.description = e.currentTarget.value || null)}
													onfocus={() => (selectedId = df.id)}
												/>
											</Table.Cell>
										</Table.Row>
									{/each}
									{#if !blocked}
										<Table.Row>
											<Table.Cell class="w-10 border-e text-center"><PlusIcon class="mx-auto size-4 text-muted-foreground" /></Table.Cell>
											<Table.Cell class="border-e py-1">
												<Input
													bind:value={newFieldName}
													placeholder="New field name"
													aria-label="New field name"
													onkeydown={(e) => e.key === 'Enter' && addNamedField()}
													onblur={addNamedField}
												/>
											</Table.Cell>
											<Table.Cell colspan={2} class="text-sm text-muted-foreground">Type a name and press Enter to add a field.</Table.Cell>
										</Table.Row>
									{/if}
								</Table.Body>
							</Table.Root>
						</div>
					{/snippet}
				</ContextMenu.Trigger>
				<ContextMenu.Content class="w-52">
					<ContextMenu.Group>
						<ContextMenu.Item disabled={!selected || blocked} onclick={() => selected && d.togglePrimaryKey(draft, selected.id)}>
							<KeyRoundIcon /> Primary key
						</ContextMenu.Item>
						<ContextMenu.Item disabled={blocked} onclick={insertRow}><ListPlusIcon /> Insert row</ContextMenu.Item>
					</ContextMenu.Group>
					<ContextMenu.Separator />
					<ContextMenu.Group>
						<ContextMenu.Item variant="destructive" disabled={!selected || blocked || draft.fields.length <= 1} onclick={() => deleteRow()}>
							<ListXIcon /> Delete row
						</ContextMenu.Item>
					</ContextMenu.Group>
				</ContextMenu.Content>
			</ContextMenu.Root>
		</div>

		<!-- Properties -->
		<div class="flex h-[42%] min-h-56 flex-col border-t">
			<Tabs.Root bind:value={propTab} class="flex min-h-0 flex-1 flex-col gap-0">
				<div class="flex items-center gap-3 border-b px-4 py-2">
					<span class="text-sm font-medium">
						{propTab === 'table' ? 'Table properties' : `Field properties${selected ? ` — ${selected.field.name || 'unnamed'}` : ''}`}
					</span>
					<span class="flex-1"></span>
					<Tabs.List>
						<Tabs.Trigger value="general">General</Tabs.Trigger>
						<Tabs.Trigger value="lookup">Lookup</Tabs.Trigger>
						<Tabs.Trigger value="table">Table</Tabs.Trigger>
					</Tabs.List>
				</div>
				<div class="min-h-0 flex-1 overflow-y-auto px-4 py-4">
					<Tabs.Content value="general">
						{#if selected}
							{@const f = selected.field}
							{@const t = f.field_type}
							{@const fixed = t.type === 'auto_number' || t.type === 'calculated'}
							<Field.Group class="max-w-3xl gap-4">
								{#if t.type === 'short_text'}
									<Field.Field orientation="horizontal">
										<Field.Content class="w-72 flex-none">{@render property('p-size', 'Field size', 'Most characters allowed. Empty means no limit.')}</Field.Content>
										<Input id="p-size" type="number" min="1" class="max-w-sm flex-1" placeholder="No limit" value={t.max_length ?? ''} oninput={(e) => (t.max_length = e.currentTarget.value === '' ? null : Number(e.currentTarget.value))} disabled={blocked} />
									</Field.Field>
								{:else if t.type === 'number'}
									<Field.Field orientation="horizontal">
										<Field.Content class="w-72 flex-none">{@render property('p-size', 'Field size', 'Whole numbers, or numbers with decimals.')}</Field.Content>
										<div class="max-w-sm flex-1">
											{@render select('p-size', [{ value: 'long', label: 'Whole number' }, { value: 'double', label: 'Decimal' }], t.real ? 'double' : 'long', (v) => (t.real = v === 'double'), '', blocked)}
										</div>
									</Field.Field>
								{:else if t.type === 'currency'}
									<Field.Field orientation="horizontal">
										<Field.Content class="w-72 flex-none">{@render property('p-scale', 'Decimal places', 'Amounts are stored exactly, never rounded.')}</Field.Content>
										<Input id="p-scale" type="number" min="0" max="6" class="max-w-sm flex-1" value={t.scale ?? 2} oninput={(e) => (t.scale = Number(e.currentTarget.value))} disabled={blocked} />
									</Field.Field>
								{:else if t.type === 'calculated'}
									<Field.Field orientation="horizontal">
										<Field.Content class="w-72 flex-none">{@render property('p-expr', 'Expression', 'Computed from other fields, e.g. Quantity * Price.')}</Field.Content>
										<Input id="p-expr" class="font-mono max-w-sm flex-1" bind:value={t.expression} disabled={blocked} />
									</Field.Field>
									<Field.Field orientation="horizontal">
										<Field.Content class="w-72 flex-none">{@render property('p-result', 'Result type', 'The kind of value the expression produces.')}</Field.Content>
										<div class="max-w-sm flex-1">
											{@render select('p-result', [{ value: 'any', label: 'Automatic' }, { value: 'text', label: 'Text' }, { value: 'integer', label: 'Whole number' }, { value: 'real', label: 'Decimal' }], t.result ?? 'any', (v) => (t.result = v as never), '', blocked)}
										</div>
									</Field.Field>
								{:else if t.type === 'unknown'}
									<Field.Field orientation="horizontal">
										<Field.Content class="w-72 flex-none">{@render property('p-decl', 'Declared type', 'The column type set by the program that created it.')}</Field.Content>
										<Input id="p-decl" class="font-mono max-w-sm flex-1" bind:value={t.declared} disabled={blocked} />
									</Field.Field>
								{/if}
								{#if !fixed}
									<Field.Field orientation="horizontal">
										<Field.Content class="w-72 flex-none">{@render property('p-default', 'Default value', 'Filled in automatically for new records.')}</Field.Content>
										<div class="flex max-w-sm flex-1 gap-2">
											<div class="w-44">
												{@render select('p-default', t.type === 'date_time' ? DEFAULT_KINDS : DEFAULT_KINDS.filter((k) => !k.value.startsWith('current')), defaultKind(f), (v) => setDefaultKind(f, v), '', blocked)}
											</div>
											{#if f.default?.kind === 'literal' || f.default?.kind === 'expression'}
												<Input class={f.default.kind === 'expression' ? 'font-mono' : ''} aria-label="Default value" value={defaultText(f)} oninput={(e) => setDefaultText(f, e.currentTarget.value)} disabled={blocked} />
											{/if}
										</div>
									</Field.Field>
									<Field.Field orientation="horizontal">
										<Field.Content class="w-72 flex-none">{@render property('p-required', 'Required', 'Records must have a value in this field.')}</Field.Content>
										<Switch id="p-required" checked={!!f.required} onCheckedChange={(v) => (f.required = v)} disabled={blocked} />
									</Field.Field>
								{/if}
								<Field.Field orientation="horizontal">
									<Field.Content class="w-72 flex-none">{@render property('p-unique', 'No duplicates', 'Every record must have a different value.')}</Field.Content>
									<Switch id="p-unique" checked={!!f.unique || !!f.primary_key} disabled={blocked || !!f.primary_key} onCheckedChange={(v) => (f.unique = v)} />
								</Field.Field>
								<Field.Field orientation="horizontal">
									<Field.Content class="w-72 flex-none">{@render property('p-check', 'Validation rule', 'A condition values must meet, e.g. Price >= 0.')}</Field.Content>
									<Input id="p-check" class="font-mono max-w-sm flex-1" value={f.check ?? ''} oninput={(e) => (f.check = e.currentTarget.value || null)} disabled={blocked} />
								</Field.Field>
							</Field.Group>
						{:else}
							<p class="text-sm text-muted-foreground">Select a field above to see its properties.</p>
						{/if}
					</Tabs.Content>

					<Tabs.Content value="lookup">
						{#if selected && selected.field.field_type.type === 'lookup'}
							{@const f = selected.field}
							{@const t = f.field_type}
							{@const fk = d.lookupTarget(draft, f.name)}
							<Field.Group class="max-w-3xl gap-4">
								<Field.Field orientation="horizontal">
									<Field.Content class="w-72 flex-none">{@render property('p-src', 'Row source', 'The table this field links to.')}</Field.Content>
									<div class="max-w-sm flex-1">
										{@render select('p-src', [...new Set([...session.tables.map((x) => x.name), table])].map((n) => ({ value: n, label: n })), fk?.ref_table ?? '', (v) => d.setLookupTarget(draft, selected.id, v, ''), 'Choose a table', blocked)}
									</div>
								</Field.Field>
								{#if fk}
									<Field.Field orientation="horizontal">
										<Field.Content class="w-72 flex-none">{@render property('p-bound', 'Bound column', 'The value stored. Usually the related record ID.')}</Field.Content>
										<div class="max-w-sm flex-1">
											{@render select('p-bound', targetFields(fk.ref_table).map((n) => ({ value: n, label: n })), fk.ref_columns?.[0] ?? '', (v) => (fk.ref_columns = v ? [v] : []), 'Record ID', blocked)}
										</div>
									</Field.Field>
									<Field.Field orientation="horizontal">
										<Field.Content class="w-72 flex-none">{@render property('p-display', 'Display column', 'Shown instead of the stored value.')}</Field.Content>
										<div class="max-w-sm flex-1">
											{@render select('p-display', targetFields(fk.ref_table).map((n) => ({ value: n, label: n })), t.type === 'lookup' ? (t.display_column ?? '') : '', (v) => t.type === 'lookup' && (t.display_column = v || null), 'Bound column', blocked)}
										</div>
									</Field.Field>
									<Field.Field orientation="horizontal">
										<Field.Content class="w-72 flex-none">{@render property('p-del', 'When the linked record is deleted', 'Protects records that point to it.')}</Field.Content>
										<div class="max-w-sm flex-1">
											{@render select('p-del', FK_ACTIONS, fk.on_delete ?? 'no_action', (v) => (fk.on_delete = v as FkAction), '', blocked)}
										</div>
									</Field.Field>
									<Field.Field orientation="horizontal">
										<Field.Content class="w-72 flex-none">{@render property('p-upd', 'When its ID changes', 'Keeps links pointing at the right record.')}</Field.Content>
										<div class="max-w-sm flex-1">
											{@render select('p-upd', FK_ACTIONS, fk.on_update ?? 'no_action', (v) => (fk.on_update = v as FkAction), '', blocked)}
										</div>
									</Field.Field>
								{/if}
							</Field.Group>
						{:else if selected}
							<Alert.Root class="max-w-xl">
								<InfoIcon />
								<Alert.Title>“{selected.field.name}” isn't a lookup field</Alert.Title>
								<Alert.Description>
									Change its data type to Relation to pick values from another table.
									<Button
										variant="outline"
										size="sm"
										class="mt-2"
										disabled={blocked}
										onclick={() => selected && d.setFieldType(draft, selected.id, 'lookup')}
									>
										Change to Relation
									</Button>
								</Alert.Description>
							</Alert.Root>
						{/if}
					</Tabs.Content>

					<Tabs.Content value="table">
						<Field.Group class="max-w-3xl gap-6">
							<Field.Field>
								<Field.Label for="t-desc">Description</Field.Label>
								<Textarea id="t-desc" rows={2} bind:value={draft.description} placeholder="What this table is for" disabled={blocked} />
							</Field.Field>
							<Field.Set>
								<Field.Legend variant="label">Validation rules</Field.Legend>
								<Field.Description>Conditions every record must meet, e.g. EndDate >= StartDate.</Field.Description>
								<div class="flex flex-col gap-2">
									{#each draft.checks as _, i (i)}
										<div class="flex gap-2">
											<Input class="font-mono" bind:value={draft.checks[i]} aria-label="Rule {i + 1}" disabled={blocked} />
											<Button variant="ghost" size="icon" aria-label="Remove rule" disabled={blocked} onclick={() => draft.checks.splice(i, 1)}><XIcon /></Button>
										</div>
									{/each}
									<div class="flex gap-2">
										<Input class="font-mono" bind:value={newRule} placeholder="New rule" aria-label="New rule" disabled={blocked} onkeydown={(e) => { if (e.key === 'Enter' && newRule.trim()) { draft.checks.push(newRule.trim()); newRule = ''; } }} />
										<Button variant="outline" disabled={blocked || !newRule.trim()} onclick={() => { draft.checks.push(newRule.trim()); newRule = ''; }}>Add</Button>
									</div>
								</div>
							</Field.Set>
							<Field.Set>
								<Field.Legend variant="label">No duplicate combinations</Field.Legend>
								<Field.Description>Fields whose values together must be unique, e.g. FirstName, LastName.</Field.Description>
								<div class="flex flex-col gap-2">
									{#each draft.unique_constraints as u, i (i)}
										<div class="flex items-center gap-2">
											{#each u as c (c)}<Badge variant="secondary">{c}</Badge>{/each}
											<Button variant="ghost" size="icon-sm" aria-label="Remove" disabled={blocked} onclick={() => draft.unique_constraints.splice(i, 1)}><XIcon /></Button>
										</div>
									{/each}
									<div class="flex gap-2">
										<Input bind:value={newUnique} placeholder="Field1, Field2" aria-label="Fields" disabled={blocked} />
										<Button
											variant="outline"
											disabled={blocked || newUnique.split(',').filter((c) => c.trim()).length < 2}
											onclick={() => {
												draft.unique_constraints.push(newUnique.split(',').map((c) => c.trim()).filter(Boolean));
												newUnique = '';
											}}>Add</Button
										>
									</div>
								</div>
							</Field.Set>
						</Field.Group>
					</Tabs.Content>
				</div>
			</Tabs.Root>
		</div>

		{#if dirty && problems.length}
			<div class={cn('flex items-center gap-2 border-t px-4 py-2 text-sm text-destructive')}>
				<TriangleAlertIcon class="size-4" /> {problems[0]}
			</div>
		{/if}
	{/if}
</div>
