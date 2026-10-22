<script lang="ts">
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
	import ChevronsUpDownIcon from '@lucide/svelte/icons/chevrons-up-down';
	import DatabaseIcon from '@lucide/svelte/icons/database';
	import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
	import EyeIcon from '@lucide/svelte/icons/eye';
	import ListIcon from '@lucide/svelte/icons/list';
	import NetworkIcon from '@lucide/svelte/icons/network';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import SearchIcon from '@lucide/svelte/icons/search';
	import Settings2Icon from '@lucide/svelte/icons/settings-2';
	import TableIcon from '@lucide/svelte/icons/table-2';
	import TrashIcon from '@lucide/svelte/icons/trash-2';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { Kbd } from '#lib/components/ui/kbd/index.js';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import { session } from '#lib/state/session.svelte.js';
	import { workspace } from '#lib/state/workspace.svelte.js';
	import Logo from './Logo.svelte';
	import NameDialog from './NameDialog.svelte';
	import ThemeToggle from './ThemeToggle.svelte';

	let {
		onBack,
		onSwitch,
		onSearch
	}: { onBack: () => void; onSwitch: (name: string) => void; onSearch: () => void } = $props();

	let creating = $state(false);
	let renaming = $state<string | null>(null);

	export function newTable() {
		creating = true;
	}

	function suggestedName(): string {
		let n = 1;
		while (session.objects.some((o) => o.name.toLowerCase() === `table ${n}`)) n++;
		return `Table ${n}`;
	}
</script>

<Sidebar.Root>
	<Sidebar.Header>
		<Sidebar.Menu>
			<Sidebar.MenuItem>
				<DropdownMenu.Root onOpenChange={(open) => open && workspace.refresh()}>
					<DropdownMenu.Trigger>
						{#snippet child({ props })}
							<Sidebar.MenuButton {...props} size="lg">
								<Logo />
								<div class="grid flex-1 text-start leading-tight">
									<span class="truncate font-semibold">{session.name}</span>
									<span class="truncate text-xs text-muted-foreground">
										{session.tables.length} {session.tables.length === 1 ? 'table' : 'tables'}
									</span>
								</div>
								<ChevronsUpDownIcon class="ms-auto" />
							</Sidebar.MenuButton>
						{/snippet}
					</DropdownMenu.Trigger>
					<DropdownMenu.Content class="w-64" align="start">
						<DropdownMenu.Group>
							<DropdownMenu.Label>Switch database</DropdownMenu.Label>
							{#each workspace.databases.filter((d) => d.valid) as db (db.file_name)}
								<DropdownMenu.Item disabled={db.name === session.name} onclick={() => onSwitch(db.name)}>
									<DatabaseIcon />
									<span class="truncate">{db.name}</span>
								</DropdownMenu.Item>
							{/each}
						</DropdownMenu.Group>
						<DropdownMenu.Separator />
						<DropdownMenu.Group>
							<DropdownMenu.Item onclick={onBack}>
								<ArrowLeftIcon />
								All databases
							</DropdownMenu.Item>
						</DropdownMenu.Group>
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Header>

	<Sidebar.Content>
		<Sidebar.Group>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					<Sidebar.MenuItem>
						<Sidebar.MenuButton isActive={session.screen === 'relationships'} onclick={() => session.showRelationships()}>
							<NetworkIcon />
							<span>Relationships</span>
						</Sidebar.MenuButton>
					</Sidebar.MenuItem>
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>

		<Sidebar.Group>
			<Sidebar.GroupLabel>Tables</Sidebar.GroupLabel>
			<Sidebar.GroupAction title="New table" onclick={() => (creating = true)}>
				<PlusIcon />
				<span class="sr-only">New table</span>
			</Sidebar.GroupAction>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					{#each session.tables as table (table.name)}
						<Sidebar.MenuItem>
							<Sidebar.MenuButton isActive={session.screen === 'table' && session.table === table.name} onclick={() => session.select(table.name)}>
								<TableIcon />
								<span>{table.name}</span>
							</Sidebar.MenuButton>
							<DropdownMenu.Root>
								<DropdownMenu.Trigger>
									{#snippet child({ props })}
										<Sidebar.MenuAction showOnHover {...props}>
											<EllipsisIcon />
											<span class="sr-only">Table options</span>
										</Sidebar.MenuAction>
									{/snippet}
								</DropdownMenu.Trigger>
								<DropdownMenu.Content side="right" align="start">
									<DropdownMenu.Group>
										<DropdownMenu.Item onclick={() => session.select(table.name, 'records')}>
											<ListIcon /> Datasheet view
										</DropdownMenu.Item>
										<DropdownMenu.Item onclick={() => session.select(table.name, 'fields')}>
											<Settings2Icon /> Design view
										</DropdownMenu.Item>
										<DropdownMenu.Item onclick={() => (renaming = table.name)}>
											<PencilIcon /> Rename
										</DropdownMenu.Item>
									</DropdownMenu.Group>
									<DropdownMenu.Separator />
									<DropdownMenu.Group>
										<DropdownMenu.Item variant="destructive" onclick={() => session.deleteTable(table.name)}>
											<TrashIcon /> Delete
										</DropdownMenu.Item>
									</DropdownMenu.Group>
								</DropdownMenu.Content>
							</DropdownMenu.Root>
						</Sidebar.MenuItem>
					{:else}
						<Sidebar.MenuItem>
							<Sidebar.MenuButton onclick={() => (creating = true)}>
								<PlusIcon />
								<span>Create a table</span>
							</Sidebar.MenuButton>
						</Sidebar.MenuItem>
					{/each}
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>

		{#if session.views.length}
			<Sidebar.Group>
				<Sidebar.GroupLabel>Views</Sidebar.GroupLabel>
				<Sidebar.GroupContent>
					<Sidebar.Menu>
						{#each session.views as view (view.name)}
							<Sidebar.MenuItem>
								<Sidebar.MenuButton isActive={session.screen === 'table' && session.table === view.name} onclick={() => session.select(view.name, 'records')}>
									<EyeIcon />
									<span>{view.name}</span>
								</Sidebar.MenuButton>
							</Sidebar.MenuItem>
						{/each}
					</Sidebar.Menu>
				</Sidebar.GroupContent>
			</Sidebar.Group>
		{/if}
	</Sidebar.Content>

	<Sidebar.Footer>
		<Sidebar.Menu>
			<Sidebar.MenuItem class="flex items-center gap-1">
				<Sidebar.MenuButton onclick={onSearch} class="flex-1">
					<SearchIcon />
					<span>Search</span>
					<Kbd class="ms-auto">⌘K</Kbd>
				</Sidebar.MenuButton>
				<ThemeToggle align="start" />
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Footer>
	<Sidebar.Rail />
</Sidebar.Root>

<NameDialog
	bind:open={creating}
	title="New table"
	description="A table holds one kind of thing — customers, orders, books. It starts with an automatic ID field."
	initial={suggestedName()}
	placeholder="e.g. Customers"
	submitLabel="Create table"
	validate={(v) => session.tableNameProblem(v)}
	onSubmit={(name) => session.createTable(name)}
/>

<NameDialog
	bind:open={() => renaming !== null, (open) => !open && (renaming = null)}
	title="Rename table"
	initial={renaming ?? ''}
	submitLabel="Rename"
	validate={(v) => session.tableNameProblem(v, renaming ?? undefined)}
	onSubmit={async (name) => {
		return name === renaming || (await session.renameTable(renaming as string, name));
	}}
/>
