<script lang="ts">
	// Types
	import type { FkAction, TableDef, TableInfo } from '#lib/api/types.js';

	// Utils
	import {
		type LinkDraft,
		type LinkKind,
		type Relationship,
		canBeDropdown,
		describeRelationship,
		junctionName,
		keyColumns,
		suggestFieldName,
		typeMismatch
	} from '#lib/logic/relationships.js';
	import { session } from '#lib/state/session.svelte.js';
	import { cn } from '#lib/utils.js';

	// Components
	import * as Alert from '#lib/components/ui/alert/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import StepDrawer from './StepDrawer.svelte';

	// Icons
	import {
		ChevronDown as ChevronDownIcon,
		ChevronRight as ChevronRightIcon,
		Link as LinkIcon,
		ListChecks as ListChecksIcon,
		Shapes as ShapesIcon,
		Table2 as Table2Icon,
		TriangleAlert as TriangleAlertIcon,
	} from '@lucide/svelte';

	// Script
	/** Links two tables in three steps: which tables, what kind of link, then the details. */
	let {
		open = $bindable(false),
		draft = $bindable(null),
		defs,
		infos,
		rels,
		busy = false,
		onCreate
	}: {
		open?: boolean;
		draft?: LinkDraft | null;
		defs: TableDef[];
		infos: Record<string, TableInfo>;
		rels: Relationship[];
		busy?: boolean;
		onCreate: () => void;
	} = $props();

	const STEPS = [
		{ label: 'Tables', icon: Table2Icon },
		{ label: 'Kind', icon: ShapesIcon },
		{ label: 'Details', icon: ListChecksIcon }
	];
	const NEW = '\u0000new';
	const KINDS: LinkKind[] = ['one_to_many', 'one_to_one', 'many_to_many'];
	const ON_DELETE: { value: FkAction; label: string }[] = [
		{ value: 'restrict', label: 'Block the delete' },
		{ value: 'cascade', label: 'Delete linked records too' },
		{ value: 'set_null', label: 'Keep them, clear the link' },
		{ value: 'no_action', label: 'Block (checked at end)' }
	];
	const ON_UPDATE: { value: FkAction; label: string }[] = [
		{ value: 'cascade', label: 'Update linked records too' },
		{ value: 'restrict', label: 'Block the change' },
		{ value: 'set_null', label: 'Clear the link' },
		{ value: 'no_action', label: 'Block (checked at end)' }
	];

	let step = $state(0);
	/** Advanced settings stay hidden until asked for. */
	let more = $state(false);

	// Start over each time it opens.
	$effect(() => {
		if (open) {
			step = 0;
			more = false;
		}
	});

	const byName = $derived(new Map(defs.map((d) => [d.name, d])));
	const tableItems = $derived(defs.map((d) => ({ value: d.name, label: d.name })));
	const parentDef = $derived(draft ? byName.get(draft.parent) : undefined);
	const childDef = $derived(draft ? byName.get(draft.child) : undefined);
	const parentKeys = $derived(parentDef ? keyColumns(parentDef) : []);
	const childFieldItems = $derived([
		{ value: NEW, label: `New field “${draft?.newField || '…'}”` },
		...(childDef?.fields ?? [])
			.filter((f) => f.field_type.type !== 'calculated' && f.field_type.type !== 'attachment')
			.map((f) => ({ value: f.name, label: f.name }))
	]);
	const childField = $derived(draft?.childColumn ? childDef?.fields.find((f) => f.name === draft?.childColumn) : undefined);
	const replaces = $derived(
		draft?.childColumn
			? rels.find((r) => r.child === draft?.child && r.childColumns.length === 1 && r.childColumns[0] === draft?.childColumn)
			: undefined
	);
	const mismatch = $derived(
		draft && parentDef && childDef && draft.kind !== 'many_to_many'
			? typeMismatch(parentDef, draft.parentColumn, childDef, draft.childColumn)
			: null
	);

	function blockedReason(name: string): string | null {
		const features = infos[name]?.unsupported_features ?? [];
		return features.length
			? `“${name}” uses ${features.join(', ')}, which this app can't rebuild, so its links can't be changed here.`
			: null;
	}

	const tablesProblem = $derived.by(() => {
		if (!draft || !parentDef || !childDef) return 'Choose both tables';
		if (draft.kind === 'many_to_many' && parentDef.name === childDef.name) return 'Choose two different tables';
		if (draft.kind !== 'many_to_many' && !parentKeys.length) return `“${parentDef.name}” has no ID or unique field to link to`;
		return null;
	});
	const detailsProblem = $derived.by(() => {
		if (tablesProblem) return tablesProblem;
		if (!draft || !parentDef || !childDef || draft.kind === 'many_to_many') return null;
		if (!parentKeys.includes(draft.parentColumn)) return `Choose a key field of “${parentDef.name}”`;
		if (draft.childColumn === null && !draft.newField.trim()) return 'Name the new field';
		if (draft.childColumn === null && childDef.fields.some((f) => f.name.toLowerCase() === draft!.newField.trim().toLowerCase())) {
			return `“${childDef.name}” already has a field called “${draft.newField.trim()}”`;
		}
		if (parentDef.name === childDef.name && draft.childColumn === draft.parentColumn) return 'A field cannot link to itself';
		return blockedReason(childDef.name);
	});

	function setParent(name: string) {
		if (!draft) return;
		const def = byName.get(name);
		draft.parent = name;
		draft.parentColumn = def ? (keyColumns(def)[0] ?? '') : '';
		resuggest();
	}

	function setChild(name: string) {
		if (!draft) return;
		draft.child = name;
		draft.childColumn = null;
		resuggest();
	}

	function resuggest() {
		if (draft && parentDef && childDef) draft.newField = suggestFieldName(parentDef.name, draft.parentColumn, childDef);
	}

	function kindLabel(kind: LinkKind, parent: string, child: string): { title: string; hint: string } {
		switch (kind) {
			case 'one_to_many':
				return { title: 'One to many', hint: `One ${parent} record can have many ${child} records.` };
			case 'one_to_one':
				return { title: 'One to one', hint: `One ${parent} record has at most one ${child} record.` };
			case 'many_to_many':
				return {
					title: 'Many to many',
					hint: `Many ${parent} ↔ many ${child}. Adds a linking table “${junctionName(parent, child, session.objects.map((o) => o.name))}”.`
				};
		}
	}

	function submit() {
		if (!detailsProblem && !busy) onCreate();
	}
