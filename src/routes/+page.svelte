<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import Icon from '../lib/components/Icon.svelte';
	import { databaseHref } from '../lib/logic/routes';
	import { databaseNameProblem, formatBytes } from '../lib/logic/values';
	import { databaseService } from '../lib/services/database';
	import { ui } from '../lib/state/ui.svelte';
	import { workspace } from '../lib/state/workspace.svelte';

	let filter = $state('');
	const visible = $derived(
		workspace.databases.filter((d) => d.name.toLowerCase().includes(filter.trim().toLowerCase()))
	);

	// Returning to this screen closes whichever database was open.
	ui.attempt(() => databaseService.close());
	workspace.load();

	function open(name: string) {
		goto(databaseHref(resolve('/db'), name));
	}

	async function createBlank() {
		const name = await ui.prompt({
			title: 'Blank database',
			label: 'File name',
			value: nextName(),
			confirmLabel: 'Create',
			validate: (v) =>
				databaseNameProblem(v) ??
				(workspace.databases.some((d) => d.name.toLowerCase() === v.toLowerCase())
					? 'A database with this name already exists'
					: null)
		});
		if (!name) return;
		if (await workspace.create(name)) open(name);
	}

	function nextName(): string {
		let n = 1;
		while (workspace.databases.some((d) => d.name.toLowerCase() === `database${n}`)) n++;
		return `Database${n}`;
	}

	function formatDate(ms: number | null): string {
		return ms === null ? '' : new Date(ms).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
	}
</script>

