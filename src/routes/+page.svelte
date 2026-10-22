<script lang="ts">
	import { goto } from '$app/navigation';
	import type { AppFile, DatabaseFile } from '#lib/api/types.js';
	import { resolve } from '$app/paths';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import DatabaseIcon from '@lucide/svelte/icons/database';
	import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import TrashIcon from '@lucide/svelte/icons/trash-2';
	import FolderOpenIcon from '@lucide/svelte/icons/folder-open';
	import InfoIcon from '@lucide/svelte/icons/info';
	import LayoutDashboardIcon from '@lucide/svelte/icons/layout-dashboard';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import SearchIcon from '@lucide/svelte/icons/search';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';
	import Logo from '#lib/components/app/Logo.svelte';
	import NameDialog from '#lib/components/app/NameDialog.svelte';
	import NewAppDialog from '#lib/components/app/NewAppDialog.svelte';
	import ThemeToggle from '#lib/components/app/ThemeToggle.svelte';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Card from '#lib/components/ui/card/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import * as InputGroup from '#lib/components/ui/input-group/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { appHref, databaseHref } from '#lib/logic/routes.js';
	import { databaseNameProblem, formatBytes } from '#lib/logic/values.js';
	import { databaseService } from '#lib/services/database.js';
	import { attempt } from '#lib/state/feedback.svelte.js';
	import { workspace } from '#lib/state/workspace.svelte.js';

	let filter = $state('');
	let creating = $state(false);
	let renaming = $state<string | null>(null);
	// The suggested copy name is fixed when the dialog opens, so the refresh
	// after duplicating doesn't change the field while it closes.
	let duplicating = $state<{ name: string; suggested: string } | null>(null);
	const visible = $derived(
		workspace.databases.filter((d) => d.name.toLowerCase().includes(filter.trim().toLowerCase()))
	);

	// Returning here closes whichever databases were open.
	attempt(async () => {
		await databaseService.close();
		await databaseService.release();
	});

	let creatingApp = $state(false);
	let renamingApp = $state<string | null>(null);
	let duplicatingApp = $state<{ name: string; suggested: string } | null>(null);
	const appNameProblem = (v: string, except?: string) =>
		databaseNameProblem(v) ?? (workspace.appNameTaken(v, except) ? 'An app with this name already exists' : null);
	const ACCENT_TEXT: Record<string, string> = {
		blue: 'text-[oklch(0.55_0.2_255)] bg-[oklch(0.55_0.2_255)]/10',
		green: 'text-[oklch(0.6_0.15_150)] bg-[oklch(0.6_0.15_150)]/10',
		amber: 'text-[oklch(0.75_0.16_75)] bg-[oklch(0.75_0.16_75)]/10',
		red: 'text-[oklch(0.6_0.22_25)] bg-[oklch(0.6_0.22_25)]/10',
		violet: 'text-[oklch(0.55_0.22_295)] bg-[oklch(0.55_0.22_295)]/10'
	};
	workspace.load();
	// Files can change outside the app; pick that up when the window regains focus.
	const refreshOnFocus = () => workspace.info && workspace.refresh();

	function open(name: string) {
		goto(databaseHref(resolve('/db'), name));
	}

	function suggestedName(): string {
		let n = 1;
		while (workspace.nameTaken(`Database ${n}`)) n++;
		return n === 1 && !workspace.nameTaken('My database') ? 'My database' : `Database ${n}`;
	}

	const relative = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' });
	function modified(ms: number | null): string {
		if (ms === null) return '';
		const minutes = Math.round((ms - Date.now()) / 60000);
		if (Math.abs(minutes) < 60) return relative.format(minutes, 'minute');
		const hours = Math.round(minutes / 60);
		if (Math.abs(hours) < 24) return relative.format(hours, 'hour');
		const days = Math.round(hours / 24);
		if (Math.abs(days) < 7) return relative.format(days, 'day');
		return new Date(ms).toLocaleDateString(undefined, { dateStyle: 'medium' });
	}
</script>

