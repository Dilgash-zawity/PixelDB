<script lang="ts" module>
	export type Selection =
		| { kind: 'relationship'; id: string }
		| { kind: 'table'; name: string }
		| { kind: 'new' }
		| null;
</script>

<script lang="ts">
	import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import LinkIcon from '@lucide/svelte/icons/link';
	import MousePointerClickIcon from '@lucide/svelte/icons/mouse-pointer-click';
	import PencilRulerIcon from '@lucide/svelte/icons/pencil-ruler';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import Rows3Icon from '@lucide/svelte/icons/rows-3';
	import Table2Icon from '@lucide/svelte/icons/table-2';
	import TrashIcon from '@lucide/svelte/icons/trash-2';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';
	import type { FkAction, TableDef, TableInfo } from '#lib/api/types.js';
	import * as Alert from '#lib/components/ui/alert/index.js';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as Item from '#lib/components/ui/item/index.js';
	import * as RadioGroup from '#lib/components/ui/radio-group/index.js';
	import { ScrollArea } from '#lib/components/ui/scroll-area/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import * as Tabs from '#lib/components/ui/tabs/index.js';
	import { fieldTypeLabel } from '#lib/logic/field-types.js';
	import {
		type Cardinality,
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

	let {
		defs,
		infos,
		rels,
		selection,
		tab = $bindable('relationship'),
		draft = $bindable(null),
		busy = false,
		onSelect,
		onCreate,
		onUpdate,
		onDelete,
		onOpen,
		onStartLink
	}: {
		defs: TableDef[];
		infos: Record<string, TableInfo>;
		rels: Relationship[];
		selection: Selection;
		tab?: string;
		draft?: LinkDraft | null;
		busy?: boolean;
		onSelect: (s: Selection) => void;
		onCreate: () => void;
		onUpdate: (rel: Relationship, change: { cardinality: Cardinality; onUpdate: FkAction; onDelete: FkAction }) => void;
		onDelete: (rel: Relationship) => void;
		onOpen: (table: string, view: 'records' | 'fields') => void;
		onStartLink: (parent?: string) => void;
	} = $props();

	const NEW = '\u0000new';
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
	const tableItems = $derived(defs.map((d) => ({ value: d.name, label: d.name })));
	const rel = $derived(selection?.kind === 'relationship' ? (rels.find((r) => r.id === selection.id) ?? null) : null);
	const table = $derived(selection?.kind === 'table' ? (byName.get(selection.name) ?? null) : null);

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

	// ── New relationship form helpers ──
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
	const draftProblem = $derived.by(() => {
		if (!draft || !parentDef || !childDef) return 'Choose both tables';
		if (draft.kind === 'many_to_many') {
			if (parentDef.name === childDef.name) return 'Choose two different tables';
			return null;
		}
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

{#snippet kinds(value: string, onChange: (v: string) => void, parent: string, child: string, options: LinkKind[])}
	<RadioGroup.Root {value} onValueChange={onChange}>
		{#each options as kind (kind)}
			{@const l = kindLabel(kind, parent, child)}
			<Field.Field orientation="horizontal">
				<RadioGroup.Item value={kind} id={`kind-${kind}`} />
				<Field.Content>
					<Field.Label for={`kind-${kind}`}>{l.title}</Field.Label>
					<Field.Description>{l.hint}</Field.Description>
				</Field.Content>
			</Field.Field>
		{/each}
	</RadioGroup.Root>
{/snippet}

{#snippet integrity(onUpdate: FkAction, onDelete: FkAction, setUpdate: (v: FkAction) => void, setDelete: (v: FkAction) => void, parent: string)}
	<Field.Set>
		<Field.Legend variant="label">Referential integrity</Field.Legend>
		<Field.Description>Links are always checked: a record can only point at a {parent} record that exists.</Field.Description>
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

<aside class="flex h-full w-80 shrink-0 flex-col border-s bg-sidebar">
	<Tabs.Root bind:value={tab} class="flex min-h-0 flex-1 flex-col gap-0">
		<div class="border-b p-2">
			<Tabs.List class="w-full">
				<Tabs.Trigger value="relationship">Relationship</Tabs.Trigger>
				<Tabs.Trigger value="table">Table details</Tabs.Trigger>
			</Tabs.List>
		</div>
		<ScrollArea class="min-h-0 flex-1">
			<Tabs.Content value="relationship" class="p-4">
				{#if selection?.kind === 'new' && draft}
					<form
						class="flex flex-col gap-6"
						onsubmit={(e) => {
							e.preventDefault();
							if (!draftProblem) onCreate();
						}}
					>
						<div class="flex flex-col gap-1">
							<h2 class="font-semibold">New relationship</h2>
							<p class="text-sm text-muted-foreground">Link records in one table to records in another.</p>
						</div>
						<Field.Set>
							<Field.Legend variant="label">The “one” side</Field.Legend>
							<Field.Group class="gap-3">
								<Field.Field orientation="horizontal">
									{@render row('p-table', 'Table')}
									{@render select('p-table', tableItems, draft.parent, setParent, 'Choose a table')}
								</Field.Field>
								{#if draft.kind !== 'many_to_many'}
									<Field.Field orientation="horizontal">
										{@render row('p-col', 'Key field')}
										{@render select('p-col', parentKeys.map((k) => ({ value: k, label: k })), draft.parentColumn, (v) => { if (draft) draft.parentColumn = v; resuggest(); }, parentKeys.length ? 'Choose a field' : 'No key fields', !parentKeys.length)}
									</Field.Field>
								{/if}
							</Field.Group>
						</Field.Set>
						<Field.Set>
							<Field.Legend variant="label">The “{draft.kind === 'one_to_one' ? 'one' : 'many'}” side</Field.Legend>
							<Field.Group class="gap-3">
								<Field.Field orientation="horizontal">
									{@render row('c-table', 'Table')}
									{@render select('c-table', tableItems, draft.child, setChild, 'Choose a table')}
								</Field.Field>
								{#if draft.kind !== 'many_to_many'}
									<Field.Field orientation="horizontal">
										{@render row('c-col', 'Linked by')}
										{@render select('c-col', childFieldItems, draft.childColumn ?? NEW, (v) => draft && (draft.childColumn = v === NEW ? null : v))}
									</Field.Field>
									{#if draft.childColumn === null}
										<Field.Field orientation="horizontal">
											{@render row('c-new', 'Field name')}
											<Input id="c-new" class="h-8" bind:value={draft.newField} autocomplete="off" />
										</Field.Field>
									{/if}
								{/if}
							</Field.Group>
						</Field.Set>
						<Field.Set>
							<Field.Legend variant="label">Relationship type</Field.Legend>
							{@render kinds(draft.kind, (v) => draft && (draft.kind = v as LinkKind), draft.parent, draft.child, ['one_to_many', 'one_to_one', 'many_to_many'])}
						</Field.Set>
						{#if draft.kind !== 'many_to_many'}
							{#if draft.childColumn === null || (childField && canBeDropdown(childField))}
								<Field.Field orientation="horizontal">
									<Field.Content>
										<Field.Label for="as-dropdown">Pick from a list</Field.Label>
										<Field.Description>Choose a {draft.parent} record from a dropdown when typing {draft.child} data.</Field.Description>
									</Field.Content>
									<Switch id="as-dropdown" checked={draft.asDropdown} onCheckedChange={(v) => draft && (draft.asDropdown = v)} />
								</Field.Field>
							{/if}
							{@render integrity(draft.onUpdate, draft.onDelete, (v) => draft && (draft.onUpdate = v), (v) => draft && (draft.onDelete = v), draft.parent)}
						{/if}
						{#if mismatch || replaces}
							<Alert.Root>
								<TriangleAlertIcon />
								<Alert.Title>Check before saving</Alert.Title>
								<Alert.Description>
									{#if mismatch}<p>{mismatch}</p>{/if}
									{#if replaces}<p>“{replaces.childColumns[0]}” already links to {replaces.parent}; this replaces that link.</p>{/if}
								</Alert.Description>
							</Alert.Root>
						{/if}
						<div class="flex flex-col gap-2">
							{#if draftProblem}<p class="text-sm text-muted-foreground">{draftProblem}</p>{/if}
							<div class="flex gap-2">
								<Button type="submit" class="flex-1" disabled={!!draftProblem || busy}>
									{#if busy}<Spinner data-icon="inline-start" />{:else}<LinkIcon data-icon="inline-start" />{/if}
									Create relationship
								</Button>
								<Button type="button" variant="outline" onclick={() => onSelect(null)}>Cancel</Button>
							</div>
						</div>
					</form>
				{:else if rel && edit}
					{@const blocked = blockedReason(rel.child)}
					<div class="flex flex-col gap-6">
						<div class="flex flex-col gap-3">
							<h2 class="font-semibold">Relationship</h2>
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
							<p class="text-sm text-muted-foreground">{describeRelationship({ ...rel, cardinality: edit.cardinality })}</p>
						</div>
						{#if rel.childColumns.length === 1}
							<Field.Set>
								<Field.Legend variant="label">Relationship type</Field.Legend>
								{@render kinds(edit.cardinality, (v) => edit && (edit.cardinality = v as Cardinality), rel.parent, rel.child, ['one_to_many', 'one_to_one'])}
							</Field.Set>
						{/if}
						{@render integrity(edit.onUpdate, edit.onDelete, (v) => edit && (edit.onUpdate = v), (v) => edit && (edit.onDelete = v), rel.parent)}
						{#if blocked}
							<Alert.Root>
								<TriangleAlertIcon />
								<Alert.Title>Read-only</Alert.Title>
								<Alert.Description>{blocked}</Alert.Description>
							</Alert.Root>
						{/if}
						<div class="flex flex-col gap-2">
							<Button disabled={!editDirty || busy || !!blocked} onclick={() => edit && onUpdate(rel, $state.snapshot(edit))}>
								{#if busy}<Spinner data-icon="inline-start" />{/if}
								Save changes
							</Button>
							<div class="flex gap-2">
								<Button variant="outline" class="flex-1" onclick={() => onOpen(rel.child, 'fields')}>
									<PencilRulerIcon data-icon="inline-start" /> Design {rel.child}
								</Button>
								<Button variant="destructive" size="icon" aria-label="Delete relationship" disabled={busy || !!blocked} onclick={() => onDelete(rel)}>
									<TrashIcon />
								</Button>
							</div>
						</div>
					</div>
				{:else}
					<Empty.Root class="p-2">
						<Empty.Header>
							<Empty.Media variant="icon"><MousePointerClickIcon /></Empty.Media>
							<Empty.Title>Link your tables</Empty.Title>
							<Empty.Description>
								Drag the dot beside any field onto another table. Drop it on a table's name to add a linking field
								automatically. Click a line to edit it.
							</Empty.Description>
						</Empty.Header>
						<Empty.Content>
							<Button onclick={() => onStartLink()} disabled={defs.length === 0}>
								<PlusIcon data-icon="inline-start" /> New relationship
							</Button>
						</Empty.Content>
					</Empty.Root>
				{/if}
			</Tabs.Content>

			<Tabs.Content value="table" class="p-4">
				{#if table}
					{@const info = infos[table.name]}
					{@const links = rels.filter((r) => r.parent === table.name || r.child === table.name)}
					<div class="flex flex-col gap-6">
						<div class="flex flex-col gap-2">
							<div class="flex items-center gap-2">
								<Table2Icon class="size-4 text-muted-foreground" />
								<h2 class="truncate font-semibold">{table.name}</h2>
							</div>
							{#if table.description}<p class="text-sm text-muted-foreground">{table.description}</p>{/if}
							<div class="flex flex-wrap gap-1.5">
								<Badge variant="secondary">{table.fields.length} {table.fields.length === 1 ? 'field' : 'fields'}</Badge>
								{#if info}<Badge variant="secondary">{info.row_count.toLocaleString()} {info.row_count === 1 ? 'record' : 'records'}</Badge>{/if}
								<Badge variant="secondary">{links.length} {links.length === 1 ? 'link' : 'links'}</Badge>
							</div>
						</div>
						<div class="flex gap-2">
							<Button variant="outline" size="sm" class="flex-1" onclick={() => onOpen(table.name, 'records')}><Rows3Icon data-icon="inline-start" /> Datasheet</Button>
							<Button variant="outline" size="sm" class="flex-1" onclick={() => onOpen(table.name, 'fields')}><PencilRulerIcon data-icon="inline-start" /> Design</Button>
						</div>
						<Field.Set>
							<Field.Legend variant="label">Fields</Field.Legend>
							<ul class="flex flex-col rounded-md border text-sm">
								{#each table.fields as f (f.name)}
									<li class="flex items-center gap-2 border-b px-3 py-1.5 last:border-b-0">
										<span class="flex w-4 justify-center">{#if f.primary_key}<KeyRoundIcon class="size-3.5 text-key" />{/if}</span>
										<span class="truncate">{f.name}</span>
										<span class="ms-auto shrink-0 text-xs text-muted-foreground">{fieldTypeLabel(f.field_type)}</span>
									</li>
								{/each}
							</ul>
						</Field.Set>
						<Field.Set>
							<Field.Legend variant="label">Relationships</Field.Legend>
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
							<Button variant="outline" size="sm" onclick={() => onStartLink(table.name)}>
								<PlusIcon data-icon="inline-start" /> Link {table.name} to…
							</Button>
						</Field.Set>
					</div>
				{:else}
					<Empty.Root class="p-2">
						<Empty.Header>
							<Empty.Media variant="icon"><Table2Icon /></Empty.Media>
							<Empty.Title>No table selected</Empty.Title>
							<Empty.Description>Click a table on the diagram to see its fields and links.</Empty.Description>
						</Empty.Header>
					</Empty.Root>
				{/if}
			</Tabs.Content>
		</ScrollArea>
	</Tabs.Root>
</aside>
