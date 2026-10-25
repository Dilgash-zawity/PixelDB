<script lang="ts" module>
	/** 12×12 bitmaps, `#` filled; drawn in the current text color. */
	const BITMAPS = {
		app: [
			'............',
			'.##########.',
			'.#........#.',
			'.##########.',
			'.#...#....#.',
			'.#...#....#.',
			'.#...######.',
			'.#...#....#.',
			'.#...#....#.',
			'.##########.',
			'............',
			'............'
		],
		database: [
			'...######...',
			'.##......##.',
			'.#........#.',
			'.##......##.',
			'.#.######.#.',
			'.#........#.',
			'.##......##.',
			'.#.######.#.',
			'.#........#.',
			'.##......##.',
			'...######...',
			'............'
		],
		table: [
			'............',
			'.##########.',
			'.##########.',
			'.#...#....#.',
			'.##########.',
			'.#...#....#.',
			'.##########.',
			'.#...#....#.',
			'.##########.',
			'............',
			'............',
			'............'
		],
		rows: [
			'............',
			'.#..........',
			'#.#.######..',
			'...#........',
			'.#..........',
			'#.#.######..',
			'...#........',
			'.#..........',
			'#.#.######..',
			'...#........',
			'............',
			'............'
		]
	} as const;

	export type PixelIconName = keyof typeof BITMAPS;

	const cells = (name: PixelIconName) =>
		BITMAPS[name].flatMap((row, y) => [...row].flatMap((c, x) => (c === '#' ? [{ x, y }] : [])));
</script>

<script lang="ts">
	// Script
	/** A pixel-art icon like the home map's tiles use. `x`/`y`/`width`/`height` place it inside another SVG. */
	let {
		name,
		...rest
	}: { name: PixelIconName; class?: string; x?: number; y?: number; width?: number; height?: number } = $props();

	const filled = $derived(cells(name));
</script>

<svg viewBox="0 0 12 12" fill="currentColor" shape-rendering="crispEdges" aria-hidden="true" {...rest}>
	{#each filled as c (`${c.x},${c.y}`)}<rect x={c.x} y={c.y} width="1" height="1" />{/each}
</svg>
