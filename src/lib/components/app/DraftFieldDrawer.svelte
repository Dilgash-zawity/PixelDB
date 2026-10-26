<script lang="ts">
	// Types
	import type { FieldDef, FieldTypeName } from '#lib/api/types.js';

	// Utils
	import { tick, untrack } from 'svelte';
	import { makeFieldType } from '#lib/logic/field-types.js';
	import { WIZARD_EXCLUDED_TYPES, fieldNameProblems } from '#lib/logic/table-wizard.js';
	import { jsonClone } from '#lib/logic/values.js';

	// Components
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import FieldProperties from './FieldProperties.svelte';
	import FieldTypeCards from './FieldTypeCards.svelte';
	import StepDrawer from './StepDrawer.svelte';

	// Icons
	import { Check as CheckIcon } from '@lucide/svelte';

	// Script
	/**
	 * Sets up one field of a table that doesn't exist yet, on top of the New
	 * table drawer: its name, type and optional rules on one page. Nothing is saved
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

	let edit = $state<FieldDef>({ name: '', field_type: { type: 'short_text' } });
	let touched = $state(false);
	let nameInput = $state<HTMLInputElement | null>(null);

	// A copy each time it opens, so Cancel leaves the field as it was.
	$effect(() => {
		if (!open) return;
		untrack(() => {
			edit = jsonClone(field);
			touched = false;
		});
		tick().then(() => nameInput?.select());
	});

	const nameProblem = $derived(fieldNameProblems([edit.name], taken)[0]);

	function setType(v: FieldTypeName) {
		if (edit.field_type.type === v) return;
		edit.field_type = makeFieldType(v);
		// A default typed for the old type rarely fits the new one.
		edit.default = null;
	}

	function finish() {
		touched = true;
		if (nameProblem) {
			nameInput?.focus();
			return;
		}
		onSave({ ...jsonClone(edit), name: edit.name.trim() });
		open = false;
	}
</script>

<StepDrawer
	nested
	bind:open
	title={isNew ? 'Add a field' : `Field “${field.name.trim() || 'Untitled'}”`}
	description={table.trim() ? `In ${table.trim()}` : undefined}
	finishLabel={isNew ? 'Add field' : 'Done'}
	finishIcon={CheckIcon}
	onFinish={finish}
>
	<div class="flex flex-col gap-6">
		<Field.Field data-invalid={touched && !!nameProblem}>
			<Field.Label for="df-name">Name</Field.Label>
			<Input id="df-name" bind:ref={nameInput} bind:value={edit.name} placeholder="e.g. Email" autocomplete="off" aria-invalid={touched && !!nameProblem} onblur={() => (touched = true)} />
			{#if touched && nameProblem}
				<Field.Error>{nameProblem}</Field.Error>
			{:else}
				<Field.Description>Short and clear, like “Email” or “Due date”.</Field.Description>
			{/if}
		</Field.Field>
		<Field.Set>
			<Field.Legend variant="label">Type</Field.Legend>
			<FieldTypeCards value={edit.field_type.type} exclude={WIZARD_EXCLUDED_TYPES} onChange={setType} />
		</Field.Set>
		<Field.Set>
			<Field.Legend variant="label">Rules</Field.Legend>
			<Field.Description>All optional.</Field.Description>
			<FieldProperties field={edit} />
		</Field.Set>
	</div>
</StepDrawer>
