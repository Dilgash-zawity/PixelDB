<script lang="ts">
	// Types
	import type { FieldDef, FieldTypeName } from '#lib/api/types.js';

	// Utils
	import { tick } from 'svelte';
	import { makeFieldType } from '#lib/logic/field-types.js';
	import { nextFreeName } from '#lib/logic/table-draft.js';
	import {
		QUICK_FIELDS,
		WIZARD_EXCLUDED_TYPES,
		fieldNameProblems,
		fieldOptionBadges,
		wizardField,
		type WizardField
	} from '#lib/logic/table-wizard.js';
	import { jsonClone } from '#lib/logic/values.js';
	import { session } from '#lib/state/session.svelte.js';

	// Components
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import DraftFieldDrawer from './DraftFieldDrawer.svelte';
	import FieldTypeIcon from './FieldTypeIcon.svelte';
	import FieldTypeSelect from './FieldTypeSelect.svelte';
	import StepDrawer from './StepDrawer.svelte';

	// Icons
	import { KeyRound as KeyRoundIcon, Plus as PlusIcon, Settings2 as Settings2Icon, X as XIcon } from '@lucide/svelte';

	// Script
	/** Creates a table on one page: its name and its fields, each editable in place (rules in a drawer on top). */
	let {
		open = $bindable(false),
		initial = '',
		/** Creates the table; returns true when the drawer should close. */
		onCreate
	}: {
		open?: boolean;
		initial?: string;
		onCreate: (name: string, fields: FieldDef[]) => Promise<boolean>;
	} = $props();

	let name = $state('');
	let fields = $state<FieldDef[]>([]);
	/** The field being set up in its own drawer: an index into `fields`, or null for a new one. */
	let editing = $state<{ index: number | null; field: FieldDef }>({ index: null, field: wizardField({ name: '', type: 'short_text' }) });
	let fieldOpen = $state(false);
	let touched = $state(false);
	let busy = $state(false);
	let nameInput = $state<HTMLInputElement | null>(null);

	// Start over each time it opens.
	$effect(() => {
		if (open) {
			name = initial;
			fields = [];
			fieldOpen = false;
			touched = false;
			tick().then(() => nameInput?.select());
		}
	});

	const nameProblem = $derived(session.tableNameProblem(name.trim()));
	const problems = $derived(fieldNameProblems(fields.map((f) => f.name)));
	const fieldsOk = $derived(problems.every((p) => p === null));
	const unused = $derived(QUICK_FIELDS.filter((q) => !fields.some((f) => f.name.trim().toLowerCase() === q.name.toLowerCase())));

	let list = $state<HTMLDivElement | null>(null);

	/** Adds a ready-made field, or a new text field whose name is selected for typing. */
	async function addField(f?: WizardField) {
		fields.push(wizardField(f ?? { name: nextFreeName('Field', ['ID', ...fields.map((x) => x.name)]), type: 'short_text' }));
		if (f) return;
		await tick();
		list?.querySelector<HTMLInputElement>('[data-field-row]:last-child input')?.select();
	}

	function setType(i: number, v: FieldTypeName) {
		if (fields[i].field_type.type === v) return;
		fields[i].field_type = makeFieldType(v);
		// A default typed for the old type rarely fits the new one.
		fields[i].default = null;
	}

	function editField(i: number) {
		editing = { index: i, field: fields[i] };
		fieldOpen = true;
	}

	function saveField(f: FieldDef) {
		if (editing.index === null) fields.push(f);
		else fields[editing.index] = f;
	}

	/** Names a field can't take: the ID and every other field. */
	const takenNames = $derived(['ID', ...fields.filter((_, i) => i !== editing.index).map((f) => f.name)]);

	function removeField(i: number) {
		fields.splice(i, 1);
	}

	async function submit() {
		touched = true;
		if (nameProblem) return nameInput?.focus();
		if (busy || !fieldsOk) return;
		busy = true;
		try {
			if (await onCreate(name.trim(), jsonClone(fields))) open = false;
		} finally {
			busy = false;
		}
	}
