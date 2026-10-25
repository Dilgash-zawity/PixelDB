<script lang="ts">
	// Utils
	import logo from '#lib/assets/logo.svg';
	import { cn } from '#lib/utils.js';

	// Script
	let { class: className = '' }: { class?: string } = $props();
</script>

<span class={cn('inline-flex size-8 shrink-0 overflow-hidden', className)}>
	<img src={logo} alt="" class="size-full" style="image-rendering: pixelated" draggable="false" />
</span>
