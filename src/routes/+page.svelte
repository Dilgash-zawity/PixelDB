<script lang="ts">
	// Services
	import { databaseService } from '#lib/services/database.js';

	// Utils
	import { resolve } from '$app/paths';
	import { attempt } from '#lib/state/feedback.svelte.js';
	import { workspace } from '#lib/state/workspace.svelte.js';

	// Components
	import FocusModeSwitch from '#lib/components/app/FocusModeSwitch.svelte';
	import Logo from '#lib/components/app/Logo.svelte';
	import SearchButton from '#lib/components/app/SearchButton.svelte';
	import ThemeToggle from '#lib/components/app/ThemeToggle.svelte';
	import WorkspaceMap from '#lib/components/app/WorkspaceMap.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';

	// Icons
	import { FolderOpen as FolderOpenIcon, Settings as SettingsIcon } from '@lucide/svelte';

	// Script
	// Returning here closes whichever databases were open.
	attempt(async () => {
		await databaseService.close();
		await databaseService.release();
	});

	workspace.load();
	let map = $state<WorkspaceMap | null>(null);
	// Files can change outside the app; pick that up when the window regains focus.
	const refreshOnFocus = () => workspace.info && workspace.refresh();
</script>

<svelte:window onfocus={refreshOnFocus} />

<div class="flex h-svh flex-col">
	<header data-bar class="app-header flex h-12 shrink-0 items-center gap-2 border-b bg-sidebar px-3">
		<Logo class="size-6" />
		<span class="font-semibold">SlateDB</span>
		<span class="flex-1"></span>
		{#if workspace.info}<SearchButton onclick={() => map?.openSearch()} />{/if}
		<Button variant="ghost" size="icon-sm" href={resolve('/settings')} aria-label="Settings" title="Settings"><SettingsIcon /></Button>
		<ThemeToggle />
	</header>

	{#if !workspace.loaded}
		<div class="flex flex-1 gap-48 bg-muted/40 p-16">
			<div class="flex flex-col gap-6">{#each [1, 2, 3] as i (i)}<Skeleton class="h-23 w-62" />{/each}</div>
			<div class="flex flex-col gap-6 pt-12">{#each [1, 2] as i (i)}<Skeleton class="h-23 w-62" />{/each}</div>
		</div>
	{:else if !workspace.info}
		<Empty.Root class="flex-1">
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
				<FocusModeSwitch class="mt-6" />
			</Empty.Content>
		</Empty.Root>
	{:else}
		<WorkspaceMap bind:this={map} />
	{/if}
</div>
