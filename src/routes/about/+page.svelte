<script lang="ts">
	import { getVersion } from '@tauri-apps/api/app';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
	import Logo from '#lib/components/app/Logo.svelte';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { attempt } from '#lib/state/feedback.svelte.js';

	const WEBSITE = 'https://dilgash.com';

	let version = $state<string | null>(null);
	// Read from the running binary, so it always matches the installed build.
	getVersion().then((v) => (version = v), () => {});
</script>

<svelte:head><title>About · SlateDB</title></svelte:head>

<div class="flex min-h-svh flex-col">
	<header class="flex h-14 items-center border-b bg-sidebar px-6">
		<Button variant="ghost" size="sm" onclick={() => history.back()}>
			<ArrowLeftIcon data-icon="inline-start" /> Back
		</Button>
	</header>

	<main class="flex flex-1 flex-col items-center justify-center gap-3 px-6 text-center">
		<Logo class="size-20" />
		<h1 class="text-3xl font-semibold tracking-tight">SlateDB</h1>
		<Badge variant="secondary">Version {version ?? '—'}</Badge>
		<p class="max-w-md text-muted-foreground">
			An offline desktop database app in the spirit of Microsoft Access. Your data stays on this computer.
		</p>
		<p class="text-sm text-muted-foreground">
			Made by
			<Button variant="link" class="h-auto p-0 cursor-pointer" onclick={() => attempt(() => openUrl(WEBSITE))}>Dilgash</Button>
		</p>
	</main>
</div>
