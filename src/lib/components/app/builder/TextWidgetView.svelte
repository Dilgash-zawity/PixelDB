<script lang="ts">
	import type { TextWidget } from '#lib/logic/app-def.js';
	import { parseMarkdown, type Span } from '#lib/logic/markdown.js';
	import { Separator } from '#lib/components/ui/separator/index.js';

	let { widget }: { widget: TextWidget } = $props();

	const blocks = $derived(parseMarkdown(widget.markdown));
</script>

{#snippet spans(list: Span[])}
	{#each list as s, i (i)}
		{#if s.code}<code class="bg-muted px-1 font-mono text-[0.9em]">{s.text}</code>
		{:else if s.bold}<strong>{s.text}</strong>
		{:else if s.italic}<em>{s.text}</em>
		{:else}{s.text}{/if}
	{/each}
{/snippet}

<div class="flex h-full flex-col gap-2 overflow-auto text-sm">
	{#each blocks as b, i (i)}
		{#if b.kind === 'heading'}
			{#if b.level === 1}<h1 class="text-2xl font-semibold tracking-tight">{@render spans(b.spans)}</h1>
			{:else if b.level === 2}<h2 class="text-xl font-semibold tracking-tight">{@render spans(b.spans)}</h2>
			{:else}<h3 class="text-base font-semibold">{@render spans(b.spans)}</h3>{/if}
		{:else if b.kind === 'paragraph'}
			<p class="leading-relaxed text-muted-foreground">{@render spans(b.spans)}</p>
		{:else if b.kind === 'list'}
			{#if b.ordered}
				<ol class="ms-5 list-decimal text-muted-foreground">{#each b.items as item, j (j)}<li>{@render spans(item)}</li>{/each}</ol>
			{:else}
				<ul class="ms-5 list-disc text-muted-foreground">{#each b.items as item, j (j)}<li>{@render spans(item)}</li>{/each}</ul>
			{/if}
		{:else if b.kind === 'quote'}
			<blockquote class="border-s-2 ps-3 text-muted-foreground italic">{@render spans(b.spans)}</blockquote>
		{:else}
			<Separator />
		{/if}
	{/each}
</div>
