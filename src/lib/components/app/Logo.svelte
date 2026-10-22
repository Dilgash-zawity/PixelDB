<script lang="ts">
	// Utils
	import logo from '#lib/assets/logo.svg';
	import { cn } from '#lib/utils.js';

	// Script
	let { class: className = '' }: { class?: string } = $props();
</script>

<!-- The artwork has its own rounded tile; the margin trims its transparent padding. -->
<span class={cn('inline-flex size-8 shrink-0 overflow-hidden', className)}>
	<img src={logo} alt="" class="-m-[10%] size-[120%] max-w-none" draggable="false" />
</span>
