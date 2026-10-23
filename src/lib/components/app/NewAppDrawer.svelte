<script lang="ts">
	// Utils
	import { tick, untrack } from 'svelte';
	import { TEMPLATES } from '#lib/logic/templates.js';
	import { databaseNameProblem } from '#lib/logic/values.js';
	import { workspace } from '#lib/state/workspace.svelte.js';
	import { cn } from '#lib/utils.js';

	// Components
	import { Badge } from '#lib/components/ui/badge/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import StepDrawer from './StepDrawer.svelte';

	// Icons
	import {
		Check as CheckIcon,
		Database as DatabaseIcon,
		LayoutDashboard as LayoutDashboardIcon,
		LayoutTemplate as LayoutTemplateIcon,
		Sparkles as SparklesIcon,
		SquareDashed as SquareDashedIcon,
		TextCursorInput as TextCursorInputIcon,
		WandSparkles as WandSparklesIcon,
	} from '@lucide/svelte';

	// Script
	/**
	 * Creates an app step by step: what to start from, its name, the
	 * databases it uses (blank apps only), then a last look.
	 */
	let {
		open = $bindable(false),
		onCreated
	}: { open?: boolean; onCreated: (name: string) => void } = $props();

	const BLANK = { id: 'blank', name: 'Blank app', description: 'Pages over databases you already have.' };

	let step = $state(0);
	let name = $state('');
	let start = $state<string>('blank');
	let databases = $state<string[]>([]);
	let build = $state(true);
	let touched = $state(false);
	let renamed = $state(false);
	let busy = $state(false);
	let nameInput = $state<HTMLInputElement | null>(null);

	const template = $derived(TEMPLATES.find((t) => t.id === start) ?? null);
	const available = $derived(workspace.databases.filter((d) => d.valid).map((d) => d.name));
	const steps = $derived([
		{ label: 'Start', icon: LayoutTemplateIcon },
		{ label: 'Name', icon: TextCursorInputIcon },
		...(template ? [] : [{ label: 'Data', icon: DatabaseIcon }]),
		{ label: 'Review', icon: SparklesIcon }
	]);
	const NAME_STEP = 1;

	function freeName(base: string): string {
		const taken = (n: string) => workspace.appNameTaken(n) || (start !== 'blank' && workspace.nameTaken(n));
		if (!taken(base)) return base;
		let n = 2;
		while (taken(`${base} ${n}`)) n++;
		return `${base} ${n}`;
	}

	// Start over each time it opens (not when the folder listing refreshes).
	$effect(() => {
		if (!open) return;
		untrack(() => {
			step = 0;
			start = 'blank';
			databases = available.length === 1 ? [...available] : [];
			build = true;
			name = freeName('My app');
			touched = false;
			renamed = false;
		});
	});

	// Ready to type on the name step.
	$effect(() => {
		if (open && step === NAME_STEP) tick().then(() => nameInput?.select());
	});

	const problem = $derived(
		databaseNameProblem(name.trim()) ??
			(workspace.appNameTaken(name.trim())
				? 'An app with this name already exists'
				: template && workspace.nameTaken(name.trim())
					? 'A database with this name already exists'
					: null)
	);

	function choose(id: string, andGo = false) {
		start = id;
		// Suggest the template's name until the user types their own.
		if (!renamed) name = freeName(TEMPLATES.find((t) => t.id === id)?.suggestedName ?? 'My app');
		if (andGo) step = NAME_STEP;
	}

	function toggle(db: string) {
		databases = databases.includes(db) ? databases.filter((x) => x !== db) : [...databases, db];
	}

	function next(at: number) {
		if (at === NAME_STEP) {
			touched = true;
			if (problem) return false;
		}
	}

	async function submit() {
		touched = true;
		if (problem) {
			step = NAME_STEP;
			return;
		}
		if (busy) return;
		busy = true;
		const n = name.trim();
		try {
			const ok = template ? await workspace.createFromTemplate(template, n) : await workspace.createApp(n, databases, build && databases.length > 0);
			if (ok) {
				open = false;
				onCreated(n);
			}
		} finally {
			busy = false;
		}
	}

	const card = (selected: boolean) =>
		cn(
			'flex items-start gap-3 rounded-lg border bg-card p-3 text-start outline-none transition-colors hover:bg-accent/50 focus-visible:ring-3 focus-visible:ring-ring/40',
			selected && 'border-primary bg-primary/5 ring-1 ring-primary'
		);
	const tile = (selected: boolean) =>
		cn('flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground', selected && 'bg-primary text-primary-foreground');
</script>

<StepDrawer
	bind:open
	bind:step
	title="New app"
	description="Pages of forms, tables and charts over your databases."
	{steps}
	finishLabel="Create app"
	finishIcon={CheckIcon}
	{busy}
	onNext={next}
	onFinish={submit}
