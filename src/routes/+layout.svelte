<script lang="ts">
	// Types
	import type { LayoutProps } from './$types';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { toast } from 'svelte-sonner';

	// Utils
	import '../app.css';
	import { ModeWatcher } from 'mode-watcher';
	import logo from '#lib/assets/logo.svg';
	import { matches, typingTarget } from '#lib/logic/shortcuts.js';
	import { startPinchZoom } from '#lib/services/pinch.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import { logError } from '#lib/state/feedback.svelte.js';
	import { session } from '#lib/state/session.svelte.js';
	import { shortcutsDialog } from '#lib/state/shortcuts.svelte.js';
	import { tour } from '#lib/state/tour.svelte.js';
	import { settings } from '#lib/state/settings.svelte.js';
	import { updates } from '#lib/state/updates.svelte.js';

	// Components
	import ConfirmHost from '#lib/components/app/ConfirmHost.svelte';
	import ShortcutsDialog from '#lib/components/app/ShortcutsDialog.svelte';
	import TourDialog from '#lib/components/app/TourDialog.svelte';
	import { Toaster } from '#lib/components/ui/sonner/index.js';
	import * as Tooltip from '#lib/components/ui/tooltip/index.js';

	// Script
	let { children }: LayoutProps = $props();
	onMount(() => updates.start());
	// Touchpad pinches zoom the canvases.
	onMount(() => startPinchZoom());
	// Errors nothing caught go to the log file, for bug reports.
	onMount(() => {
		const onError = (e: ErrorEvent) => logError('uncaught', e.error ?? e.message);
		const onRejection = (e: PromiseRejectionEvent) => logError('unhandled promise', e.reason);
		window.addEventListener('error', onError);
		window.addEventListener('unhandledrejection', onRejection);
		return () => {
			window.removeEventListener('error', onError);
			window.removeEventListener('unhandledrejection', onRejection);
		};
	});
	let announcedVersion: string | null = null;
	$effect(() => {
		const update = updates.current;
		if (update.status === 'available' && update.version !== announcedVersion) {
			announcedVersion = update.version;
			toast.info(`SlateDB ${update.version} is available`, {
				description: 'Open Settings to install when you’re ready.',
				action: { label: 'View update', onClick: () => { void goto('/settings?section=updates'); } }
			});
		}
	});

	$effect(() => {
		void settings.current;
		settings.apply();
	});


	/** Leaves for Settings the way each screen's own Settings item does, keeping unsaved work. */
	async function openSettings() {
		const route = page.url.pathname;
		if (route.startsWith('/settings')) return;
		if (route.startsWith('/db') && !(await session.close())) return;
		if (route.startsWith('/app') && !(await appSession.close())) return;
		goto(resolve('/settings'));
	}

	function onKey(e: KeyboardEvent) {
		if (e.defaultPrevented) return;
		if (matches(e, 'mod+/') || (matches(e, '?') && !typingTarget(e.target))) {
			e.preventDefault();
			shortcutsDialog.open = !shortcutsDialog.open;
		} else if (matches(e, 'mod+,')) {
			e.preventDefault();
			void openSettings();
		}
	}
</script>

<svelte:window onkeydown={onKey} />

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
<ShortcutsDialog bind:open={shortcutsDialog.open} />
<TourDialog bind:open={tour.open} />
