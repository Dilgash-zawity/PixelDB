<script lang="ts">
	// Utils
	import { settings } from '#lib/state/settings.svelte.js';
	import { cn } from '#lib/utils.js';

	// Components
	import * as Field from '#lib/components/ui/field/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';

	// Icons
	import { Focus as FocusIcon } from '@lucide/svelte';

	// Script
	let { class: className }: { class?: string } = $props();
</script>

<!-- The same switch as in Settings, offered on the first screens so people can pick a look up front. -->
<Field.Field orientation="horizontal" class={cn('w-full max-w-sm rounded-lg border bg-card p-3 text-start', className)}>
	<span class="flex size-8 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground vivid:bg-primary/10 vivid:text-primary">
		<FocusIcon class="size-4" />
	</span>
	<Field.Content>
		<Field.Label for="first-run-focus-mode">Focus mode</Field.Label>
		<Field.Description>A plain grey look with square corners. You can change this later in Settings.</Field.Description>
	</Field.Content>
	<Switch id="first-run-focus-mode" checked={settings.current.focusMode} onCheckedChange={(v) => settings.update({ focusMode: v })} />
</Field.Field>
