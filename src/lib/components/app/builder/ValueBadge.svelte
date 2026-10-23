<script lang="ts">
	// Utils
	import { toneFor, type Tone } from '#lib/logic/board.js';
	import { cn } from '#lib/utils.js';

	// Components
	import { Badge } from '#lib/components/ui/badge/index.js';

	// Script
	/** A value shown as a colored label; the same text always gets the same color (grey in Focus mode). */
	let { text, class: className }: { text: string; class?: string } = $props();

	const TONE_CLASSES: Record<Tone, string> = {
		blue: 'vivid:bg-tone-blue/15 vivid:text-tone-blue',
		violet: 'vivid:bg-tone-violet/15 vivid:text-tone-violet',
		cyan: 'vivid:bg-tone-cyan/15 vivid:text-tone-cyan',
		green: 'vivid:bg-tone-green/15 vivid:text-tone-green',
		amber: 'vivid:bg-tone-amber/15 vivid:text-tone-amber',
		pink: 'vivid:bg-tone-pink/15 vivid:text-tone-pink'
	};
</script>

{#if text}
	<Badge variant="secondary" class={cn('max-w-full bg-muted text-muted-foreground', TONE_CLASSES[toneFor(text)], className)} title={text}>
		<span class="truncate">{text}</span>
	</Badge>
{/if}
