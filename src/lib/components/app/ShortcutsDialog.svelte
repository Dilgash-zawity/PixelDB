<script lang="ts">
	// Utils
	import { page } from '$app/state';
	import { SCOPE_LABELS, SHORTCUTS, scopesFor } from '#lib/logic/shortcuts.js';
	import { appSession } from '#lib/state/app.svelte.js';

	// Components
	import * as Dialog from '#lib/components/ui/dialog/index.js';
	import Shortcut from './Shortcut.svelte';

	// Script
	/** The keyboard shortcuts of the current screen, then the ones that work everywhere. */
	let { open = $bindable(false) }: { open?: boolean } = $props();

	const sections = $derived(
		scopesFor(page.url.pathname, appSession.mode === 'edit')
			.map((scope) => ({ scope, items: SHORTCUTS.filter((s) => s.scope === scope) }))
			.filter((s) => s.items.length)
	);
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="flex max-h-[85dvh] flex-col sm:max-w-xl">
		<Dialog.Header>
			<Dialog.Title>Keyboard shortcuts</Dialog.Title>
			<Dialog.Description>Everything can be reached with Tab and Enter; these make it quicker.</Dialog.Description>
		</Dialog.Header>
		<div class="-mx-6 flex min-h-0 flex-col gap-5 overflow-y-auto px-6 pb-1">
			{#each sections as s (s.scope)}
				<section class="flex flex-col gap-1" aria-labelledby="shortcuts-{s.scope}">
					<h3 id="shortcuts-{s.scope}" class="text-xs font-medium text-muted-foreground">{SCOPE_LABELS[s.scope]}</h3>
					<dl class="flex flex-col">
						{#each s.items as item (item.keys)}
							<div class="flex items-center justify-between gap-4 border-b py-1.5 text-sm last:border-b-0">
								<dt>{item.label}</dt>
								<dd><Shortcut keys={item.keys} /></dd>
							</div>
						{/each}
					</dl>
				</section>
			{/each}
		</div>
	</Dialog.Content>
</Dialog.Root>
