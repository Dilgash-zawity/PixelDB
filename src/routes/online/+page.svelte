<script lang="ts">
	// Services
	import { databaseService } from '#lib/services/database.js';
	import { onlineService } from '#lib/services/online.js';

	// Types
	import type { ObjectSummary, OnlineInfo } from '#lib/api/types.js';

	// Utils
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { untrack } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { groupBySchema, sqlName } from '#lib/logic/online.js';
	import { databaseHref } from '#lib/logic/routes.js';
	import { matches, shortcutText, typingTarget } from '#lib/logic/shortcuts.js';
	import { attempt, cleanMessage } from '#lib/state/feedback.svelte.js';
	import { workspace } from '#lib/state/workspace.svelte.js';

	// Components
	import Logo from '#lib/components/app/Logo.svelte';
	import NewDatabaseDrawer from '#lib/components/app/NewDatabaseDrawer.svelte';
	import OnlineCommandPalette from '#lib/components/app/online/OnlineCommandPalette.svelte';
	import OnlineDataView from '#lib/components/app/online/OnlineDataView.svelte';
	import OnlineDiagramView from '#lib/components/app/online/OnlineDiagramView.svelte';
	import OnlineOverview from '#lib/components/app/online/OnlineOverview.svelte';
	import OnlineQueryView from '#lib/components/app/online/OnlineQueryView.svelte';
	import OnlineStructureView from '#lib/components/app/online/OnlineStructureView.svelte';
	import ProviderIcon from '#lib/components/app/ProviderIcon.svelte';
	import SearchButton from '#lib/components/app/SearchButton.svelte';
	import ThemeToggle from '#lib/components/app/ThemeToggle.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import * as Tabs from '#lib/components/ui/tabs/index.js';

	// Icons
	import {
		ArrowLeft as ArrowLeftIcon,
		Check as CheckIcon,
		ChevronDown as ChevronDownIcon,
		CloudOff as CloudOffIcon,
		Database as DatabaseIcon,
		Gauge as GaugeIcon,
		ListTree as ListTreeIcon,
		Network as NetworkIcon,
		RefreshCw as RefreshIcon,
		SearchCode as SearchCodeIcon,
		Settings as SettingsIcon,
		Settings2 as Settings2Icon,
		Table2 as TableIcon
	} from '@lucide/svelte';

	// Script
	type Tab = 'overview' | 'diagram' | 'data' | 'structure' | 'query';

	const name = $derived(page.url.searchParams.get('name') ?? '');
	const connection = $derived(workspace.online.find((o) => o.name === name) ?? null);
	let tab = $state<Tab>('data');
	let objects = $state<ObjectSummary[]>([]);
	let info = $state<OnlineInfo | null>(null);
	const engine = $derived(connection?.settings?.engine ?? info?.engine ?? 'postgres');
	let table = $state<string | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	/** Bumped by Reload so the views read everything again. */
	let revision = $state(0);
	let editOpen = $state(false);

	if (!workspace.loaded) workspace.load();

	$effect(() => {
		const n = name;
		if (n) untrack(() => load(n));
	});

	async function load(n: string) {
		loading = true;
		error = null;
		try {
			const [list, about] = await Promise.all([databaseService.listObjects(n), onlineService.info(n)]);
			if (name !== n) return;
			objects = list;
			info = about;
			if (!table || !list.some((o) => o.name === table)) table = list.find((o) => o.kind === 'table')?.name ?? list[0]?.name ?? null;
			revision++;
		} catch (e) {
			if (name === n) error = cleanMessage(e instanceof Error ? e.message : String(e));
		} finally {
			if (name === n) loading = false;
		}
	}

	const groups = $derived(groupBySchema(objects, info?.schemas ?? []));
	const tableItems = $derived(objects.map((o) => ({ value: o.name, label: o.name })));
	const defaultGroup = $derived(engine === 'postgres' ? 'public' : 'Tables');
	const initialSql = $derived(
		table ? `SELECT * FROM ${sqlName(engine, table, info?.schemas ?? [])}\nLIMIT 100` : ''
	);

	function show(t: string) {
		table = t;
	}

	/** Shows table `t` on the Data or Structure tab (from the diagram and the overview). */
	function openTable(t: string, view: 'data' | 'structure') {
		table = t;
		tab = view;
	}

	async function test() {
		const settings = connection?.settings;
		if (!settings) return;
		const result = await attempt(() => workspace.testOnline(settings, undefined, name));
		if (result) toast.success(`Connected to “${name}”`, { description: `${result.server} answered in ${result.millis} ms.` });
	}

	function edited(newName: string) {
		if (newName !== name) goto(databaseHref(resolve('/online'), newName), { replaceState: true });
		else load(name);
	}

	function openDatabase(db: string, online: boolean) {
		goto(databaseHref(resolve(online ? '/online' : '/db'), db));
	}

	const TABS: Tab[] = ['overview', 'diagram', 'data', 'structure', 'query'];

	let paletteOpen = $state(false);

	function onKey(e: KeyboardEvent) {
		if (e.defaultPrevented) return;
		if (matches(e, 'mod+k')) {
			e.preventDefault();
			paletteOpen = !paletteOpen;
			return;
		}
		const n = TABS.findIndex((_, i) => matches(e, `mod+${i + 1}`));
		if (n >= 0) {
			e.preventDefault();
			tab = TABS[n];
			return;
		}
		// The next or previous table, in the picker's order.
		const step = matches(e, 'alt+arrowdown') ? 1 : matches(e, 'alt+arrowup') ? -1 : 0;
		if (step && (tab === 'data' || tab === 'structure') && !typingTarget(e.target)) {
			const order = groups.flatMap((g) => g.items.map((x) => x.item.name));
			const next = order[order.indexOf(table ?? '') + step];
			if (next) {
				e.preventDefault();
				show(next);
			}
		}
	}
