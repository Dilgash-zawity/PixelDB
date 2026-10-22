<script lang="ts">
	import '../app.css';
	import { ModeWatcher } from 'mode-watcher';
	import logo from '#lib/assets/logo.svg';
	import ConfirmHost from '#lib/components/app/ConfirmHost.svelte';
	import { Toaster } from '#lib/components/ui/sonner/index.js';
	import * as Tooltip from '#lib/components/ui/tooltip/index.js';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();
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