<div class="backstage">
	<aside class="rail">
		<div class="brand">
			<span class="logo"><Icon name="database" size={22} /></span>
			<span>Database Builder</span>
		</div>
		<nav>
			<button class="rail-item active"><Icon name="home" /> Home</button>
			<button class="rail-item" disabled={!workspace.info} onclick={createBlank}><Icon name="plus" /> New</button>
			<button class="rail-item" onclick={() => workspace.chooseFolder()}><Icon name="folder" /> Change folder</button>
		</nav>
		<p class="rail-foot">Offline · SQLite</p>
	</aside>

	<main>
		{#if !workspace.loaded}
			<p class="muted">Loading…</p>
		{:else if !workspace.info}
			<section class="welcome">
				<span class="hero-icon"><Icon name="folder" size={40} stroke={1.5} /></span>
				<h1>Welcome to Database Builder</h1>
				<p class="muted">
					Choose a folder where your databases will be stored. Each database is a single SQLite file in that
					folder, and everything stays on this computer.
				</p>
				<button class="primary big" onclick={() => workspace.chooseFolder()}>
					<Icon name="folder" /> Choose folder…
				</button>
			</section>
		{:else}
			<header class="top">
				<h1>Databases</h1>
				<div class="folder">
					<Icon name="folder" />
					<span class="path mono" title={workspace.info.path}>{workspace.info.path}</span>
					<button class="ghost" onclick={() => workspace.chooseFolder()}>Change…</button>
					<button class="icon" title="Refresh" aria-label="Refresh" onclick={() => workspace.refresh()}><Icon name="refresh" /></button>
				</div>
			</header>

			<section>
				<h2>New</h2>
				<div class="tiles">
					<button class="tile" onclick={createBlank}>
						<span class="tile-art"><Icon name="plus" size={30} stroke={1.5} /></span>
						<span>Blank database</span>
					</button>
				</div>
			</section>

			<section class="list-section">
				<div class="list-head">
					<h2>Your databases <span class="count">{workspace.databases.length}</span></h2>
					{#if workspace.databases.length > 0}
						<label class="search">
							<Icon name="search" size={14} />
							<input placeholder="Search" bind:value={filter} />
						</label>
					{/if}
				</div>
				{#if workspace.databases.length === 0}
					<div class="empty">
						<Icon name="database" size={28} stroke={1.5} />
						<p>No databases in this folder yet. Create a blank database to get started.</p>
					</div>
				{:else}
					<div class="list">
						<div class="row head">
							<span>Name</span><span>Modified</span><span>Size</span>
						</div>
						{#each visible as db (db.file_name)}
							<button class="row" disabled={!db.valid} onclick={() => open(db.name)} title={db.valid ? `Open ${db.file_name}` : 'This file is not a SQLite database'}>
								<span class="name">
									<span class="db-icon"><Icon name="database" /></span>
									<span>
										<strong>{db.name}</strong>
										<small class="muted">{db.valid ? db.file_name : 'Not a SQLite database'}</small>
									</span>
								</span>
								<span class="muted">{formatDate(db.modified)}</span>
								<span class="muted">{formatBytes(db.size)}</span>
							</button>
						{:else}
							<p class="muted pad">No databases match “{filter}”.</p>
						{/each}
					</div>
				{/if}
			</section>
		{/if}
	</main>
</div>

<style>
	.backstage {
		display: grid;
		grid-template-columns: 220px 1fr;
		height: 100vh;
	}
	.rail {
		display: flex;
		flex-direction: column;
		background: linear-gradient(180deg, var(--accent-strong), #7c2a2d);
		color: #fff;
		padding: 18px 10px;
	}
	.brand {
		display: flex;
		align-items: center;
		gap: 10px;
		font-weight: 600;
		font-size: 15px;
		padding: 0 8px 22px;
	}
	.logo {
		display: grid;
		place-items: center;
		width: 34px;
		height: 34px;
		border-radius: 8px;
		background: rgba(255, 255, 255, 0.16);
	}
	nav {
		display: grid;
		gap: 2px;
	}
	.rail-item {
		justify-content: flex-start;
		gap: 10px;
		padding: 9px 10px;
		border: none;
		background: transparent;
		color: rgba(255, 255, 255, 0.9);
	}
	.rail-item:hover:not(:disabled),
	.rail-item.active {
		background: rgba(255, 255, 255, 0.14);
	}
	.rail-foot {
		margin-top: auto;
		padding: 0 10px;
		font-size: 11px;
		color: rgba(255, 255, 255, 0.6);
	}
	main {
		overflow: auto;
		padding: 36px 44px;
	}
	.welcome {
		max-width: 520px;
		margin: 12vh auto 0;
		text-align: center;
		display: grid;
		justify-items: center;
		gap: 14px;
	}
	.welcome h1 {
		font-size: 24px;
	}
	.welcome p {
		margin: 0;
		line-height: 1.6;
	}
	.hero-icon {
		display: grid;
		place-items: center;
		width: 76px;
		height: 76px;
		border-radius: 20px;
		background: var(--accent-soft);
		color: var(--accent-text);
	}
	.big {
		padding: 9px 18px;
		margin-top: 6px;
	}
	.top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		margin-bottom: 28px;
		flex-wrap: wrap;
	}
	.top h1 {
		font-size: 22px;
	}
	.folder {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
		padding: 4px 4px 4px 12px;
		border: 1px solid var(--border);
		border-radius: 999px;
		background: var(--surface);
		color: var(--text-muted);
	}
	.path {
		max-width: 420px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 12px;
	}
	section h2 {
		font-size: 13px;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
		margin-bottom: 12px;
	}
	.tiles {
		display: flex;
		gap: 14px;
		margin-bottom: 34px;
	}
	.tile {
		flex-direction: column;
		gap: 10px;
		width: 150px;
		padding: 12px;
		background: var(--surface);
		border-color: var(--border);
		border-radius: 10px;
	}
	.tile:hover:not(:disabled) {
		border-color: var(--accent);
		background: var(--surface-2);
	}
	.tile-art {
		display: grid;
		place-items: center;
		width: 100%;
		height: 90px;
		border-radius: 6px;
		background: var(--surface-3);
		color: var(--accent-text);
	}
	.list-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 12px;
	}
	.list-head h2 {
		margin: 0;
	}
	.count {
		margin-left: 6px;
		padding: 1px 7px;
		border-radius: 999px;
		background: var(--surface-3);
		font-size: 11px;
	}
	.search {
		display: flex;
		align-items: center;
		gap: 6px;
		padding-left: 8px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-sm);
		background: var(--surface);
		color: var(--text-muted);
	}
	.search input {
		border: none;
		background: transparent;
		box-shadow: none;
		width: 200px;
	}
	.list {
		border: 1px solid var(--border);
		border-radius: 10px;
		overflow: hidden;
		background: var(--surface);
	}
	.row {
		display: grid;
		grid-template-columns: 1fr 200px 90px;
		align-items: center;
		width: 100%;
		padding: 10px 16px;
		border: none;
		border-top: 1px solid var(--border);
		border-radius: 0;
		background: transparent;
		text-align: left;
	}
	.row.head {
		border-top: none;
		background: var(--surface-2);
		color: var(--text-muted);
		font-size: 12px;
		padding: 7px 16px;
	}
	.row:not(.head):hover:not(:disabled) {
		background: var(--accent-softer);
	}
	.name {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
	}
	.name > span:last-child {
		display: grid;
		min-width: 0;
	}
	.name strong,
	.name small {
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.db-icon {
		display: grid;
		place-items: center;
		width: 32px;
		height: 32px;
		border-radius: 8px;
		background: var(--accent-soft);
		color: var(--accent-text);
	}
	.empty {
		display: grid;
		justify-items: center;
		gap: 8px;
		padding: 40px;
		border: 1px dashed var(--border-strong);
		border-radius: 10px;
		color: var(--text-muted);
	}
	.empty p {
		margin: 0;
	}
	.pad {
		padding: 14px 16px;
		margin: 0;
	}
</style>
