<script lang="ts">
	import DatabasePanel from '../lib/components/DatabasePanel.svelte';
	import Datasheet from '../lib/components/Datasheet.svelte';
	import ObjectList from '../lib/components/ObjectList.svelte';
	import TableDesigner from '../lib/components/TableDesigner.svelte';
	import { app } from '../lib/state/app.svelte';

	app.run(() => app.refresh());
</script>

<h1>Database Builder</h1>

{#if app.error}
	<p role="alert">Error: {app.error} <button onclick={() => (app.error = null)}>Dismiss</button></p>
{/if}
{#if app.message}
	<p role="status">{app.message}</p>
{/if}
{#if app.busy}<p>Working…</p>{/if}

<DatabasePanel />

{#if app.info}
	<hr />
	<ObjectList />
	<hr />
	{#if app.view.kind === 'design'}
		{#key app.view.name}<TableDesigner name={app.view.name} />{/key}
	{:else if app.view.kind === 'datasheet'}
		{#key app.view.name}<Datasheet name={app.view.name} />{/key}
	{/if}
{/if}
