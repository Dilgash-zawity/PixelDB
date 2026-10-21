<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import Datasheet from '../../lib/components/Datasheet.svelte';
	import DesignView from '../../lib/components/DesignView.svelte';
	import Icon from '../../lib/components/Icon.svelte';
	import NavPane from '../../lib/components/NavPane.svelte';
	import Ribbon from '../../lib/components/Ribbon.svelte';
	import { session } from '../../lib/state/session.svelte';
	import { ui } from '../../lib/state/ui.svelte';

	const name = $derived(page.url.searchParams.get('name') ?? '');

	$effect(() => {
		if (name) session.open(name);
	});

	async function backstage() {
		if (await session.close()) goto(resolve('/'));
	}

	function tabMenu(e: MouseEvent, id: string) {
		const doc = session.docs.find((d) => d.id === id);
		if (!doc) return;
		const switchable = doc.name !== null && !session.isView(doc.name);
		ui.openMenu(e, [
			{ label: 'Datasheet View', icon: 'table', disabled: !switchable || doc.kind === 'datasheet', action: () => session.switchView(id, 'datasheet') },
			{ label: 'Design View', icon: 'design', disabled: !switchable || doc.kind === 'design', action: () => session.switchView(id, 'design') },
			{ separator: true },
			{ label: 'Save', icon: 'save', disabled: !session.handlers[id]?.save, action: () => session.handlers[id]?.save?.() },
			{ label: 'Close', icon: 'close', action: () => session.closeDoc(id) }
		]);
	}

	function onKey(e: KeyboardEvent) {
		const mod = e.metaKey || e.ctrlKey;
		if (mod && e.key.toLowerCase() === 's') {
			e.preventDefault();
			session.activeHandlers.save?.();
		} else if (mod && e.key.toLowerCase() === 'w' && session.activeId) {
			e.preventDefault();
			session.closeDoc(session.activeId);
		} else if (mod && e.key.toLowerCase() === 'f') {
			e.preventDefault();
			session.activeHandlers.find?.();
		}
	}
</script>

<svelte:window onkeydown={onKey} />
<svelte:head><title>{name} – Database Builder</title></svelte:head>

