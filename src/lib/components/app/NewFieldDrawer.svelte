<script lang="ts">
	// Services
	import { tableService } from '#lib/services/tables.js';

	// Types
	import type { FieldDef, FieldTypeName, TableDef, TableInfo } from '#lib/api/types.js';

	// Utils
	import { tick, untrack } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { makeFieldType } from '#lib/logic/field-types.js';
	import { type Cardinality, describeRelationship, fieldDesign, keyColumns, linkDesign } from '#lib/logic/relationships.js';
	import { nextFreeName } from '#lib/logic/table-draft.js';
	import { WIZARD_EXCLUDED_TYPES, fieldNameProblems } from '#lib/logic/table-wizard.js';
	import { jsonClone } from '#lib/logic/values.js';
	import { attempt } from '#lib/state/feedback.svelte.js';
	import { session } from '#lib/state/session.svelte.js';

	// Components
	import * as Alert from '#lib/components/ui/alert/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import FieldProperties from './FieldProperties.svelte';
	import FieldTypeCards from './FieldTypeCards.svelte';
	import StepDrawer from './StepDrawer.svelte';

	// Icons
	import {
		Link as LinkIcon,
		Shapes as ShapesIcon,
		ShieldCheck as ShieldCheckIcon,
		TextCursorInput as TextCursorInputIcon,
		TriangleAlert as TriangleAlertIcon,
	} from '@lucide/svelte';

	// Script
	/** Adds a field to `table` in three steps: name, type, then optional rules (or, for a relationship, the table it points to). */
	let {
		open = $bindable(false),
		table,
		onSaved
	}: {
		open?: boolean;
		table: string;
		onSaved?: (name: string) => void;
	} = $props();

	// A relationship is offered here too: its last step picks the table instead of the rules.
	const EXCLUDED = WIZARD_EXCLUDED_TYPES.filter((t) => t !== 'lookup');
	const KINDS: { value: Cardinality; label: string }[] = [
		{ value: 'one_to_many', label: 'Many can point to the same one' },
		{ value: 'one_to_one', label: 'Each one at most once' }
	];

	let step = $state(0);
	let info = $state<TableInfo | null>(null);
	let edit = $state<FieldDef | null>(null);
	let touched = $state(false);
	let busy = $state(false);
	let nameInput = $state<HTMLInputElement | null>(null);
	/** The relationship's target when the type is Relationship. */
	let link = $state<{ parent: string; kind: Cardinality; required: boolean }>({ parent: '', kind: 'one_to_many', required: false });
	let parentDef = $state<TableDef | null>(null);

	const isLink = $derived(edit?.field_type.type === 'lookup');
	const steps = $derived([
		{ label: 'Name', icon: TextCursorInputIcon },
		{ label: 'Type', icon: ShapesIcon },
		isLink ? { label: 'Link', icon: LinkIcon } : { label: 'Rules', icon: ShieldCheckIcon }
	]);
	const tableItems = $derived(session.tables.map((t) => ({ value: t.name, label: t.name })));
	const parentKey = $derived(parentDef ? keyColumns(parentDef)[0] : undefined);
	const linkProblem = $derived.by(() => {
		if (!link.parent) return 'Choose the table this field points to';
		if (!parentDef) return null;
		if (!parentKey) return `“${parentDef.name}” has no ID or unique field to point to`;
		return null;
	});

	$effect(() => {
		if (open) {
			const t = table;
			untrack(() => load(t));
		}
	});

	async function load(t: string) {
		step = 0;
		touched = false;
		info = null;
		edit = null;
		link = { parent: '', kind: 'one_to_many', required: false };
		parentDef = null;
		const loaded = await attempt(() => tableService.get(t));
		if (!loaded) return;
		info = loaded;
		edit = { name: nextFreeName('Field', loaded.definition.fields.map((x) => x.name)), field_type: makeFieldType('short_text') };
		await tick();
		nameInput?.select();
	}

	const blocked = $derived(
		info?.unsupported_features.length
			? `“${table}” was made by another program and uses ${info.unsupported_features.join(', ')}, so its fields can't be changed here.`
			: null
	);
	const nameProblem = $derived(
		edit && info ? fieldNameProblems([edit.name], info.definition.fields.map((f) => f.name))[0] : null
	);

	function setType(v: FieldTypeName) {
		if (edit && edit.field_type.type !== v) edit.field_type = makeFieldType(v);
	}

	async function setParent(name: string) {
		link.parent = name;
		parentDef = null;
		const loaded = await attempt(() => tableService.get(name));
		if (loaded && link.parent === name) parentDef = loaded.definition;
	}

	function next(at: number) {
		if (at === 0) {
			touched = true;
			if (nameProblem) return false;
		}
		// Suggest another table to point to, else this one (e.g. an employee's manager).
		if (at === 1 && isLink && !link.parent) {
			const other = session.tables.find((t) => t.name !== table) ?? session.tables[0];
			if (other) setParent(other.name);
		}
	}

	async function save(design: ReturnType<typeof fieldDesign>, name: string, message: string) {
		if (Array.isArray(design)) {
			toast.error('Check the field', { description: design.join(' · ') });
			return;
		}
		busy = true;
		try {
			const result = await attempt(() => tableService.saveDesign(table, design, false));
			if (!result) return;
			open = false;
			toast.success(message, { description: result.warnings.join(' ') || undefined });
			onSaved?.(name);
		} finally {
			busy = false;
		}
	}

	async function submit() {
		if (!info || !edit || blocked || busy) return;
		const name = edit.name.trim();
		if (!isLink) {
			const f = jsonClone(edit);
			f.name = name;
			return save(fieldDesign(info.definition, null, f), name, `Added “${name}” to ${table}`);
		}
		if (linkProblem || !parentDef || !parentKey) return;
		const design = linkDesign(info.definition, {
			parent: parentDef,
			parentColumn: parentKey,
			childColumn: null,
			newField: name,
			cardinality: link.kind,
			onUpdate: 'cascade',
			onDelete: 'restrict',
			asDropdown: true
		});
		const added = design.definition.fields.find((f) => f.name === name);
		if (added) added.required = link.required;
		await save(design, name, `Linked ${table} to ${parentDef.name}`);
	}
