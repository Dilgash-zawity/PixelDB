<script lang="ts">
	import { session } from '../state/session.svelte';
	import Icon, { type IconName } from './Icon.svelte';

	let { onFile }: { onFile: () => void } = $props();

	type Tab = 'home' | 'create' | 'design';
	let tab = $state<Tab>('home');

	const h = $derived(session.activeHandlers);
	const active = $derived(session.active);
	const isDesign = $derived(active?.kind === 'design');
	const canSwitch = $derived(!!active && active.name !== null && !session.isView(active.name));

	// Show the contextual "Table Design" tab while a design is active.
	let lastKind: string | undefined;
	$effect(() => {
		const kind = active?.kind;
		if (kind === lastKind) return;
		lastKind = kind;
		if (kind === 'design') tab = 'design';
		else if (tab === 'design') tab = 'home';
	});

	function switchView() {
		if (active) session.switchView(active.id, active.kind === 'design' ? 'datasheet' : 'design');
	}
</script>

{#snippet big(icon: IconName, label: string, onclick: (() => unknown) | undefined, pressed = false)}
	<button class="big" class:pressed disabled={!onclick} onclick={() => onclick?.()}>
		<Icon name={icon} size={22} stroke={1.5} />
		<span>{label}</span>
	</button>
{/snippet}

{#snippet small(icon: IconName, label: string, onclick: (() => unknown) | undefined)}
	<button class="small" disabled={!onclick} onclick={() => onclick?.()}>
		<Icon name={icon} size={15} />
		<span>{label}</span>
	</button>
{/snippet}

<div class="ribbon">
	<div class="tabs" role="tablist">
		<button class="file" onclick={onFile}><Icon name="back" size={14} /> File</button>
		<button role="tab" aria-selected={tab === 'home'} class:active={tab === 'home'} onclick={() => (tab = 'home')}>Home</button>
		<button role="tab" aria-selected={tab === 'create'} class:active={tab === 'create'} onclick={() => (tab = 'create')}>Create</button>
		{#if isDesign}
			<button role="tab" aria-selected={tab === 'design'} class="contextual" class:active={tab === 'design'} onclick={() => (tab = 'design')}>Table Design</button>
		{/if}
	</div>

	<div class="body">
		{#if tab === 'home'}
			<div class="group">
				<div class="items">
					{@render big(isDesign ? 'table' : 'design', isDesign ? 'Datasheet View' : 'Design View', canSwitch ? switchView : undefined)}
				</div>
				<div class="label">Views</div>
			</div>
			<div class="group">
				<div class="items">
					{@render big('refresh', 'Refresh', h.refresh)}
					<div class="stack">
						{@render small('newRecord', 'New', h.newRecord)}
						{@render small('save', 'Save', h.save)}
						{@render small('trash', 'Delete', h.deleteRecord)}
					</div>
				</div>
				<div class="label">Records</div>
			</div>
			<div class="group">
				<div class="items">
					{@render big('filter', 'Filter', h.toggleFilter)}
					<div class="stack">
						{@render small('sortAsc', 'Ascending', h.sortAscending)}
						{@render small('sortDesc', 'Descending', h.sortDescending)}
						{@render small('clearSort', 'Remove Sort', h.clearSort)}
					</div>
				</div>
				<div class="label">Sort &amp; Filter</div>
			</div>
			<div class="group">
				<div class="items">{@render big('search', 'Find', h.find)}</div>
				<div class="label">Find</div>
			</div>
		{:else if tab === 'create'}
			<div class="group">
				<div class="items">
					{@render big('table', 'Table', () => session.createTable())}
					{@render big('design', 'Table Design', () => session.openDoc('design', null))}
				</div>
				<div class="label">Tables</div>
			</div>
		{:else}
			<div class="group">
				<div class="items">{@render big('table', 'Datasheet View', canSwitch ? switchView : undefined)}</div>
				<div class="label">Views</div>
			</div>
			<div class="group">
				<div class="items">
					{@render big('key', 'Primary Key', h.primaryKey)}
					<div class="stack">
						{@render small('insertRow', 'Insert Rows', h.insertRow)}
						{@render small('deleteRow', 'Delete Rows', h.deleteRow)}
					</div>
				</div>
				<div class="label">Tools</div>
			</div>
			<div class="group">
				<div class="items">
					{@render big('sheet', 'Property Sheet', h.propertySheet)}
					{@render big('save', 'Save', h.save)}
				</div>
				<div class="label">Show/Hide</div>
			</div>
		{/if}
	</div>
</div>

<style>
	.ribbon {
		background: var(--surface-2);
		border-bottom: 1px solid var(--border);
	}
	.tabs {
		display: flex;
		align-items: stretch;
		gap: 2px;
		padding: 0 8px;
		background: var(--surface);
		border-bottom: 1px solid var(--border);
	}
	.tabs button {
		border: none;
		border-radius: 0;
		background: transparent;
		padding: 7px 14px;
		color: var(--text-muted);
		border-bottom: 2px solid transparent;
	}
	.tabs button:hover:not(:disabled) {
		background: var(--surface-2);
		color: var(--text);
	}
	.tabs button.active {
		color: var(--text);
		border-bottom-color: var(--accent);
	}
	.tabs .file {
		margin: 4px 8px 4px 0;
		padding: 3px 12px;
		border-radius: var(--radius-sm);
		background: var(--accent-strong);
		color: #fff;
		border-bottom: none;
	}
	.tabs .file:hover:not(:disabled) {
		background: var(--accent);
		color: #fff;
	}
	.tabs .contextual {
		color: var(--accent-text);
	}
	.body {
		display: flex;
		align-items: stretch;
		min-height: 92px;
		padding: 6px 6px 0;
		overflow-x: auto;
	}
	.group {
		display: flex;
		flex-direction: column;
		padding: 0 8px;
		border-right: 1px solid var(--border);
	}
	.items {
		display: flex;
		align-items: flex-start;
		gap: 2px;
		flex: 1;
	}
	.label {
		padding: 4px 0 5px;
		text-align: center;
		font-size: 11px;
		color: var(--text-faint);
	}
	.stack {
		display: grid;
		gap: 1px;
	}
	button.big {
		flex-direction: column;
		gap: 4px;
		min-width: 62px;
		max-width: 86px;
		padding: 6px 6px 5px;
		border-color: transparent;
		background: transparent;
		white-space: normal;
		text-align: center;
		line-height: 1.15;
		font-size: 12px;
	}
	button.big :global(svg) {
		color: var(--accent-text);
	}
	button.small {
		justify-content: flex-start;
		padding: 3px 8px 3px 5px;
		border-color: transparent;
		background: transparent;
		font-size: 12px;
	}
	button.big:hover:not(:disabled),
	button.small:hover:not(:disabled),
	button.pressed {
		background: var(--surface-3);
	}
	button:disabled :global(svg) {
		color: var(--text-faint);
	}
</style>
