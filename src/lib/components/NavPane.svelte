<script lang="ts">
	import type { ObjectSummary } from '../api/types';
	import { session } from '../state/session.svelte';
	import { ui } from '../state/ui.svelte';
	import Icon from './Icon.svelte';

	let collapsed = $state(false);
	let filter = $state('');
	let selected = $state<string | null>(null);
	let open = $state({ tables: true, views: true });

	const match = (o: ObjectSummary) => o.name.toLowerCase().includes(filter.trim().toLowerCase());
	const tables = $derived(session.tables.filter(match));
	const views = $derived(session.views.filter(match));

	function menu(e: MouseEvent, obj: ObjectSummary) {
		selected = obj.name;
		const isTable = obj.kind === 'table';
		ui.openMenu(e, [
			{ label: 'Open', icon: 'table', action: () => session.openDoc('datasheet', obj.name) },
			{ label: 'Design View', icon: 'design', disabled: !isTable, action: () => session.openDoc('design', obj.name) },
			{ separator: true },
			{ label: 'Rename', icon: 'pencil', disabled: !isTable, action: () => session.renameObject(obj.name) },
			{ label: 'Delete', icon: 'trash', danger: true, disabled: !isTable, action: () => session.deleteObject(obj.name) }
		]);
	}

	function paneMenu(e: MouseEvent) {
		ui.openMenu(e, [
			{ label: 'New Table', icon: 'table', action: () => session.createTable() },
			{ label: 'New Table Design', icon: 'design', action: () => session.openDoc('design', null) },
			{ separator: true },
			{ label: 'Refresh', icon: 'refresh', action: () => session.refreshObjects() }
		]);
	}
</script>

{#snippet item(obj: ObjectSummary)}
	{@const isOpen = session.active?.name?.toLowerCase() === obj.name.toLowerCase()}
	<button
		class="item"
		class:selected={selected === obj.name}
		class:open={isOpen}
		onclick={() => (selected = obj.name)}
		ondblclick={() => session.openDoc('datasheet', obj.name)}
		onkeydown={(e) => e.key === 'Enter' && session.openDoc('datasheet', obj.name)}
		oncontextmenu={(e) => menu(e, obj)}
		title={obj.kind === 'table' && !obj.managed ? `${obj.name} (created outside Database Builder)` : obj.name}
	>
		<span class="ico"><Icon name={obj.kind === 'table' ? 'table' : 'view'} size={15} /></span>
		<span class="label">{obj.name}</span>
	</button>
{/snippet}

{#if collapsed}
	<aside class="pane collapsed">
		<button class="icon" title="Show navigation pane" aria-label="Show navigation pane" onclick={() => (collapsed = false)}><Icon name="next" /></button>
		<span class="vertical">Navigation Pane</span>
	</aside>
{:else}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<aside class="pane" oncontextmenu={paneMenu}>
		<header>
			<span>All Objects</span>
			<button class="icon" title="Hide navigation pane" aria-label="Hide navigation pane" onclick={() => (collapsed = true)}><Icon name="prev" /></button>
		</header>
		<label class="search">
			<Icon name="search" size={14} />
			<input placeholder="Search…" bind:value={filter} />
		</label>
		<div class="groups">
			<button class="group" onclick={() => (open.tables = !open.tables)}>
				<Icon name={open.tables ? 'chevronDown' : 'chevronRight'} size={13} /> Tables <span class="count">{session.tables.length}</span>
			</button>
			{#if open.tables}
				{#each tables as t (t.name)}{@render item(t)}{/each}
				{#if session.tables.length === 0}
					<button class="empty" onclick={() => session.createTable()}><Icon name="plus" size={14} /> Create a table</button>
				{/if}
			{/if}
			{#if session.views.length}
				<button class="group" onclick={() => (open.views = !open.views)}>
					<Icon name={open.views ? 'chevronDown' : 'chevronRight'} size={13} /> Views <span class="count">{session.views.length}</span>
				</button>
				{#if open.views}{#each views as v (v.name)}{@render item(v)}{/each}{/if}
			{/if}
		</div>
	</aside>
{/if}

<style>
	.pane {
		display: flex;
		flex-direction: column;
		width: 240px;
		min-width: 0;
		background: var(--surface);
		border-right: 1px solid var(--border);
		overflow: hidden;
	}
	.pane.collapsed {
		width: 34px;
		align-items: center;
		padding-top: 6px;
		gap: 12px;
	}
	.vertical {
		writing-mode: vertical-rl;
		color: var(--text-muted);
		font-size: 12px;
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 6px 6px 12px;
		font-weight: 600;
	}
	.search {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 0 8px 8px;
		padding-left: 8px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-sm);
		background: var(--bg);
		color: var(--text-muted);
	}
	.search input {
		flex: 1;
		min-width: 0;
		border: none;
		background: transparent;
		box-shadow: none;
		padding-left: 0;
	}
	.groups {
		flex: 1;
		overflow: auto;
		padding-bottom: 12px;
	}
	.group {
		width: 100%;
		justify-content: flex-start;
		gap: 4px;
		padding: 6px 10px;
		border: none;
		border-radius: 0;
		background: transparent;
		color: var(--accent-text);
		font-weight: 600;
		font-size: 12px;
	}
	.group:hover {
		background: var(--surface-2);
	}
	.count {
		margin-left: auto;
		color: var(--text-faint);
		font-weight: 400;
	}
	.item {
		width: 100%;
		justify-content: flex-start;
		gap: 8px;
		padding: 5px 10px 5px 26px;
		border: none;
		border-radius: 0;
		background: transparent;
	}
	.item:hover {
		background: var(--surface-2);
	}
	.item.selected {
		background: var(--accent-soft);
	}
	.item.open .label {
		font-weight: 600;
	}
	.ico {
		color: var(--text-muted);
	}
	.label {
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.empty {
		margin: 4px 10px 4px 26px;
		border-style: dashed;
		background: transparent;
		color: var(--text-muted);
		font-size: 12px;
	}
</style>