</script>

<svelte:window onkeydown={onKey} />
<svelte:head><title>{name} · SlateDB</title></svelte:head>

<div class="flex h-svh flex-col">
	<header data-bar class="app-header flex h-12 shrink-0 items-center gap-2 border-b px-3">
		<DropdownMenu.Root onOpenChange={(open) => open && workspace.refresh()}>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Button {...props} variant="ghost" size="sm" class="max-w-64">
						<Logo class="size-5" />
						<span class="truncate font-semibold">{name}</span>
						<ChevronDownIcon data-icon="inline-end" />
					</Button>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content class="w-64" align="start">
				<DropdownMenu.Group>
					<DropdownMenu.Label>Databases</DropdownMenu.Label>
					{#each workspace.databases.filter((d) => d.valid) as db (db.file_name)}
						<DropdownMenu.Item onclick={() => openDatabase(db.name, false)}><DatabaseIcon /><span class="truncate">{db.name}</span></DropdownMenu.Item>
					{/each}
					{#each workspace.online.filter((o) => o.settings) as db (db.name)}
						<DropdownMenu.Item onclick={() => db.name !== name && openDatabase(db.name, true)}>
							<ProviderIcon provider={db.settings?.provider} />
							<span class="truncate">{db.name}</span>
							{#if db.name === name}<CheckIcon class="ms-auto" />{/if}
						</DropdownMenu.Item>
					{/each}
				</DropdownMenu.Group>
				<DropdownMenu.Separator />
				<DropdownMenu.Group>
					<DropdownMenu.Item disabled={!connection?.settings} onclick={() => (editOpen = true)}><Settings2Icon /> Edit connection…</DropdownMenu.Item>
					<DropdownMenu.Item onclick={() => goto(resolve('/'))}><ArrowLeftIcon /> All databases</DropdownMenu.Item>
					<DropdownMenu.Item onclick={() => goto(resolve('/settings'))}><SettingsIcon /> Settings</DropdownMenu.Item>
				</DropdownMenu.Group>
			</DropdownMenu.Content>
		</DropdownMenu.Root>

		<Separator orientation="vertical" class="data-vertical:h-5 data-vertical:self-center" />

		<Tabs.Root bind:value={tab}>
			<Tabs.List variant="line">
				<Tabs.Trigger value="overview" title="Overview ({shortcutText('mod+1')})"><GaugeIcon /> Overview</Tabs.Trigger>
				<Tabs.Trigger value="diagram" title="Diagram ({shortcutText('mod+2')})"><NetworkIcon /> Diagram</Tabs.Trigger>
				<Tabs.Trigger value="data" title="Data ({shortcutText('mod+3')})"><TableIcon /> Data</Tabs.Trigger>
				<Tabs.Trigger value="structure" title="Structure ({shortcutText('mod+4')})"><ListTreeIcon /> Structure</Tabs.Trigger>
				<Tabs.Trigger value="query" title="Query ({shortcutText('mod+5')})"><SearchCodeIcon /> Query</Tabs.Trigger>
			</Tabs.List>
		</Tabs.Root>

		{#if (tab === 'data' || tab === 'structure') && tableItems.length}
			<Select.Root type="single" items={tableItems} bind:value={() => table ?? '', (v) => v && show(v)}>
				<Select.Trigger size="sm" class="w-56" aria-label="Table" title="Table (next and previous: {shortcutText('alt+arrowdown')}, {shortcutText('alt+arrowup')})"><Select.Value placeholder="Choose a table" /></Select.Trigger>
				<Select.Content>
					{#each groups as g (g.schema)}
						<Select.Group>
							<Select.Label>{g.schema || defaultGroup}</Select.Label>
							{#each g.items as { label, item } (item.name)}
								<Select.Item value={item.name} label={item.name}>{label}{item.kind === 'view' ? ' (view)' : ''}</Select.Item>
							{/each}
						</Select.Group>
					{/each}
				</Select.Content>
			</Select.Root>
		{/if}

		<span class="flex-1"></span>
		<Button variant="ghost" size="icon-sm" aria-label="Reload from the server" title="Reload from the server" disabled={loading} onclick={() => load(name)}><RefreshIcon /></Button>
		<SearchButton onclick={() => (paletteOpen = true)} />
		<ThemeToggle />
	</header>

	<main class="min-h-0 flex-1">
		{#if error}
			<Empty.Root class="h-full">
				<Empty.Header>
					<Empty.Media variant="icon"><CloudOffIcon /></Empty.Media>
					<Empty.Title>Couldn't reach “{name}”</Empty.Title>
					<Empty.Description>{error}</Empty.Description>
				</Empty.Header>
				<Empty.Content>
					<div class="flex justify-center gap-2">
						<Button onclick={() => load(name)}><RefreshIcon data-icon="inline-start" /> Try again</Button>
						<Button variant="outline" disabled={!connection?.settings} onclick={() => (editOpen = true)}><Settings2Icon data-icon="inline-start" /> Edit connection</Button>
					</div>
				</Empty.Content>
			</Empty.Root>
		{:else if loading && !info}
			<div class="flex flex-col gap-2 p-6">{#each Array(5) as _, i (i)}<Skeleton class="h-9" />{/each}</div>
		{:else if tab === 'overview'}
			{#if connection?.settings && info}
				<OnlineOverview {connection} {info} {objects} onEdit={() => (editOpen = true)} onTest={test} onOpen={(t, view) => openTable(t, view)} onDiagram={() => (tab = 'diagram')} />
			{:else if !workspace.loaded}
				<div class="flex flex-col gap-2 p-6">{#each Array(4) as _, i (i)}<Skeleton class="h-9" />{/each}</div>
			{:else}
				<Empty.Root class="h-full">
					<Empty.Header>
						<Empty.Media variant="icon"><CloudOffIcon /></Empty.Media>
						<Empty.Title>The saved connection can't be read</Empty.Title>
						<Empty.Description>Its settings file is missing or damaged. Remove it on the home screen and add the database again.</Empty.Description>
					</Empty.Header>
				</Empty.Root>
			{/if}
		{:else if tab === 'diagram'}
			{#key `${name}:${revision}`}<OnlineDiagramView database={name} {objects} onOpen={(t, view) => openTable(t, view)} />{/key}
		{:else if tab === 'query'}
			{#key `${name}:${revision}`}<OnlineQueryView database={name} {engine} initial={initialSql} />{/key}
		{:else if !table}
			<Empty.Root class="h-full">
				<Empty.Header>
					<Empty.Media variant="icon"><TableIcon /></Empty.Media>
					<Empty.Title>No tables you can see</Empty.Title>
					<Empty.Description>The server has no tables or views this user may read. Tables are created with the server's own tools.</Empty.Description>
				</Empty.Header>
			</Empty.Root>
		{:else if tab === 'structure'}
			{#key `${table}:${revision}`}<OnlineStructureView database={name} {table} onOpen={show} />{/key}
		{:else}
			{#key `${table}:${revision}`}<OnlineDataView database={name} {table} />{/key}
		{/if}
	</main>
</div>

<NewDatabaseDrawer bind:open={editOpen} editing={connection} onCreated={(n) => edited(n)} />

<OnlineCommandPalette
	bind:open={paletteOpen}
	database={name}
	{objects}
	{tab}
	onTab={(t) => (tab = t)}
	onTable={(t) => openTable(t, tab === 'structure' ? 'structure' : 'data')}
	onDatabase={openDatabase}
	onReload={() => load(name)}
	onTest={test}
	onEdit={() => connection?.settings && (editOpen = true)}
	onHome={() => goto(resolve('/'))}
	onSettings={() => goto(resolve('/settings'))}
/>