>
	{#if step === 0}
		<Field.Set>
			<Field.Legend>What do you want to start from?</Field.Legend>
			<Field.Description>Double-click a card to choose it and go on.</Field.Description>
			<div class="grid gap-2 sm:grid-cols-2" role="radiogroup" aria-label="Start from">
				{#each [BLANK, ...TEMPLATES] as t (t.id)}
					{@const selected = start === t.id}
					<button type="button" role="radio" aria-checked={selected} class={card(selected)} onclick={() => choose(t.id)} ondblclick={() => choose(t.id, true)}>
						<span class={tile(selected)}>
							{#if t.id === 'blank'}<SquareDashedIcon class="size-4" />{:else}<LayoutTemplateIcon class="size-4" />{/if}
						</span>
						<span class="flex min-w-0 flex-col gap-1">
							<span class="text-sm font-medium">{t.name}</span>
							<span class="text-xs text-muted-foreground">{t.description}</span>
							{#if t.id !== 'blank'}<Badge variant="secondary" class="mt-0.5">New database with sample records</Badge>{/if}
						</span>
					</button>
				{/each}
			</div>
		</Field.Set>
	{:else if step === NAME_STEP}
		<Field.Field data-invalid={touched && !!problem}>
			<Field.Label for="na-name">What should the app be called?</Field.Label>
			<Input
				id="na-name"
				bind:ref={nameInput}
				bind:value={name}
				autocomplete="off"
				aria-invalid={touched && !!problem}
				oninput={() => (renamed = true)}
			/>
			{#if touched && problem}
				<Field.Error>{problem}</Field.Error>
			{:else if template}
				<Field.Description>The new database gets this name too.</Field.Description>
			{:else}
				<Field.Description>Like “Sales” or “Team tasks”. You can rename it later.</Field.Description>
			{/if}
		</Field.Field>
	{:else if !template && step === 2}
		<div class="flex flex-col gap-5">
			<Field.Set>
				<Field.Legend>Which databases does it use?</Field.Legend>
				<Field.Description>Pick any number; you can add or remove databases later.</Field.Description>
				{#if available.length}
					<div class="grid gap-2 sm:grid-cols-2">
						{#each available as db (db)}
							{@const selected = databases.includes(db)}
							<button type="button" role="checkbox" aria-checked={selected} class={card(selected)} onclick={() => toggle(db)}>
								<span class={tile(selected)}>
									{#if selected}<CheckIcon class="size-4" />{:else}<DatabaseIcon class="size-4" />{/if}
								</span>
								<span class="min-w-0 self-center truncate text-sm font-medium">{db}</span>
							</button>
						{/each}
					</div>
				{:else}
					<Empty.Root class="border border-dashed">
						<Empty.Header>
							<Empty.Media variant="icon"><DatabaseIcon /></Empty.Media>
							<Empty.Title>No databases yet</Empty.Title>
							<Empty.Description>You can create the app now and add databases to it later.</Empty.Description>
						</Empty.Header>
					</Empty.Root>
				{/if}
			</Field.Set>
			{#if available.length}
				<Field.Field orientation="horizontal" class="rounded-lg border p-3" data-disabled={!databases.length || undefined}>
					<Field.Content>
						<Field.Label for="na-build">Build a page for each table</Field.Label>
						<Field.Description>An overview, plus a list, a form and related records for every table.</Field.Description>
					</Field.Content>
					<Switch id="na-build" checked={build && databases.length > 0} disabled={!databases.length} onCheckedChange={(v) => (build = v)} />
				</Field.Field>
			{/if}
		</div>
	{:else}
		<div class="flex flex-col gap-3">
			<div class="flex items-center gap-3 rounded-lg border bg-card p-3">
				<span class="flex size-10 items-center justify-center rounded-lg bg-muted vivid:bg-tone-violet/15 vivid:text-tone-violet">
					<LayoutDashboardIcon class="size-5" />
				</span>
				<div class="flex min-w-0 flex-col">
					<span class="truncate font-semibold">{name.trim()}</span>
					<span class="text-xs text-muted-foreground">{template ? `From the ${template.name} template` : 'Blank app'}</span>
				</div>
			</div>
			<ul class="flex flex-col divide-y rounded-lg border text-sm">
				{#if template}
					<li class="flex items-center gap-2 px-3 py-2">
						<DatabaseIcon class="size-4 text-muted-foreground" /> Creates the database “{name.trim()}” with sample records
					</li>
					<li class="flex items-center gap-2 px-3 py-2">
						<LayoutTemplateIcon class="size-4 text-muted-foreground" /> {template.description}
					</li>
				{:else}
					{#each databases as db (db)}
						<li class="flex items-center gap-2 px-3 py-2"><DatabaseIcon class="size-4 text-muted-foreground" /> Uses {db}</li>
					{:else}
						<li class="px-3 py-2 text-muted-foreground">No databases yet — add them later under Data.</li>
					{/each}
					{#if databases.length}
						<li class="flex items-center gap-2 px-3 py-2">
							<WandSparklesIcon class="size-4 text-muted-foreground" />
							{build ? 'Builds a page for each table' : 'Starts with an empty page'}
						</li>
					{/if}
				{/if}
			</ul>
		</div>
	{/if}
</StepDrawer>
