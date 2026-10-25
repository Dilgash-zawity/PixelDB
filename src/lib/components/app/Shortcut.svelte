<script lang="ts">
	// Utils
	import { isMac, keyLabels } from '#lib/logic/shortcuts.js';

	// Components
	import { Kbd, KbdGroup } from '#lib/components/ui/kbd/index.js';

	// Script
	/** A key combo like `mod+k`, shown as this platform writes it (⌘K or Ctrl K). */
	let { keys, class: className }: { keys: string; class?: string } = $props();

	const mac = isMac();
	const labels = $derived(keyLabels(keys, mac));
</script>

<KbdGroup class={className}>
	{#each labels as label, i (i)}<Kbd>{label}</Kbd>{/each}
</KbdGroup>
