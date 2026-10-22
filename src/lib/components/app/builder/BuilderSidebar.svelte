<script lang="ts">
	import DatabaseIcon from '@lucide/svelte/icons/database';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import TrashIcon from '@lucide/svelte/icons/trash-2';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';
	import VariableIcon from '@lucide/svelte/icons/variable';
	import XIcon from '@lucide/svelte/icons/x';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as Item from '#lib/components/ui/item/index.js';
	import { ScrollArea } from '#lib/components/ui/scroll-area/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import * as Tabs from '#lib/components/ui/tabs/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import {
		ACCENTS,
		addSource,
		addVariable,
		findPage,
		removePage,
		removeSource,
		removeVariable,
		renameVariable,
		variableProblem,
		variablesRead,
		widgetsUsingSource,
		WIDGET_TYPES,
		type AppAccent,
		type Page
	} from '#lib/logic/app-def.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import { confirm } from '#lib/state/feedback.svelte.js';
	import { cn } from '#lib/utils.js';
	import NameDialog from '../NameDialog.svelte';
	import IconPicker from './IconPicker.svelte';
	import PropSelect from './PropSelect.svelte';
	import WidgetTypeIcon from './WidgetTypeIcon.svelte';

	let naming = $state<{
		title: string;
		description?: string;
		initial: string;
		submit: string;
		validate: (v: string) => string | null;
		apply: (v: string) => void;
	} | null>(null);

	const def = $derived(appSession.def!);
	const uid = $props.id();

	// ----- The current page -----

	const page = $derived(appSession.page!);
	const groups = $derived([...new Set(def.pages.map((p) => p.group).filter(Boolean))]);
	let titleDraft = $state('');
	$effect(() => {
		titleDraft = page.title;
	});
	const titleProblem = $derived(
		!titleDraft.trim()
			? 'Enter a name'
			: def.pages.some((p) => p.id !== page.id && p.title.toLowerCase() === titleDraft.trim().toLowerCase())
				? 'A page with this name already exists'
				: null
	);

	function editPage(change: (p: Page) => void, group?: string) {
		const id = page.id;
		appSession.change((d) => {
			const p = findPage(d, id);
			if (p) change(p);
		}, group ? `${id}:${group}` : undefined);
	}

	async function deletePage() {
		const { id, title } = page;
		const count = page.widgets.length;
		const ok = await confirm({
			title: `Delete page “${title}”?`,
			description: count
				? `Its ${count} widget${count === 1 ? '' : 's'} will be removed too. The data stays in the database. You can undo this.`
				: 'You can undo this.',
			action: 'Delete page',
			destructive: true
		});
		if (!ok) return;
		appSession.change((d) => removePage(d, id));
		appSession.showPage(def.startPage);
	}

	// ----- Databases -----

	const addableDatabases = $derived(
		appSession.databases.filter((d) => !def.sources.some((s) => s.database === d)).map((d) => ({ value: d, label: d }))
	);

	async function addDatabase(database: string) {
		appSession.change((d) => addSource(d, database));
		await appSession.loadSchema();
	}

	async function removeDatabase(alias: string, database: string) {
		const used = widgetsUsingSource(def, alias).length;
		if (
			used &&
			!(await confirm({
				title: `Remove “${database}” from the app?`,
				description: `${used} widget${used === 1 ? ' reads' : 's read'} from it and will need new data. The database itself is not changed. You can undo this.`,
				action: 'Remove',
				destructive: true
			}))
		) {
			return;
		}
		appSession.change((d) => removeSource(d, alias));
	}

	// ----- Variables -----

	function newVariable() {
		naming = {
			title: 'New variable',
			description: 'Filter widgets and buttons set variables; other widgets filter by them.',
			initial: '',
			submit: 'Add variable',
			validate: (v) => variableProblem(def, v),
			apply: (v) => {
				appSession.change((d) => addVariable(d, v, null));
				appSession.setVariable(v, null);
			}
		};
	}

	function renameVar(name: string) {
		naming = {
			title: 'Rename variable',
			description: 'Every widget that uses it is updated.',
			initial: name,
			submit: 'Rename',
			validate: (v) => variableProblem(def, v, name),
			apply: (v) => {
				appSession.change((d) => renameVariable(d, name, v));
				appSession.setVariable(v, appSession.vars[name] ?? null);
			}
		};
	}

	async function deleteVar(name: string) {
		const readers = def.pages.flatMap((p) => p.widgets).filter((w) => variablesRead(w).includes(name)).length;
		if (
			readers &&
			!(await confirm({
				title: `Delete variable “${name}”?`,
				description: `${readers} widget${readers === 1 ? ' filters' : 's filter'} by it; those filters are removed. You can undo this.`,
				action: 'Delete variable',
				destructive: true
			}))
		) {
			return;
		}
		appSession.change((d) => removeVariable(d, name));
	}

	const ACCENT_SWATCH: Record<AppAccent, string> = {
		neutral: 'bg-primary',
		blue: 'bg-[oklch(0.55_0.2_255)]',
		green: 'bg-[oklch(0.6_0.15_150)]',
		amber: 'bg-[oklch(0.75_0.16_75)]',
		red: 'bg-[oklch(0.6_0.22_25)]',
		violet: 'bg-[oklch(0.55_0.22_295)]'
	};
