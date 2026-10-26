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
	import { cn } from '#lib/utils.js';

	// Components
	import * as Alert from '#lib/components/ui/alert/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import FieldProperties from './FieldProperties.svelte';
	import FieldTypeCards from './FieldTypeCards.svelte';
	import StepDrawer from './StepDrawer.svelte';

	// Icons
	import {
		ArrowRight as ArrowRightIcon,
		ArrowRightLeft as ArrowRightLeftIcon,
		Link as LinkIcon,
		Rows3 as Rows3Icon,
		Square as SquareIcon,
		Table2 as Table2Icon,
		TriangleAlert as TriangleAlertIcon,
	} from '@lucide/svelte';

	// Script
	/** Adds a field to `table` on one page: name, type, then optional rules (or, for a relationship, the table it points to). */
	let {
		open = $bindable(false),
		table,
		onSaved
	}: {
		open?: boolean;
		table: string;
		onSaved?: (name: string) => void;
	} = $props();

	// A relationship is offered here too: it picks the table instead of the rules.
	const EXCLUDED = WIZARD_EXCLUDED_TYPES.filter((t) => t !== 'lookup');
	const KINDS: { value: Cardinality; label: string; hint: (child: string, parent: string) => string }[] = [
		{
			value: 'one_to_many',
			label: 'Many to one',
			hint: (child, parent) => `Many ${child} records can pick the same ${parent} record — like many orders for one customer.`
		},
		{
			value: 'one_to_one',
			label: 'One to one',
			hint: (child, parent) => `Each ${parent} record can be picked by at most one ${child} record — like one passport per person.`
		}
	];

	let info = $state<TableInfo | null>(null);
	let edit = $state<FieldDef | null>(null);
	let touched = $state(false);
	let busy = $state(false);
	let nameInput = $state<HTMLInputElement | null>(null);
	/** The relationship's target when the type is Relationship. */
	let link = $state<{ parent: string; kind: Cardinality; required: boolean }>({ parent: '', kind: 'one_to_many', required: false });
	let parentDef = $state<TableDef | null>(null);

	const isLink = $derived(edit?.field_type.type === 'lookup');
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
		if (!edit || edit.field_type.type === v) return;
		edit.field_type = makeFieldType(v);
		// Suggest another table to point to, else this one (e.g. an employee's manager).
		if (v === 'lookup' && !link.parent) {
			const other = session.tables.find((t) => t.name !== table) ?? session.tables[0];
			if (other) setParent(other.name);
		}
	}

	async function setParent(name: string) {
		link.parent = name;
		parentDef = null;
		const loaded = await attempt(() => tableService.get(name));
		if (loaded && link.parent === name) parentDef = loaded.definition;
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
		touched = true;
		if (nameProblem) return nameInput?.focus();
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
	title="New field"
	description="In {table}"
	finishLabel={isLink ? 'Link' : 'Add field'}
	finishIcon={isLink ? LinkIcon : undefined}
	{busy}
	disabled={!edit || !!blocked || (isLink && (!!linkProblem || !parentDef))}
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
	{:else}
		<div class="flex flex-col gap-6">
			<Field.Field data-invalid={touched && !!nameProblem}>
				<Field.Label for="nf-name">Name</Field.Label>
				<Input id="nf-name" bind:ref={nameInput} bind:value={edit.name} placeholder="e.g. Email" autocomplete="off" aria-invalid={touched && !!nameProblem} onblur={() => (touched = true)} />
				{#if touched && nameProblem}
					<Field.Error>{nameProblem}</Field.Error>
				{:else}
					<Field.Description>Short and clear, like “Email” or “Due date”.</Field.Description>
				{/if}
			</Field.Field>
			<Field.Set>
				<Field.Legend variant="label">Type</Field.Legend>
				<FieldTypeCards value={edit.field_type.type} exclude={EXCLUDED} onChange={setType} />
			</Field.Set>
			{#if isLink}
			<Field.Set>
				<Field.Legend variant="label">Which table does “{edit.name.trim()}” point to?</Field.Legend>
				<Field.Description>Each {table} record picks one record from it.</Field.Description>
				<div class="grid grid-cols-2 gap-2 sm:grid-cols-3" role="radiogroup" aria-label="Table to point to">
					{#each session.tables as t (t.name)}
						{@const selected = link.parent === t.name}
						<button
							type="button"
							role="radio"
							aria-checked={selected}
							class={cn(
								'flex items-center gap-2.5 rounded-lg border bg-card p-3 text-start outline-none transition-colors hover:bg-accent/50 focus-visible:ring-3 focus-visible:ring-ring/40',
								selected && 'border-primary bg-primary/5 ring-1 ring-primary'
							)}
							onclick={() => link.parent !== t.name && setParent(t.name)}
						>
							<span class={cn('flex size-8 shrink-0 items-center justify-center rounded-md bg-muted', selected && 'bg-primary text-primary-foreground')}>
								<Table2Icon class="size-4" />
							</span>
							<span class="flex min-w-0 flex-col">
								<span class="truncate text-sm font-medium">{t.name}</span>
								{#if t.name === table}<span class="text-xs text-muted-foreground">This table</span>{/if}
							</span>
						</button>
					{/each}
				</div>
				{#if link.parent && linkProblem}
					<Field.Error>{linkProblem}</Field.Error>
				{/if}
			</Field.Set>
			{#if link.parent}
				<Field.Set>
					<Field.Legend variant="label">How many {table} records can pick the same {link.parent} record?</Field.Legend>
					<div class="grid gap-2 sm:grid-cols-2" role="radiogroup" aria-label="Kind of link">
						{#each KINDS as k (k.value)}
							{@const selected = link.kind === k.value}
							<button
								type="button"
								role="radio"
								aria-checked={selected}
								class={cn(
									'flex flex-col items-start gap-2 rounded-lg border bg-card p-3 text-start outline-none transition-colors hover:bg-accent/50 focus-visible:ring-3 focus-visible:ring-ring/40',
									selected && 'border-primary bg-primary/5 ring-1 ring-primary'
								)}
								onclick={() => (link.kind = k.value)}
							>
								<!-- A small picture of the link: many boxes to one, or one to one. -->
								<span class={cn('flex items-center gap-1.5 rounded-md bg-muted px-2 py-1.5 text-muted-foreground', selected && 'bg-primary text-primary-foreground')} aria-hidden="true">
									{#if k.value === 'one_to_many'}
										<Rows3Icon class="size-4" /><ArrowRightIcon class="size-3.5" /><SquareIcon class="size-4" />
									{:else}
										<SquareIcon class="size-4" /><ArrowRightLeftIcon class="size-3.5" /><SquareIcon class="size-4" />
									{/if}
								</span>
								<span class="text-sm font-medium">{k.label}</span>
								<span class="text-xs text-muted-foreground">{k.hint(table, link.parent)}</span>
							</button>
						{/each}
					</div>
				</Field.Set>
				<Field.Field orientation="horizontal" class="rounded-lg border p-3">
					<Field.Content>
						<Field.Label for="nf-required">Required</Field.Label>
						<Field.Description>Every {table} record must pick one.</Field.Description>
					</Field.Content>
					<Switch id="nf-required" checked={link.required} onCheckedChange={(v) => (link.required = v)} />
				</Field.Field>
				<p class="flex items-start gap-2 rounded-lg bg-muted/50 p-3 text-sm text-muted-foreground">
					<LinkIcon class="mt-0.5 size-4 shrink-0" />
					{describeRelationship({ parent: link.parent, child: table, cardinality: link.kind })}
				</p>
			{/if}
			{:else}
			<Field.Set>
				<Field.Legend variant="label">Rules</Field.Legend>
				<Field.Description>All optional — click “Add field” to keep the defaults.</Field.Description>
				<FieldProperties field={edit} />
			</Field.Set>
			{/if}
		</div>
	{/if}
</StepDrawer>
