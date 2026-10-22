<script lang="ts">
	// Types
	import type { ButtonWidget } from '#lib/logic/app-def.js';

	// Utils
	import { appSession } from '#lib/state/app.svelte.js';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';

	// Script
	let { widget, editing }: { widget: ButtonWidget; editing: boolean } = $props();
</script>

<div class="flex h-full items-center">
	<Button
		class="w-full"
		variant={widget.variant}
		disabled={editing}
		onclick={() => appSession.run(widget.action)}
	>
		{widget.label || 'Button'}
	</Button>
</div>
