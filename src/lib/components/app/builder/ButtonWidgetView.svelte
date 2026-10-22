<script lang="ts">
	import type { ButtonWidget } from '#lib/logic/app-def.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { appSession } from '#lib/state/app.svelte.js';

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
