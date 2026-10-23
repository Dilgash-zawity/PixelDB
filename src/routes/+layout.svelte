<script lang="ts">
	// Types
	import type { LayoutProps } from './$types';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';

	// Utils
	import '../app.css';
	import { ModeWatcher } from 'mode-watcher';
	import logo from '#lib/assets/logo.svg';
	import { settings } from '#lib/state/settings.svelte.js';
	import { updates } from '#lib/state/updates.svelte.js';

	// Components
	import ConfirmHost from '#lib/components/app/ConfirmHost.svelte';
	import { Toaster } from '#lib/components/ui/sonner/index.js';
	import * as Tooltip from '#lib/components/ui/tooltip/index.js';

	// Script
	let { children }: LayoutProps = $props();
	onMount(() => updates.start());
	let announcedVersion: string | null = null;
	$effect(() => {
		const update = updates.current;
		if (update.status === 'available' && update.version !== announcedVersion) {
			announcedVersion = update.version;
			toast.info(`SlateDB ${update.version} is available`, {
				description: 'Open Settings to install when you’re ready.',
				action: { label: 'View update', onClick: () => { void goto('/settings#updates'); } }
			});
		}
	});

	$effect(() => {
		void settings.current;
		settings.apply();
	});
</script>

<svelte:head>
	<link rel="icon" type="image/svg+xml" href={logo} />
	<title>SlateDB</title>
</svelte:head>

<ModeWatcher defaultMode="dark" />
<Tooltip.Provider>
	{@render children()}
</Tooltip.Provider>
<Toaster richColors position="bottom-right" offset={{ bottom: 72, right: 24 }} />
<ConfirmHost />