</script>

<aside class="flex h-full w-80 shrink-0 flex-col border-s bg-sidebar print:hidden">
	<Tabs.Root bind:value={appSession.builderTab} class="flex min-h-0 flex-1 flex-col gap-0">
		<div class="border-b p-2">
			<Tabs.List class="w-full">
				<Tabs.Trigger value="add">Add</Tabs.Trigger>
				<Tabs.Trigger value="page">Page</Tabs.Trigger>
				<Tabs.Trigger value="data">Data</Tabs.Trigger>
				<Tabs.Trigger value="style">Style</Tabs.Trigger>
			</Tabs.List>
		</div>
		<ScrollArea class="min-h-0 flex-1">
			<Tabs.Content value="add" class="flex flex-col gap-1 p-2">
				{#each WIDGET_TYPES as t (t.type)}
					<Item.Root size="sm" class="cursor-pointer hover:bg-accent" onclick={() => appSession.addWidget(t.type)}>
						{#snippet child({ props })}
							<button type="button" {...props}>
								<Item.Media variant="icon"><WidgetTypeIcon type={t.type} /></Item.Media>
								<Item.Content class="text-start">
									<Item.Title>{t.label}</Item.Title>
									<Item.Description>{t.description}</Item.Description>
								</Item.Content>
							</button>
						{/snippet}
					</Item.Root>
				{/each}
				{#if !def.sources.length}
					<p class="p-2 text-xs text-muted-foreground">Add a database under <strong>Data</strong> to show records.</p>
				{/if}
			</Tabs.Content>

			<Tabs.Content value="page" class="flex flex-col gap-5 p-3">
				<Field.Group class="gap-4">
					<Field.Field data-invalid={titleProblem ? true : undefined}>
						<Field.Label for="{uid}-ptitle">Name</Field.Label>
						<div class="flex gap-1.5">
							<IconPicker value={page.icon} label="Page icon" onchange={(icon) => editPage((p) => (p.icon = icon))} />
							<Input
								id="{uid}-ptitle"
								class="h-8"
								bind:value={titleDraft}
								aria-invalid={titleProblem ? true : undefined}
								oninput={() => {
									const v = titleDraft.trim();
									if (!titleProblem) editPage((p) => (p.title = v), 'title');
								}}
								onblur={() => {
									appSession.seal();
									titleDraft = page.title;
								}}
							/>
						</div>
						{#if titleProblem}<Field.Error>{titleProblem}</Field.Error>{/if}
					</Field.Field>
					<Field.Field>
						<Field.Label for="{uid}-pgroup">Sidebar heading</Field.Label>
						<Input
							id="{uid}-pgroup"
							class="h-8"
							placeholder="None"
							list="{uid}-pgroups"
							value={page.group}
							oninput={(e) => {
								const v = e.currentTarget.value;
								editPage((p) => (p.group = v.trim()), 'group');
							}}
							onblur={() => appSession.seal()}
						/>
						<datalist id="{uid}-pgroups">{#each groups as g (g)}<option value={g}></option>{/each}</datalist>
						<Field.Description>Pages with the same heading are listed together. Drag pages in the sidebar to reorder them.</Field.Description>
					</Field.Field>
					<Field.Field orientation="horizontal">
						<Switch id="{uid}-pshow" checked={!page.hidden} onCheckedChange={(v) => editPage((p) => (p.hidden = !v))} />
						<Field.Label for="{uid}-pshow" class="font-normal">Show in the sidebar</Field.Label>
					</Field.Field>
					<Field.Field orientation="horizontal">
						<Switch
							id="{uid}-pfirst"
							checked={def.startPage === page.id}
							disabled={def.startPage === page.id}
							onCheckedChange={(v) => v && appSession.change((d) => (d.startPage = page.id))}
						/>
						<Field.Label for="{uid}-pfirst" class="font-normal">Open this page first</Field.Label>
					</Field.Field>
				</Field.Group>
				<Button size="sm" variant="destructive" disabled={def.pages.length === 1} onclick={deletePage}>
					<TrashIcon data-icon="inline-start" /> Delete page
				</Button>
			</Tabs.Content>

			<Tabs.Content value="data" class="flex flex-col gap-5 p-2">
				<Field.Set>
					<Field.Legend variant="label">Databases</Field.Legend>
					<div class="flex flex-col gap-1">
						{#each def.sources as s (s.alias)}
							{@const missing = !appSession.databases.includes(s.database)}
							<div class="flex items-center gap-2 px-1 py-1 text-sm">
								{#if missing}<TriangleAlertIcon class="size-4 shrink-0 text-warning" />{:else}<DatabaseIcon class="size-4 shrink-0 text-muted-foreground" />{/if}
								<span class="min-w-0 flex-1 truncate" title={missing ? 'This database is missing from the folder' : s.database}>{s.database}</span>
								<Button size="icon-xs" variant="ghost" aria-label="Remove {s.database}" onclick={() => removeDatabase(s.alias, s.database)}><XIcon /></Button>
							</div>
						{:else}
							<p class="px-1 text-xs text-muted-foreground">No databases yet.</p>
						{/each}
						{#if addableDatabases.length}
							<PropSelect label="Add a database" value="" options={addableDatabases} placeholder="Add a database…" onchange={addDatabase} />
						{/if}
					</div>
				</Field.Set>
				<Field.Set>
					<Field.Legend variant="label">Variables</Field.Legend>
					<div class="flex flex-col gap-2">
						{#each def.variables as v, i (v.name)}
							<div class="flex flex-col gap-1 border p-2">
								<div class="flex items-center gap-1 text-sm">
									<VariableIcon class="size-4 shrink-0 text-muted-foreground" />
									<span class="min-w-0 flex-1 truncate font-mono text-xs">{v.name}</span>
									<Button size="icon-xs" variant="ghost" aria-label="Rename {v.name}" onclick={() => renameVar(v.name)}><PencilIcon /></Button>
									<Button size="icon-xs" variant="ghost" aria-label="Delete {v.name}" onclick={() => deleteVar(v.name)}><TrashIcon /></Button>
								</div>
								<Input
									class="h-7"
									aria-label="Starting value of {v.name}"
									placeholder="Starts empty"
									value={v.default === null ? '' : String(v.default)}
									oninput={(e) => {
										const raw = e.currentTarget.value;
										appSession.change((d) => (d.variables[i].default = raw === '' ? null : raw), `var-default-${v.name}`);
										appSession.setVariable(v.name, raw === '' ? null : raw);
									}}
									onblur={() => appSession.seal()}
								/>
							</div>
						{/each}
						<Button size="sm" variant="outline" onclick={newVariable}><PlusIcon data-icon="inline-start" /> New variable</Button>
					</div>
				</Field.Set>
			</Tabs.Content>

			<Tabs.Content value="style" class="flex flex-col gap-5 p-2">
				<Field.Set>
					<Field.Legend variant="label">Accent color</Field.Legend>
					<div class="flex flex-wrap gap-2">
						{#each ACCENTS as a (a)}
							<button
								type="button"
								class={cn('size-7 border-2 border-transparent', ACCENT_SWATCH[a], def.accent === a && 'ring-2 ring-ring ring-offset-2 ring-offset-sidebar')}
								aria-label="{a} accent"
								aria-pressed={def.accent === a}
								onclick={() => appSession.change((d) => (d.accent = a))}
							></button>
						{/each}
					</div>
					<Field.Description>Used for charts and highlights.</Field.Description>
				</Field.Set>
				<Field.Field>
					<Field.Label for="{uid}-desc">Description</Field.Label>
					<Textarea
						id="{uid}-desc"
						rows={4}
						placeholder="What this app is for"
						value={def.description}
						oninput={(e) => {
							const v = e.currentTarget.value;
							appSession.change((d) => (d.description = v), 'description');
						}}
						onblur={() => appSession.seal()}
					/>
					<Field.Description>Shown on the app's card on the home screen.</Field.Description>
				</Field.Field>
			</Tabs.Content>
		</ScrollArea>
	</Tabs.Root>
</aside>

<NameDialog
	bind:open={() => naming !== null, (o) => !o && (naming = null)}
	title={naming?.title ?? ''}
	description={naming?.description}
	initial={naming?.initial ?? ''}
	submitLabel={naming?.submit ?? 'Save'}
	validate={(v) => naming?.validate(v) ?? null}
	onSubmit={async (v) => {
		naming?.apply(v);
		return true;
	}}
/>