<div class="app">
	<div class="titlebar">
		<span class="db"><Icon name="database" size={15} /> {name}</span>
		{#if session.info}<span class="path mono" title={session.info.path}>{session.info.path}</span>{/if}
	</div>

	{#if session.loadError}
		<div class="failed">
			<Icon name="warning" size={32} stroke={1.5} />
			<h2>Could not open “{name}”</h2>
			<p class="muted">{session.loadError}</p>
			<button class="primary" onclick={() => goto(resolve('/'))}><Icon name="back" /> Back to databases</button>
		</div>
	{:else}
		<Ribbon onFile={backstage} />
		<div class="workspace">
			<NavPane />
			<main>
				{#if session.docs.length}
					<div class="doc-tabs" role="tablist">
						{#each session.docs as doc (doc.id)}
							<div class="doc-tab" class:active={doc.id === session.activeId} role="presentation">
								<button
									class="tab-main"
									role="tab"
									aria-selected={doc.id === session.activeId}
									onclick={() => (session.activeId = doc.id)}
									onauxclick={(e) => e.button === 1 && session.closeDoc(doc.id)}
									oncontextmenu={(e) => tabMenu(e, doc.id)}
								>
									<Icon name={doc.kind === 'design' ? 'design' : doc.name && session.isView(doc.name) ? 'view' : 'table'} size={14} />
									<span>{doc.name ?? 'New table'}</span>
									{#if doc.dirty}<span class="dot" title="Unsaved changes"></span>{/if}
								</button>
								<button class="tab-close icon" aria-label="Close {doc.name ?? 'new table'}" onclick={() => session.closeDoc(doc.id)}><Icon name="close" size={12} /></button>
							</div>
						{/each}
					</div>
					<div class="doc-body">
						{#each session.docs as doc (doc.id)}
							<div class="doc" hidden={doc.id !== session.activeId}>
								{#if doc.kind === 'datasheet'}<Datasheet {doc} />{:else}<DesignView {doc} />{/if}
							</div>
						{/each}
					</div>
				{:else}
					<div class="start">
						<Icon name="table" size={36} stroke={1.25} />
						{#if session.tables.length}
							<h2>Open an object</h2>
							<p class="muted">Double-click a table in the navigation pane, or right-click it for more options.</p>
						{:else}
							<h2>Create your first table</h2>
							<p class="muted">Tables hold your data. Start with a blank table and add fields, or design the table first.</p>
						{/if}
						<div class="start-actions">
							<button class="primary" onclick={() => session.createTable()}><Icon name="table" /> New Table</button>
							<button onclick={() => session.openDoc('design', null)}><Icon name="design" /> Table Design</button>
						</div>
					</div>
				{/if}
			</main>
		</div>
	{/if}

	<footer class="status">
		<span>{session.active ? (session.active.kind === 'design' ? 'Design View' : 'Datasheet View') : 'Ready'}</span>
		<span class="spacer"></span>
		{#if session.info}
			<span>{session.info.table_count} tables</span>
			<span class:ok={session.info.foreign_keys}>{session.info.foreign_keys ? 'Relationships enforced' : 'Relationships NOT enforced'}</span>
			<span>SQLite {session.info.sqlite_version}</span>
		{/if}
	</footer>
</div>

<style>
	.app {
		display: flex;
		flex-direction: column;
		height: 100vh;
	}
	.titlebar {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 6px 12px;
		background: var(--accent-strong);
		color: #fff;
		font-size: 12px;
	}
	.db {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		font-weight: 600;
	}
	.path {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		opacity: 0.75;
		font-size: 11px;
	}
	.workspace {
		flex: 1;
		display: flex;
		min-height: 0;
	}
	main {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
	}
	.doc-tabs {
		display: flex;
		gap: 1px;
		padding: 4px 6px 0;
		background: var(--surface);
		border-bottom: 1px solid var(--border);
		overflow-x: auto;
	}
	.doc-tab {
		display: flex;
		align-items: center;
		border: 1px solid transparent;
		border-bottom: none;
		border-radius: var(--radius) var(--radius) 0 0;
		color: var(--text-muted);
	}
	.doc-tab.active {
		background: var(--bg);
		border-color: var(--border);
		color: var(--text);
		box-shadow: inset 0 2px 0 var(--accent);
		margin-bottom: -1px;
	}
	.tab-main {
		border: none;
		background: transparent;
		padding: 6px 4px 6px 10px;
		border-radius: var(--radius) 0 0 0;
	}
	.tab-main:hover:not(:disabled) {
		background: transparent;
		color: var(--text);
	}
	.doc-tab.active .tab-main :global(svg) {
		color: var(--accent-text);
	}
	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--accent-text);
	}
	.tab-close {
		margin-right: 4px;
		padding: 3px;
		opacity: 0.6;
	}
	.tab-close:hover {
		opacity: 1;
	}
	.doc-body {
		flex: 1;
		min-height: 0;
		position: relative;
	}
	.doc {
		position: absolute;
		inset: 0;
	}
	.start,
	.failed {
		margin: auto;
		display: grid;
		justify-items: center;
		gap: 10px;
		max-width: 440px;
		text-align: center;
		color: var(--text-muted);
	}
	.start h2,
	.failed h2 {
		color: var(--text);
		font-size: 17px;
	}
	.start p,
	.failed p {
		margin: 0;
		line-height: 1.5;
	}
	.failed {
		margin-top: 18vh;
		color: var(--danger);
	}
	.start-actions {
		display: flex;
		gap: 8px;
		margin-top: 8px;
	}
	.status {
		display: flex;
		align-items: center;
		gap: 16px;
		padding: 3px 12px;
		background: var(--surface-2);
		border-top: 1px solid var(--border);
		font-size: 11px;
		color: var(--text-muted);
	}
	.status .ok {
		color: var(--success);
	}
	.spacer {
		flex: 1;
	}
</style>
