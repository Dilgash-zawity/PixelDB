<script lang="ts">
	// Types
	import type { Binding } from '#lib/logic/app-def.js';

	// Utils
	import { appSession } from '#lib/state/app.svelte.js';

	// Components
	import PropSelect from './PropSelect.svelte';

	// Script
	let {
		id,
		binding,
		tablesOnly = false,
		onchange
	}: { id?: string; binding: Binding | null; tablesOnly?: boolean; onchange: (b: Binding) => void } = $props();

	const options = $derived(
		(appSession.def?.sources ?? []).flatMap((s) =>
			(appSession.objects[s.alias] ?? [])
				.filter((o) => !tablesOnly || o.kind === 'table')
				.map((o) => ({
					value: JSON.stringify({ source: s.alias, table: o.name }),
					label: o.kind === 'view' ? `${o.name} (query)` : o.name,
					group: s.database
				}))
		)
	);
</script>

<PropSelect
	{id}
	value={binding ? JSON.stringify({ source: binding.source, table: binding.table }) : ''}
	{options}
	placeholder={options.length ? 'Choose a table' : 'Add a database first'}
	onchange={(v) => onchange(JSON.parse(v))}
/>