</script>

<StepDrawer
	bind:open
	bind:step
	title="New field"
	description="In {table}"
	{steps}
	finishLabel={isLink ? 'Link' : 'Add field'}
	finishIcon={isLink ? LinkIcon : undefined}
	{busy}
	disabled={!edit || !!blocked || (step === 2 && isLink && (!!linkProblem || !parentDef))}
	onNext={next}
	onFinish={submit}
>
	{#if !edit}
		<div class="flex flex-col gap-3">{#each [1, 2] as i (i)}<Skeleton class="h-9" />{/each}</div>
	{:else if blocked}
		<Alert.Root class="border-warning/40 bg-warning/10 *:[svg]:text-warning">
			<TriangleAlertIcon />
			<Alert.Title class="text-warning">Read-only</Alert.Title>
			<Alert.Description>{blocked}</Alert.Description>
		</Alert.Root>
	{:else if step === 0}
		<Field.Field data-invalid={touched && !!nameProblem}>
			<Field.Label for="nf-name">What should this field be called?</Field.Label>
			<Input id="nf-name" bind:ref={nameInput} bind:value={edit.name} placeholder="e.g. Email" autocomplete="off" aria-invalid={touched && !!nameProblem} />
			{#if touched && nameProblem}
				<Field.Error>{nameProblem}</Field.Error>
			{:else}
				<Field.Description>Short and clear, like “Email” or “Due date”.</Field.Description>
			{/if}
		</Field.Field>
	{:else if step === 1}
		<FieldTypeCards
			value={edit.field_type.type}
			exclude={EXCLUDED}
			onChange={setType}
			onPick={() => next(1) !== false && (step = 2)}
		/>
	{:else if isLink}
		<Field.Group class="gap-5">
			<Field.Field data-invalid={!!link.parent && !!linkProblem}>
				<Field.Label for="nf-parent">Which table does “{edit.name.trim()}” point to?</Field.Label>
				<Select.Root type="single" items={tableItems} value={link.parent} onValueChange={setParent}>
					<Select.Trigger id="nf-parent" class="w-full"><Select.Value placeholder="Choose a table" /></Select.Trigger>
					<Select.Content>
						<Select.Group>
							{#each tableItems as item (item.value)}<Select.Item value={item.value} label={item.label}>{item.label}</Select.Item>{/each}
						</Select.Group>
					</Select.Content>
				</Select.Root>
				{#if link.parent && linkProblem}
					<Field.Error>{linkProblem}</Field.Error>
				{:else}
					<Field.Description>Each {table} record picks one record from it in a dropdown.</Field.Description>
				{/if}
			</Field.Field>
			<Field.Field>
				<Field.Label for="nf-kind">How many {table} records can pick the same one?</Field.Label>
				<Select.Root type="single" items={KINDS} value={link.kind} onValueChange={(v) => (link.kind = v as Cardinality)}>
					<Select.Trigger id="nf-kind" class="w-full"><Select.Value /></Select.Trigger>
					<Select.Content>
						<Select.Group>
							{#each KINDS as k (k.value)}<Select.Item value={k.value} label={k.label}>{k.label}</Select.Item>{/each}
						</Select.Group>
					</Select.Content>
				</Select.Root>
				{#if link.parent}
					<Field.Description>{describeRelationship({ parent: link.parent, child: table, cardinality: link.kind })}</Field.Description>
				{/if}
			</Field.Field>
			<Field.Field orientation="horizontal">
				<Field.Content>
					<Field.Label for="nf-required">Required</Field.Label>
					<Field.Description>Every {table} record must pick one.</Field.Description>
				</Field.Content>
				<Switch id="nf-required" checked={link.required} onCheckedChange={(v) => (link.required = v)} />
			</Field.Field>
		</Field.Group>
	{:else}
		<div class="flex flex-col gap-4">
			<p class="text-sm text-muted-foreground">All optional — click “Add field” to keep the defaults.</p>
			<FieldProperties field={edit} />
		</div>
	{/if}
</StepDrawer>
