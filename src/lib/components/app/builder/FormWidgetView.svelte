<script lang="ts">
	// Services
	import { recordService } from '#lib/services/records.js';

	// Types
	import type { RecordPage } from '#lib/api/types.js';

	// Utils
	import { tick, untrack } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { isBlank, prefillFromFilters, resolveFilters, variablesRead, type FormWidget } from '#lib/logic/app-def.js';
	import { loadLookups, lookupLabel, type LookupOption } from '#lib/logic/lookups.js';
	import * as rb from '#lib/logic/record-buffer.js';
	import { displayValue, toEditorValue } from '#lib/logic/values.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import { attempt, cleanMessage, confirm } from '#lib/state/feedback.svelte.js';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Kbd } from '#lib/components/ui/kbd/index.js';
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
	let formEl = $state<HTMLFormElement | null>(null);

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

	// ----- Quick entry -----

	const quick = $derived(widget.quickEntry && widget.allowNew);
	const keyField = $derived.by(() => {
		const pk = def?.fields.filter((f) => f.primary_key) ?? [];
		return pk.length === 1 ? pk[0].name : null;
	});
	/** Variables of "key is …" filters: whichever record a list click picked to edit. */
	const pickVariables = $derived(
		widget.filters.flatMap((f) => (f.column === keyField && f.op === 'eq' && f.value.kind === 'variable' ? [f.value.name] : []))
	);
	/** A record was picked to edit; otherwise a quick-entry form shows a blank one. */
	const picked = $derived(pickVariables.some((v) => !isBlank(appSession.vars[v])));

	/** Forgets the picked record; true if there was one (the form then reloads by itself). */
	function clearPick(): boolean {
		const set = pickVariables.filter((v) => !isBlank(appSession.vars[v]));
		for (const v of set) appSession.setVariable(v, null);
		return set.length > 0;
	}

	/** Puts the cursor in the first field, ready for the next record. */
	async function focusFirst() {
		await tick();
		formEl?.querySelector<HTMLElement>('input:not([type=hidden]):not([disabled]), textarea, [role=combobox], button[data-slot=select-trigger]')?.focus();
	}

	/** Loads the record at `at`. A quick-entry form shows a blank one instead unless it is `browsing` its records. */
	async function load(at: number, browsing = false) {
		const d = widget.data;
		if (!d || !database || !def) return;
		const definition = def;
		try {
			const res = await recordService.list(d.table, { offset: at, limit: 1, filters }, database);
			page = res;
			position = at;
			submitted = false;
			if (quick && !picked && !browsing) buf = newRecord();
			else if (res.rows.length) buf = rb.newBuffer(definition, res.rows[0], res.columns);
			else if (at > 0 && res.total > 0) await load(res.total - 1, browsing);
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
	// Whatever changed them (a row click, a button, a filter control), unsaved
	// typing is only replaced after asking; kept, the form stays as it is.
	$effect(() => {
		void filterKey;
		if (!def || !database) return;
		untrack(async () => {
			if (await canLeave()) load(0);
		});
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
			untrack(() => !dirty && load(position, !isNew));
		}
	});

	/** Asks before throwing away unsaved edits; discarding them here means nobody asks again. */
	async function canLeave(): Promise<boolean> {
		if (!dirty) return true;
		const ok = await confirm({
			title: 'Discard changes?',
			description: `Your changes in “${widget.title || 'the form'}” have not been saved.`,
			action: 'Discard',
			destructive: true
		});
		if (ok && buf) rb.undoAll(buf);
		return ok;
	}

	// Asked before leaving the page, and before a click elsewhere changes the variables this form filters by.
	$effect(() => {
		appSession.guard(widget.id, dirty && !editing ? canLeave : null, variablesRead(widget));
		return () => appSession.guard(widget.id, null);
	});

	async function go(at: number) {
		if (await canLeave()) load(at, true);
	}

	/** Leaves the picked record and goes back to a blank one. */
	async function cancelEdit() {
		if (!(await canLeave())) return;
		submitted = false;
		if (!clearPick()) {
			buf = newRecord();
			focusFirst();
		}
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
		if (quick) {
			// Ready for the next record straight away.
			loadedRevision = ++appSession.dataRevision;
			buf = newRecord();
			submitted = false;
			if (!clearPick()) await load(0);
			focusFirst();
			return;
		}
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
		loadedRevision = ++appSession.dataRevision;
		if (quick) {
			// The deleted record can't stay picked.
			buf = newRecord();
			if (!clearPick()) await load(0);
			return;
		}
		buf = null;
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
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<form
			bind:this={formEl}
			class="min-h-0 flex-1 overflow-y-auto pe-1"
			onsubmit={(e) => {
				e.preventDefault();
				save();
			}}
			onkeydown={(e) => {
				// Enter in a box saves (handled here: hidden submit buttons don't reliably submit in every webview).
				// Enter in long text adds a line; ⌘Enter saves from anywhere.
				if (e.key !== 'Enter' || e.isComposing || readOnly) return;
				const t = e.target as HTMLElement;
				const inBox = t instanceof HTMLInputElement && t.type !== 'file';
				if (inBox || e.metaKey || e.ctrlKey) {
					e.preventDefault();
					save();
				}
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
		{#if quick}
			{#if isNew}
				<span class="flex items-center gap-1.5 px-1 text-sm text-muted-foreground">
					New record
					<span class="text-xs">· <Kbd>Enter</Kbd> to add</span>
				</span>
				{#if total > 0 && !picked}
					<!-- Without a list to pick from, the records are browsed here. -->
					<Button size="sm" variant="ghost" disabled={editing} onclick={() => go(total - 1)}>
						<ChevronLeftIcon data-icon="inline-start" /> Records
					</Button>
				{/if}
			{:else}
				{#if !picked && total > 1}
					<Button size="icon-sm" variant="ghost" aria-label="Previous record" disabled={editing || position === 0} onclick={() => go(position - 1)}>
						<ChevronLeftIcon />
					</Button>
					<span class="px-1 text-sm text-muted-foreground tabular-nums">{position + 1} of {total}</span>
					<Button size="icon-sm" variant="ghost" aria-label="Next record" disabled={editing || position + 1 >= total} onclick={() => go(position + 1)}>
						<ChevronRightIcon />
					</Button>
				{:else}
					<span class="px-1 text-sm text-muted-foreground">Editing</span>
				{/if}
				<Button size="sm" variant="ghost" disabled={editing} onclick={cancelEdit}>{dirty ? 'Cancel' : 'New record'}</Button>
			{/if}
		{:else if isNew}
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
		{#if widget.allowNew && !isNew && !quick}
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
