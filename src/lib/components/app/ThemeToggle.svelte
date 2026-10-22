<script lang="ts">
	// Utils
	import { resetMode, setMode } from 'mode-watcher';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';

	// Icons
	import { Sun as SunIcon, Moon as MoonIcon, Monitor as MonitorIcon } from '@lucide/svelte';

	// Script
	let { align = 'end' }: { align?: 'start' | 'end' } = $props();
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			<Button {...props} variant="ghost" size="icon" aria-label="Change theme">
				<SunIcon class="scale-100 rotate-0 transition-all dark:scale-0 dark:-rotate-90" />
				<MoonIcon class="absolute scale-0 rotate-90 transition-all dark:scale-100 dark:rotate-0" />
			</Button>
		{/snippet}
	</DropdownMenu.Trigger>
	<DropdownMenu.Content {align}>
		<DropdownMenu.Group>
			<DropdownMenu.Item onclick={() => setMode('light')}><SunIcon /> Light</DropdownMenu.Item>
			<DropdownMenu.Item onclick={() => setMode('dark')}><MoonIcon /> Dark</DropdownMenu.Item>
			<DropdownMenu.Item onclick={() => resetMode()}><MonitorIcon /> System</DropdownMenu.Item>
		</DropdownMenu.Group>
	</DropdownMenu.Content>
</DropdownMenu.Root>
