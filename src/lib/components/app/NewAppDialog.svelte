<script lang="ts">
	// Utils
	import { untrack } from 'svelte';
	import { TEMPLATES } from '#lib/logic/templates.js';
	import { databaseNameProblem } from '#lib/logic/values.js';
	import { workspace } from '#lib/state/workspace.svelte.js';
	import { cn } from '#lib/utils.js';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import { Checkbox } from '#lib/components/ui/checkbox/index.js';
	import * as Dialog from '#lib/components/ui/dialog/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as Item from '#lib/components/ui/item/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';

	// Icons
	import {
		LayoutTemplate as LayoutTemplateIcon,
		SquareDashed as SquareDashedIcon,
	} from '@lucide/svelte';

	// Script
	let {
		open = $bindable(false),
		onCreated
	}: { open?: boolean; onCreated: (name: string) => void } = $props();

	const uid = $props.id();
	let name = $state('');
	let start = $state<string>('blank');
	let databases = $state<string[]>([]);
	let touched = $state(false);
	let busy = $state(false);

	const template = $derived(TEMPLATES.find((t) => t.id === start) ?? null);
	const available = $derived(workspace.databases.filter((d) => d.valid).map((d) => d.name));

	function freeName(base: string): string {
		const taken = (n: string) => workspace.appNameTaken(n) || (start !== 'blank' && workspace.nameTaken(n));
		if (!taken(base)) return base;
		let n = 2;
		while (taken(`${base} ${n}`)) n++;
		return `${base} ${n}`;
	}

	// Reset each time the dialog opens (not when the folder listing refreshes).
	$effect(() => {
		if (!open) return;
		untrack(() => {
			start = 'blank';
			databases = available.length === 1 ? [...available] : [];
			name = freeName('My app');
			touched = false;
		});
	});

	const problem = $derived(
		databaseNameProblem(name) ??
			(workspace.appNameTaken(name)
				? 'An app with this name already exists'
				: template && workspace.nameTaken(name)
					? 'A database with this name already exists'
					: null)
	);

	function choose(id: string) {
		start = id;
		// Suggest the template's name until the user types their own.
		if (!touched) name = freeName(TEMPLATES.find((t) => t.id === id)?.suggestedName ?? 'My app');
	}

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		touched = true;
		if (problem || busy) return;
		busy = true;
		try {
			const ok = template
				? await workspace.createFromTemplate(template, name)
				: await workspace.createApp(name, databases);
			if (ok) {
				open = false;
				onCreated(name);
			}
		} finally {
			busy = false;
		}
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-lg">
		<form onsubmit={submit} class="flex flex-col gap-6">
			<Dialog.Header>
				<Dialog.Title>New app</Dialog.Title>
				<Dialog.Description>Pages of forms, tables and charts over your databases.</Dialog.Description>
			</Dialog.Header>

			<Field.Group>
				<Field.Field data-invalid={touched && problem ? true : undefined}>
					<Field.Label for="{uid}-name">Name</Field.Label>
					<Input
						id="{uid}-name"
						bind:value={name}
						autocomplete="off"
						aria-invalid={touched && problem ? true : undefined}
						oninput={() => (touched = true)}
					/>
					{#if touched && problem}<Field.Error>{problem}</Field.Error>{/if}
				</Field.Field>

				<Field.Set>
					<Field.Legend variant="label">Start from</Field.Legend>
					<div class="flex flex-col gap-2">
						{#each [{ id: 'blank', name: 'Blank app', description: 'Connect it to databases you already have.' }, ...TEMPLATES] as t (t.id)}
							<Item.Root variant="outline" size="sm" class={cn('cursor-pointer', start === t.id && 'border-ring bg-accent')}>
								{#snippet child({ props })}
									<button type="button" {...props} aria-pressed={start === t.id} onclick={() => choose(t.id)}>
										<Item.Media variant="icon">
											{#if t.id === 'blank'}<SquareDashedIcon />{:else}<LayoutTemplateIcon />{/if}
										</Item.Media>
										<Item.Content class="text-start">
											<Item.Title>{t.name}</Item.Title>
											<Item.Description>
												{t.description}{t.id !== 'blank' ? ' Creates a new database with sample records.' : ''}
											</Item.Description>
										</Item.Content>
									</button>
								{/snippet}
							</Item.Root>
						{/each}
					</div>
				</Field.Set>

				{#if !template}
					<Field.Set>
						<Field.Legend variant="label">Databases</Field.Legend>
						{#if available.length}
							<div class="flex max-h-40 flex-col gap-2 overflow-y-auto">
								{#each available as db (db)}
									<Field.Field orientation="horizontal">
										<Checkbox
											id="{uid}-{db}"
											checked={databases.includes(db)}
											onCheckedChange={(v) => (databases = v ? [...databases, db] : databases.filter((x) => x !== db))}
										/>
										<Field.Label for="{uid}-{db}" class="font-normal">{db}</Field.Label>
									</Field.Field>
								{/each}
							</div>
							<Field.Description>You can add or remove databases later.</Field.Description>
						{:else}
							<Field.Description>There are no databases yet; you can add them to the app later.</Field.Description>
						{/if}
					</Field.Set>
				{/if}
			</Field.Group>

			<Dialog.Footer>
				<Button type="button" variant="outline" onclick={() => (open = false)}>Cancel</Button>
				<Button type="submit" disabled={busy}>
					{#if busy}<Spinner data-icon="inline-start" />{/if}
					Create app
				</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>
