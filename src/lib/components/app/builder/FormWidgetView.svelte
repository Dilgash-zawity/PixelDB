<script lang="ts">
	// Services
	import { recordService } from '#lib/services/records.js';

	// Types
	import type { RecordPage } from '#lib/api/types.js';

	// Utils
	import { untrack } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { prefillFromFilters, resolveFilters, type FormWidget } from '#lib/logic/app-def.js';
	import { loadLookups, lookupLabel, type LookupOption } from '#lib/logic/lookups.js';
	import * as rb from '#lib/logic/record-buffer.js';
	import { displayValue, toEditorValue } from '#lib/logic/values.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import { attempt, cleanMessage, confirm } from '#lib/state/feedback.svelte.js';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import FieldInput from '../FieldInput.svelte';

	// Icons
	import {
		ChevronLeft as ChevronLeftIcon,
		ChevronRight as ChevronRightIcon,
		Plus as PlusIcon,
		Trash2 as TrashIcon,
		Undo2 as Undo2Icon,
		FileText as FileTextIcon,
	} from '@lucide/svelte';

	// Script
	let { widget, editing }: { widget: FormWidget; editing: boolean } = $props();

	const uid = $props.id();
	/** Position of the shown record among the matching ones. */
	let position = $state(0);
	let page = $state<RecordPage | null>(null);
	let buf = $state<rb.RecordBuffer | null>(null);
	let error = $state<string | null>(null);
	let submitted = $state(false);
	let saving = $state(false);
	let lookups = $state<Record<string, LookupOption[]>>({});
	/** Revision of the data last loaded, so saving here doesn't reload the record being looked at twice. */
	let loadedRevision = -1;

	const database = $derived(widget.data ? appSession.databaseOf(widget.data.source) : undefined);
	const schema = $derived(widget.data ? appSession.knownSchema(widget.data.source, widget.data.table) : undefined);
	const def = $derived(schema?.def ?? null);
	const fields = $derived.by(() => {
		if (!def) return [];
		if (!widget.fields.length) return def.fields;
		return widget.fields.flatMap((n) => def!.fields.filter((f) => f.name === n));
	});

	$effect(() => {
		if (widget.data) appSession.objectSchema(widget.data.source, widget.data.table);
	});

	$effect(() => {
		const d = def;
		const db = database;
		if (d && db) loadLookups(d, db).then((l) => (lookups = l));
	});

	const filters = $derived(resolveFilters(widget.filters, appSession.vars));
	const filterKey = $derived(JSON.stringify([widget.data, filters]));
	const isNew = $derived(buf?.key === null);
	const dirty = $derived(buf ? rb.isDirty(buf) : false);
	const missing = $derived(buf && def ? rb.missingRequired(buf, def) : []);
	const readOnly = $derived(!isNew && !widget.allowEdit);
	const total = $derived(page?.total ?? 0);

	async function load(at: number) {
		const d = widget.data;
		if (!d || !database || !def) return;
		const definition = def;
		try {
			const res = await recordService.list(d.table, { offset: at, limit: 1, filters }, database);
			page = res;
			position = at;
			submitted = false;
			if (res.rows.length) buf = rb.newBuffer(definition, res.rows[0], res.columns);
			else if (at > 0 && res.total > 0) await load(res.total - 1);
			else buf = widget.allowNew ? newRecord() : null;
			error = null;
		} catch (e) {
			error = cleanMessage(e instanceof Error ? e.message : String(e));
		}
	}

	function newRecord(): rb.RecordBuffer | null {
		if (!def) return null;
		const b = rb.newBuffer(def, null);
		// Fields pinned by an "is" filter start with that value.
		for (const [k, v] of Object.entries(prefillFromFilters(widget.filters, appSession.vars))) {
			const f = def.fields.find((x) => x.name === k);
			if (f) b.current[k] = toEditorValue(v, f.field_type);
		}
		return b;
	}

	// Different matching records (or a different table): start from the first.
	$effect(() => {
		void filterKey;
		if (def && database) untrack(() => load(0));
	});

	// Something changed the data elsewhere (a refresh, another widget saving).
	$effect(() => {
		const rev = appSession.dataRevision;
		if (loadedRevision === -1) {
			loadedRevision = rev;
			return;
		}
		if (rev !== loadedRevision) {
			loadedRevision = rev;
			untrack(() => !dirty && load(position));
		}
	});

	/** Asks before throwing away unsaved edits. */
	async function canLeave(): Promise<boolean> {
		if (!dirty) return true;
		return confirm({
			title: 'Discard changes?',
			description: `Your changes in “${widget.title || 'the form'}” have not been saved.`,
			action: 'Discard',
			destructive: true
		});
	}

	$effect(() => {
		appSession.guard(widget.id, dirty && !editing ? canLeave : null);
		return () => appSession.guard(widget.id, null);
	});

	async function go(at: number) {
		if (await canLeave()) load(at);
	}

	async function startNew() {
		if (!(await canLeave())) return;
		buf = newRecord();
		submitted = false;
	}

	let seenIntent = appSession.intent.n;
	$effect(() => {
		const i = appSession.intent;
		if (i.n === seenIntent) return;
		seenIntent = i.n;
		if (i.widget === widget.id && widget.allowNew) startNew();
	});

	async function save() {
		const b = buf;
		const d = widget.data;
		if (!b || !d || !def || saving || !database) return;
		submitted = true;
		if (missing.length) return;
		const values = rb.submission(b, def);
		saving = true;
		const ok = await attempt(async () => {
			if (b.key === null) await recordService.insert(d.table, values, database);
			else if (Object.keys(values).length) await recordService.update(d.table, b.key, values, database);
			return true;
		});
		saving = false;
		if (!ok) return;
		toast.success(b.key === null ? 'Record added' : 'Changes saved');
		const wasNew = b.key === null;
		buf = null;
		// Let other widgets show the change; this form reloads below.
		loadedRevision = ++appSession.dataRevision;
		const res = await attempt(() => recordService.list(d.table, { limit: 1, filters }, database));
		await load(wasNew ? Math.max(0, (res?.total ?? 1) - 1) : position);
	}

	async function remove() {
		const b = buf;
		const d = widget.data;
		if (!b?.key || !d || !database) return;
		const key = b.key;
		const ok = await confirm({
			title: 'Delete this record?',
			description: 'It will be permanently deleted. This cannot be undone.',
			action: 'Delete record',
			destructive: true
		});
		if (!ok) return;
		if ((await attempt(() => recordService.delete(d.table, [key], true, database))) === undefined) return;
		toast.success('Record deleted');
		buf = null;
		loadedRevision = ++appSession.dataRevision;
		await load(Math.max(0, position - 1));
	}

	function shownValue(name: string): string {
		const f = def?.fields.find((x) => x.name === name);
		const i = page?.columns.indexOf(name) ?? -1;
		const raw = page && i >= 0 ? page.rows[0]?.values[i] : null;
		if (f?.field_type.type === 'lookup') return lookupLabel(lookups[name], raw ?? null) ?? displayValue(raw ?? null);
		return displayValue(raw ?? null, f?.field_type);
	}
