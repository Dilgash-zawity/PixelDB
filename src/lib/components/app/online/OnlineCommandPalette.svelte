<script lang="ts">
	// Types
	import type { ObjectSummary } from '#lib/api/types.js';

	// Utils
	import { toggleMode } from 'mode-watcher';
	import { shortcutsDialog } from '#lib/state/shortcuts.svelte.js';
	import { workspace } from '#lib/state/workspace.svelte.js';

	// Components
	import * as Command from '#lib/components/ui/command/index.js';
	import ProviderIcon from '../ProviderIcon.svelte';
	import Shortcut from '../Shortcut.svelte';

	// Icons
	import {
		ArrowLeft as ArrowLeftIcon,
		Database as DatabaseIcon,
		Eye as EyeIcon,
		Gauge as GaugeIcon,
		Keyboard as KeyboardIcon,
		ListTree as ListTreeIcon,
		Network as NetworkIcon,
		PlugZap as PlugZapIcon,
		RefreshCw as RefreshIcon,
		SearchCode as SearchCodeIcon,
		Settings as SettingsIcon,
		Settings2 as Settings2Icon,
		SunMoon as SunMoonIcon,
		Table2 as TableIcon
	} from '@lucide/svelte';

	// Script
	type Tab = 'overview' | 'diagram' | 'data' | 'structure' | 'query';

	/** Search on an online database's page: a table, a tab, another database or an action. */
	let {
		open = $bindable(false),
		database,
		objects,
		tab,
		onTab,
		onTable,
		onDatabase,
		onReload,
		onTest,
		onEdit,
		onHome,
		onSettings
	}: {
		open?: boolean;
		database: string;
		objects: ObjectSummary[];
		tab: Tab;
		onTab: (tab: Tab) => void;
		/** Shows a table on Structure when that tab is open, else on Data. */
		onTable: (table: string) => void;
		onDatabase: (name: string, online: boolean) => void;
		onReload: () => void;
		onTest: () => void;
		onEdit: () => void;
		onHome: () => void;
		onSettings: () => void;
	} = $props();

	const TABS: { id: Tab; label: string; icon: typeof GaugeIcon }[] = [
		{ id: 'overview', label: 'Overview', icon: GaugeIcon },
		{ id: 'diagram', label: 'Diagram', icon: NetworkIcon },
		{ id: 'data', label: 'Data', icon: TableIcon },
		{ id: 'structure', label: 'Structure', icon: ListTreeIcon },
		{ id: 'query', label: 'Query', icon: SearchCodeIcon }
	];

	function run(action: () => void) {
		open = false;
		action();
	}
</script>

<Command.Dialog bind:open title="Search" description="Jump to a table, a tab or another database">
	<Command.Input placeholder="Search tables, tabs and actions…" />
	<Command.List>
		<Command.Empty>Nothing found.</Command.Empty>
		{#if objects.length}
			<Command.Group heading={tab === 'structure' ? 'Show the structure of' : 'Open the rows of'}>
				{#each objects as o (o.name)}
					<Command.Item value={`table ${o.name}`} onSelect={() => run(() => onTable(o.name))}>
						{#if o.kind === 'view'}<EyeIcon />{:else}<TableIcon />{/if}
						{o.name}
						{#if o.kind === 'view'}<Command.Shortcut class="tracking-normal">View</Command.Shortcut>{/if}
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}
		<Command.Group heading="Go to">
			{#each TABS as t, i (t.id)}
				<Command.Item value={`tab ${t.label}`} disabled={t.id === tab} onSelect={() => run(() => onTab(t.id))}>
					<t.icon /> {t.label}
					<Command.Shortcut><Shortcut keys={`mod+${i + 1}`} /></Command.Shortcut>
				</Command.Item>
			{/each}
		</Command.Group>
		{#if workspace.databases.length || workspace.online.length > 1}
			<Command.Group heading="Other databases">
				{#each workspace.databases.filter((d) => d.valid) as d (d.file_name)}
					<Command.Item value={`database ${d.name}`} onSelect={() => run(() => onDatabase(d.name, false))}><DatabaseIcon /> {d.name}</Command.Item>
				{/each}
				{#each workspace.online.filter((o) => o.settings && o.name !== database) as o (o.name)}
					<Command.Item value={`database online ${o.name}`} onSelect={() => run(() => onDatabase(o.name, true))}>
						<ProviderIcon provider={o.settings?.provider} /> {o.name}
						<Command.Shortcut class="tracking-normal">Online</Command.Shortcut>
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}
		<Command.Separator />
		<Command.Group heading="General">
			<Command.Item value="reload from the server" onSelect={() => run(onReload)}><RefreshIcon /> Reload from the server</Command.Item>
			<Command.Item value="test connection" onSelect={() => run(onTest)}><PlugZapIcon /> Test the connection</Command.Item>
			<Command.Item value="edit connection" onSelect={() => run(onEdit)}><Settings2Icon /> Edit the connection…</Command.Item>
			<Command.Item onSelect={() => run(toggleMode)}><SunMoonIcon /> Toggle light / dark</Command.Item>
			<Command.Item value="keyboard shortcuts" onSelect={() => run(() => (shortcutsDialog.open = true))}><KeyboardIcon /> Keyboard shortcuts</Command.Item>
			<Command.Item value="home all databases" onSelect={() => run(onHome)}><ArrowLeftIcon /> Home</Command.Item>
			<Command.Item onSelect={() => run(onSettings)}><SettingsIcon /> Settings</Command.Item>
		</Command.Group>
	</Command.List>
</Command.Dialog>
