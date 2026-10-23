<script lang="ts">
	// Types
	import type { FieldDef, FieldTypeName } from '#lib/api/types.js';

	// Utils
	import { tick, untrack } from 'svelte';
	import { fieldTypeLabel, makeFieldType } from '#lib/logic/field-types.js';
	import { WIZARD_EXCLUDED_TYPES, fieldNameProblems, fieldOptionBadges } from '#lib/logic/table-wizard.js';
	import { jsonClone } from '#lib/logic/values.js';

	// Components
	import { Badge } from '#lib/components/ui/badge/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import FieldProperties from './FieldProperties.svelte';
	import FieldTypeCards from './FieldTypeCards.svelte';
	import FieldTypeIcon from './FieldTypeIcon.svelte';
	import StepDrawer from './StepDrawer.svelte';

	// Icons
	import {
		Check as CheckIcon,
		Shapes as ShapesIcon,
		ShieldCheck as ShieldCheckIcon,
		TextCursorInput as TextCursorInputIcon,
	} from '@lucide/svelte';

	// Script
	/**
	 * Sets up one field of a table that doesn't exist yet, on top of the New
	 * table drawer: its name, its type, then optional rules. Nothing is saved
	 * until the table is created; `onSave` gets the finished field.
	 */
	let {
		open = $bindable(false),
		field,
		table,
		taken,
		isNew = false,
		onSave
	}: {
		open?: boolean;
		/** The field to change; a fresh one for "Add field". */
		field: FieldDef;
		/** Name of the table being made, for the wording. */
		table: string;
		/** Names the other fields already use (including ID). */
		taken: string[];
		/** Whether this adds a new field (else it changes one). */
		isNew?: boolean;
		onSave: (field: FieldDef) => void;
	} = $props();

	const STEPS = [
		{ label: 'Name', icon: TextCursorInputIcon },
		{ label: 'Type', icon: ShapesIcon },
		{ label: 'Rules', icon: ShieldCheckIcon }
	];

	let step = $state(0);
	let edit = $state<FieldDef>({ name: '', field_type: { type: 'short_text' } });
	let touched = $state(false);
	let nameInput = $state<HTMLInputElement | null>(null);

	// A copy each time it opens, so Cancel leaves the field as it was.
	$effect(() => {
		if (!open) return;
		untrack(() => {
			edit = jsonClone(field);
			step = 0;
			touched = false;
		});
		tick().then(() => nameInput?.select());
	});

	const nameProblem = $derived(fieldNameProblems([edit.name], taken)[0]);
	const badges = $derived(fieldOptionBadges(edit));

	function setType(v: FieldTypeName) {
		if (edit.field_type.type === v) return;
		edit.field_type = makeFieldType(v);
		// A default typed for the old type rarely fits the new one.
		edit.default = null;
	}

	function next(at: number) {
		if (at === 0) {
			touched = true;
			if (nameProblem) return false;
		}
	}

	function finish() {
		touched = true;
		if (nameProblem) {
			step = 0;
			return;
		}
		onSave({ ...jsonClone(edit), name: edit.name.trim() });
		open = false;
	}
</script>

<StepDrawer
	nested
	bind:open
	bind:step
	title={isNew ? 'Add a field' : `Field “${field.name.trim() || 'Untitled'}”`}
	description={table.trim() ? `In ${table.trim()}` : undefined}
	steps={STEPS}
	finishLabel={isNew ? 'Add field' : 'Done'}
	finishIcon={CheckIcon}
	onNext={next}
	onFinish={finish}
>
	{#if step === 0}
		<Field.Field data-invalid={touched && !!nameProblem}>
			<Field.Label for="df-name">What should this field be called?</Field.Label>
			<Input id="df-name" bind:ref={nameInput} bind:value={edit.name} placeholder="e.g. Email" autocomplete="off" aria-invalid={touched && !!nameProblem} />
			{#if touched && nameProblem}
				<Field.Error>{nameProblem}</Field.Error>
			{:else}
				<Field.Description>Short and clear, like “Email” or “Due date”.</Field.Description>
			{/if}
		</Field.Field>
	{:else if step === 1}
		<div class="flex flex-col gap-3">
			<p class="text-sm text-muted-foreground">What kind of value does “{edit.name.trim()}” hold? Double-click a card to choose it and go on.</p>
			<FieldTypeCards value={edit.field_type.type} exclude={WIZARD_EXCLUDED_TYPES} onChange={setType} onPick={() => (step = 2)} />
		</div>
	{:else}
		<div class="flex flex-col gap-4">
			<div class="flex items-center gap-3 rounded-lg border bg-card p-3">
				<span class="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground vivid:bg-tone-blue/15 vivid:text-tone-blue">
					<FieldTypeIcon type={edit.field_type.type} class="size-4" />
				</span>
				<div class="flex min-w-0 flex-1 flex-col">
					<span class="truncate font-medium">{edit.name.trim()}</span>
					<span class="text-xs text-muted-foreground">{fieldTypeLabel(edit.field_type)}</span>
				</div>
				{#if badges.length}
					<div class="flex flex-wrap justify-end gap-1">{#each badges as b (b)}<Badge variant="secondary">{b}</Badge>{/each}</div>
				{/if}
			</div>
			<p class="text-sm text-muted-foreground">All optional — click “{isNew ? 'Add field' : 'Done'}” to keep the defaults.</p>
			<FieldProperties field={edit} />
		</div>
	{/if}
</StepDrawer>
