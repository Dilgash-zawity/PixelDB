<script lang="ts">
	import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';
	import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
	import DatabaseIcon from '@lucide/svelte/icons/database';
	import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
	import FileIcon from '@lucide/svelte/icons/file';
	import HouseIcon from '@lucide/svelte/icons/house';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import TrashIcon from '@lucide/svelte/icons/trash-2';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';
	import VariableIcon from '@lucide/svelte/icons/variable';
	import XIcon from '@lucide/svelte/icons/x';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as Item from '#lib/components/ui/item/index.js';
	import { ScrollArea } from '#lib/components/ui/scroll-area/index.js';
	import * as Tabs from '#lib/components/ui/tabs/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import {
		ACCENTS,
		addPage,
		addSource,
		addVariable,
		findPage,
		movePage,
		removePage,
		removeSource,
		removeVariable,
		renameVariable,
		variableProblem,
		variablesRead,
		widgetsUsingSource,
		WIDGET_TYPES,
		type AppAccent,
		type WidgetType
	} from '#lib/logic/app-def.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import { confirm } from '#lib/state/feedback.svelte.js';
	import { cn } from '#lib/utils.js';
	import NameDialog from '../NameDialog.svelte';
	import PropSelect from './PropSelect.svelte';
	import WidgetTypeIcon from './WidgetTypeIcon.svelte';

	let tab = $state('add');
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

	const pageTitleProblem = (v: string, except?: string) =>
		!v.trim() ? 'Enter a name' : def.pages.some((p) => p.id !== except && p.title.toLowerCase() === v.trim().toLowerCase()) ? 'A page with this name already exists' : null;

	function newPage() {
		naming = {
			title: 'New page',
			initial: `Page ${def.pages.length + 1}`,
			submit: 'Add page',
			validate: (v) => pageTitleProblem(v),
			apply: (v) => {
				let id = '';
				appSession.change((d) => (id = addPage(d, v.trim()).id));
				appSession.showPage(id);
			}
		};
	}

	function renamePage(id: string, title: string) {
		naming = {
			title: 'Rename page',
			initial: title,
			submit: 'Rename',
			validate: (v) => pageTitleProblem(v, id),
			apply: (v) => appSession.change((d) => {
				const p = findPage(d, id);
				if (p) p.title = v.trim();
			})
		};
	}

	async function deletePage(id: string, title: string) {
		const count = findPage(def, id)?.widgets.length ?? 0;
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
		if (appSession.pageId === id) appSession.showPage(def.pages[0].id);
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

<aside class="flex h-full w-64 shrink-0 flex-col border-e bg-sidebar print:hidden">
	<Tabs.Root bind:value={tab} class="flex min-h-0 flex-1 flex-col gap-0">
		<div class="border-b p-2">
			<Tabs.List class="w-full">
				<Tabs.Trigger value="add">Add</Tabs.Trigger>
				<Tabs.Trigger value="pages">Pages</Tabs.Trigger>
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

			<Tabs.Content value="pages" class="flex flex-col gap-1 p-2">
				{#each def.pages as p, i (p.id)}
					<div
						class={cn(
							'group/page flex items-center gap-1 px-1 py-0.5 text-sm hover:bg-accent',
							p.id === appSession.page?.id && 'bg-accent font-medium'
						)}
					>
						<button type="button" class="flex min-w-0 flex-1 items-center gap-2 py-1 text-start" onclick={() => appSession.showPage(p.id)}>
							{#if p.id === def.startPage}<HouseIcon class="size-4 shrink-0" />{:else}<FileIcon class="size-4 shrink-0 text-muted-foreground" />{/if}
							<span class="truncate">{p.title}</span>
						</button>
						<DropdownMenu.Root>
							<DropdownMenu.Trigger>
								{#snippet child({ props })}
									<Button {...props} size="icon-xs" variant="ghost" aria-label="Page options"><EllipsisIcon /></Button>
								{/snippet}
							</DropdownMenu.Trigger>
							<DropdownMenu.Content align="end">
								<DropdownMenu.Group>
									<DropdownMenu.Item onclick={() => renamePage(p.id, p.title)}><PencilIcon /> Rename</DropdownMenu.Item>
									<DropdownMenu.Item disabled={p.id === def.startPage} onclick={() => appSession.change((d) => (d.startPage = p.id))}>
										<HouseIcon /> Open first
									</DropdownMenu.Item>
									<DropdownMenu.Item disabled={i === 0} onclick={() => appSession.change((d) => movePage(d, p.id, i - 1))}><ArrowUpIcon /> Move up</DropdownMenu.Item>
									<DropdownMenu.Item disabled={i === def.pages.length - 1} onclick={() => appSession.change((d) => movePage(d, p.id, i + 1))}>
										<ArrowDownIcon /> Move down
									</DropdownMenu.Item>
								</DropdownMenu.Group>
								<DropdownMenu.Separator />
								<DropdownMenu.Group>
									<DropdownMenu.Item variant="destructive" disabled={def.pages.length === 1} onclick={() => deletePage(p.id, p.title)}>
										<TrashIcon /> Delete
									</DropdownMenu.Item>
								</DropdownMenu.Group>
							</DropdownMenu.Content>
						</DropdownMenu.Root>
					</div>
				{/each}
				<Button size="sm" variant="outline" class="mt-1" onclick={newPage}><PlusIcon data-icon="inline-start" /> New page</Button>
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