</script>

<div class="flex h-full min-h-0 flex-col gap-3">
	{#if error}
		<p class="text-sm text-destructive">{error}</p>
	{:else if !def || (!buf && !page)}
		<div class="flex flex-col gap-3">{#each Array(4) as _, i (i)}<Skeleton class="h-9" />{/each}</div>
	{:else if !buf}
		<Empty.Root class="h-full">
			<Empty.Header>
				<Empty.Media variant="icon"><FileTextIcon /></Empty.Media>
				<Empty.Title>No records</Empty.Title>
				<Empty.Description>Nothing matches this form's filters.</Empty.Description>
			</Empty.Header>
		</Empty.Root>
	{:else}
		{@const b = buf}
		<form
			class="min-h-0 flex-1 overflow-y-auto pe-1"
			onsubmit={(e) => {
				e.preventDefault();
				save();
			}}
		>
			<Field.Group class="gap-4">
				{#each fields as f (f.name)}
					{@const id = `${uid}-${f.name}`}
					{@const invalid = submitted && missing.includes(f.name)}
					<Field.Field data-invalid={invalid || undefined}>
						<Field.Label for={id}>
							{f.name}
							{#if !readOnly && (f.required || (f.primary_key && f.field_type.type !== 'auto_number'))}<span class="text-destructive">*</span>{/if}
						</Field.Label>
						{#if readOnly}
							<p {id} class="min-h-9 border bg-muted/40 px-3 py-2 text-sm">{shownValue(f.name) || '—'}</p>
						{:else}
							<FieldInput {id} field={f} value={b.current[f.name]} {invalid} options={lookups[f.name]} onchange={(v) => rb.setValue(b, f.name, v)} />
						{/if}
						{#if invalid}<Field.Error>This field is required.</Field.Error>{:else if f.description}<Field.Description>{f.description}</Field.Description>{/if}
					</Field.Field>
				{/each}
			</Field.Group>
			<button type="submit" hidden aria-hidden="true"></button>
		</form>
	{/if}

	<div class="flex flex-wrap items-center gap-1 border-t pt-2">
		{#if isNew}
			<span class="px-1 text-sm text-muted-foreground">New record</span>
			{#if total > 0}
				<Button size="sm" variant="ghost" disabled={editing} onclick={() => go(position)}>Cancel</Button>
			{/if}
		{:else}
			<Button size="icon-sm" variant="ghost" aria-label="Previous record" disabled={editing || position === 0} onclick={() => go(position - 1)}>
				<ChevronLeftIcon />
			</Button>
			<span class="px-1 text-sm text-muted-foreground tabular-nums">{total ? position + 1 : 0} of {total}</span>
			<Button size="icon-sm" variant="ghost" aria-label="Next record" disabled={editing || position + 1 >= total} onclick={() => go(position + 1)}>
				<ChevronRightIcon />
			</Button>
		{/if}
		<span class="flex-1"></span>
		{#if buf && !isNew && widget.allowDelete}
			<Button size="icon-sm" variant="destructive" aria-label="Delete record" disabled={editing} onclick={remove}><TrashIcon /></Button>
		{/if}
		{#if widget.allowNew && !isNew}
			<Button size="sm" variant="outline" disabled={editing} onclick={startNew}><PlusIcon data-icon="inline-start" /> New</Button>
		{/if}
		{#if buf && !readOnly}
			<Button size="icon-sm" variant="ghost" aria-label="Undo changes" disabled={editing || !dirty} onclick={() => buf && rb.undoAll(buf)}>
				<Undo2Icon />
			</Button>
			<Button size="sm" disabled={editing || saving || (!isNew && !dirty)} onclick={save}>
				{#if saving}<Spinner data-icon="inline-start" />{/if}
				{isNew ? 'Add' : 'Save'}
			</Button>
		{/if}
	</div>
</div>
