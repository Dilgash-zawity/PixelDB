<script lang="ts">
	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Tooltip from '#lib/components/ui/tooltip/index.js';
	import Shortcut from './Shortcut.svelte';

	// Icons
	import { Search as SearchIcon } from '@lucide/svelte';

	// Script
	/** The top bar's search: opens the screen's ⌘K palette. */
	let { onclick }: { onclick: () => void } = $props();
</script>

<Tooltip.Root>
	<Tooltip.Trigger>
		{#snippet child({ props })}
			<Button {...props} variant="ghost" size="icon-sm" aria-label="Search" aria-keyshortcuts="Control+K Meta+K" {onclick}><SearchIcon /></Button>
		{/snippet}
	</Tooltip.Trigger>
	<Tooltip.Content>Search <Shortcut keys="mod+k" /></Tooltip.Content>
</Tooltip.Root>
