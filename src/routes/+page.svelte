<script lang="ts">
	import { goto } from '$app/navigation';
	import type { DatabaseFile } from '#lib/api/types.js';
	import { resolve } from '$app/paths';
	import DatabaseIcon from '@lucide/svelte/icons/database';
	import FolderOpenIcon from '@lucide/svelte/icons/folder-open';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import SearchIcon from '@lucide/svelte/icons/search';
	import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';
	import NameDialog from '#lib/components/app/NameDialog.svelte';
	import ThemeToggle from '#lib/components/app/ThemeToggle.svelte';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Card from '#lib/components/ui/card/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import * as InputGroup from '#lib/components/ui/input-group/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import * as Tooltip from '#lib/components/ui/tooltip/index.js';
	import { databaseHref } from '#lib/logic/routes.js';
	import { databaseNameProblem, formatBytes } from '#lib/logic/values.js';
	import { databaseService } from '#lib/services/database.js';
	import { attempt } from '#lib/state/feedback.svelte.js';
	import { workspace } from '#lib/state/workspace.svelte.js';

	let filter = $state('');
	let creating = $state(false);
	const visible = $derived(
		workspace.databases.filter((d) => d.name.toLowerCase().includes(filter.trim().toLowerCase()))
	);

	// Returning here closes whichever database was open.
	attempt(() => databaseService.close());
	workspace.load();

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

{#snippet dbCard(db: DatabaseFile)}
	<Card.Root class="h-full transition-colors hover:bg-accent/40">
		<Card.Header>
			<Card.Title class="flex items-center gap-2">
				<DatabaseIcon />
				<span class="truncate">{db.name}</span>
			</Card.Title>
			<Card.Description>{db.file_name}</Card.Description>
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

<div class="flex min-h-svh flex-col">
	<header class="flex h-14 items-center gap-3 border-b px-6">
		<DatabaseIcon />
		<span class="font-semibold">Tables</span>
		<span class="flex-1"></span>
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
					<Empty.Media variant="icon"><FolderOpenIcon /></Empty.Media>
					<Empty.Title>Where should your databases live?</Empty.Title>
					<Empty.Description>
						Pick a folder on this computer. Each database is saved there as a single file, and nothing ever leaves
						your machine.
					</Empty.Description>
				</Empty.Header>
				<Empty.Content>
					<Button onclick={() => workspace.chooseFolder()}>
						<FolderOpenIcon data-icon="inline-start" />
						Choose a folder
					</Button>
				</Empty.Content>
			</Empty.Root>
		{:else}
			<div class="flex flex-wrap items-end justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h1 class="text-2xl font-semibold tracking-tight">Your databases</h1>
					<div class="flex items-center gap-1 text-muted-foreground">
						<span class="max-w-md truncate text-sm" title={workspace.info.path}>{workspace.info.path}</span>
						<Button variant="link" size="sm" onclick={() => workspace.chooseFolder()}>Change</Button>
					</div>
				</div>
				<div class="flex items-center gap-2">
					{#if workspace.databases.length > 0}
						<InputGroup.Root class="w-56">
							<InputGroup.Input placeholder="Search databases" bind:value={filter} />
							<InputGroup.Addon><SearchIcon /></InputGroup.Addon>
						</InputGroup.Root>
					{/if}
					<Tooltip.Root>
						<Tooltip.Trigger>
							{#snippet child({ props })}
								<Button {...props} variant="outline" size="icon" aria-label="Refresh" onclick={() => workspace.refresh()}>
									<RefreshCwIcon />
								</Button>
							{/snippet}
						</Tooltip.Trigger>
						<Tooltip.Content>Refresh</Tooltip.Content>
					</Tooltip.Root>
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
							<a href={databaseHref(resolve('/db'), db.name)} class="rounded-lg focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none">
								{@render dbCard(db)}
							</a>
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
