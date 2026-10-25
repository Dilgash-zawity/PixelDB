<script lang="ts">
	// Utils
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { toggleMode } from 'mode-watcher';
	import { shortcutsDialog } from '#lib/state/shortcuts.svelte.js';
	import { workspace } from '#lib/state/workspace.svelte.js';

	// Components
	import * as Command from '#lib/components/ui/command/index.js';
	import PixelIcon from './PixelIcon.svelte';
	import ProviderIcon from './ProviderIcon.svelte';
	import Shortcut from './Shortcut.svelte';

	// Icons
	import {
		FolderOpen as FolderOpenIcon,
		Keyboard as KeyboardIcon,
		LayoutGrid as LayoutGridIcon,
		Maximize as MaximizeIcon,
		Plus as PlusIcon,
		Settings as SettingsIcon,
		SunMoon as SunMoonIcon
	} from '@lucide/svelte';

	// Script
	/** Search on the home screen: open an app or a database, or make one. */
	let {
		open = $bindable(false),
		canOpen,
		onOpen,
		onNewDatabase,
		onNewApp,
		onTidy,
		onFit
	}: {
		open?: boolean;
		canOpen: (kind: 'app' | 'db', name: string) => boolean;
		onOpen: (kind: 'app' | 'db', name: string) => void;
		onNewDatabase: () => void;
		onNewApp: () => void;
		onTidy: () => void;
		onFit: () => void;
	} = $props();

	function run(action: () => void) {
		open = false;
		action();
	}
</script>

<Command.Dialog bind:open title="Search" description="Open an app or a database, or run an action">
	<Command.Input placeholder="Search apps, databases and actions…" />
	<Command.List>
		<Command.Empty>Nothing found.</Command.Empty>
		{#if workspace.apps.length}
			<Command.Group heading="Apps">
				{#each workspace.apps as a (a.name)}
					<Command.Item value={`app ${a.name}`} disabled={!canOpen('app', a.name)} onSelect={() => run(() => onOpen('app', a.name))}>
						<PixelIcon name="app" class="size-4" /> {a.name}
						{#if a.description}<Command.Shortcut class="tracking-normal truncate">{a.description}</Command.Shortcut>{/if}
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}
		{#if workspace.databases.length || workspace.online.length}
			<Command.Group heading="Databases">
				{#each workspace.databases as d (d.file_name)}
					<Command.Item value={`database ${d.name}`} disabled={!canOpen('db', d.name)} onSelect={() => run(() => onOpen('db', d.name))}>
						<PixelIcon name="database" class="size-4" /> {d.name}
					</Command.Item>
				{/each}
				{#each workspace.online as o (o.name)}
					<Command.Item value={`database online ${o.name} ${o.settings?.provider ?? ''}`} disabled={!canOpen('db', o.name)} onSelect={() => run(() => onOpen('db', o.name))}>
						<ProviderIcon provider={o.settings?.provider} /> {o.name}
						<Command.Shortcut class="tracking-normal">Online</Command.Shortcut>
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}
		<Command.Separator />
		<Command.Group heading="Create">
			<Command.Item value="new database" onSelect={() => run(onNewDatabase)}><PlusIcon /> New database… <Command.Shortcut><Shortcut keys="n" /></Command.Shortcut></Command.Item>
			<Command.Item value="new app" onSelect={() => run(onNewApp)}><PlusIcon /> New app… <Command.Shortcut><Shortcut keys="shift+n" /></Command.Shortcut></Command.Item>
		</Command.Group>
		<Command.Group heading="Map">
			<Command.Item value="tidy up arrange" onSelect={() => run(onTidy)}><LayoutGridIcon /> Tidy up</Command.Item>
			<Command.Item value="fit to screen" onSelect={() => run(onFit)}><MaximizeIcon /> Fit to screen</Command.Item>
		</Command.Group>
		<Command.Group heading="General">
			<Command.Item value="change workspace folder" onSelect={() => run(() => workspace.chooseFolder())}><FolderOpenIcon /> Change the folder…</Command.Item>
			<Command.Item onSelect={() => run(toggleMode)}><SunMoonIcon /> Toggle light / dark</Command.Item>
			<Command.Item value="keyboard shortcuts" onSelect={() => run(() => (shortcutsDialog.open = true))}><KeyboardIcon /> Keyboard shortcuts</Command.Item>
			<Command.Item onSelect={() => run(() => goto(resolve('/settings')))}><SettingsIcon /> Settings</Command.Item>
		</Command.Group>
	</Command.List>
</Command.Dialog>
