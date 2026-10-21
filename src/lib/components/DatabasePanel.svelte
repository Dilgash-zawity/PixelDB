<script lang="ts">
	import { app } from '../state/app.svelte';

	let manualPath = $state('');
</script>

<section>
	<h2>Database</h2>
	<p>
		<button disabled={app.busy} onclick={() => app.createDatabase()}>New database…</button>
		<button disabled={app.busy} onclick={() => app.openDatabase()}>Open database…</button>
		<button disabled={app.busy || !app.info} onclick={() => app.closeDatabase()}>Close database</button>
	</p>
	<p>
		<label>Or enter an absolute path: <input bind:value={manualPath} size="60" placeholder="/path/to/file.db" /></label>
		<button disabled={app.busy || !manualPath.trim()} onclick={() => app.createDatabase(manualPath)}>Create</button>
		<button disabled={app.busy || !manualPath.trim()} onclick={() => app.openDatabase(manualPath)}>Open</button>
	</p>
	{#if app.info}
		<table>
			<tbody>
				<tr><th>File</th><td>{app.info.path}</td></tr>
				<tr><th>Size</th><td>{app.info.file_size} bytes ({app.info.page_count} pages of {app.info.page_size})</td></tr>
				<tr><th>SQLite</th><td>{app.info.sqlite_version}, {app.info.encoding}, journal {app.info.journal_mode}</td></tr>
				<tr><th>Foreign keys</th><td>{app.info.foreign_keys ? 'enforced' : 'NOT enforced'}</td></tr>
				<tr><th>Objects</th><td>{app.info.table_count} tables, {app.info.view_count} views</td></tr>
			</tbody>
		</table>
	{:else}
		<p>No database is open.</p>
	{/if}
</section>
