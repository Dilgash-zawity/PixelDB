<script lang="ts" module>
	// Stroke icons on a 24×24 grid, drawn with currentColor.
	const PATHS = {
		table: 'M3 6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z M3 10h18 M3 15h18 M9 10v10',
		view: 'M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6z',
		design: 'M3 17 17 3l4 4L7 21H3z M14 6l2 2 M11 9l2 2 M8 12l2 2',
		key: 'M7.5 12a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7z M10 13l9-9 M16 7l2 2 M13.5 9.5l2 2',
		plus: 'M12 5v14 M5 12h14',
		trash: 'M4 7h16 M10 11v6 M14 11v6 M6 7l1 13h10l1-13 M9 7V4h6v3',
		save: 'M5 3h11l3 3v13a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z M8 3v5h7V3 M8 21v-7h8v7',
		refresh: 'M20 11a8 8 0 1 0-2.3 5.7 M20 4v7h-7',
		filter: 'M3 5h18l-7 8v6l-4 2v-8z',
		sortAsc: 'M7 4v16 M3 8l4-4 4 4 M14 6h7 M14 12h5 M14 18h3',
		sortDesc: 'M7 4v16 M3 16l4 4 4-4 M14 6h3 M14 12h5 M14 18h7',
		clearSort: 'M7 4v16 M3 8l4-4 4 4 M15 9l6 6 M21 9l-6 6',
		undo: 'M9 14 4 9l5-5 M4 9h10a6 6 0 0 1 0 12h-3',
		close: 'M6 6l12 12 M18 6 6 18',
		folder: 'M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z',
		database: 'M4 5c0-1.7 3.6-3 8-3s8 1.3 8 3-3.6 3-8 3-8-1.3-8-3z M4 5v14c0 1.7 3.6 3 8 3s8-1.3 8-3V5 M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3',
		search: 'M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14z M20 20l-4-4',
		first: 'M17 18l-6-6 6-6 M7 6v12',
		prev: 'M15 18l-6-6 6-6',
		next: 'M9 6l6 6-6 6',
		last: 'M7 6l6 6-6 6 M17 6v12',
		newRecord: 'M5 6l6 6-6 6 M17 8v8 M13 12h8',
		insertRow: 'M3 6h18 M3 12h9 M3 18h18 M18 9v6 M15 12h6',
		deleteRow: 'M3 6h18 M3 12h9 M3 18h18 M15 9.5l5 5 M20 9.5l-5 5',
		sheet: 'M6 3h12a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z M8 8h8 M8 12h8 M8 16h5',
		back: 'M19 12H5 M12 19l-7-7 7-7',
		chevronDown: 'M6 9l6 6 6-6',
		chevronRight: 'M9 6l6 6-6 6',
		paperclip: 'M21 11l-8.5 8.5a5 5 0 0 1-7-7L14 4a3.5 3.5 0 0 1 5 5l-8.5 8.5a2 2 0 0 1-3-3L15 7',
		pencil: 'M4 20h4L19 9l-4-4L4 16z M13.5 6.5l4 4',
		warning: 'M12 3l10 18H2z M12 10v4 M12 17.5v.01',
		check: 'M5 12l5 5 9-10',
		info: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z M12 11v5 M12 7.5v.01',
		panel: 'M5 4h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z M9 4v16',
		home: 'M3 11l9-7 9 7 M5 10v10h14V10',
		file: 'M6 3h8l5 5v11a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z M14 3v5h5'
	} as const;
	export type IconName = keyof typeof PATHS;
</script>

<script lang="ts">
	let { name, size = 16, stroke = 1.75 }: { name: IconName; size?: number; stroke?: number } = $props();
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width={stroke}
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
><path d={PATHS[name]} /></svg>

<style>
	svg {
		flex: none;
		display: block;
	}
</style>