</script>

<StepDrawer
	bind:open
	title="New table"
	description="A table holds one kind of thing, like customers or orders."
	finishLabel="Create table"
	{busy}
	disabled={!fieldsOk}
	onFinish={submit}
>
	<div class="flex flex-col gap-6">
		<Field.Field data-invalid={touched && !!nameProblem}>
			<Field.Label for="nt-name">Table name</Field.Label>
			<Input
				id="nt-name"
				bind:ref={nameInput}
				bind:value={name}
				placeholder="e.g. Customers"
				autocomplete="off"
				aria-invalid={touched && !!nameProblem}
				onblur={() => (touched = true)}
			/>
			{#if touched && nameProblem}
				<Field.Error>{nameProblem}</Field.Error>
			{:else}
				<Field.Description>Use a plural name, like “Customers” or “Orders”.</Field.Description>
			{/if}
		</Field.Field>

		<Field.Set>
			<Field.Legend variant="label">Fields</Field.Legend>
			<Field.Description>Name each field and pick its type. You can add more fields and links later on the Diagram.</Field.Description>
			<div class="flex flex-col gap-2" bind:this={list}>
				<!-- The automatic ID is always there. -->
				<div class="flex items-center gap-2 rounded-md border border-dashed px-3 py-2 text-sm text-muted-foreground">
					<KeyRoundIcon class="size-4 text-warning" />
					<span class="font-medium text-foreground">ID</span>
					<span class="flex-1">numbers each record for you</span>
					<Badge variant="secondary">Auto ID</Badge>
				</div>
				{#each fields as f, i (i)}
					{@const problem = problems[i]}
					{@const badges = fieldOptionBadges(f)}
					<div class="flex flex-col gap-1.5 rounded-md border p-2" data-field-row data-invalid={problem ? true : undefined}>
						<div class="flex flex-wrap items-center gap-2">
							<Input
								class="h-8 min-w-40 flex-1"
								bind:value={f.name}
								aria-label="Field name"
								aria-invalid={!!problem}
								autocomplete="off"
								onkeydown={(e) => {
									// Enter adds the next field rather than creating the table.
									if (e.key === 'Enter' && !e.metaKey && !e.ctrlKey) {
										e.preventDefault();
										addField();
									}
								}}
							/>
							<div class="w-44">
								<FieldTypeSelect value={f.field_type.type} exclude={WIZARD_EXCLUDED_TYPES} onChange={(v) => setType(i, v)} />
							</div>
							<Button variant="ghost" size="sm" onclick={() => editField(i)}>
								<Settings2Icon data-icon="inline-start" /> Rules{badges.length ? ` (${badges.length})` : ''}
							</Button>
							<Button variant="ghost" size="icon-sm" aria-label="Remove {f.name || 'field'}" onclick={() => removeField(i)}>
								<XIcon />
							</Button>
						</div>
						{#if problem}
							<span class="px-1 text-xs text-destructive">{problem}</span>
						{:else if badges.length}
							<span class="flex flex-wrap gap-1">{#each badges as b (b)}<Badge variant="secondary">{b}</Badge>{/each}</span>
						{/if}
					</div>
				{/each}
				<Button variant="outline" class="self-start" onclick={() => addField()}>
					<PlusIcon data-icon="inline-start" /> Add field
				</Button>
			</div>
			{#if unused.length}
				<div class="flex flex-col gap-2">
					<span class="text-xs font-medium text-muted-foreground">Quick add</span>
					<div class="flex flex-wrap gap-1.5">
						{#each unused as q (q.name)}
							<Button variant="secondary" size="sm" onclick={() => addField({ ...q })}>
								<FieldTypeIcon type={q.type} data-icon="inline-start" />
								{q.name}
							</Button>
						{/each}
					</div>
				</div>
			{/if}
		</Field.Set>
	</div>

	<DraftFieldDrawer
		bind:open={fieldOpen}
		field={editing.field}
		table={name}
		taken={takenNames}
		isNew={editing.index === null}
		onSave={saveField}
	/>
</StepDrawer>
