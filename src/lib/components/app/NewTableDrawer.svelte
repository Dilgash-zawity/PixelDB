<script lang="ts">
	// Types
	import type { FieldDef, FieldTypeName } from '#lib/api/types.js';

	// Utils
	import { tick } from 'svelte';
	import { fieldTypeLabel, makeFieldType } from '#lib/logic/field-types.js';
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
	import FieldProperties from './FieldProperties.svelte';
	import FieldTypeIcon from './FieldTypeIcon.svelte';
	import FieldTypeSelect from './FieldTypeSelect.svelte';
	import StepDrawer from './StepDrawer.svelte';

	// Icons
	import {
		KeyRound as KeyRoundIcon,
		ListPlus as ListPlusIcon,
		Plus as PlusIcon,
		SlidersHorizontal as SlidersHorizontalIcon,
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
	/** The field whose options are showing; one at a time keeps the list short. */
	let expanded = $state<number | null>(null);
	let touched = $state(false);
	let busy = $state(false);
	let nameInput = $state<HTMLInputElement | null>(null);

	// Start over each time it opens.
	$effect(() => {
		if (open) {
			step = 0;
			name = initial;
			fields = [];
			expanded = null;
			touched = false;
			tick().then(() => nameInput?.select());
		}
	});

	const nameProblem = $derived(session.tableNameProblem(name.trim()));
	const problems = $derived(fieldNameProblems(fields.map((f) => f.name)));
	const fieldsOk = $derived(problems.every((p) => p === null));
	const unused = $derived(QUICK_FIELDS.filter((q) => !fields.some((f) => f.name.trim().toLowerCase() === q.name.toLowerCase())));

	async function addField(f?: WizardField) {
		fields.push(wizardField(f ?? { name: nextFreeName('Field', ['ID', ...fields.map((x) => x.name)]), type: 'short_text' }));
		if (f) return;
		await tick();
		document.querySelector<HTMLInputElement>(`#nt-field-${fields.length - 1}`)?.select();
	}

	function setType(f: FieldDef, v: FieldTypeName) {
		if (f.field_type.type === v) return;
		f.field_type = makeFieldType(v);
		f.default = null;
	}

	function removeField(i: number) {
		fields.splice(i, 1);
		if (expanded === i) expanded = null;
		else if (expanded !== null && expanded > i) expanded--;
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
					{@const problem = touched || f.name.trim() ? problems[i] : null}
					{@const open = expanded === i}
					{@const badges = fieldOptionBadges(f)}
					<!-- The name box and message show the problem; the type and options stay their normal color. -->
					<Field.Field class="gap-1.5 rounded-md border p-2">
						<div class="flex items-center gap-2">
							<Input
								id="nt-field-{i}"
								class="flex-1"
								bind:value={f.name}
								aria-label="Field name"
								aria-invalid={!!problem}
								autocomplete="off"
							/>
							<div class="w-44 shrink-0">
								<FieldTypeSelect value={f.field_type.type} exclude={WIZARD_EXCLUDED_TYPES} onChange={(v) => setType(f, v)} />
							</div>
							<Button
								variant={open ? 'secondary' : 'ghost'}
								size="sm"
								aria-expanded={open}
								aria-controls="nt-options-{i}"
								onclick={() => (expanded = open ? null : i)}
							>
								<SlidersHorizontalIcon data-icon="inline-start" /> Options
							</Button>
							<Button variant="ghost" size="icon-sm" aria-label="Remove {f.name || 'field'}" onclick={() => removeField(i)}>
								<XIcon />
							</Button>
						</div>
						{#if problem}<Field.Error>{problem}</Field.Error>{/if}
						{#if open}
							<div id="nt-options-{i}" class="rounded-md bg-muted/40 p-3">
								<FieldProperties field={f} />
							</div>
						{:else if badges.length}
							<div class="flex flex-wrap gap-1">
								{#each badges as b (b)}<Badge variant="secondary">{b}</Badge>{/each}
							</div>
						{/if}
					</Field.Field>
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
			<p class="text-xs text-muted-foreground">Use Options for a maximum length, required, no duplicates and more. You can add fields and links later on the Diagram.</p>
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
</StepDrawer>
