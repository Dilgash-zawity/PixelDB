<script lang="ts">
	import { app } from '../state/app.svelte';

	let renaming = $state<string | null>(null);
	let newName = $state('');

	function startRename(name: string) {
		renaming = name;
		newName = name;
	}

	async function finishRename() {
		if (renaming && newName.trim() && newName !== renaming) await app.renameTable(renaming, newName.trim());
		renaming = null;
	}
</script>

<section>
	<h2>Tables and views</h2>
	<p><button disabled={app.busy} onclick={() => (app.view = { kind: 'design', name: null })}>New table…</button></p>
	{#if app.objects.length === 0}
		<p>This database has no tables yet.</p>
	{:else}
		<table>
			<thead>
				<tr><th>Name</th><th>Type</th><th>Actions</th></tr>
			</thead>
			<tbody>
				{#each app.objects as obj (obj.kind + obj.name)}
					<tr>
						<td>
							{#if renaming === obj.name}
								<input bind:value={newName} onkeydown={(e) => e.key === 'Enter' && finishRename()} />
								<button onclick={finishRename}>OK</button>
								<button onclick={() => (renaming = null)}>Cancel</button>
							{:else}
								{obj.name}
							{/if}
						</td>
						<td>{obj.kind}{obj.kind === 'table' && !obj.managed ? ' (external schema)' : ''}</td>
						<td>
							<button onclick={() => (app.view = { kind: 'datasheet', name: obj.name })}>Open</button>
							{#if obj.kind === 'table'}
								<button onclick={() => (app.view = { kind: 'design', name: obj.name })}>Design</button>
								<button disabled={app.busy} onclick={() => startRename(obj.name)}>Rename</button>
								<button disabled={app.busy} onclick={() => app.dropTable(obj.name)}>Delete</button>
							{/if}
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
</section>