</script>

{#snippet select(id: string, items: { value: string; label: string }[], value: string, onChange: (v: string) => void, placeholder = '', disabled = false)}
	<Select.Root type="single" {items} {value} onValueChange={onChange} {disabled}>
		<Select.Trigger {id} class="w-full min-w-0"><Select.Value {placeholder} /></Select.Trigger>
		<Select.Content>
			<Select.Group>
				{#each items as item (item.value)}<Select.Item value={item.value} label={item.label}>{item.label}</Select.Item>{/each}
			</Select.Group>
		</Select.Content>
	</Select.Root>
{/snippet}

<StepDrawer
	bind:open
	bind:step
	title="Link two tables"
	description="Connect records in one table to records in another."
	steps={STEPS}
	finishLabel="Link"
	finishIcon={LinkIcon}
	{busy}
	disabled={!draft || (step === 0 && !!tablesProblem) || (step === STEPS.length - 1 && !!detailsProblem)}
	onFinish={submit}
>
	{#if !draft}
		<p class="text-sm text-muted-foreground">Add a table first.</p>
	{:else if step === 0}
		<Field.Group class="gap-4">
			<Field.Field>
				<Field.Label for="nl-parent">From</Field.Label>
				{@render select('nl-parent', tableItems, draft.parent, setParent, 'Choose a table')}
				<Field.Description>The table whose records others point to, like Customers.</Field.Description>
			</Field.Field>
			<Field.Field>
				<Field.Label for="nl-child">To</Field.Label>
				{@render select('nl-child', tableItems, draft.child, setChild, 'Choose a table')}
				<Field.Description>The table that points to them, like Orders.</Field.Description>
			</Field.Field>
			{#if tablesProblem && parentDef && childDef}<Field.Error>{tablesProblem}</Field.Error>{/if}
		</Field.Group>
	{:else if step === 1}
		<div class="grid gap-2 sm:grid-cols-3" role="radiogroup" aria-label="Kind of link">
			{#each KINDS as kind (kind)}
				{@const selected = draft.kind === kind}
				{@const label = kindLabel(kind, draft.parent, draft.child)}
				<button
					type="button"
					role="radio"
					aria-checked={selected}
					class={cn(
						'flex flex-col items-start gap-1.5 rounded-lg border bg-card p-3 text-start outline-none transition-colors hover:bg-accent/50 focus-visible:ring-3 focus-visible:ring-ring/40',
						selected && 'border-primary bg-primary/5 ring-1 ring-primary'
					)}
					onclick={() => draft && (draft.kind = kind)}
					ondblclick={() => draft && ((draft.kind = kind), step++)}
				>
					<span class="text-sm font-medium">{label.title}</span>
					<span class="text-xs text-muted-foreground">{label.hint}</span>
				</button>
			{/each}
		</div>
	{:else}
		<div class="flex flex-col gap-5">
			<p class="text-sm">
				{#if draft.kind === 'many_to_many'}
					{kindLabel('many_to_many', draft.parent, draft.child).hint}
				{:else}
					{describeRelationship({ parent: draft.parent, child: draft.child, cardinality: draft.kind })}
				{/if}
			</p>
			{#if draft.kind !== 'many_to_many'}
				{#if draft.childColumn === null}
					<Field.Field>
						<Field.Label for="nl-field">New field in {draft.child}</Field.Label>
						<Input id="nl-field" bind:value={draft.newField} autocomplete="off" />
						<Field.Description>Holds which {draft.parent} record each {draft.child} record belongs to.</Field.Description>
					</Field.Field>
				{/if}
				<Button variant="ghost" size="sm" class="self-start" onclick={() => (more = !more)} aria-expanded={more}>
					{#if more}<ChevronDownIcon data-icon="inline-start" />{:else}<ChevronRightIcon data-icon="inline-start" />{/if}
					More options
				</Button>
				{#if more}
					<Field.Group class="gap-4">
						<Field.Field>
							<Field.Label for="nl-key">Key field of {draft.parent}</Field.Label>
							{@render select('nl-key', parentKeys.map((k) => ({ value: k, label: k })), draft.parentColumn, (v) => { if (draft) draft.parentColumn = v; resuggest(); }, parentKeys.length ? 'Choose a field' : 'No key fields', !parentKeys.length)}
						</Field.Field>
						<Field.Field>
							<Field.Label for="nl-by">Linked by</Field.Label>
							{@render select('nl-by', childFieldItems, draft.childColumn ?? NEW, (v) => draft && (draft.childColumn = v === NEW ? null : v))}
						</Field.Field>
						{#if draft.childColumn === null || (childField && canBeDropdown(childField))}
							<Field.Field orientation="horizontal">
								<Field.Content>
									<Field.Label for="nl-dropdown">Pick from a list</Field.Label>
									<Field.Description>Choose a {draft.parent} record from a dropdown.</Field.Description>
								</Field.Content>
								<Switch id="nl-dropdown" checked={draft.asDropdown} onCheckedChange={(v) => draft && (draft.asDropdown = v)} />
							</Field.Field>
						{/if}
						<Field.Field>
							<Field.Label for="nl-update">When a {draft.parent} key changes</Field.Label>
							{@render select('nl-update', ON_UPDATE, draft.onUpdate, (v) => draft && (draft.onUpdate = v as FkAction))}
						</Field.Field>
						<Field.Field>
							<Field.Label for="nl-delete">When a {draft.parent} record is deleted</Field.Label>
							{@render select('nl-delete', ON_DELETE, draft.onDelete, (v) => draft && (draft.onDelete = v as FkAction))}
						</Field.Field>
					</Field.Group>
				{/if}
			{/if}
			{#if mismatch || replaces}
				<Alert.Root class="border-warning/40 bg-warning/10 *:[svg]:text-warning">
					<TriangleAlertIcon />
					<Alert.Title class="text-warning">Check before linking</Alert.Title>
					<Alert.Description>
						{#if mismatch}<p>{mismatch}</p>{/if}
						{#if replaces}<p>“{replaces.childColumns[0]}” already links to {replaces.parent}; this replaces that link.</p>{/if}
					</Alert.Description>
				</Alert.Root>
			{/if}
			{#if detailsProblem}<p class="text-sm text-muted-foreground">{detailsProblem}</p>{/if}
		</div>
	{/if}
</StepDrawer>
