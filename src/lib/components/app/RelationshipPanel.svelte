<script lang="ts" module>
	// Script
	export type Selection =
		| { kind: 'relationship'; id: string }
		| { kind: 'table'; name: string }
		| { kind: 'field'; table: string; field: string | null }
		| null;
</script>

<script lang="ts">
	// Types
	import type { FkAction, TableDef, TableInfo } from '#lib/api/types.js';

	// Utils
	import { fieldTypeLabel } from '#lib/logic/field-types.js';
	import {
		type Cardinality,
		type LinkKind,
		type Relationship,
		describeRelationship,
		junctionName
	} from '#lib/logic/relationships.js';
	import { session } from '#lib/state/session.svelte.js';

	// Components
	import * as Alert from '#lib/components/ui/alert/index.js';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import * as Item from '#lib/components/ui/item/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import * as ToggleGroup from '#lib/components/ui/toggle-group/index.js';
	import StepDrawer from './StepDrawer.svelte';

	// Icons
	import {
		ArrowRight as ArrowRightIcon,
		KeyRound as KeyRoundIcon,
		Link as LinkIcon,
		ListTree as ListTreeIcon,
		Pencil as PencilIcon,
		Plus as PlusIcon,
		Rows3 as Rows3Icon,
		ShieldCheck as ShieldCheckIcon,
		Table2 as Table2Icon,
		Trash2 as TrashIcon,
		TriangleAlert as TriangleAlertIcon,
	} from '@lucide/svelte';

	// Script
	/** The Diagram's details drawer: a table (overview, fields, links) or a relationship (kind, rules), step by step. */
	let {
		defs,
		infos,
		rels,
		selection,
		busy = false,
		scope,
		onSelect,
		onUpdate,
		onDelete,
		onOpen,
		onStartLink,
		onRenameTable,
		onDeleteTable
	}: {
		defs: TableDef[];
		infos: Record<string, TableInfo>;
		rels: Relationship[];
		selection: Selection;
		busy?: boolean;
		/** Confirms opened from this drawer use this scope. */
		scope: string;
		onSelect: (s: Selection) => void;
		onUpdate: (rel: Relationship, change: { cardinality: Cardinality; onUpdate: FkAction; onDelete: FkAction }) => void;
		onDelete: (rel: Relationship) => void;
		onOpen: (table: string) => void;
		onStartLink: (parent?: string) => void;
		onRenameTable: (table: string) => void;
		onDeleteTable: (table: string) => void;
	} = $props();

	const KIND_NAMES: Record<LinkKind, string> = { one_to_many: 'One to many', one_to_one: 'One to one', many_to_many: 'Many to many' };
	const TABLE_STEPS = [
		{ label: 'Table', icon: Table2Icon },
		{ label: 'Fields', icon: ListTreeIcon },
		{ label: 'Links', icon: LinkIcon }
	];
	const LINK_STEPS = [
		{ label: 'Kind', icon: LinkIcon },
		{ label: 'Rules', icon: ShieldCheckIcon }
	];
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

	const byName = $derived(new Map(defs.map((d) => [d.name, d])));
	const rel = $derived(selection?.kind === 'relationship' ? (rels.find((r) => r.id === selection.id) ?? null) : null);
	const table = $derived(selection?.kind === 'table' ? (byName.get(selection.name) ?? null) : null);

	let step = $state(0);
	// Start at the first step whenever something else is selected.
	$effect(() => {
		void (selection?.kind === 'table' ? selection.name : selection?.kind === 'relationship' ? selection.id : null);
		step = 0;
	});

	// Editable copy of the selected relationship's options.
	let edit = $state<{ cardinality: Cardinality; onUpdate: FkAction; onDelete: FkAction } | null>(null);
	$effect(() => {
		edit = rel ? { cardinality: rel.cardinality, onUpdate: rel.onUpdate, onDelete: rel.onDelete } : null;
	});
	const editDirty = $derived(
		!!rel && !!edit && (edit.cardinality !== rel.cardinality || edit.onUpdate !== rel.onUpdate || edit.onDelete !== rel.onDelete)
	);

	function blockedReason(name: string): string | null {
		const features = infos[name]?.unsupported_features ?? [];
		return features.length
			? `“${name}” uses ${features.join(', ')}, which this app can't rebuild, so its links can't be changed here.`
			: null;
	}

	function kindLabel(kind: LinkKind, parent: string, child: string): { title: string; hint: string } {
		switch (kind) {
			case 'one_to_many':
				return { title: 'One to many', hint: `One ${parent} record can have many ${child} records` };
			case 'one_to_one':
				return { title: 'One to one', hint: `One ${parent} record has at most one ${child} record` };
			case 'many_to_many':
				return {
					title: 'Many to many',
					hint: `Many ${parent} ↔ many ${child} (adds a linking table “${junctionName(parent, child, session.objects.map((o) => o.name))}”)`
				};
		}
	}
