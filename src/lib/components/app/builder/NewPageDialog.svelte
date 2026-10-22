<script lang="ts">
	import * as Dialog from '#lib/components/ui/dialog/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import * as Item from '#lib/components/ui/item/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import { addPageOfKind, findPage, PAGE_KINDS, type Binding, type PageKind } from '#lib/logic/app-def.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import { cn } from '#lib/utils.js';
	import AppIcon from './AppIcon.svelte';
	import BindingPicker from './BindingPicker.svelte';

	let { open = $bindable(false) }: { open?: boolean } = $props();

	const uid = $props.id();
	let kind = $state<PageKind>('list');
	let binding = $state<Binding | null>(null);
	let title = $state('');
	let titleEdited = $state(false);
	let group = $state('');
	let touched = $state(false);
	let busy = $state(false);

	const def = $derived(appSession.def);
	const groups = $derived([...new Set((def?.pages ?? []).map((p) => p.group).filter(Boolean))]);
	const spec = $derived(PAGE_KINDS.find((k) => k.kind === kind)!);
	const tablesOnly = $derived(kind === 'form' || kind === 'list-form');
	const isTable = $derived(
		!!binding && appSession.objects[binding.source]?.find((o) => o.name === binding!.table)?.kind === 'table'
	);

	// Reset each time the dialog opens.
	$effect(() => {
		if (!open) return;
		kind = 'list';
		binding = null;
		title = '';
		titleEdited = false;
		group = appSession.page?.group ?? '';
		touched = false;
	});

	// A saved query can't back a form.
	$effect(() => {
		if (tablesOnly && binding && !isTable) binding = null;
	});

	function suggestedTitle(): string {
		if (kind === 'blank' || !binding) return `Page ${(def?.pages.length ?? 0) + 1}`;
		if (kind === 'dashboard') return `${binding.table} overview`;
		if (kind === 'query') return `${binding.table} query`;
		return binding.table;
	}

	const shownTitle = $derived(titleEdited ? title : suggestedTitle());

	const problem = $derived.by(() => {
		const t = shownTitle.trim();
		if (!t) return 'Enter a name';
		if (def?.pages.some((p) => p.title.toLowerCase() === t.toLowerCase())) return 'A page with this name already exists';
		if (spec.needsData && !binding) return tablesOnly ? 'Choose a table' : 'Choose a table or saved query';
		return null;
	});

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		touched = true;
		if (problem || busy || !def) return;
		busy = true;
		try {
			const data = spec.needsData ? binding : null;
			let key: string | undefined;
			if (kind === 'list-form' && data) {
				const schema = await appSession.objectSchema(data.source, data.table);
				const fields = schema?.def?.fields ?? [];
				key = (fields.find((f) => f.primary_key) ?? fields[0])?.name;
			}
			let id = '';
			appSession.change((d) => (id = addPageOfKind(d, kind, shownTitle.trim(), data, { group: group.trim(), key }).id));
			const page = appSession.def && findPage(appSession.def, id);
			for (const w of page?.widgets ?? []) appSession.loadWidgetSchema(w);
			await appSession.showPage(id);
			open = false;
		} finally {
			busy = false;
		}
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-xl">
		<form onsubmit={submit} class="flex flex-col gap-6">
			<Dialog.Header>
				<Dialog.Title>New page</Dialog.Title>
				<Dialog.Description>Start from a ready-made page; you can change everything on it afterwards.</Dialog.Description>
			</Dialog.Header>
			<div class="grid grid-cols-2 gap-2" role="radiogroup" aria-label="Kind of page">
				{#each PAGE_KINDS as k (k.kind)}
					<Item.Root
						variant="outline"
						size="sm"
						class={cn('cursor-pointer hover:bg-accent', kind === k.kind && 'border-ring bg-accent')}
					>
						{#snippet child({ props })}
							<button type="button" role="radio" aria-checked={kind === k.kind} {...props} onclick={() => (kind = k.kind)}>
								<Item.Media variant="icon"><AppIcon name={k.icon} /></Item.Media>
								<Item.Content class="text-start">
									<Item.Title>{k.label}</Item.Title>
									<Item.Description>{k.description}</Item.Description>
								</Item.Content>
							</button>
						{/snippet}
					</Item.Root>
				{/each}
			</div>
			<Field.Group>
				{#if spec.needsData}
					<Field.Field data-invalid={touched && !binding ? true : undefined}>
						<Field.Label for="{uid}-data">{kind === 'query' ? 'Start from' : 'Table'}</Field.Label>
						<BindingPicker id="{uid}-data" {binding} {tablesOnly} onchange={(b) => (binding = b)} />
						{#if kind === 'query'}<Field.Description>The query starts as all of its records; edit the SQL on the page.</Field.Description>{/if}
					</Field.Field>
				{/if}
				<div class="grid grid-cols-2 gap-3">
					<Field.Field data-invalid={touched && !!problem && !!binding ? true : undefined}>
						<Field.Label for="{uid}-title">Name</Field.Label>
						<Input
							id="{uid}-title"
							autocomplete="off"
							value={shownTitle}
							oninput={(e) => {
								title = e.currentTarget.value;
								titleEdited = true;
							}}
						/>
					</Field.Field>
					<Field.Field>
						<Field.Label for="{uid}-group">Sidebar heading</Field.Label>
						<Input id="{uid}-group" autocomplete="off" placeholder="None" list="{uid}-groups" bind:value={group} />
						<datalist id="{uid}-groups">{#each groups as g (g)}<option value={g}></option>{/each}</datalist>
					</Field.Field>
				</div>
				{#if touched && problem}<Field.Error>{problem}</Field.Error>{/if}
			</Field.Group>
			<Dialog.Footer>
				<Button type="button" variant="outline" onclick={() => (open = false)}>Cancel</Button>
				<Button type="submit" disabled={busy}>
					{#if busy}<Spinner data-icon="inline-start" />{/if}
					Add page
				</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>
