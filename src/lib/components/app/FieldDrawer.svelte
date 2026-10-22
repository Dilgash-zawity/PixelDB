<script lang="ts">
	import { untrack } from 'svelte';
	import { toast } from 'svelte-sonner';
	import TrashIcon from '@lucide/svelte/icons/trash-2';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';
	import type { FieldDef, FieldTypeName, TableInfo } from '#lib/api/types.js';
	import * as Alert from '#lib/components/ui/alert/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Drawer from '#lib/components/ui/drawer/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import { makeFieldType } from '#lib/logic/field-types.js';
	import { fieldDesign, withoutField } from '#lib/logic/relationships.js';
	import { nextFreeName } from '#lib/logic/table-draft.js';
	import { jsonClone } from '#lib/logic/values.js';
	import { tableService } from '#lib/services/tables.js';
	import { attempt, confirm } from '#lib/state/feedback.svelte.js';
	import { session } from '#lib/state/session.svelte.js';
	import ConfirmHost from './ConfirmHost.svelte';
	import FieldProperties from './FieldProperties.svelte';
	import FieldTypeSelect from './FieldTypeSelect.svelte';

	/** Adds (`field` null) or edits one field of `table`, saving straight away. */
	let {
		open = $bindable(false),
		table,
		field,
		/** Called after a save or delete with the field's new name, or null when deleted. */
		onSaved
	}: {
		open?: boolean;
		table: string;
		field: string | null;
		onSaved?: (name: string | null) => void;
	} = $props();

	const SCOPE = 'field-drawer';

	let info = $state<TableInfo | null>(null);
	let edit = $state<FieldDef | null>(null);
	let baseline = $state('');
	let busy = $state(false);

	$effect(() => {
		if (open) {
			const [t, f] = [table, field];
			untrack(() => load(t, f));
		}
	});

	async function load(t: string, f: string | null) {
		info = null;
		edit = null;
		const loaded = await attempt(() => tableService.get(t));
		if (!loaded) return;
		info = loaded;
		const existing = f === null ? undefined : loaded.definition.fields.find((x) => x.name === f);
		edit = existing
			? jsonClone(existing)
			: { name: nextFreeName('Field', loaded.definition.fields.map((x) => x.name)), field_type: makeFieldType('short_text') };
		baseline = JSON.stringify(edit);
	}

	const blocked = $derived(
		info?.unsupported_features.length
			? `“${table}” was made by another program and uses ${info.unsupported_features.join(', ')}, so its fields can't be changed here.`
			: null
	);
	const dirty = $derived(!!edit && (field === null || JSON.stringify(edit) !== baseline));

	function setType(v: FieldTypeName) {
		if (!edit) return;
		edit.field_type = makeFieldType(v);
		if (v === 'calculated') {
			edit.primary_key = false;
			edit.default = null;
		}
	}

	async function save(e: SubmitEvent) {
		e.preventDefault();
		if (!info || !edit || busy || !dirty) return;
		const f = jsonClone(edit);
		f.name = f.name.trim();
		const design = fieldDesign(info.definition, field, f);
		if (Array.isArray(design)) {
			toast.error('Check the field', { description: design.join(' · ') });
			return;
		}
		busy = true;
		try {
			const result = await attempt(() => tableService.saveDesign(table, design, false));
			if (!result) return;
			for (const r of design.renames) session.renamedField(table, r.from, r.to);
			open = false;
			toast.success(field === null ? `Added “${f.name}” to ${table}` : `Saved “${f.name}”`, {
				description: result.warnings.join(' ') || undefined
			});
			onSaved?.(f.name);
		} finally {
			busy = false;
		}
	}

	async function remove() {
		if (!info || field === null || busy) return;
		const ok = await confirm({
			title: `Delete “${field}”?`,
			description: `The field and the data stored in it are removed from every ${table} record. Links that use it are removed too.`,
			action: 'Delete field',
			destructive: true,
			scope: SCOPE
		});
		if (!ok) return;
		busy = true;
		try {
			const result = await attempt(() => tableService.saveDesign(table, withoutField(info!.definition, field!), true));
			if (!result) return;
			open = false;
			toast.success(`Deleted “${field}”`);
			onSaved?.(null);
		} finally {
			busy = false;
		}
	}
</script>

<Drawer.Root bind:open direction="right">
	<Drawer.Content>
		<form class="flex min-h-0 flex-1 flex-col" onsubmit={save}>
			<Drawer.Header>
				<Drawer.Title>{field === null ? 'New field' : field}</Drawer.Title>
				<Drawer.Description>In {table}</Drawer.Description>
			</Drawer.Header>
			<div class="min-h-0 flex-1 overflow-y-auto px-4 pb-4">
				{#if !edit}
					<div class="flex flex-col gap-3">
						{#each [1, 2, 3, 4] as i (i)}<Skeleton class="h-9" />{/each}
					</div>
				{:else}
					<div class="flex flex-col gap-5">
						{#if blocked}
							<Alert.Root class="border-warning/40 bg-warning/10 *:[svg]:text-warning">
								<TriangleAlertIcon />
								<Alert.Title class="text-warning">Read-only</Alert.Title>
								<Alert.Description>{blocked}</Alert.Description>
							</Alert.Root>
						{/if}
						<Field.Group class="gap-4">
							<Field.Field>
								<Field.Label for="fd-name">Name</Field.Label>
								<Input id="fd-name" class="h-8" bind:value={edit.name} autocomplete="off" disabled={!!blocked} />
							</Field.Field>
							<Field.Field>
								<Field.Label for="fd-type">Type</Field.Label>
								<FieldTypeSelect
									id="fd-type"
									value={edit.field_type.type}
									exclude={['auto_number', 'lookup']}
									disabled={!!blocked || edit.field_type.type === 'auto_number'}
									onChange={setType}
								/>
								{#if edit.field_type.type === 'lookup'}
									<Field.Description>Picks a record from a linked table. Change the link on the Diagram.</Field.Description>
								{/if}
							</Field.Field>
						</Field.Group>
						<FieldProperties field={edit} disabled={!!blocked} />
					</div>
				{/if}
			</div>
			<Drawer.Footer class="flex-row">
				<Button type="submit" class="flex-1" disabled={!dirty || busy || !!blocked || !edit?.name.trim()}>
					{#if busy}<Spinner data-icon="inline-start" />{/if}
					{field === null ? 'Add field' : 'Save'}
				</Button>
				{#if field !== null && edit && !edit.primary_key}
					<Button type="button" variant="destructive" disabled={busy || !!blocked} onclick={remove}>
						<TrashIcon data-icon="inline-start" /> Delete
					</Button>
				{/if}
			</Drawer.Footer>
		</form>
		<ConfirmHost scope={SCOPE} />
	</Drawer.Content>
</Drawer.Root>
