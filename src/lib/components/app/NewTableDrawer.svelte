<script lang="ts">
	// Types
	import type { FieldDef } from '#lib/api/types.js';

	// Utils
	import { tick } from 'svelte';
	import { fieldTypeLabel } from '#lib/logic/field-types.js';
	import { nextFreeName } from '#lib/logic/table-draft.js';
	import {
		QUICK_FIELDS,
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
	import StepDrawer from './StepDrawer.svelte';

	// Icons
	import {
		KeyRound as KeyRoundIcon,
		ChevronRight as ChevronRightIcon,
		ListPlus as ListPlusIcon,
		Plus as PlusIcon,
		Sparkles as SparklesIcon,
		Table2 as Table2Icon,
		X as XIcon,
	} from '@lucide/svelte';

	// Script
	/** Creates a table in three steps: its name, its fields, then a last look. */
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

	const STEPS = [
		{ label: 'Name', icon: Table2Icon },
		{ label: 'Fields', icon: ListPlusIcon },
		{ label: 'Review', icon: SparklesIcon }
	];

	let step = $state(0);
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
			step = 0;
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

	/** Quick add puts a ready-made field straight in; "Add field" walks through its steps first. */
	function addField(f?: WizardField) {
		if (f) {
			fields.push(wizardField(f));
			return;
		}
		editing = { index: null, field: wizardField({ name: nextFreeName('Field', ['ID', ...fields.map((x) => x.name)]), type: 'short_text' }) };
		fieldOpen = true;
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

	function next(at: number) {
		touched = true;
		if (at === 0 && nameProblem) return false;
		if (at === 1 && !fieldsOk) return false;
		touched = false;
	}

	async function submit() {
		if (busy || nameProblem || !fieldsOk) return;
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
	bind:step
	title="New table"
	description="A table holds one kind of thing, like customers or orders."
	steps={STEPS}
	finishLabel="Create table"
	{busy}
	onNext={next}
	nextLabel={step === 1 && !fields.length ? 'Skip for now' : undefined}
	onFinish={submit}
>
	{#if step === 0}
		<Field.Field data-invalid={touched && !!nameProblem}>
			<Field.Label for="nt-name">What will this table hold?</Field.Label>
			<Input
				id="nt-name"
				bind:ref={nameInput}
				bind:value={name}
				placeholder="e.g. Customers"
				autocomplete="off"
				aria-invalid={touched && !!nameProblem}
			/>
			{#if touched && nameProblem}
				<Field.Error>{nameProblem}</Field.Error>
			{:else}
				<Field.Description>Use a plural name, like “Customers” or “Orders”.</Field.Description>
			{/if}
		</Field.Field>
	{:else if step === 1}
		<div class="flex flex-col gap-4">
			<div class="flex flex-col gap-2">
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
					<div class="flex items-center gap-1 rounded-md border pe-1 has-[button.field-card:hover]:border-ring/60" data-invalid={problem ? true : undefined}>
						<button type="button" class="field-card flex min-w-0 flex-1 items-center gap-3 px-3 py-2 text-start" onclick={() => editField(i)}>
							<span class="flex size-8 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground vivid:bg-tone-blue/15 vivid:text-tone-blue">
								<FieldTypeIcon type={f.field_type.type} class="size-4" />
							</span>
							<span class="flex min-w-0 flex-1 flex-col gap-1">
								<span class="flex min-w-0 items-baseline gap-2">
									<span class="truncate text-sm font-medium">{f.name.trim() || 'Untitled'}</span>
									<span class="shrink-0 text-xs text-muted-foreground">{fieldTypeLabel(f.field_type)}</span>
								</span>
								{#if problem}
									<span class="text-xs text-destructive">{problem}</span>
								{:else if badges.length}
									<span class="flex flex-wrap gap-1">{#each badges as b (b)}<Badge variant="secondary">{b}</Badge>{/each}</span>
								{/if}
							</span>
							<span class="hidden shrink-0 text-xs text-muted-foreground sm:inline">Edit</span>
							<ChevronRightIcon class="size-4 shrink-0 text-muted-foreground" />
						</button>
						<Button variant="ghost" size="icon-sm" aria-label="Remove {f.name || 'field'}" onclick={() => removeField(i)}>
							<XIcon />
						</Button>
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
			<p class="text-xs text-muted-foreground">Click a field to change its name, type and rules, step by step. You can add fields and links later on the Diagram.</p>
		</div>
	{:else}
		<div class="flex flex-col gap-3">
			<div class="flex items-center gap-3 rounded-lg border bg-card p-3">
				<span class="flex size-10 items-center justify-center rounded-lg bg-muted vivid:bg-tone-blue/15 vivid:text-tone-blue">
					<Table2Icon class="size-5" />
				</span>
				<div class="flex min-w-0 flex-col">
					<span class="truncate font-semibold">{name.trim()}</span>
					<span class="text-xs text-muted-foreground">{fields.length + 1} field{fields.length ? 's' : ''}</span>
				</div>
			</div>
			<ul class="flex flex-col divide-y rounded-lg border">
				<li class="flex items-center gap-2 px-3 py-2 text-sm">
					<KeyRoundIcon class="size-4 text-warning" /> ID <span class="ms-auto text-xs text-muted-foreground">Auto ID</span>
				</li>
				{#each fields as f, i (i)}
					<li class="flex flex-wrap items-center gap-2 px-3 py-2 text-sm">
						<FieldTypeIcon type={f.field_type.type} class="size-4 text-muted-foreground" />
						{f.name.trim()}
						{#each fieldOptionBadges(f) as b (b)}<Badge variant="outline">{b}</Badge>{/each}
						<span class="ms-auto text-xs text-muted-foreground">{fieldTypeLabel(f.field_type)}</span>
					</li>
				{/each}
			</ul>
		</div>
	{/if}

	<DraftFieldDrawer
		bind:open={fieldOpen}
		field={editing.field}
		table={name}
		taken={takenNames}
		isNew={editing.index === null}
		onSave={saveField}
	/>
</StepDrawer>