{#snippet appCard(app: AppFile)}
	<Card.Root class="relative h-full transition-colors hover:bg-accent/40 has-[a:focus-visible]:ring-2 has-[a:focus-visible]:ring-ring">
		<Card.Header>
			<Card.Title class="flex min-w-0 items-center gap-3">
				<span class="flex size-9 shrink-0 items-center justify-center {ACCENT_TEXT[app.accent] ?? 'bg-primary/10 text-primary'}">
					<LayoutDashboardIcon class="size-4.5" />
				</span>
				{#if app.valid}
					<a href={appHref(resolve('/app'), app.name)} class="truncate outline-none after:absolute after:inset-0">{app.name}</a>
				{:else}
					<span class="truncate">{app.name}</span>
				{/if}
			</Card.Title>
			<Card.Description class="line-clamp-2">
				{app.description || (app.databases.length ? `Uses ${app.databases.join(', ')}` : 'Not connected to a database yet')}
			</Card.Description>
			<Card.Action class="relative">
				<DropdownMenu.Root>
					<DropdownMenu.Trigger>
						{#snippet child({ props })}
							<Button {...props} variant="ghost" size="icon-sm" aria-label="App options"><EllipsisIcon /></Button>
						{/snippet}
					</DropdownMenu.Trigger>
					<DropdownMenu.Content align="end">
						<DropdownMenu.Group>
							<DropdownMenu.Item onclick={() => (renamingApp = app.name)}><PencilIcon /> Rename</DropdownMenu.Item>
							<DropdownMenu.Item
								disabled={!app.valid}
								onclick={() => (duplicatingApp = { name: app.name, suggested: workspace.copyName(app.name, (n) => workspace.appNameTaken(n)) })}
							>
								<CopyIcon /> Duplicate
							</DropdownMenu.Item>
						</DropdownMenu.Group>
						<DropdownMenu.Separator />
						<DropdownMenu.Group>
							<DropdownMenu.Item variant="destructive" onclick={() => workspace.removeApp(app.name)}><TrashIcon /> Delete</DropdownMenu.Item>
						</DropdownMenu.Group>
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			</Card.Action>
		</Card.Header>
		<Card.Footer class="flex items-center justify-between gap-2 text-sm text-muted-foreground">
			{#if app.valid}
				<span>Edited {modified(app.modified)}</span>
				{#if app.databases.some((d) => !workspace.databases.some((x) => x.name === d))}
					<Badge variant="outline" class="border-warning/40 bg-warning/10 text-warning"><TriangleAlertIcon /> Missing database</Badge>
				{/if}
			{:else}
				<Badge variant="destructive"><TriangleAlertIcon /> Can't be read</Badge>
			{/if}
		</Card.Footer>
	</Card.Root>
{/snippet}

{#snippet dbCard(db: DatabaseFile)}
	<Card.Root class="relative h-full transition-colors hover:bg-accent/40 has-[a:focus-visible]:ring-2 has-[a:focus-visible]:ring-ring">
		<Card.Header>
			<Card.Title class="flex min-w-0 items-center gap-3">
				<span class="flex size-9 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
					<DatabaseIcon class="size-4.5" />
				</span>
				{#if db.valid}
					<!-- The link's ::after covers the card so the whole card opens the database. -->
					<a href={databaseHref(resolve('/db'), db.name)} class="truncate outline-none after:absolute after:inset-0">
						{db.name}
					</a>
				{:else}
					<span class="truncate">{db.name}</span>
				{/if}
			</Card.Title>
			<Card.Description>{db.file_name}</Card.Description>
			<Card.Action class="relative">
				<DropdownMenu.Root>
					<DropdownMenu.Trigger>
						{#snippet child({ props })}
							<Button {...props} variant="ghost" size="icon-sm" aria-label="Database options">
								<EllipsisIcon />
							</Button>
						{/snippet}
					</DropdownMenu.Trigger>
					<DropdownMenu.Content align="end">
						<DropdownMenu.Group>
							<DropdownMenu.Item onclick={() => (renaming = db.name)}>
								<PencilIcon /> Rename
							</DropdownMenu.Item>
							<DropdownMenu.Item disabled={!db.valid} onclick={() => (duplicating = { name: db.name, suggested: workspace.copyName(db.name) })}>
								<CopyIcon /> Duplicate
							</DropdownMenu.Item>
						</DropdownMenu.Group>
						<DropdownMenu.Separator />
						<DropdownMenu.Group>
							<DropdownMenu.Item variant="destructive" onclick={() => workspace.remove(db.name)}>
								<TrashIcon /> Delete
							</DropdownMenu.Item>
						</DropdownMenu.Group>
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			</Card.Action>
		</Card.Header>
		<Card.Footer class="flex items-center justify-between gap-2 text-sm text-muted-foreground">
			{#if db.valid}
				<span>Edited {modified(db.modified)}</span>
				<Badge variant="secondary">{formatBytes(db.size)}</Badge>
			{:else}
				<Badge variant="destructive"><TriangleAlertIcon /> Not a database file</Badge>
			{/if}
		</Card.Footer>
	</Card.Root>
{/snippet}

<svelte:window onfocus={refreshOnFocus} />

<div class="flex min-h-svh flex-col">
	<header class="flex h-14 items-center gap-3 border-b bg-sidebar px-6">
		<Logo class="size-7" />
		<span class="font-semibold">SlateDB</span>
		<span class="flex-1"></span>
		<Button variant="ghost" size="icon-sm" href={resolve('/about')} aria-label="About SlateDB" title="About SlateDB"><InfoIcon /></Button>
		<ThemeToggle />
	</header>

	<main class="mx-auto flex w-full max-w-5xl flex-1 flex-col gap-8 px-6 py-10">
		{#if !workspace.loaded}
			<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
				{#each [1, 2, 3] as i (i)}<Skeleton class="h-32" />{/each}
			</div>
		{:else if !workspace.info}
			<Empty.Root class="my-auto">
				<Empty.Header>
					<Empty.Media><Logo class="size-24" /></Empty.Media>
					<Empty.Title class="text-2xl">Welcome to SlateDB</Empty.Title>
					<Empty.Description>Choose a folder for your databases. Everything stays on this computer.</Empty.Description>
				</Empty.Header>
				<Empty.Content>
					<Button onclick={() => workspace.chooseFolder()}>
						<FolderOpenIcon data-icon="inline-start" />
						Choose a folder
					</Button>
				</Empty.Content>
			</Empty.Root>
		{:else}
			<section class="flex flex-col gap-4">
				<div class="flex flex-wrap items-end justify-between gap-4">
					<div class="flex flex-col gap-1">
						<h1 class="text-2xl font-semibold tracking-tight">Your apps</h1>
						<p class="text-sm text-muted-foreground">Forms, tables and charts over one or more databases.</p>
					</div>
					<Button variant="outline" onclick={() => (creatingApp = true)}>
						<PlusIcon data-icon="inline-start" />
						New app
					</Button>
				</div>
				{#if workspace.apps.length}
					<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
						{#each workspace.apps as app (app.name)}
							{#if app.valid}
								{@render appCard(app)}
							{:else}
								<div class="opacity-60" title="This file is not a readable app">{@render appCard(app)}</div>
							{/if}
						{/each}
					</div>
				{:else}
					<Empty.Root class="border border-dashed p-6">
						<Empty.Header>
							<Empty.Media variant="icon"><LayoutDashboardIcon /></Empty.Media>
							<Empty.Title>No apps yet</Empty.Title>
							<Empty.Description>Build screens for entering and exploring data, or start from a template.</Empty.Description>
						</Empty.Header>
						<Empty.Content>
							<Button variant="outline" onclick={() => (creatingApp = true)}><PlusIcon data-icon="inline-start" /> New app</Button>
						</Empty.Content>
					</Empty.Root>
				{/if}
			</section>

			<div class="flex flex-wrap items-end justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h2 class="text-2xl font-semibold tracking-tight">Your databases</h2>
					<div class="flex items-center gap-1 text-muted-foreground">
						<span class="max-w-md truncate text-sm" title={workspace.info.path}>{workspace.info.path}</span>
						<Button variant="link" size="sm" onclick={() => workspace.chooseFolder()}>Change</Button>
					</div>
				</div>
				<div class="flex items-center gap-2">
					{#if workspace.databases.length > 6}
						<InputGroup.Root class="w-56">
							<InputGroup.Input placeholder="Search databases" bind:value={filter} />
							<InputGroup.Addon><SearchIcon /></InputGroup.Addon>
						</InputGroup.Root>
					{/if}
					<Button onclick={() => (creating = true)}>
						<PlusIcon data-icon="inline-start" />
						New database
					</Button>
				</div>
			</div>

			{#if workspace.databases.length === 0}
				<Empty.Root class="border border-dashed">
					<Empty.Header>
						<Empty.Media variant="icon"><DatabaseIcon /></Empty.Media>
						<Empty.Title>No databases yet</Empty.Title>
						<Empty.Description>Create your first database to start organising your data into tables.</Empty.Description>
					</Empty.Header>
					<Empty.Content>
						<Button onclick={() => (creating = true)}>
							<PlusIcon data-icon="inline-start" />
							New database
						</Button>
					</Empty.Content>
				</Empty.Root>
			{:else if visible.length === 0}
				<p class="text-muted-foreground">No databases match “{filter}”.</p>
			{:else}
				<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
					{#each visible as db (db.file_name)}
						{#if db.valid}
							{@render dbCard(db)}
						{:else}
							<div class="opacity-60" title="This file is not a SQLite database">{@render dbCard(db)}</div>
						{/if}
					{/each}
				</div>
			{/if}
		{/if}
	</main>
</div>

<NameDialog
	bind:open={creating}
	title="New database"
	description="Give it a name you'll recognise. You can add tables right after."
	initial={suggestedName()}
	submitLabel="Create database"
	validate={(v) => databaseNameProblem(v) ?? (workspace.nameTaken(v) ? 'A database with this name already exists' : null)}
	onSubmit={async (name) => {
		if (!(await workspace.create(name))) return false;
		open(name);
		return true;
	}}
/>

<NameDialog
	bind:open={() => renaming !== null, (o) => !o && (renaming = null)}
	title="Rename database"
	description="The database file in your folder is renamed too."
	initial={renaming ?? ''}
	submitLabel="Rename"
	validate={(v) =>
		databaseNameProblem(v) ?? (workspace.nameTaken(v, renaming ?? undefined) ? 'A database with this name already exists' : null)}
	onSubmit={async (name) => (renaming ? workspace.rename(renaming, name) : true)}
/>

<NameDialog
	bind:open={() => duplicating !== null, (o) => !o && (duplicating = null)}
	title="Duplicate database"
	description={`Makes a full copy of “${duplicating?.name ?? ''}”, including all tables and records.`}
	initial={duplicating?.suggested ?? ''}
	submitLabel="Duplicate"
	validate={(v) => databaseNameProblem(v) ?? (workspace.nameTaken(v) ? 'A database with this name already exists' : null)}
	onSubmit={async (name) => (duplicating ? workspace.duplicate(duplicating.name, name) : true)}
/>

<NewAppDialog bind:open={creatingApp} onCreated={(name) => goto(appHref(resolve('/app'), name))} />

<NameDialog
	bind:open={() => renamingApp !== null, (o) => !o && (renamingApp = null)}
	title="Rename app"
	initial={renamingApp ?? ''}
	submitLabel="Rename"
	validate={(v) => appNameProblem(v, renamingApp ?? undefined)}
	onSubmit={async (name) => (renamingApp ? workspace.renameApp(renamingApp, name) : true)}
/>

<NameDialog
	bind:open={() => duplicatingApp !== null, (o) => !o && (duplicatingApp = null)}
	title="Duplicate app"
	description={`Copies the pages and widgets of “${duplicatingApp?.name ?? ''}”. Its databases are shared, not copied.`}
	initial={duplicatingApp?.suggested ?? ''}
	submitLabel="Duplicate"
	validate={(v) => appNameProblem(v)}
	onSubmit={async (name) => (duplicatingApp ? workspace.duplicateApp(duplicatingApp.name, name) : true)}
/>