</script>

{#snippet select(id: string, items: { value: string; label: string }[], value: string, onChange: (v: string) => void, placeholder = '', disabled = false)}
	<Select.Root type="single" {items} {value} onValueChange={onChange} {disabled}>
		<Select.Trigger {id} size="sm" class="w-full min-w-0"><Select.Value {placeholder} /></Select.Trigger>
		<Select.Content>
			<Select.Group>
				{#each items as item (item.value)}<Select.Item value={item.value} label={item.label}>{item.label}</Select.Item>{/each}
			</Select.Group>
		</Select.Content>
	</Select.Root>
{/snippet}

{#snippet row(id: string, label: string)}
	<Field.Label for={id} class="w-20 flex-none font-normal whitespace-nowrap text-muted-foreground">{label}</Field.Label>
{/snippet}

{#snippet kinds(value: LinkKind, onChange: (v: LinkKind) => void, parent: string, child: string, options: LinkKind[])}
	<ToggleGroup.Root type="single" variant="outline" size="sm" class="w-full" {value} onValueChange={(v) => v && onChange(v as LinkKind)}>
		{#each options as kind (kind)}
			<ToggleGroup.Item value={kind} class="flex-1">{KIND_NAMES[kind]}</ToggleGroup.Item>
		{/each}
	</ToggleGroup.Root>
	<Field.Description>{kindLabel(value, parent, child).hint}.</Field.Description>
{/snippet}

{#snippet integrity(onUpdate: FkAction, onDelete: FkAction, setUpdate: (v: FkAction) => void, setDelete: (v: FkAction) => void, parent: string)}
	<Field.Set>
		<Field.Legend variant="label">Rules</Field.Legend>
		<Field.Description>What happens to linked records when a {parent} record changes.</Field.Description>
		<Field.Group class="gap-3">
			<Field.Field orientation="horizontal">
				{@render row('ri-update', 'On update')}
				{@render select('ri-update', ON_UPDATE, onUpdate, (v) => setUpdate(v as FkAction))}
			</Field.Field>
			<Field.Field orientation="horizontal">
				{@render row('ri-delete', 'On delete')}
				{@render select('ri-delete', ON_DELETE, onDelete, (v) => setDelete(v as FkAction))}
			</Field.Field>
		</Field.Group>
	</Field.Set>
{/snippet}

{#if rel && edit}
	{@const blocked = blockedReason(rel.child)}
	<StepDrawer
		bind:open={() => true, (o) => !o && onSelect(null)}
		bind:step
		title="Relationship"
		description="{rel.parent} → {rel.child}"
		steps={LINK_STEPS}
		finishLabel="Save"
		{busy}
		disabled={step === LINK_STEPS.length - 1 && (!editDirty || !!blocked)}
		onFinish={() => edit && onUpdate(rel, $state.snapshot(edit))}
		{scope}
	>
		{#snippet actions()}
			<Button variant="destructive" disabled={busy || !!blocked} onclick={() => onDelete(rel)}>
				<TrashIcon data-icon="inline-start" /> Unlink
			</Button>
		{/snippet}
		<div class="flex flex-col gap-5">
			{#if blocked}
				<Alert.Root class="border-warning/40 bg-warning/10 *:[svg]:text-warning">
					<TriangleAlertIcon />
					<Alert.Title class="text-warning">Read-only</Alert.Title>
					<Alert.Description>{blocked}</Alert.Description>
				</Alert.Root>
			{/if}
			{#if step === 0}
				<Item.Root variant="outline" size="sm">
					<Item.Content>
						<Item.Title>{rel.parent}</Item.Title>
						<Item.Description>{rel.parentColumns.join(', ')}</Item.Description>
					</Item.Content>
					<Item.Media><ArrowRightIcon /></Item.Media>
					<Item.Content class="text-end">
						<Item.Title class="ms-auto">{rel.child}</Item.Title>
						<Item.Description>{rel.childColumns.join(', ')}</Item.Description>
					</Item.Content>
				</Item.Root>
				{#if rel.childColumns.length === 1}
					<Field.Field>
						{@render kinds(edit.cardinality, (v) => edit && (edit.cardinality = v as Cardinality), rel.parent, rel.child, ['one_to_many', 'one_to_one'])}
					</Field.Field>
				{:else}
					<p class="text-sm text-muted-foreground">{describeRelationship({ ...rel, cardinality: edit.cardinality })}</p>
				{/if}
			{:else}
				{@render integrity(edit.onUpdate, edit.onDelete, (v) => edit && (edit.onUpdate = v), (v) => edit && (edit.onDelete = v), rel.parent)}
			{/if}
		</div>
	</StepDrawer>
{:else if table}
	{@const info = infos[table.name]}
	{@const links = rels.filter((r) => r.parent === table.name || r.child === table.name)}
	<StepDrawer
		bind:open={() => true, (o) => !o && onSelect(null)}
		bind:step
		title={table.name}
		description={table.description ?? 'A table on the Diagram'}
		steps={TABLE_STEPS}
		finishLabel="Done"
		onFinish={() => onSelect(null)}
		{scope}
	>
		<div class="flex flex-col gap-5">
			{#if step === 0}
				<div class="flex flex-wrap gap-1.5">
					<Badge variant="secondary">{table.fields.length} {table.fields.length === 1 ? 'field' : 'fields'}</Badge>
					{#if info}<Badge variant="secondary">{info.row_count.toLocaleString()} {info.row_count === 1 ? 'record' : 'records'}</Badge>{/if}
					<Badge variant="secondary">{links.length} {links.length === 1 ? 'link' : 'links'}</Badge>
				</div>
				<div class="flex flex-col gap-2">
					<Button variant="outline" class="justify-start" onclick={() => onOpen(table.name)}><Rows3Icon data-icon="inline-start" /> Open datasheet</Button>
					<Button variant="outline" class="justify-start" onclick={() => onRenameTable(table.name)}><PencilIcon data-icon="inline-start" /> Rename table</Button>
					<Button variant="destructive" class="justify-start" onclick={() => onDeleteTable(table.name)}><TrashIcon data-icon="inline-start" /> Delete table</Button>
				</div>
			{:else if step === 1}
				<ul class="flex flex-col rounded-md border text-sm">
					{#each table.fields as f (f.name)}
						<li class="border-b last:border-b-0">
							<button type="button" class="flex w-full items-center gap-2 px-3 py-2 text-start hover:bg-accent" onclick={() => onSelect({ kind: 'field', table: table.name, field: f.name })}>
								<span class="flex w-4 justify-center">{#if f.primary_key}<KeyRoundIcon class="size-3.5 text-warning" />{/if}</span>
								<span class="truncate">{f.name}</span>
								<span class="ms-auto shrink-0 text-xs text-muted-foreground">{fieldTypeLabel(f.field_type)}</span>
							</button>
						</li>
					{/each}
				</ul>
				<Button variant="outline" class="self-start" onclick={() => onSelect({ kind: 'field', table: table.name, field: null })}>
					<PlusIcon data-icon="inline-start" /> Add field
				</Button>
			{:else}
				{#if links.length}
					<Item.Group class="gap-1.5">
						{#each links as r (r.id)}
							<Item.Root variant="outline" size="sm">
								{#snippet child({ props })}
									<button {...props} type="button" onclick={() => onSelect({ kind: 'relationship', id: r.id })}>
										<Item.Media><LinkIcon /></Item.Media>
										<Item.Content>
											<Item.Title>{r.parent} → {r.child}</Item.Title>
											<Item.Description>{r.cardinality === 'one_to_one' ? 'One to one' : 'One to many'} · {r.childColumns.join(', ')}</Item.Description>
										</Item.Content>
									</button>
								{/snippet}
							</Item.Root>
						{/each}
					</Item.Group>
				{:else}
					<p class="text-sm text-muted-foreground">Not linked to any table yet.</p>
				{/if}
				<Button variant="outline" class="self-start" onclick={() => onStartLink(table.name)}>
					<PlusIcon data-icon="inline-start" /> Link {table.name} to…
				</Button>
			{/if}
		</div>
	</StepDrawer>
{/if}
