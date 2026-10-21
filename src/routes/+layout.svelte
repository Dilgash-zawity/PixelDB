<script lang="ts">
	import '../app.css';
	import favicon from '#lib/assets/favicon.svg';
	import Overlays from '../lib/components/Overlays.svelte';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<title>Database Builder</title>
</svelte:head>

{@render children()}
<Overlays />
